//! Phase 1.4 — lease locks + monotonic fencing + pidlock single-instance + reaper (RED first).
//! ARCHITECTURE ADR-008 (cross-restart SQLite lease + monotonic fencing + pidlock),
//! §7.2 (leases authoritative; expired → reclaim w/ a new fencing token), §17 line 387
//! (stale-token write → fencing_conflict; the 1.4 primitive makes a stale token DETECTABLE),
//! §16 (backup-before-migrate), §12 (reaper unit). **Safety rule #6 — fencing mandatory.**
//!
//! Integration tests (public surface) per the 1.1/1.2/1.3 convention; `StepClock` implements
//! the pub `Clock` trait for deterministic expiry. Layered:
//!   L1 (1–6) — leases (migration 5) + lease primitive + monotonic fencing + the live-lease
//!             authority oracle (`validate_held`, human-ruled Option B). ⚠️ safety-critical
//!   L2 (7–8) — pidlock single-instance (std advisory file lock). ⚠️ protects single-writer
//!   L3 (9–10) — reaper (reap_once) + restart survival.

use std::sync::Mutex;

use nexusopsd::clock::{Clock, FixedClock};
use nexusopsd::eventstore::EventStore;
use nexusopsd::locks::{
    FencingToken, LeaseError, LeaseKind, OwnerId, PidLock, PidLockError, ResourceId,
};

// ---- shared fixtures --------------------------------------------------------

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
        Box::new(nexusopsd::eventstore::PrefixRedactor),
    )
    .expect("open event store")
}

fn res(s: &str) -> ResourceId {
    ResourceId(s.to_string())
}
fn kind() -> LeaseKind {
    LeaseKind::resource_mutation()
}
fn owner(s: &str) -> OwnerId {
    OwnerId(s.to_string())
}

/// a clock the test advances by setting an explicit RFC3339 (drives TTL/expiry).
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

fn count(path: &std::path::Path, sql: &str) -> i64 {
    nexusopsd::eventstore::open_read_only(path)
        .unwrap()
        .query_row(sql, [], |r| r.get(0))
        .unwrap()
}

/// the persisted fencing high-water mark for a resource (kind = the MVP seed).
fn high_water(path: &std::path::Path, resource: &str) -> i64 {
    nexusopsd::eventstore::open_read_only(path)
        .unwrap()
        .query_row(
            "SELECT fencing_token FROM leases WHERE resource_id=?1 AND lease_kind='resource_mutation'",
            [resource],
            |r| r.get(0),
        )
        .unwrap()
}

/// true if the slot is free (holder fields NULLed; the high-water token is kept).
fn lease_is_free(path: &std::path::Path, resource: &str) -> bool {
    nexusopsd::eventstore::open_read_only(path)
        .unwrap()
        .query_row(
            "SELECT owner_id IS NULL FROM leases WHERE resource_id=?1 AND lease_kind='resource_mutation'",
            [resource],
            |r| r.get(0),
        )
        .unwrap()
}

fn current_owner(path: &std::path::Path, resource: &str) -> Option<String> {
    nexusopsd::eventstore::open_read_only(path)
        .unwrap()
        .query_row(
            "SELECT owner_id FROM leases WHERE resource_id=?1 AND lease_kind='resource_mutation'",
            [resource],
            |r| r.get(0),
        )
        .unwrap()
}

/// build a NON-EMPTY v1 db (the proven 1.1 backup-test construction): only the
/// `events` table + one valid event + user_version=1. Opening it raises 1→…→5 over
/// existing data, which must back the file up first (§16) and create `leases` (M5).
fn build_v1_with_event(path: &std::path::Path) {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.execute_batch(nexusopsd::eventstore::MIGRATION_1_EVENTS)
        .unwrap();
    conn.execute(
        "INSERT INTO events (event_id, seq, event_type, event_version, occurred_at, \
         recorded_at, workspace_id, actor_type, actor_id, source_type, source_id, \
         correlation_id, sensitivity, payload_json, schema_version) \
         VALUES ('evt_01ARZ3NDEKTSV4RRFFQ69G5FAV',1,'SessionStarted',1,\
         '2026-06-08T00:00:00Z','2026-06-08T00:00:00Z','ws_01ARZ3NDEKTSV4RRFFQ69G5FAV',\
         'user','u','desktop_ui','s','c','internal','{}','event-envelope-v1')",
        [],
    )
    .unwrap();
    conn.pragma_update(None, "user_version", 1).unwrap();
}

// ======================= L1 — leases + fencing (1–5) =========================

// ---- Test 1 — migration 5 creates leases (ADR-008 / §16) --------------------

