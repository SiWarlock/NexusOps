//! P2.4 — the Action Gateway's §17 failure-mode safety behaviors (the deterministic LOGIC + seams;
//! real external re-reads land with the adapters, Phase 3/5/7/8). Exercised via the §14
//! fault-injection hook (`nexusopsd::fault`, the `fault-injection` feature) + a fake clock + a fake
//! `PreconditionOracle` — NO real SIGKILL/threads (a real kill is flaky + un-assertable).
//!
//! **L2 (fail-closed on audit-write, §15/§17 INV-SEC-1):** an audit-required action whose terminal
//! event (`ActionSucceeded`/`ActionFailed`) cannot be written ABORTS — the completion txn rolls back,
//! the action stays `executing` (reconciled on restart by L5), and is NEVER acked succeeded. A side
//! effect APPLIED but its terminal event unwritable → `ActionPartiallySucceeded` (the loud,
//! consumer-visible audit-integrity record) + the action settles `partially_succeeded`, best-effort.

use std::sync::{Arc, Mutex};

use nexusops_shared::actions::{
    ActionError, ActionPreview, ActionRequest, RequesterType, ResourceRef, ResourceType, RiskLevel,
};
use nexusops_shared::events::ActionFailed;
use nexusops_shared::ids::ActionRequestId;
use nexusops_shared::status::ActionRequestStatus;
use nexusops_shared::time::Timestamp;
use nexusopsd::clock::{Clock, FixedClock};
use nexusopsd::eventstore::{EventStore, PrefixRedactor};
use nexusopsd::fault::{arm, arm_n, FaultPoint};
use nexusopsd::gateway::executor::{ActionExecutor, ExecError, ExecutionOutcome, StubExecutor};
use nexusopsd::gateway::policy::CatalogPolicy;
use nexusopsd::gateway::precondition::{PreconditionOracle, PreconditionStatus};
use nexusopsd::gateway::{recovery, Gateway};
use nexusopsd::idgen::UlidGen;
use nexusopsd::locks::{LeaseKind, OwnerId, ResourceId};

// ---- helpers (integration tests are separate crates → the small helpers are per-file, the codebase
// convention; mirrors policy.rs / gateway.rs) ----------------------------------------------------

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    (dir, path)
}

fn open(path: &std::path::Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(FixedClock::new("2026-06-11T00:00:00Z")),
        Box::new(PrefixRedactor),
    )
    .expect("open event store")
}

/// the PRODUCTION Gateway (CatalogPolicy + the side-effect-free StubExecutor).
fn catalog_gateway() -> Gateway {
    Gateway::new(Box::new(CatalogPolicy), Box::new(StubExecutor))
}

fn sample_request(action_type: &str, claimed_risk: RiskLevel) -> ActionRequest {
    ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id: None,
        action_type: action_type.to_string(),
        requester_type: RequesterType::User,
        requester_id: "u_local".to_string(),
        resource_refs: vec![],
        inputs: serde_json::json!({ "k": "v" }),
        risk_level: claimed_risk,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        created_at: Timestamp::parse("2026-06-11T00:00:00Z").unwrap(),
    }
}

/// the `status` of the single action_requests row (each test submits EXACTLY ONE action to a fresh
/// temp DB → the unqualified SELECT is deterministic; the convention shared with policy.rs/gateway.rs).
fn action_status(path: &std::path::Path) -> Option<String> {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row("SELECT status FROM action_requests", [], |r| r.get(0))
        .ok()
}

fn approval_id_of(path: &std::path::Path) -> String {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row("SELECT approval_id FROM approvals", [], |r| r.get(0))
        .expect("approval_id")
}

fn action_event_types(store: &EventStore) -> Vec<String> {
    store
        .read_all()
        .unwrap()
        .iter()
        .filter(|e| e.event_type.starts_with("Action"))
        .map(|e| e.event_type.clone())
        .collect()
}

