//! OQ-DATA-SPIKE-3 — SQLite single-writer event-store load test (MVP task 0.4).
//!
//! THROWAWAY measurement scaffolding. This is *not* the real event store; it
//! models the single-writer commit path closely enough to quantify the §18
//! "event write latency (intent→committed) p95 < 100ms at N=20 concurrent
//! agents" budget and find the contention ceiling.
//!
//! Design mirrors DATA_MODEL.md §2.1: ONE long-lived write connection owns all
//! writes (a serialized write-actor fed by an mpsc channel); readers use
//! separate read-only WAL connections. N "agents" are closed-loop: each submits
//! an intent and blocks until the commit is durable before submitting the next
//! — so the measured latency is submit→durable-commit (queue wait + commit),
//! exactly the intent-commit metric §18 names.
//!
//! Each commit is ONE transaction writing what the real event-commit txn writes
//! (§7.1 + §7.2 "one event-commit transaction updates multiple projections"):
//!   events row (full ~25-col envelope, all 6 indexes incl. unique idempotency)
//!   + 2 object_refs rows (FK → events) + 1 FTS5 row + 2 projection upserts.
//!
//! Run:  cargo run --release            # full sweep + sync=FULL comparison
//!       cargo run --release -- --agents 20 --commits 1000 --sync normal

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use rusqlite::{Connection, OpenFlags};

// ---- representative payload (kept off the writer's critical path) -----------

/// ~400-byte JSON blob, the rough size of a real mutation event payload. Content
/// is irrelevant to write cost (only size + the column/index work matter); we do
/// NOT compute a real sha256 here (the daemon will) — that is a few µs of CPU,
/// negligible beside the commit, and noted as such in the writeup.
fn make_payload(seq: i64) -> String {
    format!(
        "{{\"action_type\":\"git.create_worktree\",\"seq\":{seq},\
\"path\":\"/Users/dev/projects/nexusops/worktrees/feature-{seq}\",\
\"branch\":\"feature/agent-{seq}\",\"base\":\"main\",\
\"risk\":2,\"approval\":\"standing_grant\",\
\"detail\":\"representative event payload padded to approximate the typical \
serialized mutation event size carried on the single-writer commit path so the \
INSERT + index maintenance cost is realistic rather than trivially small xxxxxx\"}}"
    )
}

// ---- one unit of work handed to the serialized write-actor ------------------

struct Job {
    payload_json: String,
    idem_key: String,
    obj_a: String,
    obj_b: String,
    project_id: String,
    session_id: String,
    correlation_id: String,
    /// writer sends () the instant the commit returns durable
    ack: SyncSender<()>,
}

const PRAGMA_HEADER: &str = "\
PRAGMA journal_mode=WAL;\
PRAGMA foreign_keys=ON;\
PRAGMA busy_timeout=5000;";

fn open_writer(path: &str, sync_full: bool) -> Connection {
    let conn = Connection::open(path).expect("open writer");
    conn.execute_batch(PRAGMA_HEADER).expect("writer pragmas");
    // synchronous: NORMAL is the LOCKED value (ADR-003); FULL run is for the
    // durability-cost comparison only.
    conn.execute_batch(if sync_full {
        "PRAGMA synchronous=FULL;"
    } else {
        "PRAGMA synchronous=NORMAL;"
    })
    .expect("synchronous");
    conn
}

fn open_reader(path: &str) -> Connection {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .expect("open reader");
    conn.execute_batch("PRAGMA query_only=ON; PRAGMA busy_timeout=5000;")
        .expect("reader pragmas");
    conn
}