#[test]
fn test_migration_5_creates_leases() {
    // a fresh open migrates to 5 + creates the lease table + its expiry index
    let (_d, path) = temp_db();
    let store = open(&path);
    // exact pin (this is M5's own migration test); relax to `>= 5` when M6 lands.
    assert_eq!(store.user_version(), 5, "open migrates to user_version 5");
    let t = tables(&path);
    assert!(t.contains("leases"), "migration 5 creates the leases table");
    assert!(
        t.contains("ix_leases_expiry"),
        "expiry index created (reaper scan, §12)"
    );
    assert!(
        t.contains("events") && t.contains("proj_session") && t.contains("outbox"),
        "spine + projections + outbox intact across the migration"
    );
    drop(store);

    // backup-before-migrate (§16): opening a NON-EMPTY old db backs the file up
    // before raising user_version, AND M5 creates `leases` over the existing data.
    let (_d2, path2) = temp_db();
    build_v1_with_event(&path2);
    let _store2 = open(&path2);
    let bak = std::path::PathBuf::from(format!("{}.bak-1", path2.display()));
    assert!(
        bak.exists(),
        "auto-backup wrote .bak-1 before raising user_version through M5 (§16)"
    );
    assert!(
        tables(&path2).contains("leases"),
        "leases created over existing data"
    );
    assert_eq!(
        count(&path2, "SELECT COUNT(*) FROM events"),
        1,
        "the pre-existing event survived the migration"
    );
}

// ---- Test 2 — acquire / renew / release happy path (ADR-008 / §7.2) ---------

#[test]
fn test_acquire_renew_release_happy() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let clock = StepClock::new("2026-06-08T00:00:00Z");

    // acquire on a free slot: owner set, expires = now + ttl, first token is 1
    let lease = store
        .acquire_lease(&res("wt_1"), &kind(), &owner("sess_a"), 60, &clock)
        .unwrap();
    assert_eq!(lease.fencing_token, FencingToken(1), "first token is 1");
    assert_eq!(lease.owner_id, owner("sess_a"), "owner recorded");
    assert_eq!(
        lease.expires_at, "2026-06-08T00:01:00Z",
        "expires_at = now + 60s"
    );

    // renew (same owner+token): extends expires_at, token UNCHANGED (renew ≠ reclaim)
    clock.set("2026-06-08T00:00:30Z");
    let renewed = store.renew_lease(&lease, 60, &clock).unwrap();
    assert_eq!(
        renewed.fencing_token,
        FencingToken(1),
        "renew keeps the fencing token (renew is not a reclaim)"
    );
    assert_eq!(
        renewed.expires_at, "2026-06-08T00:01:30Z",
        "renew extends expires_at to now + ttl"
    );

    // release: frees the slot but PRESERVES the fencing high-water mark
    store.release_lease(&renewed).unwrap();
    assert!(
        lease_is_free(&path, "wt_1"),
        "released slot is free (owner NULL)"
    );
    assert_eq!(
        high_water(&path, "wt_1"),
        1,
        "release preserves the fencing token (next acquire still increments)"
    );
}

// ---- Test 3 — acquire refused while a LIVE lease is held (§7.2) -------------

#[test]
fn test_acquire_refused_while_live_held() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let clock = StepClock::new("2026-06-08T00:00:00Z");

    let _a = store
        .acquire_lease(&res("wt_1"), &kind(), &owner("sess_a"), 60, &clock)
        .unwrap();
    // a DIFFERENT owner cannot take a non-expired lease
    let r = store.acquire_lease(&res("wt_1"), &kind(), &owner("sess_b"), 60, &clock);
    assert!(
        matches!(r, Err(LeaseError::Held { .. })),
        "a live lease held by another owner → typed Held error"
    );
    // and the refusal mutated nothing
    assert_eq!(
        high_water(&path, "wt_1"),
        1,
        "refused acquire did not bump the token"
    );
    assert_eq!(
        current_owner(&path, "wt_1"),
        Some("sess_a".to_string()),
        "owner unchanged by the refusal"
    );
}

// ---- Test 4 — stale token rejected after reclaim (safety rule #6 / §17) -----

#[test]
fn test_stale_token_rejected_after_reclaim() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let clock = StepClock::new("2026-06-08T00:00:00Z");

    // A acquires (token 1), then the lease expires
    let a = store
        .acquire_lease(&res("wt_1"), &kind(), &owner("sess_a"), 60, &clock)
        .unwrap();
    assert_eq!(a.fencing_token, FencingToken(1));
    clock.set("2026-06-08T01:00:00Z"); // well past expires_at

    // B reclaims the expired lease → strictly-greater token
    let b = store
        .acquire_lease(&res("wt_1"), &kind(), &owner("sess_b"), 60, &clock)
        .unwrap();
    assert_eq!(
        b.fencing_token,
        FencingToken(2),
        "reclaim of an expired lease mints token N+1"
    );

    // the PAUSED holder A is no longer the authority — superseded. The safety-rule-#6
    // pin via the unified `validate_held` oracle (owner ∧ token==high-water ∧ live).
    assert!(
        !store
            .validate_lease_held(
                &res("wt_1"),
                &kind(),
                &owner("sess_a"),
                a.fencing_token,
                &clock
            )
            .unwrap(),
        "A's authority is gone after B reclaims (superseded → gateway rejects, §17)"
    );
    assert!(
        store
            .validate_lease_held(
                &res("wt_1"),
                &kind(),
                &owner("sess_b"),
                b.fencing_token,
                &clock
            )
            .unwrap(),
        "B is the live authority"
    );
    // and A can no longer renew (not the holder anymore)
    assert!(
        matches!(
            store.renew_lease(&a, 60, &clock),
            Err(LeaseError::NotHolder)
        ),
        "a stale-token renew is rejected"
    );
}

