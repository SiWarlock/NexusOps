//! Brief 054 — P4.0b-2c: the daemon-wide **audit-backbone circuit-breaker** (CAT-1-adjacent,
//! §17/§15 #5 fail-secure). The single audited mutator must NOT operate when it can no longer audit.
//!
//! Per-action audit-write faults already deny + raise the C2 per-incident durable alarm
//! (`integrity.rs` / `route_intercept_live`). This slice adds the **SYSTEMIC** layer: N-consecutive
//! (or a clearly-unrecoverable: disk-full / DB-corruption) audit-write faults trip a **latched**
//! breaker → the daemon **quiesce-and-refuses** (RULED B, lead 2026-06-13): every mutation
//! fail-closed-denies WITHOUT attempting an audit-write, while reads/subscribe/terminals stay live.
//!
//! Test families:
//!   * PURE breaker state-machine (no DB, no fault feature) — `observe()`/`classify_audit_fault()`.
//!     The fault-injection vehicle only emits `SQLITE_IOERR` (an UNRECOVERABLE class per Q3), so the
//!     RECOVERABLE-class behaviors (one-off-no-trip / N-consecutive / reset) are pinned by driving
//!     `observe()` directly — exactly the "breaker counter/trip is pure logic" acceptance bullet.
//!   * WIRING — the daemon-wide feed classification (#6) + the intercept path feeding the breaker
//!     (#8, Q4) + the RULED-(B) quiesce-and-refuse gate driven through the **real write-actor**
//!     (`run_actor` — the production gate+feed seam; #9/#10).

use nexusops_shared::actions::{ActionRequest, RequesterType, RiskLevel};
use nexusops_shared::ids::ActionRequestId;
use nexusops_shared::status::ActionRequestStatus as AR;
use nexusops_shared::time::Timestamp;
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{open_read_only, EventStore, EventStoreError, PrefixRedactor};
use nexusopsd::fault::{arm, FaultPoint};
use nexusopsd::gateway::circuit_breaker::{
    classify_audit_fault, AuditBackboneBreaker, AuditOutcome, BreakerAction,
    DEFAULT_AUDIT_BREAKER_THRESHOLD,
};
use nexusopsd::gateway::executor::StubExecutor;
use nexusopsd::gateway::policy::CatalogPolicy;
use nexusopsd::gateway::{Gateway, GatewayError};
use nexusopsd::harness::claude::intercept::{route_intercept_live, InterceptOutcome};
use nexusopsd::harness::MutationVerdict;
use nexusopsd::idgen::UlidGen;
use nexusopsd::integrity::{IntegrityAlarm, IntegrityKind, RecordingIntegrityAlarm};
use nexusopsd::runtime::WriteActor;

use std::path::{Path, PathBuf};
use std::sync::Arc;

// ---- helpers ------------------------------------------------------------------------------------

/// a breaker (Arc — shared between the write-actor and the test, which trips it via the held handle)
/// bound to a recording alarm, with an explicit threshold N (so the N-consecutive trip is fast + the
/// "below N" fast-trip is unambiguous).
fn breaker_n(n: u32) -> (Arc<AuditBackboneBreaker>, Arc<RecordingIntegrityAlarm>) {
    let alarm = Arc::new(RecordingIntegrityAlarm::new());
    let breaker = Arc::new(AuditBackboneBreaker::with_threshold(alarm.clone(), n));
    (breaker, alarm)
}

/// a synthetic `EventStoreError::Write(SqliteFailure(code))` — the realistic shape of an audit-write
/// failure (the eventstore fault-injection wraps faults exactly this way).
fn sqlite_write(code: i32) -> EventStoreError {
    EventStoreError::Write(rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error::new(code),
        Some(format!("synthetic sqlite code {code}")),
    ))
}

fn temp_db() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    (dir, path)
}