/// a fake executor that reports a side effect WAS APPLIED (`side_effect_applied: true`) — the L2
/// case-2 lever: the real adapters (Phase 3/5/7/8) report `true` after a durable change; the 2.4
/// stub/catalog executors report `false` (no side effect). Drives the partial-success path.
struct AppliedExecutor;
impl ActionExecutor for AppliedExecutor {
    fn validate(&self, _req: &ActionRequest) -> Result<(), ExecError> {
        Ok(())
    }
    fn execute(&self, req: &ActionRequest) -> ExecutionOutcome {
        ExecutionOutcome::Succeeded {
            changed_resources: req.resource_refs.clone(),
            detail: "fake: side effect APPLIED".to_string(),
            side_effect_applied: true,
        }
    }
    fn preview(&self, req: &ActionRequest, generated_at: Timestamp) -> ActionPreview {
        ActionPreview {
            action_request_id: req.action_request_id.clone(),
            generated_at,
            risk_level: req.risk_level,
            risk_reasons: vec![],
            summary: "applied".to_string(),
            changed_resources: vec![],
            cannot_preview_reason: None,
        }
    }
}

// ---- L2 RED #1 — fail-closed: a risk>=1 action whose terminal event can't be written aborts -------

#[test]
fn audit_write_failure_aborts_risk1_action() {
    // spec(§15/§17 INV-SEC-1) — an audit-required action whose terminal event (ActionSucceeded)
    // cannot be written ABORTS: the completion txn rolls back, the action stays `executing` (the
    // executing-commit already landed; reconciled on restart by L5), NEVER acked succeeded, and no
    // ActionSucceeded persists. The StubExecutor reports NO side effect → clean fail-closed (no
    // ActionPartiallySucceeded). Fault: the §14 TerminalEventWrite checkpoint fails the terminal append.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gateway();

    // risk-1 (session.attach_terminal) → awaiting_approval + an approval row.
    gw.submit_action(
        &mut store,
        sample_request("session.attach_terminal", RiskLevel::Level1),
    )
    .expect("submit");
    assert_eq!(action_status(&path).as_deref(), Some("awaiting_approval"));
    let appr = approval_id_of(&path);

    // arm the audit-write fault, then approve → drive execute → the terminal ActionSucceeded write fails.
    arm(FaultPoint::TerminalEventWrite);
    let err = gw
        .approve(&mut store, &appr)
        .expect_err("the terminal-event write fails → the action is NOT acked succeeded");

    // fail-closed: nothing acked succeeded; the action stays `executing` (txn-A committed it); the
    // success event never persisted (the terminal txn rolled back). The error is the §15/§17 signal.
    assert!(
        matches!(err, nexusopsd::gateway::GatewayError::AuditWriteFailed(_)),
        "fail-closed signal is AuditWriteFailed, got {err:?}"
    );
    assert_eq!(
        action_status(&path).as_deref(),
        Some("executing"),
        "the action stays executing (orphaned → L5 reconciles), NOT succeeded"
    );
    let types = action_event_types(&store);
    assert!(
        types.contains(&"ActionStarted".to_string()),
        "ActionStarted (txn-A) committed before the fault"
    );
    assert!(
        !types.contains(&"ActionSucceeded".to_string()),
        "NO ActionSucceeded persisted — the terminal write failed closed"
    );
    assert!(
        !types.contains(&"ActionPartiallySucceeded".to_string()),
        "the stub applied no side effect → clean rollback, no partial-success record"
    );
}

// ---- L2 RED #2 — side effect applied but the terminal event can't be written → partially_succeeded -

