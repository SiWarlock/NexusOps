//! Phase 1.3 — transactional outbox (RED first). ARCHITECTURE §7 (event+projection+
//! **outbox** one txn), §12 (drainer), §15 (sync sink — outbox payload derives from
//! the already-redacted event), §17 (integration-failure contract), DATA_MODEL §2.5.
//!
//! Integration tests (public surface) per the 1.1/1.2 convention; FakeDestination /
//! StepClock implement the pub Destination / Clock traits. Layered:
//!   L1 (1–3) — outbox table (migration 4) + in-txn write + §15 sync gate.
//!   L2 (4–9) — drainer (drain_once / classify / backoff / dead-letter) + crash redeliver. [L2]

use std::collections::HashSet;
use std::sync::Mutex;

use nexusops_shared::actor::ActorType;
use nexusops_shared::event_envelope::{RedactionStatus, Sensitivity, SourceType};
use nexusops_shared::ids::{ProjectId, SessionId, WorkspaceId};
use nexusopsd::clock::{Clock, FixedClock};
use nexusopsd::eventstore::{
    AppendIntent, DeliveryOutcome, Destination, EventStore, EventStoreError, JsonlMirror,
    PrefixRedactor, RedactionOutcome, Redactor,
};

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    (dir, path)
}

fn open(path: &std::path::Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(nexusopsd::idgen::UlidGen),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        Box::new(PrefixRedactor),
    )
    .expect("open event store")
}

/// a SessionStarted intent carrying identity + a type-specific payload.
fn session_intent(payload: &str) -> AppendIntent {
    AppendIntent {
        event_type: "SessionStarted".to_string(),
        event_version: 1,
        occurred_at: "2026-06-08T00:00:00Z".to_string(),
        workspace_id: WorkspaceId::new(),
        actor_type: ActorType::User,
        actor_id: "u_1".to_string(),
        source_type: SourceType::DesktopUi,
        source_id: "src_1".to_string(),
        correlation_id: "corr_1".to_string(),
        sensitivity: Sensitivity::Internal,
        payload_json: payload.to_string(),
        schema_version: "event-envelope-v1".to_string(),
        idempotency_key: None,
        project_id: Some(ProjectId::new()),
        session_id: Some(SessionId::new()),
        agent_team_id: None,
        visibility: None,
    }
}

fn count(path: &std::path::Path, sql: &str) -> i64 {
    nexusopsd::eventstore::open_read_only(path)
        .unwrap()
        .query_row(sql, [], |r| r.get(0))
        .unwrap()
}

fn tables(path: &std::path::Path) -> std::collections::BTreeSet<String> {
    let conn = nexusopsd::eventstore::open_read_only(path).unwrap();
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type IN ('table','index')")
        .unwrap();
    let out = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    out
}

/// a Redactor that refuses to redact — drives the §15 fail-closed gate abort.
struct NeverRedacts;
impl Redactor for NeverRedacts {
    fn redact(&self, payload_json: &str) -> RedactionOutcome {
        RedactionOutcome {
            status: RedactionStatus::Unredacted,
            payload_json: payload_json.to_string(),
            engine_version: "never".to_string(),
        }
    }
}

// ---- Test 1 — migration 4 creates the outbox (§2.5/§16) ---------------------

#[test]
fn test_migration_4_creates_outbox() {
    let (_d, path) = temp_db();
    let store = open(&path);
    // outbox was introduced at migration 4; later migrations raise the version further
    // (the exact-version pin lives in each migration's own test, e.g. M5 in tests/locks.rs).
    assert!(
        store.user_version() >= 4,
        "open migrates at/above the outbox migration (4)"
    );
    let t = tables(&path);
    assert!(t.contains("outbox"), "migration 4 creates outbox");
    assert!(t.contains("ix_outbox_due"), "due-index created");
    assert!(
        t.contains("events") && t.contains("proj_session"),
        "spine + projections intact"
    );
}

// ---- Test 2 — outbox rows written in the append txn (§7 transactional) -------

#[test]
fn test_append_writes_outbox_rows_in_txn() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    store
        .append(session_intent("{\"status\":\"starting\"}"))
        .unwrap();

    // one pending row per subscribed destination (jsonl_mirror in Phase 1)
    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM outbox"),
        1,
        "one pending row"
    );
    assert_eq!(
        count(
            &path,
            "SELECT COUNT(*) FROM outbox WHERE status='pending' AND destination='jsonl_mirror'"
        ),
        1,
        "pending jsonl_mirror row"
    );

    // boundary (§7.2): a full rebuild reconstructs read models only — it must NOT
    // re-emit / resurrect outbox delivery intents.
    store.rebuild_projections().unwrap();
    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM outbox"),
        1,
        "rebuild leaves the outbox untouched (no re-delivery of history)"
    );

    // atomicity: a redaction-gate abort persists NOTHING — not the event, not the
    // projections, not the outbox (they commit / roll back together).
    let mut gated = EventStore::open(
        &path,
        Box::new(nexusopsd::idgen::UlidGen),
        Box::new(FixedClock::new("2026-06-08T00:00:00Z")),
        Box::new(NeverRedacts),
    )
    .unwrap();
    let before_events = count(&path, "SELECT COUNT(*) FROM events");
    let res = gated.append(session_intent("{\"status\":\"active\"}"));
    assert!(
        matches!(res, Err(EventStoreError::RedactionRequired)),
        "the §15 gate refuses (fail-closed), proving the abort actually fired"
    );
    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM events"),
        before_events,
        "no event"
    );
    assert_eq!(
        count(&path, "SELECT COUNT(*) FROM outbox"),
        1,
        "no new outbox row"
    );
}