fn open(path: &Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(FixedClock::new("2026-06-13T00:00:00Z")),
        Box::new(PrefixRedactor),
    )
    .expect("open event store")
}

fn catalog_gw() -> Gateway {
    Gateway::new(Box::new(CatalogPolicy), Box::new(StubExecutor))
}

fn pp(tool: &str) -> nexusopsd::harness::claude::intercept::HookPayload {
    let json = serde_json::json!({
        "tool_name": tool,
        "tool_input": { "command": "ls -la" },
        "session_id": "sess_01ARZ3NDEKTSV4RRFFQ69G5FAV",
        "permission_mode": "default",
    })
    .to_string();
    nexusopsd::harness::claude::intercept::parse_payload(&json).expect("well-formed payload")
}

/// a minimal catalogued agent-mutation `ActionRequest` (`agent.bash` risk-2 = require_approval,
/// `agent.todo_write` risk-0). Used to drive the real write-actor mutation path.
fn agent_req(action_type: &str) -> ActionRequest {
    ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id: None,
        action_type: action_type.to_string(),
        requester_type: RequesterType::AgentSession,
        requester_id: "sess_01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
        resource_refs: vec![],
        inputs: serde_json::json!({ "command": "ls -la" }),
        risk_level: RiskLevel::Level0,
        idempotency_key: None,
        fencing_token: None,
        status: AR::Submitted,
        preview: None,
        created_at: Timestamp::parse("2026-06-13T00:00:00Z").unwrap(),
    }
}

/// count rows of `table` via a fresh read-only WAL conn (the production read path) — proves what the
/// write-actor durably committed without touching the actor's writable store.
fn count_ro(path: &Path, table: &str) -> i64 {
    let conn = open_read_only(path).expect("read-only conn");
    conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
        .expect("read-only count")
}

fn recording_sink() -> Arc<dyn IntegrityAlarm> {
    Arc::new(RecordingIntegrityAlarm::new())
}

// =================================================================================================
// PURE breaker state machine — the §17 fail-secure counter/trip logic (no DB, no fault feature).
// =================================================================================================

// ---- 054 #1 — a single recoverable fault does NOT trip (the per-action deny already protected it) -

#[test]
fn test_single_recoverable_fault_does_not_trip() {
    // spec(§17) — a transient/one-off audit-write fault is handled by the per-action deny + the C2
    // durable alarm; the SYSTEMIC breaker counts it (consecutive = 1, < N) but does NOT trip.
    let (breaker, alarm) = breaker_n(5);
    assert_eq!(
        breaker.observe(AuditOutcome::RecoverableFault),
        BreakerAction::Counted
    );
    assert_eq!(
        breaker.consecutive(),
        1,
        "one recoverable fault → counter = 1"
    );
    assert!(
        !breaker.is_tripped(),
        "a lone transient fault does NOT trip"
    );
    assert!(
        alarm.incidents().is_empty(),
        "no SYSTEMIC alarm for a single fault (the per-action C2 alarm is the one-off surface)"
    );
}

// ---- 054 #2 — N consecutive faults trip + raise the distinguishable systemic alarm ---------------

#[test]
fn test_n_consecutive_recoverable_faults_trip_with_systemic_alarm() {
    // spec(§17 / §15 #5) — N consecutive audit-write faults (no intervening success) → the breaker
    // TRIPS on the Nth, raising EXACTLY ONE durable alarm carrying the NEW systemic IntegrityKind
    // (distinguishable from the per-incident AuditWriteFailed).
    let n = 3;
    let (breaker, alarm) = breaker_n(n);
    for i in 1..n {
        assert_eq!(
            breaker.observe(AuditOutcome::RecoverableFault),
            BreakerAction::Counted,
            "fault {i} (< N) counts, does not trip"
        );
        assert!(!breaker.is_tripped());
    }
    // the Nth consecutive fault trips.
    assert_eq!(
        breaker.observe(AuditOutcome::RecoverableFault),
        BreakerAction::Tripped,
        "the Nth consecutive fault trips the breaker"
    );
    assert!(breaker.is_tripped(), "tripped at exactly N");
    let seen = alarm.incidents();
    assert_eq!(
        seen.len(),
        1,
        "the SYSTEMIC alarm is raised exactly once on the trip"
    );
    assert_eq!(
        seen[0].kind,
        IntegrityKind::AuditBackboneSystemicFailure,
        "the systemic incident is distinguishable from the per-incident AuditWriteFailed kind"
    );
}