#[test]
fn side_effect_applied_event_unwritable_partially_succeeds() {
    // spec(§17 side-effect-applied record) — a fake executor reports the side effect WAS applied +
    // the terminal ActionSucceeded write fails (the §14 fault). The real-world change can't be rolled
    // back, so the gateway records the divergence BEST-EFFORT: emit ActionPartiallySucceeded (the
    // loud, consumer-visible audit-integrity record) + settle the action `partially_succeeded`.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    // brain.ask is catalog-risk-0 → auto-executes (no approval); the AppliedExecutor reports applied.
    // (Using the risk-0 auto-execute path proves the fail-closed/partial logic is risk-AGNOSTIC: the
    // gateway fail-closes UNIFORMLY across all execute paths — a deliberate over-satisfaction of §17's
    // "audit-required = risk≥1" scoping, NOT an INV-SEC-1 mandate (risk-0 is catalog mutation-free).)
    let gw = Gateway::new(Box::new(CatalogPolicy), Box::new(AppliedExecutor));

    arm(FaultPoint::TerminalEventWrite);
    let ack = gw
        .submit_action(&mut store, sample_request("brain.ask", RiskLevel::Level0))
        .expect("partial success is a settled terminal outcome, acked (not an Err)");

    assert_eq!(
        ack.status,
        ActionRequestStatus::PartiallySucceeded,
        "a side-effect-applied + unwritable-terminal action settles partially_succeeded"
    );
    assert_eq!(
        action_status(&path).as_deref(),
        Some("partially_succeeded"),
        "the row settles partially_succeeded (not stuck executing — the side effect is real + recorded)"
    );
    let types = action_event_types(&store);
    assert!(
        types.contains(&"ActionPartiallySucceeded".to_string()),
        "the ActionPartiallySucceeded audit-integrity record is emitted, got {types:?}"
    );
    assert!(
        !types.contains(&"ActionSucceeded".to_string()),
        "NO ActionSucceeded — the terminal success write failed (that's the whole point)"
    );
}

// ---- L2 RED #3 — audit FULLY broken: even the partial-success record can't be written → fail closed -

#[test]
fn audit_fully_broken_stays_executing_never_partial() {
    // spec(§15/§17 ultimate fail-closed) — a side effect was applied AND both the terminal
    // ActionSucceeded write (txn-B) AND the best-effort ActionPartiallySucceeded record (txn-C) fail.
    // The gateway must NOT claim a partial-success it could not even record: it returns
    // AuditWriteFailed + leaves the action `executing` (orphaned → L5), NO ActionPartiallySucceeded.
    // (Never assert an outcome the audit log doesn't hold — the strongest form of the §17 invariant.)
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = Gateway::new(Box::new(CatalogPolicy), Box::new(AppliedExecutor));

    // count=2 → the fault fires on BOTH the ActionSucceeded (txn-B) AND the ActionPartiallySucceeded
    // (txn-C) terminal appends — models a fully-unavailable audit write.
    arm_n(FaultPoint::TerminalEventWrite, 2);
    let err = gw
        .submit_action(&mut store, sample_request("brain.ask", RiskLevel::Level0))
        .expect_err("audit fully broken → AuditWriteFailed, never a partial-success ack");

    assert!(
        matches!(err, nexusopsd::gateway::GatewayError::AuditWriteFailed(_)),
        "fully-broken audit returns AuditWriteFailed, got {err:?}"
    );
    assert_eq!(
        action_status(&path).as_deref(),
        Some("executing"),
        "stays executing (orphaned → L5) — never partially_succeeded with no record of it"
    );
    let types = action_event_types(&store);
    assert!(
        !types.contains(&"ActionSucceeded".to_string())
            && !types.contains(&"ActionPartiallySucceeded".to_string()),
        "neither terminal record persisted — no outcome is claimed that the log doesn't hold, got {types:?}"
    );
}

// =================================================================================================
// L3 — fencing-conflict (the real 1.4 validate_held; §17 / safety rule #6, Option B).
// =================================================================================================

/// a clock the test advances by setting an explicit RFC3339 (drives lease TTL/expiry). Arc-shared so
/// the test holds one handle while the EventStore holds another — advancing affects both.
#[derive(Clone)]
struct StepClock(Arc<Mutex<String>>);
impl StepClock {
    fn new(ts: &str) -> Self {
        Self(Arc::new(Mutex::new(ts.to_string())))
    }
    fn set(&self, ts: &str) {
        *self.0.lock().unwrap() = ts.to_string();
    }
}
impl Clock for StepClock {
    fn now_rfc3339(&self) -> String {
        self.0.lock().unwrap().clone()
    }
}