fn create_schema(conn: &Connection) {
    conn.execute_batch(
        "\
CREATE TABLE events (
  event_id            TEXT PRIMARY KEY,
  seq                 INTEGER NOT NULL,
  event_type          TEXT NOT NULL,
  event_version       INTEGER NOT NULL,
  occurred_at         TEXT NOT NULL,
  recorded_at         TEXT NOT NULL,
  workspace_id        TEXT NOT NULL,
  project_id          TEXT,
  actor_type          TEXT NOT NULL,
  actor_id            TEXT NOT NULL,
  source_type         TEXT NOT NULL,
  source_id           TEXT NOT NULL,
  correlation_id      TEXT NOT NULL,
  causation_id        TEXT,
  action_request_id   TEXT,
  approval_id         TEXT,
  session_id          TEXT,
  agent_team_id       TEXT,
  workflow_run_id     TEXT,
  idempotency_key     TEXT,
  sensitivity         TEXT NOT NULL,
  visibility          TEXT NOT NULL DEFAULT 'project',
  payload_json        TEXT NOT NULL,
  payload_hash        TEXT,
  previous_event_hash TEXT,
  schema_version      TEXT,
  app_version         TEXT
);
CREATE UNIQUE INDEX ux_events_seq         ON events(seq);
CREATE INDEX        ix_events_project_seq ON events(project_id, seq);
CREATE INDEX        ix_events_correlation ON events(correlation_id, seq);
CREATE INDEX        ix_events_type_seq    ON events(event_type, seq);
CREATE INDEX        ix_events_session     ON events(session_id, seq);
CREATE UNIQUE INDEX ux_events_idempotency ON events(idempotency_key) WHERE idempotency_key IS NOT NULL;

CREATE TABLE object_refs (
  event_id    TEXT NOT NULL REFERENCES events(event_id),
  object_type TEXT NOT NULL,
  object_id   TEXT NOT NULL,
  PRIMARY KEY (event_id, object_type, object_id)
);
CREATE INDEX ix_object_refs_obj ON object_refs(object_type, object_id);

-- redaction-safe audit text index (§7.1 / 1.1 FTS5)
CREATE VIRTUAL TABLE fts_events USING fts5(event_id UNINDEXED, body);

-- two representative projections updated in the same commit txn (§7.2)
CREATE TABLE proj_project_activity (
  project_id  TEXT PRIMARY KEY,
  last_seq    INTEGER NOT NULL,
  event_count INTEGER NOT NULL
);
CREATE TABLE proj_session (
  session_id  TEXT PRIMARY KEY,
  last_seq    INTEGER NOT NULL,
  event_count INTEGER NOT NULL
);
",
    )
    .expect("create schema");
}

/// Bulk-load `count` representative events (+object_refs +fts +proj) before the
/// timed run, so the index B-trees + FTS index are at production scale. Batched
/// into large transactions purely for load speed — NOT a latency measurement.
fn preseed(path: &str, count: usize) {
    let conn = open_writer(path, false);
    let batch = 50_000usize;
    let mut seq: i64 = 0;
    let mut done = 0usize;
    while done < count {
        let this = batch.min(count - done);
        conn.execute_batch("BEGIN IMMEDIATE;")
            .expect("preseed begin");
        for _ in 0..this {
            seq += 1;
            let event_id = format!("evt_{seq:026}");
            let project_id = format!("proj_{:03}", seq % 4);
            let session_id = format!("sess_{:03}", seq % 64);
            conn.execute(
                "INSERT INTO events (event_id,seq,event_type,event_version,occurred_at,\
recorded_at,workspace_id,project_id,actor_type,actor_id,source_type,source_id,\
correlation_id,causation_id,action_request_id,approval_id,session_id,agent_team_id,\
workflow_run_id,idempotency_key,sensitivity,visibility,payload_json,payload_hash,\
previous_event_hash,schema_version,app_version) VALUES \
(?1,?2,'ActionExecuted',1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z',\
'ws_00000000000000000000000001',?3,'agent','agent_x','harness','src_1',?4,NULL,\
NULL,NULL,?5,NULL,NULL,?6,'internal','project',?7,'hash',NULL,'event-envelope-v1','0.0.0')",
                rusqlite::params![
                    event_id,
                    seq,
                    project_id,
                    format!("corr_{:05}", seq % 5000),
                    session_id,
                    format!("seed_idem_{seq:020}"),
                    make_payload(seq),
                ],
            )
            .expect("preseed event");
            conn.execute(
                "INSERT INTO object_refs (event_id,object_type,object_id) VALUES (?1,'worktree',?2)",
                rusqlite::params![event_id, format!("wt_{seq:022}")],
            ).expect("preseed oref");
            conn.execute(
                "INSERT INTO fts_events (event_id,body) VALUES (?1,'ActionExecuted git.create_worktree worktree branch feature')",
                rusqlite::params![event_id],
            ).expect("preseed fts");
        }
        conn.execute_batch("COMMIT;").expect("preseed commit");
        done += this;
    }
}