// ---- 054 #3 — a clean commit between faults RESETS the consecutive run ---------------------------

#[test]
fn test_success_between_faults_resets_the_counter() {
    // spec(§17) — "consecutive" is the systemic signal, NOT cumulative-ever: a successful audit-commit
    // between faults clears the run. fault, fault, success, fault → counter = 1, not tripped.
    let (breaker, alarm) = breaker_n(5);
    breaker.observe(AuditOutcome::RecoverableFault);
    breaker.observe(AuditOutcome::RecoverableFault);
    assert_eq!(breaker.consecutive(), 2);
    assert_eq!(
        breaker.observe(AuditOutcome::Success),
        BreakerAction::Reset,
        "a clean audit-commit resets the consecutive run"
    );
    assert_eq!(breaker.consecutive(), 0, "the run is cleared to 0");
    breaker.observe(AuditOutcome::RecoverableFault);
    assert_eq!(breaker.consecutive(), 1, "the post-reset run restarts at 1");
    assert!(!breaker.is_tripped());
    assert!(
        alarm.incidents().is_empty(),
        "never tripped → no systemic alarm"
    );
}

// ---- 054 #4 — a clearly-unrecoverable class FAST-TRIPS below N (Q3) ------------------------------

#[test]
fn test_unrecoverable_class_fast_trips_below_n() {
    // spec(§17) — a clearly-unrecoverable fault (disk-full / DB-corruption / IO failure) trips on the
    // FIRST occurrence (well below N); a transient (SQLITE_BUSY / lock) counts toward N but does NOT
    // fast-trip. (The user named these exact cases — task line 512.)
    let (breaker, alarm) = breaker_n(5);
    assert_eq!(
        breaker.observe(AuditOutcome::UnrecoverableFault),
        BreakerAction::Tripped,
        "an unrecoverable fault trips on the FIRST occurrence (1 < N)"
    );
    assert!(breaker.is_tripped());
    assert_eq!(alarm.incidents().len(), 1);
    assert_eq!(
        alarm.incidents()[0].kind,
        IntegrityKind::AuditBackboneSystemicFailure
    );
}

// ---- 054 #4b — the recoverable/unrecoverable classification of an EventStoreError ----------------

#[test]
fn test_classify_audit_fault_recoverable_vs_unrecoverable() {
    // spec(§17) — the Q3 split: SQLITE_FULL (disk full) / SQLITE_CORRUPT / SQLITE_IOERR / SQLITE_NOTADB
    // = UNRECOVERABLE (fast-trip); SQLITE_BUSY / SQLITE_LOCKED = RECOVERABLE (transient, counts toward
    // N). An unknown / non-SQLite write error → RECOVERABLE (conservative — a sustained run still trips
    // at N; never a false fast-trip on an unclassified transient — the per-action deny already protects
    // the invariant on each fault).
    for code in [
        rusqlite::ffi::SQLITE_FULL,
        rusqlite::ffi::SQLITE_CORRUPT,
        rusqlite::ffi::SQLITE_IOERR,
        rusqlite::ffi::SQLITE_NOTADB,
    ] {
        assert_eq!(
            classify_audit_fault(&sqlite_write(code)),
            AuditOutcome::UnrecoverableFault,
            "sqlite code {code} is a clearly-unrecoverable audit-backbone failure"
        );
    }
    for code in [rusqlite::ffi::SQLITE_BUSY, rusqlite::ffi::SQLITE_LOCKED] {
        assert_eq!(
            classify_audit_fault(&sqlite_write(code)),
            AuditOutcome::RecoverableFault,
            "sqlite code {code} is a transient (recoverable) fault — counts toward N, no fast-trip"
        );
    }
    // a non-SqliteFailure write error → recoverable (conservative default).
    assert_eq!(
        classify_audit_fault(&EventStoreError::Write(
            rusqlite::Error::QueryReturnedNoRows
        )),
        AuditOutcome::RecoverableFault,
        "an unclassified write error is recoverable (conservative — sustained still trips at N)"
    );
}