/// open a store whose internal clock is the advanceable `StepClock` (so the gateway's lease ops +
/// event timestamps both advance with the test).
fn open_with_clock(path: &std::path::Path, clock: StepClock) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(clock),
        Box::new(PrefixRedactor),
    )
    .expect("open event store")
}

/// a §6.2 ActionRequest fixture that mutates a resource (so the execute path holds a lease over it).
fn sample_request_with_resource(
    action_type: &str,
    claimed_risk: RiskLevel,
    resource_id: &str,
) -> ActionRequest {
    let mut req = sample_request(action_type, claimed_risk);
    req.resource_refs = vec![ResourceRef {
        resource_type: ResourceType::Worktree,
        id: resource_id.to_string(),
        uri: None,
    }];
    req
}

/// the `ActionError` carried on the single action's `ActionFailed` event, if one was emitted.
fn action_failed_error(store: &EventStore) -> Option<ActionError> {
    store
        .read_all()
        .unwrap()
        .iter()
        .find(|e| e.event_type == ActionFailed::EVENT_TYPE)
        .map(|e| {
            serde_json::from_str::<ActionFailed>(&e.payload_json)
                .unwrap()
                .error
        })
}

/// the `preview_json` of the single action_requests row (NULL until a preview is persisted).
fn action_preview_json(path: &std::path::Path) -> Option<String> {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row("SELECT preview_json FROM action_requests", [], |r| {
        r.get::<_, Option<String>>(0)
    })
    .ok()
    .flatten()
}

// ---- L3 RED #1 — a stale fencing token (the resource held by another live owner) fails closed ------

#[test]
fn stale_fencing_token_fails_closed() {
    // spec(§17 / 1.4 Option-B) — when the action's resource is held by a LIVE lease under ANOTHER
    // owner ("stale" = NOT a live lease of OURS — superseded/held-by-another), the execute path's
    // fencing check fails closed: ActionFailed(fencing_conflict), the action is terminal-`failed`,
    // NEVER executed, NEVER auto-resolved (the hard-conflict surface a human resolves).
    let clock = StepClock::new("2026-06-11T00:00:00Z");
    let (_d, path) = temp_db();
    let mut store = open_with_clock(&path, clock.clone());
    let gw = catalog_gateway(); // CatalogPolicy + StubExecutor

    // another owner holds the action's resource (a LIVE lease) — the concurrent-mutation conflict.
    store
        .acquire_lease(
            &ResourceId("wt_x".into()),
            &LeaseKind::resource_mutation(),
            &OwnerId("other_session".into()),
            300,
            &clock,
        )
        .expect("the other owner acquires the resource lease");

    // git.create_worktree is catalog-risk-2 → awaiting_approval → approve drives execute.
    gw.submit_action(
        &mut store,
        sample_request_with_resource("git.create_worktree", RiskLevel::Level2, "wt_x"),
    )
    .expect("submit");
    let appr = approval_id_of(&path);
    // Option B (the leaned design): the fencing conflict is RECORDED as a terminal ActionFailed (the
    // audit + hard-conflict surface) AND the caller is told via a typed GatewayError::FencingConflict
    // (→ §6.4 fencing_conflict, the never-auto-resolved card; the Q7 mapping). Both: audit + caller-error.
    let err = gw
        .approve(&mut store, &appr)
        .expect_err("a fencing conflict refuses the mutation with a typed error");

    assert!(
        matches!(err, nexusopsd::gateway::GatewayError::FencingConflict),
        "the caller gets a typed FencingConflict (→ §6.4 fencing_conflict), got {err:?}"
    );
    assert_eq!(
        action_status(&path).as_deref(),
        Some("failed"),
        "the action is recorded terminal-failed (the audit record of the fenced mutation)"
    );
    assert_eq!(
        action_failed_error(&store),
        Some(ActionError::FencingConflict),
        "the ActionFailed event carries the FencingConflict taxonomy (the hard-conflict surface)"
    );
    assert!(
        !action_event_types(&store).contains(&"ActionSucceeded".to_string()),
        "the action NEVER executed — no ActionSucceeded (fenced before the executor)"
    );
}