fn writer_loop(path: String, sync_full: bool, start_seq: i64, rx: Receiver<Job>) {
    let conn = open_writer(&path, sync_full);
    let mut seq: i64 = start_seq;
    // Closed-loop over the channel: ends when every agent (Sender) has dropped.
    for job in rx {
        seq += 1;
        let event_id = format!("evt_{seq:026}");
        let now = "2026-06-07T18:00:00.000Z"; // fixed; timestamp formatting is off-path

        conn.execute_batch("BEGIN IMMEDIATE;").expect("begin");

        {
            let mut s = conn
                .prepare_cached(
                    "INSERT INTO events (event_id,seq,event_type,event_version,occurred_at,\
recorded_at,workspace_id,project_id,actor_type,actor_id,source_type,source_id,\
correlation_id,causation_id,action_request_id,approval_id,session_id,agent_team_id,\
workflow_run_id,idempotency_key,sensitivity,visibility,payload_json,payload_hash,\
previous_event_hash,schema_version,app_version) VALUES \
(?1,?2,'ActionExecuted',1,?3,?3,'ws_00000000000000000000000001',?4,'agent',?5,\
'harness','src_1',?6,NULL,?7,?8,?9,NULL,NULL,?10,'internal','project',?11,?12,\
NULL,'event-envelope-v1','0.0.0')",
                )
                .expect("prep events");
            s.execute(rusqlite::params![
                event_id,
                seq,
                now,
                job.project_id,
                format!("agent_{}", seq % 20),
                job.correlation_id,
                format!("areq_{seq:022}"),
                format!("appr_{seq:022}"),
                job.session_id,
                job.idem_key,
                job.payload_json,
                format!("{:016x}", seq.wrapping_mul(0x9E3779B97F4A7C15u64 as i64)),
            ])
            .expect("insert event");
        }
        {
            let mut s = conn
                .prepare_cached(
                    "INSERT INTO object_refs (event_id,object_type,object_id) VALUES (?1,?2,?3)",
                )
                .expect("prep oref");
            s.execute(rusqlite::params![event_id, "worktree", job.obj_a])
                .expect("oref a");
            s.execute(rusqlite::params![event_id, "branch", job.obj_b])
                .expect("oref b");
        }
        {
            let mut s = conn
                .prepare_cached("INSERT INTO fts_events (event_id,body) VALUES (?1,?2)")
                .expect("prep fts");
            s.execute(rusqlite::params![
                event_id,
                "ActionExecuted git.create_worktree feature branch worktree created"
            ])
            .expect("fts");
        }
        {
            let mut s = conn
                .prepare_cached(
                    "INSERT INTO proj_project_activity (project_id,last_seq,event_count) \
VALUES (?1,?2,1) ON CONFLICT(project_id) DO UPDATE SET last_seq=?2, event_count=event_count+1",
                )
                .expect("prep proj1");
            s.execute(rusqlite::params![job.project_id, seq])
                .expect("proj1");
        }
        {
            let mut s = conn
                .prepare_cached(
                    "INSERT INTO proj_session (session_id,last_seq,event_count) \
VALUES (?1,?2,1) ON CONFLICT(session_id) DO UPDATE SET last_seq=?2, event_count=event_count+1",
                )
                .expect("prep proj2");
            s.execute(rusqlite::params![job.session_id, seq])
                .expect("proj2");
        }

        conn.execute_batch("COMMIT;").expect("commit");
        let _ = job.ack.send(()); // durable (WAL-committed; fsync per `synchronous`)
    }
}

fn agent_loop(idx: usize, commits: usize, tx: SyncSender<Job>) -> Vec<f64> {
    let (ack_tx, ack_rx) = sync_channel::<()>(1); // one outstanding intent at a time
    let mut lat = Vec::with_capacity(commits);
    let project_id = format!("proj_{:03}", idx % 4); // small project set so readers hit rows
    let session_id = format!("sess_{idx:03}");
    let correlation_id = format!("corr_{idx:03}");
    for c in 0..commits {
        let uniq = idx as i64 * 10_000_000 + c as i64;
        let job = Job {
            payload_json: make_payload(uniq),
            idem_key: format!("idem_{uniq:024}"),
            obj_a: format!("wt_{uniq:022}"),
            obj_b: format!("br_{uniq:022}"),
            project_id: project_id.clone(),
            session_id: session_id.clone(),
            correlation_id: correlation_id.clone(),
            ack: ack_tx.clone(),
        };
        let t0 = Instant::now();
        tx.send(job).expect("submit");
        ack_rx.recv().expect("ack");
        lat.push(t0.elapsed().as_secs_f64() * 1000.0);
    }
    lat
}