// ---- 054 #5 — the trip LATCHES; a later success does NOT auto-reset it (the §17 never-auto-resolved
//               safety-state ethos, rule #6 family) -----------------------------------------------

#[test]
fn test_tripped_breaker_does_not_auto_reset() {
    // spec(§17, rule #6 family) — once tripped, the breaker stays tripped across ALL subsequent
    // observations (recovery is an explicit operator restart). A success after a trip is IGNORED, and
    // it raises NO second alarm (the trip incident fires exactly once).
    let (breaker, alarm) = breaker_n(2);
    breaker.observe(AuditOutcome::RecoverableFault);
    assert_eq!(
        breaker.observe(AuditOutcome::RecoverableFault),
        BreakerAction::Tripped
    );
    assert!(breaker.is_tripped());
    // a clean commit AFTER the trip must NOT clear the latch.
    assert_eq!(
        breaker.observe(AuditOutcome::Success),
        BreakerAction::AlreadyTripped,
        "a success after a trip is ignored — the latch holds"
    );
    assert!(
        breaker.is_tripped(),
        "still tripped (latched until restart)"
    );
    // even another fault after the trip stays AlreadyTripped (no re-trip, no second alarm).
    assert_eq!(
        breaker.observe(AuditOutcome::UnrecoverableFault),
        BreakerAction::AlreadyTripped
    );
    assert_eq!(
        alarm.incidents().len(),
        1,
        "the systemic alarm fires exactly once (the first trip), never re-raised"
    );
}

// ---- 054 #5b — the default threshold N is a sane sustained-fault floor ---------------------------

#[test]
fn test_default_threshold_is_five() {
    // spec(§17) — the default N rides out a brief transient cluster but trips on a sustained fault
    // (each fault still per-action-denies + alarms). 5 is the documented default (Q2).
    assert_eq!(DEFAULT_AUDIT_BREAKER_THRESHOLD, 5);
    let alarm = Arc::new(RecordingIntegrityAlarm::new());
    let breaker = AuditBackboneBreaker::new(alarm);
    for _ in 0..4 {
        breaker.observe(AuditOutcome::RecoverableFault);
        assert!(
            !breaker.is_tripped(),
            "< 5 consecutive recoverable faults do not trip"
        );
    }
    assert_eq!(
        breaker.observe(AuditOutcome::RecoverableFault),
        BreakerAction::Tripped
    );
}

// =================================================================================================
// WIRING — the daemon-wide feed classification (#6), the intercept path (#8, Q4), and the RULED-(B)
// quiesce-and-refuse gate driven through the REAL write-actor (`run_actor`; #9/#10).
// =================================================================================================

// ---- 054 #6 — the daemon-wide Gateway-Result feed: reset ONLY on a proven audit commit (Ok) ------