// ---- Test 3 — outbox payload carries no secret (§15 sync sink) ---------------

#[test]
fn test_outbox_payload_has_no_secret() {
    let (_d, path) = temp_db();
    let mut store = open(&path); // PrefixRedactor redacts at the 1.1 gate
    store
        .append(session_intent(
            "{\"status\":\"starting\",\"token\":\"ghp_SECRETSECRETSECRETSECRET\"}",
        ))
        .unwrap();
    let payload: String = nexusopsd::eventstore::open_read_only(&path)
        .unwrap()
        .query_row("SELECT payload_json FROM outbox", [], |r| r.get(0))
        .unwrap();
    assert!(
        !payload.contains("ghp_SECRETSECRETSECRETSECRET"),
        "the secret never reaches the outbox payload"
    );
    assert!(
        payload.contains("[REDACTED]"),
        "the outbox mirrors the redacted event"
    );
}

// ======================= L2 — drainer + destinations (4–9) ===================

/// a clock the test advances by setting an explicit RFC3339 (drives backoff due-ness).
struct StepClock {
    now: Mutex<String>,
}
impl StepClock {
    fn new(ts: &str) -> Self {
        Self {
            now: Mutex::new(ts.to_string()),
        }
    }
    fn set(&self, ts: &str) {
        *self.now.lock().unwrap() = ts.to_string();
    }
}
impl Clock for StepClock {
    fn now_rfc3339(&self) -> String {
        self.now.lock().unwrap().clone()
    }
}

/// a scripted destination: one settable outcome, counts deliver calls, and dedups
/// effects by payload (an idempotent consumer — proves no-double-apply).
struct FakeDestination {
    name: &'static str,
    outcome: Mutex<DeliveryOutcome>,
    calls: Mutex<usize>,
    applied: Mutex<HashSet<String>>,
}
impl FakeDestination {
    fn new(name: &'static str, outcome: DeliveryOutcome) -> Self {
        Self {
            name,
            outcome: Mutex::new(outcome),
            calls: Mutex::new(0),
            applied: Mutex::new(HashSet::new()),
        }
    }
    fn set_outcome(&self, o: DeliveryOutcome) {
        *self.outcome.lock().unwrap() = o;
    }
    fn calls(&self) -> usize {
        *self.calls.lock().unwrap()
    }
    fn applied(&self) -> usize {
        self.applied.lock().unwrap().len()
    }
}
impl Destination for FakeDestination {
    fn name(&self) -> &str {
        self.name
    }
    fn deliver(&self, payload: &str) -> DeliveryOutcome {
        *self.calls.lock().unwrap() += 1;
        let outcome = self.outcome.lock().unwrap().clone();
        if matches!(outcome, DeliveryOutcome::Delivered) {
            self.applied.lock().unwrap().insert(payload.to_string()); // idempotent dedup
        }
        outcome
    }
}

fn status_of(path: &std::path::Path) -> String {
    nexusopsd::eventstore::open_read_only(path)
        .unwrap()
        .query_row("SELECT status FROM outbox LIMIT 1", [], |r| r.get(0))
        .unwrap()
}

// ---- Test 4 — delivery flows ONLY from outbox rows (INV-SEC-1 analogue) ------

#[test]
fn test_outbox_is_only_external_path() {
    let (_d, path) = temp_db();
    let mut store = open(&path); // no append → no outbox rows
    let fake = FakeDestination::new("jsonl_mirror", DeliveryOutcome::Delivered);
    store
        .drain_once(&StepClock::new("2030-01-01T00:00:00Z"), &fake)
        .unwrap();
    assert_eq!(
        fake.calls(),
        0,
        "no outbox row ⇒ no delivery (outbox is the only path)"
    );
}

// ---- Test 5 — drain delivers once, marks delivered (§12) — real jsonl_mirror -

#[test]
fn test_drain_delivers_once_marks_delivered() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    store
        .append(session_intent("{\"status\":\"starting\"}"))
        .unwrap();

    // the real Phase-1 destination appends the redacted payload as a JSONL line
    let mirror_path = path.with_extension("mirror.jsonl");
    let mirror = JsonlMirror::new(mirror_path.clone());
    let summary = store
        .drain_once(&StepClock::new("2030-01-01T00:00:00Z"), &mirror)
        .unwrap();
    assert_eq!(summary.delivered, 1, "summary reports one delivery");

    assert_eq!(status_of(&path), "delivered", "row marked delivered");
    let mirrored = std::fs::read_to_string(&mirror_path).unwrap();
    assert_eq!(mirrored.lines().count(), 1, "one JSONL line written");
    assert!(
        mirrored.contains("\"event_type\":\"SessionStarted\""),
        "the redacted event was mirrored"
    );
}