// ---- L3 RED #2 — the same-owner re-acquire contract: idempotent renew-like, NOT a conflict --------

#[test]
fn same_owner_reacquire_is_idempotent_renew() {
    // spec(1.4 L1 flag — the gateway-owned contract) — 1.4's `acquire` returns Held{owner:self} on
    // ANY live re-acquire incl. the SAME owner (it deliberately did not pin the semantics). The
    // gateway's contract: a self-owned live lease is idempotent renew-like (a heartbeat), NEVER a
    // fencing_conflict — the holder remains the authority + can renew its own token.
    let clock = StepClock::new("2026-06-11T00:00:00Z");
    let (_d, path) = temp_db();
    let mut store = open_with_clock(&path, clock.clone());
    let owner = OwnerId("act_self".into());
    let res = ResourceId("wt_self".into());
    let kind = LeaseKind::resource_mutation();

    let first = store
        .acquire_lease(&res, &kind, &owner, 60, &clock)
        .unwrap();
    // a same-owner re-acquire of the LIVE lease → 1.4 returns Held{owner:self} (the gateway adjudicates
    // THIS as renew, not conflict — the decision point this layer pins).
    let reacq = store.acquire_lease(&res, &kind, &owner, 60, &clock);
    assert!(
        matches!(reacq, Err(nexusopsd::locks::LeaseError::Held { ref owner_id }) if owner_id == "act_self"),
        "1.4 returns Held{{owner:self}} on a live self re-acquire (got {reacq:?})"
    );
    // pins the LOCKS-LAYER contract the gateway relies on (NOT gateway-path coverage — this calls the
    // 1.4 wrappers directly): the self-owned live lease is STILL the authority (the re-acquire did not
    // break it) and is renewable (idempotent — token unchanged); the gateway maps this `Held{self}` to
    // renew, never fence (the execute-path adjudication is exercised by the held-by-ANOTHER test above).
    assert!(
        store
            .validate_lease_held(&res, &kind, &owner, first.fencing_token, &clock)
            .unwrap(),
        "the self-owned live lease remains the authority"
    );
    let renewed = store
        .renew_lease(&first, 60, &clock)
        .expect("the owner renews its own live lease (the heartbeat)");
    assert_eq!(
        renewed.fencing_token, first.fencing_token,
        "renew keeps the token (idempotent renew, not a fresh acquire)"
    );
}

// ---- L3 RED #3 — a long action that renews before its boundary does not self-fence ----------------

#[test]
fn long_action_renews_before_self_fence() {
    // spec(1.4 strict-boundary flag) — a long-running action that RENEWS strictly BEFORE the
    // `expires_at > now` boundary stays live (does not self-fence on its own expiry). The renew
    // extends `expires_at`; it does not make the lease immortal (past the RENEWED boundary it expires).
    let clock = StepClock::new("2026-06-11T00:00:00Z");
    let (_d, path) = temp_db();
    let mut store = open_with_clock(&path, clock.clone());
    let owner = OwnerId("act_long".into());
    let res = ResourceId("wt_long".into());
    let kind = LeaseKind::resource_mutation();

    let lease = store
        .acquire_lease(&res, &kind, &owner, 60, &clock)
        .unwrap(); // expires 00:01:00
                   // the heartbeat: renew at 00:00:50 (strictly BEFORE the 00:01:00 boundary) → extends to 00:01:50.
    clock.set("2026-06-11T00:00:50Z");
    let renewed = store
        .renew_lease(&lease, 60, &clock)
        .expect("renew strictly before the boundary succeeds");

    // advance PAST the ORIGINAL 00:01:00 boundary — without the renew this would be self-fenced;
    // WITH the renew the lease is still the live authority.
    clock.set("2026-06-11T00:01:30Z");
    assert!(
        store
            .validate_lease_held(&res, &kind, &owner, renewed.fencing_token, &clock)
            .unwrap(),
        "a renewed long action does not self-fence past its ORIGINAL expiry"
    );
    // past the RENEWED 00:01:50 boundary it finally expires (renew extended, did not make immortal).
    clock.set("2026-06-11T00:02:30Z");
    assert!(
        !store
            .validate_lease_held(&res, &kind, &owner, renewed.fencing_token, &clock)
            .unwrap(),
        "past the RENEWED boundary the lease expires (a heartbeat extends, never makes immortal)"
    );
}