#[test]
fn test_observe_gateway_result_resets_only_on_proven_audit_commit() {
    // spec(§17 / call-2) — the consecutive-fault run is "audit faults UNINTERRUPTED by a successful
    // audit COMMIT". `Ok` ⟺ an event was appended = backbone proven healthy → Reset. `AuditWriteFailed`
    // → Count/Trip. EVERY OTHER `Err` (a pre-audit reject: PolicyDenied / Serialize / NotFound …) is
    // `Unaffected` (no-op) — NOT Reset: else an agent spamming denied/uncatalogued calls could MASK a
    // concurrent systemic audit fault (the breaker would never trip). Only a proven commit clears the run.
    let (breaker, _alarm) = breaker_n(5);
    let ok: Result<(), GatewayError> = Ok(());
    assert_eq!(breaker.observe_gateway_result(&ok), BreakerAction::Reset);

    // interleave: a routine policy-deny between two faults does NOT reset the run.
    let denied: Result<(), GatewayError> = Err(GatewayError::PolicyDenied);
    let recoverable: Result<(), GatewayError> = Err(GatewayError::AuditWriteFailed(sqlite_write(
        rusqlite::ffi::SQLITE_BUSY,
    )));
    assert_eq!(
        breaker.observe_gateway_result(&recoverable),
        BreakerAction::Counted
    );
    assert_eq!(
        breaker.observe_gateway_result(&denied),
        BreakerAction::Unaffected,
        "a routine policy-deny is a no-op (NOT a reset — it never touched the audit-write path)"
    );
    assert_eq!(
        breaker.observe_gateway_result(&recoverable),
        BreakerAction::Counted
    );
    assert_eq!(
        breaker.consecutive(),
        2,
        "fault, policy-deny(no-op), fault → consecutive = 2 (the deny did not break the run)"
    );

    // an AuditWriteFailed wrapping an unrecoverable class → fast-trip.
    let audit_fault: Result<(), GatewayError> = Err(GatewayError::AuditWriteFailed(sqlite_write(
        rusqlite::ffi::SQLITE_IOERR,
    )));
    assert_eq!(
        breaker.observe_gateway_result(&audit_fault),
        BreakerAction::Tripped
    );
    assert!(breaker.is_tripped());
}

// ---- 054 #8 — an intercept (call-2) audit-fault feeds the SAME daemon-wide breaker (Q4) ----------

#[test]
fn test_intercept_audit_fault_feeds_the_daemon_wide_breaker() {
    // spec(§17 / call-2 / Q4) — the C2 intercept audit-fault path is part of the audit backbone the
    // breaker observes: `route_intercept_live` under an injected audit-write fault still (a) fails
    // closed to Deny and (b) raises the C2 per-incident alarm — AND now ALSO feeds the daemon-wide
    // breaker (the `Option<&breaker>` param). The injected class is SQLITE_IOERR (unrecoverable) → the
    // intercept fault trips the breaker, proving the C2 intercept path generalizes daemon-wide.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gw();
    let (breaker, systemic) = breaker_n(5); // the daemon-wide breaker + its systemic alarm sink
    let per_incident = RecordingIntegrityAlarm::new(); // the C2 per-incident alarm (route's own sink)
    arm(FaultPoint::AuditEventWrite);
    let outcome = route_intercept_live(&gw, &mut store, &pp("Bash"), &per_incident, Some(&breaker));
    assert!(
        matches!(
            outcome,
            InterceptOutcome::Resolved(MutationVerdict::Deny { .. })
        ),
        "an intercept audit-fault still fails closed to Deny"
    );
    // the C2 per-incident alarm fired (unchanged behavior).
    assert_eq!(per_incident.incidents().len(), 1);
    assert_eq!(
        per_incident.incidents()[0].kind,
        IntegrityKind::AuditWriteFailed
    );
    // and the daemon-wide breaker observed it — an unrecoverable (IOERR) intercept fault trips it.
    assert!(
        breaker.is_tripped(),
        "the intercept audit-fault fed the daemon-wide breaker (generalizes C2 → systemic)"
    );
    assert_eq!(
        systemic.incidents()[0].kind,
        IntegrityKind::AuditBackboneSystemicFailure
    );
}

// ---- 054 #9 [RULED B] — a tripped breaker fail-closed-denies EVERY mutation through the write-actor
//               WITHOUT an audit-write (the production gate; spy the append — NOT called) -----------