fn reader_loop(path: String, stop: Arc<AtomicBool>) -> Vec<f64> {
    let conn = open_reader(&path);
    let mut lat = Vec::new();
    let mut round: i64 = 0;
    while !stop.load(Ordering::Relaxed) {
        round += 1;
        let project_id = format!("proj_{:03}", round % 4);
        let session_id = format!("sess_{:03}", round % 20);

        // Q1: recent-activity feed (the sidebar / project timeline read)
        let t0 = Instant::now();
        {
            let mut s = conn
                .prepare_cached(
                    "SELECT event_id,seq,event_type FROM events WHERE project_id=?1 \
ORDER BY seq DESC LIMIT 50",
                )
                .expect("prep q1");
            let rows = s
                .query_map([&project_id], |r| r.get::<_, i64>(1))
                .expect("q1");
            for r in rows {
                let _ = r;
            }
        }
        lat.push(t0.elapsed().as_secs_f64() * 1000.0);

        // Q2: per-session count (a projection-style aggregate read)
        let t1 = Instant::now();
        {
            let mut s = conn
                .prepare_cached("SELECT count(*) FROM events WHERE session_id=?1")
                .expect("prep q2");
            let _: i64 = s.query_row([&session_id], |r| r.get(0)).unwrap_or(0);
        }
        lat.push(t1.elapsed().as_secs_f64() * 1000.0);

        // Q3: FTS audit search
        let t2 = Instant::now();
        {
            let mut s = conn
                .prepare_cached(
                    "SELECT event_id FROM fts_events WHERE fts_events MATCH 'worktree' LIMIT 20",
                )
                .expect("prep q3");
            let rows = s.query_map([], |r| r.get::<_, String>(0)).expect("q3");
            for r in rows {
                let _ = r;
            }
        }
        lat.push(t2.elapsed().as_secs_f64() * 1000.0);
    }
    lat
}

// ---- stats -----------------------------------------------------------------

struct Stats {
    p50: f64,
    p95: f64,
    p99: f64,
    max: f64,
    mean: f64,
}

fn stats(mut v: Vec<f64>) -> Stats {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    let pct = |p: f64| -> f64 {
        if n == 0 {
            return 0.0;
        }
        let idx = ((p / 100.0) * (n as f64 - 1.0)).round() as usize;
        v[idx.min(n - 1)]
    };
    let mean = if n == 0 {
        0.0
    } else {
        v.iter().sum::<f64>() / n as f64
    };
    Stats {
        p50: pct(50.0),
        p95: pct(95.0),
        p99: pct(99.0),
        max: if n == 0 { 0.0 } else { v[n - 1] },
        mean,
    }
}

struct RunResult {
    agents: usize,
    sync_full: bool,
    commit: Stats,
    read: Stats,
    throughput: f64, // commits/sec
    wall_s: f64,
    wal_autocheckpoint: i64,
}

fn run(
    path: &str,
    agents: usize,
    commits_per_agent: usize,
    readers: usize,
    sync_full: bool,
    preseed_count: usize,
) -> RunResult {
    // fresh DB file per run
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(format!("{path}-wal"));
    let _ = std::fs::remove_file(format!("{path}-shm"));
    {
        let conn = open_writer(path, sync_full);
        create_schema(&conn);
    }
    if preseed_count > 0 {
        preseed(path, preseed_count);
    }
    let wal_autocheckpoint = {
        let conn = open_reader(path);
        conn.query_row("PRAGMA wal_autocheckpoint", [], |r| r.get::<_, i64>(0))
            .unwrap_or(-1)
    };

    let (tx, rx) = sync_channel::<Job>(agents * 2); // bounded; small backlog allowed
    let writer_path = path.to_string();
    let start_seq = preseed_count as i64; // timed writer continues past the seeded rows
    let writer = thread::spawn(move || writer_loop(writer_path, sync_full, start_seq, rx));

    let stop = Arc::new(AtomicBool::new(false));
    let mut reader_handles = Vec::new();
    for _ in 0..readers {
        let p = path.to_string();
        let s = stop.clone();
        reader_handles.push(thread::spawn(move || reader_loop(p, s)));
    }

    // small warmup so readers have rows + caches are hot before timing the wall clock
    let wall0 = Instant::now();
    let mut agent_handles = Vec::new();
    for i in 0..agents {
        let t = tx.clone();
        agent_handles.push(thread::spawn(move || agent_loop(i, commits_per_agent, t)));
    }
    drop(tx); // writer ends when all agent senders are gone

    let mut commit_lat = Vec::new();
    for h in agent_handles {
        commit_lat.extend(h.join().expect("agent join"));
    }
    let wall_s = wall0.elapsed().as_secs_f64();
    writer.join().expect("writer join");

    stop.store(true, Ordering::Relaxed);
    let mut read_lat = Vec::new();
    for h in reader_handles {
        read_lat.extend(h.join().expect("reader join"));
    }

    let total_commits = agents * commits_per_agent;
    RunResult {
        agents,
        sync_full,
        commit: stats(commit_lat),
        read: stats(read_lat),
        throughput: total_commits as f64 / wall_s,
        wall_s,
        wal_autocheckpoint,
    }
}