// =================================================================================================
// L4 — stale-precondition re-check (§6.2 AG-16.4 / §7.2; the fake PreconditionOracle). After
// lock+fencing (L3), BEFORE execute: re-read the live source; CHANGED → don't execute the now-stale
// mutation → ActionFailed(stale_precondition) + fresh approval required.
// =================================================================================================

/// a fake oracle reporting the live source CHANGED since preview/approval (the L4 stale lever; the real
/// git2 worktree / octocrab PR re-reads land Phase 5/7).
struct ChangedOracle;
impl PreconditionOracle for ChangedOracle {
    fn recheck(&self, _req: &ActionRequest) -> PreconditionStatus {
        PreconditionStatus::Changed
    }
}

// ---- L4 RED #1 — a changed precondition refuses the stale mutation + requires fresh approval --------

#[test]
fn precondition_changed_requires_fresh_approval() {
    // spec(§6.2 AG-16.4 / §7.2) — after lock+fencing, the gateway re-reads the live source; if it
    // CHANGED since the preview/approval, the approved mutation is stale → the gateway does NOT execute
    // a DIFFERENT mutation than was approved: it records ActionFailed(stale_precondition) (action→
    // failed, fresh approval required) + refuses with Err(StalePrecondition) → §6.4 precondition_stale.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = Gateway::with_precondition(
        Box::new(CatalogPolicy),
        Box::new(StubExecutor),
        Box::new(ChangedOracle),
    );

    // session.attach_terminal is catalog-risk-1, no resource_ref → skips L3 fencing, isolating L4.
    gw.submit_action(
        &mut store,
        sample_request("session.attach_terminal", RiskLevel::Level1),
    )
    .expect("submit");
    let appr = approval_id_of(&path);
    let err = gw
        .approve(&mut store, &appr)
        .expect_err("a stale precondition refuses the mutation with a typed error");

    assert!(
        matches!(err, nexusopsd::gateway::GatewayError::StalePrecondition),
        "the caller gets a typed StalePrecondition (→ §6.4 precondition_stale), got {err:?}"
    );
    assert_eq!(
        action_status(&path).as_deref(),
        Some("failed"),
        "the stale action is recorded failed (a fresh approval cycle is required, not this stale one)"
    );
    assert_eq!(
        action_failed_error(&store),
        Some(ActionError::StalePrecondition),
        "the ActionFailed event carries the StalePrecondition taxonomy"
    );
    assert!(
        !action_event_types(&store).contains(&"ActionSucceeded".to_string()),
        "the stale mutation NEVER executed (no ActionSucceeded) — never a different mutation than approved"
    );
    // the preview was REGENERATED before the fail (so the failed action's preview reflects the changed
    // state for the human's fresh decision) — submit persists no preview, so a non-NULL preview_json
    // here proves the stale-path regen ran. (Structural-only in 2.4; a real diff-regen lands Phase 5/7.)
    assert!(
        action_preview_json(&path).is_some(),
        "the preview was regenerated on the stale path (preview_json persisted before the fail)"
    );
}

// ---- L4 RED #2 — an unchanged precondition executes normally (the Null production default) ----------