#[tokio::test]
async fn test_tripped_breaker_denies_mutations_through_write_actor_without_audit_write() {
    // spec(§15 #5 / §17, RULED B) — driven through the REAL write-actor (run_actor, the production
    // gate+feed seam): once latched, both submit_action AND approve fail-closed-deny with
    // `AuditBackboneDown` WITHOUT attempting an audit-write — the event log is UNCHANGED across the
    // gated calls (the append seam is never reached; "no mutation slips the trip window", pin i).
    let (_d, path) = temp_db();
    let (breaker, _a) = breaker_n(5);
    let actor = WriteActor::spawn_with_alarm_and_breaker(
        open(&path),
        Box::new(FixedClock::new("2026-06-13T00:00:00Z")),
        catalog_gw(),
        recording_sink(),
        breaker.clone(),
    );
    let handle = actor.handle();

    // pre-trip: a mutating tool opens an approval (so there is a live approval_id to gate `approve`).
    let h = handle.clone();
    let ack =
        tokio::task::spawn_blocking(move || h.submit_action_blocking(agent_req("agent.bash")))
            .await
            .unwrap()
            .expect("actor reachable")
            .expect("pre-trip submit");
    assert_eq!(
        ack.status,
        AR::AwaitingApproval,
        "agent.bash (risk-2) awaits approval"
    );
    let appr_id = {
        let conn = open_read_only(&path).expect("read-only conn");
        conn.query_row("SELECT approval_id FROM approvals", [], |r| {
            r.get::<_, String>(0)
        })
        .expect("approval_id")
    };

    // TRIP the breaker (atomically latched on a systemic fault).
    assert_eq!(
        breaker.observe(AuditOutcome::UnrecoverableFault),
        BreakerAction::Tripped
    );
    let baseline = count_ro(&path, "events");

    // submit_action through the actor is gated: AuditBackboneDown, NO audit-write (event log unchanged).
    let h = handle.clone();
    let err = tokio::task::spawn_blocking(move || {
        h.submit_action_blocking(agent_req("agent.todo_write"))
    })
    .await
    .unwrap()
    .expect("actor reachable")
    .expect_err("a tripped breaker refuses a mutation submit");
    assert!(
        matches!(err, GatewayError::AuditBackboneDown),
        "the tripped gate denies with AuditBackboneDown: {err:?}"
    );
    assert_eq!(
        count_ro(&path, "events"),
        baseline,
        "no audit-write attempted on a gated submit (the append seam was never reached)"
    );

    // approve through the actor is gated identically (every mutation path — not just submit).
    let h = handle.clone();
    let appr = appr_id.clone();
    let err = tokio::task::spawn_blocking(move || h.approve_blocking(appr))
        .await
        .unwrap()
        .expect("actor reachable")
        .expect_err("a tripped breaker refuses an approve");
    assert!(
        matches!(err, GatewayError::AuditBackboneDown),
        "the tripped gate denies approve with AuditBackboneDown: {err:?}"
    );
    assert_eq!(
        count_ro(&path, "events"),
        baseline,
        "no audit-write attempted on a gated approve"
    );

    // deny is gated too (the approval is still awaiting — the gated approve never resolved it).
    // (GatewayPlanSubmit shares the BYTE-IDENTICAL gate arm in run_actor — covered by construction;
    // submit/approve/deny exercise the three distinct reachable mutation entries here.)
    let h = handle.clone();
    let appr = appr_id.clone();
    let err = tokio::task::spawn_blocking(move || h.deny_blocking(appr, "op declined".to_string()))
        .await
        .unwrap()
        .expect("actor reachable")
        .expect_err("a tripped breaker refuses a deny");
    assert!(
        matches!(err, GatewayError::AuditBackboneDown),
        "the tripped gate denies deny with AuditBackboneDown: {err:?}"
    );
    assert_eq!(
        count_ro(&path, "events"),
        baseline,
        "no audit-write attempted on a gated deny"
    );

    actor.shutdown().await;
}