// ---- Test 5 — expired holder rejected even if unsuperseded (Option B / §17) --

#[test]
fn test_expired_holder_rejected_even_if_unsuperseded() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let clock = StepClock::new("2026-06-08T00:00:00Z");

    let a = store
        .acquire_lease(&res("wt_1"), &kind(), &owner("sess_a"), 60, &clock)
        .unwrap();
    assert_eq!(a.fencing_token, FencingToken(1));

    // while live, A IS the authority (owner + token + not-expired all hold)
    clock.set("2026-06-08T00:00:30Z");
    assert!(
        store
            .validate_lease_held(
                &res("wt_1"),
                &kind(),
                &owner("sess_a"),
                a.fencing_token,
                &clock
            )
            .unwrap(),
        "a live holder is the authority"
    );

    // advance past expiry with NO reclaim — the fencing token is still N (unsuperseded)
    clock.set("2026-06-08T00:02:00Z");
    assert_eq!(
        high_water(&path, "wt_1"),
        1,
        "token unchanged — the lease was never reclaimed/superseded"
    );
    // …yet an EXPIRED holder is NOT the authority. Authority = a LIVE lease, not a
    // merely-unsuperseded token — the gap §17 line 387 left open (human-ruled Option B).
    assert!(
        !store
            .validate_lease_held(
                &res("wt_1"),
                &kind(),
                &owner("sess_a"),
                a.fencing_token,
                &clock
            )
            .unwrap(),
        "stale = NOT a live lease (expired OR superseded) → §17 fencing_conflict"
    );
}

// ---- Test 6 — fencing token strictly monotonic across cycles (Q2) ----------

#[test]
fn test_fencing_token_strictly_monotonic() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let clock = StepClock::new("2026-06-08T00:00:00Z");

    let mut prev = 0u64;
    for i in 0..5 {
        let l = store
            .acquire_lease(&res("wt_1"), &kind(), &owner("sess"), 60, &clock)
            .unwrap();
        assert!(
            l.fencing_token.0 > prev,
            "token strictly increases each acquire (cycle {i})"
        );
        prev = l.fencing_token.0;
        // release frees the slot but keeps the high-water mark → next acquire = prev+1
        store.release_lease(&l).unwrap();
    }
    assert_eq!(
        prev, 5,
        "5 acquire/release cycles → token 5 (release never lowers the high-water mark)"
    );
}

// ===================== L2 — pidlock single-instance (7–8) ====================
//
// Single-instance via a std advisory file lock (ADR-008). The OS holds the lock
// against the open fd → auto-released on process death (even a crash) → IMMUNE to
// PID reuse. The PID written into the file is diagnostic only — NEVER a kill(pid,0)
// liveness oracle (PID reuse → false positives). ⚠️ protects the single-writer.

// ---- Test 7 — a second instance is refused while the first holds it ---------

#[test]
fn test_pidlock_refuses_second_instance() {
    let dir = tempfile::tempdir().unwrap();
    let lock_path = dir.path().join("daemon.lock");

    let first = PidLock::acquire(&lock_path).expect("first instance acquires the lock");
    // while the first holds the OS lock, a second acquire on the same path is refused
    let second = PidLock::acquire(&lock_path);
    assert!(
        matches!(second, Err(PidLockError::AlreadyHeld)),
        "second instance refused while the first holds the lock"
    );
    // dropping the first closes its fd → the OS releases the lock → a fresh acquire wins
    drop(first);
    let third = PidLock::acquire(&lock_path);
    assert!(
        third.is_ok(),
        "after the first releases, a fresh acquire succeeds"
    );
}

// ---- Test 8 — PID reuse does not yield a false single-instance --------------