#[test]
fn precondition_unchanged_executes() {
    // spec(§6.2) — the production-default NullPreconditionOracle reports UNCHANGED → execute proceeds
    // (the L4 seam does NOT break the happy path; the real live-source re-read swaps in Phase 5/7).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gateway(); // Gateway::new defaults to NullPreconditionOracle (Unchanged)
                                // brain.ask is risk-0 → auto-executes via submit (Routed::Execute → execute()), so it DOES reach
                                // the L4 check (the seam runs for every action, not just the approve path) — pinning that the Null
                                // default lets the happy path through.
    let ack = gw
        .submit_action(&mut store, sample_request("brain.ask", RiskLevel::Level0))
        .expect("risk-0 auto-executes");
    assert_eq!(
        ack.status,
        ActionRequestStatus::Succeeded,
        "an unchanged precondition executes through to succeeded"
    );
    assert_eq!(action_status(&path).as_deref(), Some("succeeded"));
}

// =================================================================================================
// L5 — crash-reconcile (the capstone, §17 daemon-crash-mid-action). On restart, the bootstrap scans
// orphaned `executing`/`queued` action rows + drives each to a terminal state, emitting the missing
// terminal event. `executing` (maybe-side-effect) → unknown_outcome + KEEP idempotency_key (dedup
// protects a double-run); `queued` (execute never started → no side effect) → failed + CLEAR the key
// (safe to re-submit). The §14 BeforeExecutingTxn / BeforeTerminalTxn checkpoints simulate the crash.
// =================================================================================================

/// the `idempotency_key` of the single action_requests row (NULL after a queued-orphan reconcile).
fn action_idempotency_key(path: &std::path::Path) -> Option<String> {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row("SELECT idempotency_key FROM action_requests", [], |r| {
        r.get::<_, Option<String>>(0)
    })
    .ok()
    .flatten()
}

// ---- L5 RED #1 — an orphaned `executing` action is reconciled to unknown_outcome on restart ---------

#[test]
fn orphaned_executing_reconciled_on_restart() {
    // spec(§17 daemon-crash-mid-action) — a crash AFTER the executing-commit but BEFORE the terminal
    // event leaves an orphaned `executing` row. On restart, reconcile drives it to a terminal state +
    // emits the missing terminal event. With 2.4's stub executor there is no real side-effect re-read,
    // so the outcome is un-reconcilable → `unknown_outcome` (ActionFailed{UnknownOutcome}, the loud
    // audit-integrity record). The action ends terminal (never stuck `executing`).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gateway();

    // §14 crash mid-execute: brain.ask (risk-0) auto-executes; BeforeTerminalTxn aborts after txn-A.
    arm(FaultPoint::BeforeTerminalTxn);
    let _ = gw.submit_action(&mut store, sample_request("brain.ask", RiskLevel::Level0));
    assert_eq!(
        action_status(&path).as_deref(),
        Some("executing"),
        "the crash left an orphaned `executing` row (txn-A committed, the terminal txn never ran)"
    );

    // restart reconcile.
    let n = recovery::reconcile_orphans(&mut store).expect("reconcile orphans");
    assert_eq!(n, 1, "exactly one orphan reconciled");
    assert_eq!(
        action_status(&path).as_deref(),
        Some("failed"),
        "the orphan is driven to a terminal state (no longer stuck executing)"
    );
    assert_eq!(
        action_failed_error(&store),
        Some(ActionError::UnknownOutcome),
        "an executing orphan is un-reconcilable in 2.4 → unknown_outcome (the loud audit-integrity record)"
    );
    assert!(
        action_event_types(&store).contains(&"ActionFailed".to_string()),
        "the missing terminal event is emitted on reconcile"
    );
}

// ---- L5 RED #2 — N orphans are ALL reconciled (the cascade form) -----------------------------------