// ---- 054 #10 [RULED B] — reads stay LIVE while the breaker is tripped (mutation-only gating) ------

#[tokio::test]
async fn test_reads_stay_live_when_breaker_tripped() {
    // spec(§17, RULED B) — the latch gates ONLY the write-actor mutation path; the read path (read-only
    // WAL conn / projections) is never consulted, so reads stay live: the operator can still SEE the
    // incident. A mutation denies (AuditBackboneDown) while a read of the same store succeeds.
    let (_d, path) = temp_db();
    let (breaker, _a) = breaker_n(5);
    let actor = WriteActor::spawn_with_alarm_and_breaker(
        open(&path),
        Box::new(FixedClock::new("2026-06-13T00:00:00Z")),
        catalog_gw(),
        recording_sink(),
        breaker.clone(),
    );
    let handle = actor.handle();

    // pre-trip: a mutating tool commits an action_requests row (so the post-trip read returns data).
    let h = handle.clone();
    tokio::task::spawn_blocking(move || h.submit_action_blocking(agent_req("agent.bash")))
        .await
        .unwrap()
        .expect("actor reachable")
        .expect("pre-trip submit");

    // TRIP.
    breaker.observe(AuditOutcome::UnrecoverableFault);
    assert!(breaker.is_tripped());

    // a mutation is gated...
    let h = handle.clone();
    let err = tokio::task::spawn_blocking(move || {
        h.submit_action_blocking(agent_req("agent.todo_write"))
    })
    .await
    .unwrap()
    .expect("actor reachable")
    .expect_err("mutation gated while tripped");
    assert!(matches!(err, GatewayError::AuditBackboneDown));

    // ...but a READ stays live (the read path is never gated by the breaker).
    assert!(
        count_ro(&path, "action_requests") >= 1,
        "the pre-trip action is still readable while the breaker is tripped"
    );

    actor.shutdown().await;
}

// ---- 054 #11 [RULED B] — the INTERCEPT path (the 5th mutation entry) is gated-or-fails-closed too --

#[tokio::test]
async fn test_intercept_when_tripped_denies_without_audit_write() {
    // spec(§17 / INV-SEC-1, RULED B) — EVERY mutation path is gated-or-fails-closed when tripped, NOT
    // just submit/approve/deny: an intercepted agent tool-call through the real write-actor is refused
    // (the adjudication route is NEVER entered → no audit-write) → fail-closed Deny. No path persists a
    // mutation without audit while tripped.
    let (_d, path) = temp_db();
    let (breaker, _a) = breaker_n(5);
    let actor = WriteActor::spawn_with_alarm_and_breaker(
        open(&path),
        Box::new(FixedClock::new("2026-06-13T00:00:00Z")),
        catalog_gw(),
        recording_sink(),
        breaker.clone(),
    );
    let handle = actor.handle();

    // TRIP, then snapshot the audit log.
    breaker.observe(AuditOutcome::UnrecoverableFault);
    let baseline = count_ro(&path, "events");

    // an intercepted tool-call through the real write-actor → fail-closed Deny, no audit-write.
    let h = handle.clone();
    let outcome = tokio::task::spawn_blocking(move || h.intercept_blocking(pp("Bash")))
        .await
        .unwrap()
        .expect("actor reachable");
    assert!(
        matches!(
            outcome,
            InterceptOutcome::Resolved(MutationVerdict::Deny { .. })
        ),
        "a tripped breaker refuses the intercepted tool-call (fail-closed Deny)"
    );
    assert_eq!(
        count_ro(&path, "events"),
        baseline,
        "no audit-write attempted on a gated intercept (the adjudication route was never entered)"
    );

    actor.shutdown().await;
}