// ---- Test 6 — retryable backs off, re-drains after the delay (§17) -----------

#[test]
fn test_retryable_backs_off() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    store
        .append(session_intent("{\"status\":\"starting\"}"))
        .unwrap();
    let fake = FakeDestination::new("jsonl_mirror", DeliveryOutcome::Retryable("503".into()));
    let clock = StepClock::new("2030-01-01T00:00:00Z");

    let summary = store.drain_once(&clock, &fake).unwrap();
    assert_eq!(summary.retried, 1, "summary reports one retry scheduled");
    let (status, retry, next): (String, i64, Option<String>) =
        nexusopsd::eventstore::open_read_only(&path)
            .unwrap()
            .query_row(
                "SELECT status, retry_count, next_attempt_at FROM outbox",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
    assert_eq!(status, "failed", "retryable → failed");
    assert_eq!(retry, 1, "retry_count incremented");
    assert!(
        next.unwrap().as_str() > "2030-01-01T00:00:00Z",
        "backed off to the future"
    );

    // not due at the same clock — a re-drain delivers nothing
    store.drain_once(&clock, &fake).unwrap();
    assert_eq!(status_of(&path), "failed", "still failed (not due yet)");

    // advance past the backoff + flip the destination to success → delivered
    fake.set_outcome(DeliveryOutcome::Delivered);
    clock.set("2030-01-02T00:00:00Z");
    store.drain_once(&clock, &fake).unwrap();
    assert_eq!(
        status_of(&path),
        "delivered",
        "re-drains + delivers after the delay"
    );
}

// ---- Test 7 — terminal goes dead immediately (§17) --------------------------

#[test]
fn test_terminal_goes_dead() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    store
        .append(session_intent("{\"status\":\"starting\"}"))
        .unwrap();
    let fake = FakeDestination::new("jsonl_mirror", DeliveryOutcome::Terminal("401".into()));
    store
        .drain_once(&StepClock::new("2030-01-01T00:00:00Z"), &fake)
        .unwrap();
    assert_eq!(status_of(&path), "dead", "terminal → dead, no retry");
    assert_eq!(fake.calls(), 1, "attempted once");
}

// ---- Test 8 — retry budget exhausts to dead (§17 bounded budget) -------------

#[test]
fn test_retry_budget_exhausts_to_dead() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    store
        .append(session_intent("{\"status\":\"starting\"}"))
        .unwrap();
    let fake = FakeDestination::new("jsonl_mirror", DeliveryOutcome::Retryable("503".into()));
    let clock = StepClock::new("2030-01-01T00:00:00Z");

    // drain repeatedly, advancing past each backoff, until the bounded budget is hit
    for day in 1..20 {
        if status_of(&path) == "dead" {
            break;
        }
        clock.set(&format!("2030-02-{day:02}T00:00:00Z"));
        store.drain_once(&clock, &fake).unwrap();
    }
    assert_eq!(
        status_of(&path),
        "dead",
        "repeated retryable → dead after the budget"
    );
    assert_eq!(
        fake.calls(),
        6,
        "exactly 6 deliver attempts (1 initial + MAX_RETRIES=5) — pins the budget boundary"
    );
}

// ---- Test 9 — crash redelivers, idempotent consumer does not double-apply ----

#[test]
fn test_crash_redelivers_no_double_apply() {
    let (_d, path) = temp_db();
    // ONE idempotent destination spans the crash boundary, so its call/apply counts
    // accumulate across both drains.
    let fake = FakeDestination::new("jsonl_mirror", DeliveryOutcome::Delivered);
    {
        let mut store = open(&path);
        store
            .append(session_intent("{\"status\":\"starting\"}"))
            .unwrap();
        store
            .drain_once(&StepClock::new("2030-01-01T00:00:00Z"), &fake)
            .unwrap();
        assert_eq!(status_of(&path), "delivered");
    }
    // simulate a crash that LOST the delivered-commit: the row is stuck in_flight
    // (the deliver happened; its confirmation never persisted).
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute("UPDATE outbox SET status='in_flight'", [])
            .unwrap();
    }
    // reopen → crash recovery resets in_flight→pending → the row is re-deliverable
    let mut store = open(&path);
    assert_eq!(
        status_of(&path),
        "pending",
        "in_flight reset to pending on restart"
    );
    store
        .drain_once(&StepClock::new("2030-01-02T00:00:00Z"), &fake)
        .unwrap();

    assert_eq!(status_of(&path), "delivered", "redelivered");
    assert_eq!(
        fake.calls(),
        2,
        "deliver attempted twice (at-least-once across the crash)"
    );
    assert_eq!(
        fake.applied(),
        1,
        "idempotent consumer applied the effect exactly once"
    );
}