#[test]
fn cascade_orphans_all_reconciled() {
    // spec(§17 / 2.1c carry-forward) — the reconcile covers the N-orphan form, not just a single
    // action (a plan cascade strands N steps; the reconcile logic is source-agnostic — exercised here
    // by N aborted submits). Every orphan is driven to terminal; none is left `executing`.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gateway();

    // 3 actions, each crash-aborted mid-execute → 3 orphaned `executing` rows.
    arm_n(FaultPoint::BeforeTerminalTxn, 3);
    for _ in 0..3 {
        let _ = gw.submit_action(&mut store, sample_request("brain.ask", RiskLevel::Level0));
    }
    let executing_before = nexusopsd::eventstore::open_read_only(&path)
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM action_requests WHERE status = 'executing'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap();
    assert_eq!(executing_before, 3, "3 orphaned executing rows");

    let n = recovery::reconcile_orphans(&mut store).expect("reconcile orphans");
    assert_eq!(n, 3, "ALL 3 orphans reconciled (not just one)");
    let executing_after = nexusopsd::eventstore::open_read_only(&path)
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM action_requests WHERE status = 'executing'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap();
    assert_eq!(
        executing_after, 0,
        "no orphan is left executing — all driven to terminal"
    );
}

// ---- L5 RED #3 — a `queued` orphan reconciles to failed + CLEARS the idempotency_key (Q6) ----------

#[test]
fn reconcile_queued_orphan_clears_idempotency_key() {
    // spec(§17 / Q6 dedup-key) — a `queued` orphan (the crash happened after approve+queue but BEFORE
    // execute started → no side effect) reconciles to `failed` and CLEARS its idempotency_key: the
    // action definitively never ran, so it is safe to re-submit (clearing the key avoids the 2.3 dedup
    // re-run lockout). session.send_message is FromInputs-keyed (risk-2 → approve path; FromInputs →
    // no resource-lease fence). (session.create is risk-0 since P4.0b-1, so it no longer takes the
    // approve→queued path this orphan-reconcile test needs — any FromInputs risk-2 type works.)
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gateway();

    gw.submit_action(
        &mut store,
        sample_request("session.send_message", RiskLevel::Level2),
    )
    .expect("submit");
    assert!(
        action_idempotency_key(&path).is_some(),
        "session.send_message derives a FromInputs idempotency_key"
    );
    let appr = approval_id_of(&path);
    // §14 crash BEFORE the executing-commit → leaves the action `queued`.
    arm(FaultPoint::BeforeExecutingTxn);
    let _ = gw.approve(&mut store, &appr);
    assert_eq!(
        action_status(&path).as_deref(),
        Some("queued"),
        "the crash left the action `queued` (the decision txn committed queued; execute never began)"
    );

    let n = recovery::reconcile_orphans(&mut store).expect("reconcile orphans");
    assert_eq!(n, 1);
    assert_eq!(
        action_status(&path).as_deref(),
        Some("failed"),
        "a queued orphan → failed (never ran)"
    );
    assert!(
        action_idempotency_key(&path).is_none(),
        "the idempotency_key is CLEARED — safe to re-submit (no side effect happened)"
    );
}

// ---- L5 RED #4 — an `executing` orphan KEEPS its idempotency_key (Q6 dedup-protect) -----------------

#[test]
fn reconcile_executing_orphan_keeps_idempotency_key() {
    // spec(§17 / Q6 dedup-key) — an `executing` orphan (a crash mid-execute → a side effect MAY have
    // landed) reconciles to unknown_outcome and KEEPS its idempotency_key: dedup must protect against a
    // double-run of a maybe-applied mutation (a re-submit replays the original, not a 2nd execution).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gateway();

    gw.submit_action(
        &mut store,
        sample_request("session.send_message", RiskLevel::Level2),
    )
    .expect("submit");
    let key_before = action_idempotency_key(&path).expect("a FromInputs key");
    let appr = approval_id_of(&path);
    // §14 crash AFTER the executing-commit → leaves the action `executing`.
    arm(FaultPoint::BeforeTerminalTxn);
    let _ = gw.approve(&mut store, &appr);
    assert_eq!(action_status(&path).as_deref(), Some("executing"));

    let n = recovery::reconcile_orphans(&mut store).expect("reconcile orphans");
    assert_eq!(n, 1);
    assert_eq!(
        action_failed_error(&store),
        Some(ActionError::UnknownOutcome)
    );
    assert_eq!(
        action_idempotency_key(&path).as_deref(),
        Some(key_before.as_str()),
        "the idempotency_key is KEPT — dedup protects against a double-run of a maybe-applied mutation"
    );
}