fn print_header() {
    println!(
        "\n{:>6} {:>5} {:>9} {:>9} {:>9} {:>9} {:>9} {:>10} {:>9} {:>9} {:>9}",
        "agents",
        "sync",
        "c_p50",
        "c_p95",
        "c_p99",
        "c_max",
        "c_mean",
        "thrpt/s",
        "r_p50",
        "r_p95",
        "r_max"
    );
    println!("{}", "-".repeat(108));
}

fn print_row(r: &RunResult) {
    println!(
        "{:>6} {:>5} {:>9.2} {:>9.2} {:>9.2} {:>9.2} {:>9.2} {:>10.0} {:>9.2} {:>9.2} {:>9.2}",
        r.agents,
        if r.sync_full { "FULL" } else { "NORM" },
        r.commit.p50,
        r.commit.p95,
        r.commit.p99,
        r.commit.max,
        r.commit.mean,
        r.throughput,
        r.read.p50,
        r.read.p95,
        r.read.max,
    );
}

fn parse_arg(flag: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1).cloned())
}

fn main() {
    let dir = std::env::temp_dir().join("nexusops-sqlite-spike");
    std::fs::create_dir_all(&dir).expect("tmp dir");
    let path = dir.join("loadtest.db");
    let path = path.to_str().unwrap().to_string();

    let single_agents = parse_arg("--agents").and_then(|s| s.parse::<usize>().ok());
    let commits = parse_arg("--commits").and_then(|s| s.parse::<usize>().ok());
    let readers = parse_arg("--readers")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(2);
    let sync_full = parse_arg("--sync").map(|s| s == "full").unwrap_or(false);
    let preseed_count = parse_arg("--preseed")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);

    println!("# OQ-DATA-SPIKE-3 — SQLite single-writer load test");
    println!("# db: {path}");
    println!("# pragmas: journal_mode=WAL, foreign_keys=ON, busy_timeout=5000");
    println!("# commit txn: events(+6 idx) + 2 object_refs(FK) + 1 fts5 + 2 proj upserts");
    println!("# readers: {readers} (concurrent read-only WAL conns)");

    if let Some(a) = single_agents {
        let c = commits.unwrap_or(1000);
        if preseed_count > 0 {
            println!("# preseeding {preseed_count} events before the timed run...");
        }
        print_header();
        let r = run(&path, a, c, readers, sync_full, preseed_count);
        print_row(&r);
        println!(
            "\nwall={:.2}s total_commits={} preseeded={} wal_autocheckpoint={} pages",
            r.wall_s,
            a * c,
            preseed_count,
            r.wal_autocheckpoint
        );
        let _ = std::fs::remove_dir_all(&dir);
        return;
    }

    // Default: ceiling sweep at sync=NORMAL (the locked value) + one FULL run at N=20.
    print_header();
    let sweep = [1usize, 5, 10, 20, 30, 50, 100];
    let mut results = Vec::new();
    for &a in &sweep {
        // scale commits so each level has a solid sample (>= ~4000 total, capped per-agent)
        let c = (4000 / a).clamp(200, 2000);
        let r = run(&path, a, c, readers, false, 0);
        print_row(&r);
        results.push(r);
    }
    // durability-cost comparison: N=20 at synchronous=FULL
    let full = run(&path, 20, 200, readers, true, 0);
    print_row(&full);

    println!("\n# Ceiling = lowest agent count where commit p95 >= 100ms (§18 budget).");
    let breach = results.iter().find(|r| r.commit.p95 >= 100.0);
    match breach {
        Some(r) => println!("# commit p95 first breaches 100ms at N={}", r.agents),
        None => println!(
            "# commit p95 stays < 100ms across the entire sweep (max tested N={})",
            sweep.last().unwrap()
        ),
    }
    if let Some(r20) = results.iter().find(|r| r.agents == 20) {
        println!(
            "# @N=20 (sync=NORMAL): commit p95={:.2}ms reader p95={:.2}ms throughput={:.0}/s",
            r20.commit.p95, r20.read.p95, r20.throughput
        );
    }
    println!(
        "# @N=20 (sync=FULL): commit p95={:.2}ms throughput={:.0}/s",
        full.commit.p95, full.throughput
    );

    let _ = std::fs::remove_dir_all(&dir);
}