#[test]
fn test_pidlock_pid_reuse_no_false_single_instance() {
    let dir = tempfile::tempdir().unwrap();
    let lock_path = dir.path().join("daemon.lock");

    // a STALE lock file containing a foreign/reused PID, but NO live process holds the
    // OS advisory lock (we only write bytes; we take no lock on the handle).
    std::fs::write(&lock_path, "99999\n").unwrap();

    // the oracle is the OS advisory lock, not the file's PID contents — so acquire SUCCEEDS
    // (a dead/foreign PID in the file neither grants nor falsely blocks).
    let got = PidLock::acquire(&lock_path);
    assert!(
        got.is_ok(),
        "a foreign PID in the file must not falsely block (no kill(pid,0) oracle)"
    );
}

// =============== L3 — reaper + restart survival (9–10) ========================

// ---- Test 9 — reap_once frees expired leases, keeps the token (§12) ---------

#[test]
fn test_reap_once_frees_expired() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let clock = StepClock::new("2026-06-08T00:00:00Z");

    // one short-lived lease (expires 00:01:00) + one long-lived (expires 01:00:00)
    let exp = store
        .acquire_lease(&res("wt_exp"), &kind(), &owner("a"), 60, &clock)
        .unwrap();
    let _live = store
        .acquire_lease(&res("wt_live"), &kind(), &owner("b"), 3600, &clock)
        .unwrap();

    // advance past wt_exp's expiry but not wt_live's, then reap
    clock.set("2026-06-08T00:30:00Z");
    let reclaimed = store.reap_leases(&clock).unwrap();
    assert_eq!(reclaimed.len(), 1, "only the expired lease is reaped");
    assert_eq!(
        reclaimed[0].0,
        res("wt_exp"),
        "the expired resource was reclaimed"
    );
    assert_eq!(reclaimed[0].1, kind(), "with its lease kind");

    // expired lease freed (holder NULL) but the fencing high-water mark is KEPT
    assert!(
        lease_is_free(&path, "wt_exp"),
        "expired lease freed by the reaper"
    );
    assert_eq!(
        high_water(&path, "wt_exp"),
        exp.fencing_token.0 as i64,
        "reap preserves the fencing token (no reuse on the next acquire)"
    );
    // the live lease is untouched
    assert!(
        !lease_is_free(&path, "wt_live"),
        "non-expired lease untouched"
    );
}

// ---- Test 10 — restart preserves fencing monotonicity (ADR-008, integration) -

#[test]
fn test_restart_preserves_fencing_monotonicity() {
    let (_d, path) = temp_db();

    // a lease is held at token N, then expires; the store is dropped (restart boundary)
    let a_token;
    {
        let mut store = open(&path);
        let clock = StepClock::new("2026-06-08T00:00:00Z");
        let a = store
            .acquire_lease(&res("wt_1"), &kind(), &owner("a"), 60, &clock)
            .unwrap();
        a_token = a.fencing_token;
    } // store dropped → DB closed: simulated daemon restart

    // reopen the DB; a new acquire (past the old expiry) mints a STRICTLY-GREATER token —
    // proving the high-water mark was persisted on disk, not held in memory.
    let mut store2 = open(&path);
    let clock2 = StepClock::new("2026-06-08T02:00:00Z");
    let b = store2
        .acquire_lease(&res("wt_1"), &kind(), &owner("b"), 60, &clock2)
        .unwrap();
    assert!(
        b.fencing_token > a_token,
        "post-restart token strictly greater (high-water persisted across restart)"
    );

    // the pre-restart token is no longer authoritative after the post-restart reclaim
    assert!(
        !store2
            .validate_lease_held(&res("wt_1"), &kind(), &owner("a"), a_token, &clock2)
            .unwrap(),
        "pre-restart token fails validation after restart + reclaim"
    );
}

// ---- Test 11 — reap at the exact expiry boundary; re-acquire increments (§12) -

#[test]
fn test_reap_boundary_then_reacquire_increments() {
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let clock = StepClock::new("2026-06-08T00:00:00Z");

    // acquire (token 1, expires exactly 00:01:00)
    let a = store
        .acquire_lease(&res("wt_1"), &kind(), &owner("a"), 60, &clock)
        .unwrap();
    assert_eq!(a.fencing_token, FencingToken(1));

    // boundary: now == expires_at. `live` is `expires_at > now` (strict), so the lease is
    // NOT live; reap (`expires_at <= now`) MUST reclaim it — the two checks are complements.
    clock.set("2026-06-08T00:01:00Z");
    let reclaimed = store.reap_leases(&clock).unwrap();
    assert_eq!(
        reclaimed.len(),
        1,
        "a lease at exactly expires_at is reaped"
    );
    assert!(lease_is_free(&path, "wt_1"), "reaped slot is free");

    // re-acquiring the reaped slot increments from the KEPT high-water (reap didn't reset it)
    let b = store
        .acquire_lease(&res("wt_1"), &kind(), &owner("b"), 60, &clock)
        .unwrap();
    assert_eq!(
        b.fencing_token,
        FencingToken(2),
        "post-reap acquire mints token N+1 (reap preserved the high-water, no reuse)"
    );
}
