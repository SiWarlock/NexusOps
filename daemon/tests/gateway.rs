//! P2.1b — single-action Action Gateway pipeline (the INV-SEC-1 mutation chokepoint).
//!
//! L1 = the durable rows (`action_requests`/`approvals`, DATA_MODEL §2.9) + the ActionRequest(15)/
//! Approval(10) transition guards (R-9 legal-edge enforcement). L2/L3 (submit/pipeline/events/
//! INV-SEC-1 + approve/deny/preview) are added after the Step-2.5 architecture review.

use nexusops_shared::status::{ActionRequestStatus as AR, ApprovalStatus as AP};
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{EventStore, PrefixRedactor};
use nexusopsd::idgen::UlidGen;

fn temp_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    (dir, path)
}

/// open a store (runs migrations, incl. the new MIGRATION_7 action_requests/approvals).
fn open(path: &std::path::Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(FixedClock::new("2026-06-11T00:00:00Z")),
        Box::new(PrefixRedactor),
    )
    .expect("open event store")
}

// ---- L1 RED #1 — durable rows match DATA_MODEL §2.9 -----------------------------------------

/// the `(name, type, notnull, pk)` tuples of a table via PRAGMA table_info (column contract).
fn columns(
    conn: &rusqlite::Connection,
    table: &str,
) -> std::collections::BTreeMap<String, (String, bool, bool)> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .expect("table_info");
    let rows = stmt
        .query_map([], |r| {
            let name: String = r.get(1)?;
            let ty: String = r.get(2)?;
            let notnull: i64 = r.get(3)?;
            let pk: i64 = r.get(5)?;
            Ok((name, (ty, notnull != 0, pk != 0)))
        })
        .expect("query table_info")
        .map(|r| r.unwrap())
        .collect();
    rows
}

#[test]
fn test_action_requests_approvals_migrations_match_data_model() {
    // spec(§6.2 / DATA_MODEL §2.9) — the binding DDL: act_/appr_ PKs, the column set, and the
    // ux_action_idem unique partial index on idempotency_key. Opening the store runs MIGRATION_7.
    let (_d, path) = temp_db();
    let _store = open(&path);
    let conn = nexusopsd::eventstore::open_read_only(&path).expect("read-only conn");

    // action_requests — DATA_MODEL §2.9 column set + the act_ PK
    let ar = columns(&conn, "action_requests");
    for col in [
        "action_request_id",
        "project_id",
        "action_type",
        "requester_type",
        "requester_id",
        "resource_refs_json",
        "inputs_json",
        "risk_level",
        "idempotency_key",
        "fencing_token",
        "status",
        "preview_json",
        "created_at",
    ] {
        assert!(ar.contains_key(col), "action_requests.{col} missing");
    }
    assert!(ar["action_request_id"].2, "action_request_id is the PK");
    // every §2.9 NOT NULL column — so the chokepoint can never persist an unattributable row
    for col in [
        "action_type",
        "requester_type",
        "requester_id",
        "resource_refs_json",
        "risk_level",
        "status",
        "created_at",
    ] {
        assert!(ar[col].1, "action_requests.{col} must be NOT NULL");
    }
    // nullable columns stay nullable (project_id/idempotency_key/fencing_token/preview_json/inputs_json)
    assert!(
        !ar["project_id"].1,
        "action_requests.project_id is nullable"
    );
    assert!(
        !ar["idempotency_key"].1,
        "action_requests.idempotency_key is nullable (partial-index dedup)"
    );

    // approvals — DATA_MODEL §2.9 column set + the appr_ PK + the NOT NULL action_request_id FK
    let ap = columns(&conn, "approvals");
    for col in [
        "approval_id",
        "action_request_id",
        "status",
        "required_approver",
        "decided_by",
        "decided_at",
        "expires_at",
        "created_at",
    ] {
        assert!(ap.contains_key(col), "approvals.{col} missing");
    }
    assert!(ap["approval_id"].2, "approval_id is the PK");
    for col in ["status", "created_at"] {
        assert!(ap[col].1, "approvals.{col} must be NOT NULL");
    }
    // 2.1c / MIGRATION_8: action_request_id became NULLABLE (a plan-level approve-all approval has
    // no single action — the frozen §6.2 Approval already typed it Option; the table caught up).
    assert!(
        !ap["action_request_id"].1,
        "approvals.action_request_id is now nullable (the 2.1c plan dimension)"
    );

    // the ux_action_idem index — must be UNIQUE + partial (the uniqueness IS the idempotency-dedup
    // invariant; a regression to a plain CREATE INDEX would silently allow duplicate submits).
    let idx_sql: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='index' AND name='ux_action_idem'",
            [],
            |r| r.get(0),
        )
        .expect("ux_action_idem index exists");
    let up = idx_sql.to_uppercase();
    assert!(
        up.contains("UNIQUE"),
        "ux_action_idem must be UNIQUE (dedup)"
    );
    assert!(up.contains("WHERE"), "ux_action_idem must be partial");
    assert!(
        idx_sql.contains("idempotency_key"),
        "ux_action_idem keys on idempotency_key"
    );
}

// ---- L1 RED #2 — ActionRequest(15) transition guard (R-9 legal/illegal edges) ----------------

#[test]
fn test_action_request_transition_guard_legal_and_illegal() {
    // spec(§5.1 R-9) — the gateway enforces the legal edge set; an illegal edge is a typed error,
    // never silently applied. The canonical happy path + the branch/cancel edges are legal;
    // skipping stages, going backwards, or leaving a terminal state is illegal.
    use nexusopsd::gateway::request::can_transition as t;

    // legal — the canonical happy path (brief RED #2)
    for (from, to) in [
        (AR::Submitted, AR::Previewed),
        (AR::Previewed, AR::PolicyDecided),
        (AR::PolicyDecided, AR::AwaitingApproval),
        (AR::AwaitingApproval, AR::Approved),
        (AR::Approved, AR::Queued),
        (AR::Queued, AR::Executing),
        (AR::Executing, AR::Succeeded),
    ] {
        assert!(t(from, to), "legal edge {from:?}→{to:?} must be accepted");
    }
    // legal — branch edges
    for (from, to) in [
        (AR::PolicyDecided, AR::Queued), // policy=allow skips approval (2.2)
        (AR::PolicyDecided, AR::Denied), // policy deny
        (AR::AwaitingApproval, AR::Denied),
        (AR::AwaitingApproval, AR::Expired),
        (AR::Executing, AR::Failed),
        (AR::Executing, AR::PartiallySucceeded),
    ] {
        assert!(t(from, to), "legal edge {from:?}→{to:?} must be accepted");
    }
    // legal — cancel is reachable from EVERY pre-execution state
    for from in [
        AR::Submitted,
        AR::Previewed,
        AR::PolicyDecided,
        AR::AwaitingApproval,
        AR::Approved,
        AR::Queued,
    ] {
        assert!(t(from, AR::Cancelled), "cancel must be legal from {from:?}");
    }
    // illegal — skipping stages / backwards / leaving a terminal state
    for (from, to) in [
        (AR::Submitted, AR::Succeeded), // skips the whole pipeline
        (AR::Succeeded, AR::Executing), // terminal → no outgoing
        (AR::Executing, AR::Submitted), // backwards
        (AR::Approved, AR::Executing),  // skips queued
        (AR::Cancelled, AR::Queued),    // terminal → no outgoing
        (AR::Expired, AR::Approved),    // terminal → no outgoing
    ] {
        assert!(
            !t(from, to),
            "illegal edge {from:?}→{to:?} must be rejected"
        );
    }
    // Denied is a sink in the guard (no legal outgoing) — consistent with status.rs `is_terminal()`,
    // which now marks Denied terminal (2.1b Option-A reconcile: terminal-by-nature, never executes).
    for to in [
        AR::Approved,
        AR::Queued,
        AR::Executing,
        AR::Cancelled,
        AR::Submitted,
    ] {
        assert!(
            !t(AR::Denied, to),
            "Denied is a sink: Denied→{to:?} rejected"
        );
    }
    // a self-loop is not a transition
    assert!(
        !t(AR::Submitted, AR::Submitted),
        "self-edge is not a transition"
    );
}

// ---- 2.4 L1 RED — the §5.1 rollback transition edges (succeeded/partially → rolled_back/rollback_failed) ----

#[test]
fn test_rollback_transition_edges() {
    // spec(§5.1) — 2.4 adds the rollback edges to the R-9 guard: a settled outcome (succeeded OR
    // partially_succeeded) may transition to rolled_back or rollback_failed (the rollback seam; the
    // default executor rollback fails closed → rollback_failed). rolled_back/rollback_failed are TRUE
    // sinks. Pre-2.4 succeeded had no outgoing edge — a rollback must NOT be a backdoor to re-execute.
    use nexusopsd::gateway::request::can_transition as t;

    // legal — the NEW rollback edges
    for (from, to) in [
        (AR::Succeeded, AR::RolledBack),
        (AR::Succeeded, AR::RollbackFailed),
        (AR::PartiallySucceeded, AR::RolledBack),
        (AR::PartiallySucceeded, AR::RollbackFailed),
    ] {
        assert!(
            t(from, to),
            "rollback edge {from:?}→{to:?} must be legal (2.4)"
        );
    }
    // rolled_back / rollback_failed are TRUE sinks — NO legal outgoing edge to ANY state (iterate the
    // full machine so a future edit that accidentally opens an outgoing edge is caught, incl. backwards
    // edges like RolledBack→Failed/Approved/AwaitingApproval that a 5-state spot-check would miss).
    for from in [AR::RolledBack, AR::RollbackFailed] {
        for &to in AR::ALL {
            assert!(
                !t(from, to),
                "{from:?} is a TRUE sink: {from:?}→{to:?} must be rejected"
            );
        }
    }
    // a rollback is NOT a backdoor to re-execute: a settled outcome can't return to executing/queued
    for (from, to) in [
        (AR::Succeeded, AR::Executing),
        (AR::Succeeded, AR::Queued),
        (AR::PartiallySucceeded, AR::Executing),
    ] {
        assert!(
            !t(from, to),
            "rollback must not re-open execution: {from:?}→{to:?} rejected"
        );
    }
}

// ---- L1 RED #3 — Approval(10) transition guard (R-9) -----------------------------------------

#[test]
fn test_approval_transition_guard() {
    // spec(§5.1 R-9) — the human/policy decision axis: requested→awaiting_approval→
    // approved/denied/expired legal; a post-terminal transition is rejected.
    use nexusopsd::gateway::approval::can_transition as t;

    // legal
    for (from, to) in [
        (AP::Requested, AP::AwaitingApproval),
        (AP::Requested, AP::Previewed),
        (AP::Previewed, AP::AwaitingApproval),
        (AP::AwaitingApproval, AP::Approved),
        (AP::AwaitingApproval, AP::Denied),
        (AP::AwaitingApproval, AP::Expired),
        (AP::AwaitingApproval, AP::Edited),
        (AP::AwaitingApproval, AP::Escalated),
        (AP::Requested, AP::AutoApprovedByPolicy),
        (AP::Requested, AP::Cancelled),
        (AP::Previewed, AP::Cancelled),
    ] {
        assert!(t(from, to), "legal approval edge {from:?}→{to:?}");
    }
    // illegal — post-terminal (every terminal is a sink) + backwards
    for (from, to) in [
        (AP::Approved, AP::Denied),                       // terminal
        (AP::Denied, AP::AwaitingApproval),               // terminal → no outgoing
        (AP::Expired, AP::Approved),                      // terminal
        (AP::Escalated, AP::Approved),                    // terminal → no outgoing
        (AP::AutoApprovedByPolicy, AP::AwaitingApproval), // terminal → no outgoing
        (AP::Edited, AP::Approved),                       // terminal → no outgoing
        (AP::AwaitingApproval, AP::Requested),            // backwards
    ] {
        assert!(!t(from, to), "illegal approval edge {from:?}→{to:?}");
    }
}

// =============================================================================================
// L2 — submit_action + the staged pipeline + the ActionExecution event family + INV-SEC-1.
// The chokepoint: every transition's {durable-row write + authoritative event via the §15 gate}
// is one atomic txn on the single write-actor → fail-closed.
// =============================================================================================

use nexusops_shared::actions::{ActionRequest, RequesterType, RiskLevel};
use nexusops_shared::event_envelope::RedactionStatus;
use nexusops_shared::ids::ActionRequestId;
use nexusops_shared::status::ActionRequestStatus;
use nexusops_shared::time::Timestamp;
use nexusopsd::eventstore::{RedactionOutcome, Redactor};
use nexusopsd::gateway::{Gateway, GatewayError};

/// a test Redactor that refuses to redact — drives the §15 fail-closed gate (RED #6).
struct NeverRedacts;
impl Redactor for NeverRedacts {
    fn redact(&self, payload_json: &str) -> RedactionOutcome {
        RedactionOutcome {
            status: RedactionStatus::Unredacted,
            payload_json: payload_json.to_string(),
            engine_version: "never".to_string(),
            quarantine: None,
        }
    }
}

fn open_with(path: &std::path::Path, redactor: Box<dyn Redactor>) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(FixedClock::new("2026-06-11T00:00:00Z")),
        redactor,
    )
    .expect("open event store")
}

/// the staged-pipeline default: a stub-policy + stub-executor Gateway (require-approval for all
/// until 2.2; no real side effect until 2.3).
fn stub_gateway() -> Gateway {
    Gateway::new(
        Box::new(nexusopsd::gateway::policy::StubPolicy),
        Box::new(nexusopsd::gateway::executor::StubExecutor),
    )
}

fn sample_request(action_type: &str, risk: RiskLevel) -> ActionRequest {
    ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id: None,
        action_type: action_type.to_string(),
        requester_type: RequesterType::User,
        requester_id: "u_local".to_string(),
        resource_refs: vec![],
        inputs: serde_json::json!({ "k": "v" }),
        risk_level: risk,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        created_at: Timestamp::parse("2026-06-11T00:00:00Z").unwrap(),
    }
}

/// COUNT(*) of a table over a read-only connection (sees committed rows).
fn count(path: &std::path::Path, table: &str) -> i64 {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .expect("count")
}

/// the `status` of the single action_requests row, over a read-only connection.
fn action_status(path: &std::path::Path) -> Option<String> {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row("SELECT status FROM action_requests", [], |r| r.get(0))
        .ok()
}

/// the `status` of the single approvals row, over a read-only connection.
fn approval_status(path: &std::path::Path) -> Option<String> {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row("SELECT status FROM approvals", [], |r| r.get(0))
        .ok()
}

// ---- L2 RED #4 — submit persists the row + emits ActionRequested -----------------------------

#[test]
fn test_submit_action_emits_action_requested_and_persists_row() {
    // spec(§6.1/§6.2/AG8.2) — submit_action runs the staged pipeline to awaiting_approval (the
    // stub policy require-approval), persists the action_requests row, and emits ActionRequested.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();
    let req = sample_request("git.create_worktree", RiskLevel::Level2);
    let req_id = req.action_request_id.clone();

    let ack = gw.submit_action(&mut store, req).expect("submit");
    assert_eq!(
        ack.action_request_id,
        req_id.as_str(),
        "ack carries the act_ id"
    );
    assert_eq!(
        ack.status,
        ActionRequestStatus::AwaitingApproval,
        "stub policy → awaiting_approval"
    );

    // the durable row exists at awaiting_approval; an approvals row was opened at awaiting_approval
    assert_eq!(count(&path, "action_requests"), 1);
    assert_eq!(action_status(&path), Some("awaiting_approval".to_string()));
    assert_eq!(count(&path, "approvals"), 1, "an approval was requested");
    assert_eq!(
        approval_status(&path),
        Some("awaiting_approval".to_string()),
        "the approval row is opened awaiting the human"
    );

    // an ActionRequested event exists, correlated to the action_request_id, on the FK column
    let events = store.read_all().unwrap();
    let requested: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == "ActionRequested")
        .collect();
    assert_eq!(requested.len(), 1, "exactly one ActionRequested");
    assert_eq!(
        requested[0].action_request_id.as_ref().map(|x| x.as_str()),
        Some(req_id.as_str()),
        "ActionRequested carries the action_request_id envelope FK"
    );

    // ActionApprovalRequested carries BOTH FK envelope columns (action_request_id + approval_id)
    let approval_req = events
        .iter()
        .find(|e| e.event_type == "ActionApprovalRequested")
        .expect("exactly one ActionApprovalRequested");
    assert_eq!(
        approval_req.action_request_id.as_ref().map(|x| x.as_str()),
        Some(req_id.as_str()),
        "ActionApprovalRequested correlated to the action"
    );
    assert!(
        approval_req
            .approval_id
            .as_deref()
            .is_some_and(|a| a.starts_with("appr_")),
        "ActionApprovalRequested carries the appr_ approval_id envelope FK"
    );
}

// ---- L2 RED #5 — INV-SEC-1: every transition has an event; submit does NOT auto-execute -------

#[test]
fn test_every_mutation_has_an_event_row_and_no_auto_execute() {
    // spec(§15 INV-SEC-1 / §14) — every Gateway state change is recorded as an event via the §15
    // gate (no `unredacted` persists); and submit alone NEVER reaches execution — the approval
    // gate holds, so no executor runs (no ActionStarted/ActionSucceeded) without an approve.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();
    let req = sample_request("project.rescan", RiskLevel::Level1);
    let req_id = req.action_request_id.clone();
    gw.submit_action(&mut store, req).expect("submit");

    let events = store.read_all().unwrap();
    let family: Vec<&str> = events
        .iter()
        .filter(|e| e.event_type.starts_with("Action"))
        .map(|e| e.event_type.as_str())
        .collect();
    // exactly the submit-path milestone events — NO execution events (the gate holds)
    assert!(
        family.contains(&"ActionRequested"),
        "ActionRequested emitted"
    );
    assert!(
        family.contains(&"ActionApprovalRequested"),
        "ActionApprovalRequested emitted"
    );
    for forbidden in ["ActionStarted", "ActionSucceeded", "ActionApproved"] {
        assert!(
            !family.contains(&forbidden),
            "submit must NOT {forbidden} — nothing executes without approval (INV-SEC-1)"
        );
    }
    // every Gateway event is correlated to the action + passed the §15 gate (audited path)
    for e in events.iter().filter(|e| e.event_type.starts_with("Action")) {
        assert_eq!(
            e.action_request_id.as_ref().map(|x| x.as_str()),
            Some(req_id.as_str()),
            "{} correlated to the action_request_id",
            e.event_type
        );
        assert_eq!(
            e.redaction_status,
            RedactionStatus::Redacted,
            "{} persisted only via the §15 redaction gate",
            e.event_type
        );
    }
    // the action rests at awaiting_approval (not executing/succeeded)
    assert_eq!(action_status(&path), Some("awaiting_approval".to_string()));
}

// ---- L2 RED #6 — fail-closed on audit-write -------------------------------------------------

#[test]
fn test_fail_closed_on_audit_write() {
    // spec(§15/§17 fail-closed) — if the authoritative event can't be written (the Redactor refuses
    // → the §15 gate blocks the append), submit aborts with a typed GatewayError and NOTHING
    // persists: no action_requests row, no approvals row, no event (the txn rolled back).
    let (_d, path) = temp_db();
    let mut store = open_with(&path, Box::new(NeverRedacts));
    let gw = stub_gateway();
    let req = sample_request("github.create_pr", RiskLevel::Level3);

    let err = gw
        .submit_action(&mut store, req)
        .expect_err("must fail closed");
    assert!(
        matches!(err, GatewayError::AuditWriteFailed(_)),
        "fail-closed → AuditWriteFailed, got {err:?}"
    );
    // nothing acknowledged, nothing persisted (atomic rollback)
    assert_eq!(count(&path, "action_requests"), 0, "no row persisted");
    assert_eq!(count(&path, "approvals"), 0, "no approval persisted");
    assert_eq!(
        store.read_all().unwrap().len(),
        0,
        "no event persisted (the gate blocked the append)"
    );
}

// ---- L2 RED #7 — the ActionRequested payload passes the §15 redaction gate -------------------

#[test]
fn test_action_requested_payload_redacted() {
    // spec(§15 redaction-before-persist) — the Gateway emits through the same append path, so the
    // event carries redaction_status=redacted + a redaction engine version (never `unredacted`).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();
    gw.submit_action(&mut store, sample_request("brain.ask", RiskLevel::Level0))
        .expect("submit");

    let events = store.read_all().unwrap();
    let requested = events
        .iter()
        .find(|e| e.event_type == "ActionRequested")
        .expect("ActionRequested");
    assert_eq!(requested.redaction_status, RedactionStatus::Redacted);
    assert!(
        requested.redaction_engine_version.is_some(),
        "the redaction engine version is recorded (provenance)"
    );
}

// =============================================================================================
// L3 — approve/deny/preview + execution completion (via the StubExecutor) + CONTRACT 0.16.0.
// The execute phase is a STRUCTURALLY-DISTINCT seam from approve (the executor runs BETWEEN the
// ActionApproved txn and the ActionStarted/Succeeded txn) so 2.3's real executors move off-thread.
// =============================================================================================

/// the `approval_id` of the single approvals row, over a read-only connection.
fn approval_id_of(path: &std::path::Path) -> String {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row("SELECT approval_id FROM approvals", [], |r| r.get(0))
        .expect("an approvals row exists")
}

/// the set of `Action*` event types emitted, in seq order.
fn action_event_types(store: &EventStore) -> Vec<String> {
    store
        .read_all()
        .unwrap()
        .iter()
        .filter(|e| e.event_type.starts_with("Action"))
        .map(|e| e.event_type.clone())
        .collect()
}

// ---- L3 RED #8 — approve drives execution to succeeded -------------------------------------

#[test]
fn test_approve_drives_execute_to_succeeded() {
    // spec(§6.1/AG8.8-8.11) — approve → queued→executing→succeeded via the StubExecutor; the
    // ActionApproved + ActionStarted + ActionSucceeded events; the action ends `succeeded`.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();
    gw.submit_action(
        &mut store,
        sample_request("git.create_worktree", RiskLevel::Level2),
    )
    .expect("submit");
    let appr = approval_id_of(&path);

    let ack = gw.approve(&mut store, &appr).expect("approve");
    assert_eq!(
        ack.status,
        ActionRequestStatus::Succeeded,
        "action succeeded"
    );
    assert_eq!(action_status(&path), Some("succeeded".to_string()));
    assert_eq!(approval_status(&path), Some("approved".to_string()));

    let family = action_event_types(&store);
    for ev in ["ActionApproved", "ActionStarted", "ActionSucceeded"] {
        assert!(family.contains(&ev.to_string()), "{ev} emitted");
    }
    // the approval-decision events carry the approval_id envelope FK
    let approved = store
        .read_all()
        .unwrap()
        .into_iter()
        .find(|e| e.event_type == "ActionApproved")
        .expect("ActionApproved");
    assert!(
        approved.approval_id.as_deref().is_some_and(|a| a == appr),
        "ActionApproved carries the approval_id FK"
    );
}

// ---- L3 RED #9 — deny is terminal; the executor is NOT invoked ------------------------------

#[test]
fn test_deny_with_reason_terminal() {
    // spec(§6.1/AG8.8) — deny → ActionDenied, the action is terminal (`denied`), the executor is
    // NEVER invoked (no ActionStarted/ActionSucceeded).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();
    gw.submit_action(
        &mut store,
        sample_request("github.create_pr", RiskLevel::Level3),
    )
    .expect("submit");
    let appr = approval_id_of(&path);

    let ack = gw
        .deny(&mut store, &appr, "policy: not allowed in MVP")
        .expect("deny");
    assert_eq!(ack.status, ActionRequestStatus::Denied);
    assert_eq!(action_status(&path), Some("denied".to_string()));
    assert_eq!(approval_status(&path), Some("denied".to_string()));

    let family = action_event_types(&store);
    assert!(family.contains(&"ActionDenied".to_string()), "ActionDenied");
    for forbidden in ["ActionStarted", "ActionSucceeded"] {
        assert!(
            !family.contains(&forbidden.to_string()),
            "deny must NOT execute — {forbidden} forbidden"
        );
    }
}

// ---- L3 RED #10 — an expired approval is not executed ---------------------------------------

#[test]
fn test_expired_approval_not_executed() {
    // spec(§17/AG8.8) — approving an approval past its expires_at → ActionExpired; the executor is
    // NEVER invoked. The fixed clock (store-open time) is after the (test-set) past expiry.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();
    gw.submit_action(
        &mut store,
        sample_request("project.rescan", RiskLevel::Level1),
    )
    .expect("submit");
    // test fixture: stamp a PAST expiry on the approval (a denied/expired path needs an expiry; the
    // stub submit opens an open-ended approval). A throwaway writable conn — fixture-only setup.
    {
        let c = rusqlite::Connection::open(&path).expect("fixture conn");
        c.busy_timeout(std::time::Duration::from_secs(2)).unwrap();
        c.execute(
            "UPDATE approvals SET expires_at = '2020-01-01T00:00:00Z'",
            [],
        )
        .expect("stamp past expiry");
    }
    let appr = approval_id_of(&path);

    let ack = gw.approve(&mut store, &appr).expect("approve-expired");
    assert_eq!(ack.status, ActionRequestStatus::Expired, "action expired");
    assert_eq!(action_status(&path), Some("expired".to_string()));
    assert_eq!(approval_status(&path), Some("expired".to_string()));

    let family = action_event_types(&store);
    assert!(
        family.contains(&"ActionExpired".to_string()),
        "ActionExpired"
    );
    for forbidden in ["ActionStarted", "ActionSucceeded"] {
        assert!(
            !family.contains(&forbidden.to_string()),
            "an expired approval must NOT execute — {forbidden} forbidden"
        );
    }
}

// ---- L3 RED #11 — preview_action returns a stub preview envelope ----------------------------

#[test]
fn test_preview_action_returns_catalog_preview() {
    // spec(§6.1/§6.2/§6.3) — preview_action returns a catalog-class ActionPreview (2.3 L2 — the typed
    // preview framework, no longer the 2.1b stub). It references the action + carries a class-specific
    // summary; in 2.3 every preview is structural-only (no real adapter), so cannot_preview_reason is
    // set (the detailed dispatch/escalation/persist contract is pinned in tests/executor.rs #7-9).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();
    let ack = gw
        .submit_action(&mut store, sample_request("brain.ask", RiskLevel::Level0))
        .expect("submit");

    let preview = gw
        .preview_action(&mut store, &ack.action_request_id)
        .expect("preview");
    assert_eq!(
        preview.action_request_id.as_str(),
        ack.action_request_id,
        "preview references the action"
    );
    assert!(!preview.summary.is_empty(), "the preview carries a summary");
    assert!(
        preview.cannot_preview_reason.is_some(),
        "2.3 has no real adapter → every preview is structural-only (cannot_preview_reason set)"
    );
}

// ---- L3 — §15/rule-#4: inputs are redacted at rest in the registry row ----------------------

#[test]
fn test_submit_redacts_inputs_at_rest() {
    // spec(§15 / rule-#4) — the open `inputs` blob comes from UNTRUSTED proposers; the Gateway runs
    // the SAME §15 Redactor over it before the action_requests row persists, so a secret cannot
    // land unredacted at rest. A normal (non-secret) input is a no-op (the redactor masks in place).
    // Uses the real PrefixRedactor (the `open` helper) — the masking path, not the test stub.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();

    // a 40-char high-entropy KEY=value secret in BOTH the open inputs blob AND a resource_ref uri
    // (e.g. a token in a git-remote URL) — the §15 row-redaction gate covers every caller-supplied
    // registry-row payload column (inputs_json + resource_refs_json), not just inputs.
    let secret = "Zx9Kq2Lm7Wp4Rn1Vc6Bt3Hy8Fd5Gj0Aa2Ss4Dd";
    let ref_secret = "Pq8Wm3Nx7Kc2Vb5Rt9Hy4Fd6Gj1Bb3Ss5Dd7Aa";
    let mut req = sample_request("brain.send_raw_transcript_to_cloud", RiskLevel::Level4);
    req.inputs = serde_json::json!({ "env": format!("API_SECRET={secret}") });
    req.resource_refs = vec![nexusops_shared::actions::ResourceRef {
        resource_type: nexusops_shared::actions::ResourceType::Repo,
        id: "repo_01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
        uri: Some(format!("https://github.com/x/y?TOKEN={ref_secret}")),
    }];
    gw.submit_action(&mut store, req).expect("submit");

    let conn = nexusopsd::eventstore::open_read_only(&path).unwrap();
    let (inputs_json, resource_refs_json): (String, String) = conn
        .query_row(
            "SELECT inputs_json, resource_refs_json FROM action_requests",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert!(
        !inputs_json.contains(secret),
        "the inputs secret must be masked at rest: {inputs_json}"
    );
    assert!(
        inputs_json.contains("[REDACTED]"),
        "inputs masked in place: {inputs_json}"
    );
    assert!(
        !resource_refs_json.contains(ref_secret),
        "the resource_ref uri secret must be masked at rest: {resource_refs_json}"
    );
    assert!(
        resource_refs_json.contains("[REDACTED]"),
        "resource_refs masked in place: {resource_refs_json}"
    );

    // a non-secret input is a no-op — the redactor never corrupts a legitimate keychain-ref/value
    let (_d2, path2) = temp_db();
    let mut store2 = open(&path2);
    let mut req2 = sample_request("git.create_worktree", RiskLevel::Level2);
    req2.inputs = serde_json::json!({ "branch": "feature/x", "base": "main" });
    gw.submit_action(&mut store2, req2).expect("submit2");
    let conn2 = nexusopsd::eventstore::open_read_only(&path2).unwrap();
    let inputs2: String = conn2
        .query_row("SELECT inputs_json FROM action_requests", [], |r| r.get(0))
        .unwrap();
    assert!(
        inputs2.contains("feature/x") && inputs2.contains("main"),
        "a non-secret input survives unchanged (no-op): {inputs2}"
    );
}

// =============================================================================================
// P2.1c L1 — the proj_approval_queue projector (Human Input Queue read model) + subscribe-delta.
// The projector folds the Gateway's approval events into proj_approval_queue: status from the
// EVENT TYPE, immutable fields (risk/requester/project/expires_at) sibling-read from the
// action_requests/approvals rows (rebuild-safe — those never change post-submit). AG §8/§14.3.
// =============================================================================================

/// one proj_approval_queue row by approval_id: (status, risk_level, requester_type, requester_id,
/// expires_at, sort_key) over a read-only connection. None if absent.
#[allow(clippy::type_complexity)]
fn queue_row(
    path: &std::path::Path,
    approval_id: &str,
) -> Option<(String, i64, String, String, Option<String>, Option<String>)> {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row(
        "SELECT status, risk_level, requester_type, requester_id, expires_at, sort_key \
         FROM proj_approval_queue WHERE approval_id = ?1",
        [approval_id],
        |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        },
    )
    .ok()
}

/// COUNT(*) of proj_approval_queue rows.
fn queue_count(path: &std::path::Path) -> i64 {
    count(path, "proj_approval_queue")
}

/// a stable fingerprint of the whole approval-queue surface (for rebuild-equivalence).
fn queue_fingerprint(path: &std::path::Path) -> String {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    let mut stmt = conn
        .prepare(
            "SELECT approval_id, status, risk_level, sort_key FROM proj_approval_queue \
             ORDER BY approval_id",
        )
        .unwrap();
    let rows: Vec<String> = stmt
        .query_map([], |r| {
            Ok(format!(
                "{}|{}|{}|{}",
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, Option<String>>(3)?.unwrap_or_default(),
            ))
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    rows.join(",")
}

// ---- L1 RED #1 — fold a single-action approval into the open queue ---------------------------

#[test]
fn test_approval_queue_folds_single_action_approval() {
    // spec(§7 / AG §8) — submit_action (ActionRequested + ActionApprovalRequested) folds ONE
    // proj_approval_queue row at awaiting_approval; risk_level/requester_type/requester_id come
    // from the action_requests row, expires_at from the approval (the in-band §7 fold).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();
    gw.submit_action(
        &mut store,
        sample_request("git.create_worktree", RiskLevel::Level2),
    )
    .expect("submit");

    assert_eq!(
        queue_count(&path),
        1,
        "one queue row for the single approval"
    );
    let appr = approval_id_of(&path);
    let (status, risk, rtype, rid, _exp, sort_key) =
        queue_row(&path, &appr).expect("a proj_approval_queue row");
    assert_eq!(status, "awaiting_approval", "open at awaiting_approval");
    assert_eq!(risk, 2, "risk_level sibling-read from action_requests");
    assert_eq!(
        rtype, "user",
        "requester_type sibling-read from action_requests"
    );
    assert_eq!(
        rid, "u_local",
        "requester_id sibling-read from action_requests"
    );
    // sort_key = "{4-risk}_{requested_at}" — risk-2 → the "2_" rank prefix (not merely non-NULL).
    assert!(
        sort_key.as_deref().is_some_and(|k| k.starts_with("2_")),
        "sort_key carries the 4-risk rank prefix (risk 2 → '2_'): {sort_key:?}"
    );
}

// ---- L1 RED #2b — deny + expire advance the queue status (the resolution event types) --------

#[test]
fn test_approval_queue_folds_deny_and_expire() {
    // spec(§5.1 Approval / AG §14.3) — the projector advances the row's status for EVERY
    // resolution event type, derived from the event: ActionDenied→denied, ActionExpired→expired
    // (sibling of the approve→approved path). Both rows stay for history.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();

    // deny path
    gw.submit_action(&mut store, sample_request("a", RiskLevel::Level3))
        .expect("submit a");
    let appr_a = approval_id_of(&path);
    gw.deny(&mut store, &appr_a, "not allowed in MVP")
        .expect("deny");
    let (status_a, ..) = queue_row(&path, &appr_a).expect("queue row a");
    assert_eq!(status_a, "denied", "ActionDenied → denied (event-derived)");

    // expire path — a second action with a past expiry stamped on its approval, then approve→expire
    let (_d2, path2) = temp_db();
    let mut store2 = open(&path2);
    gw.submit_action(&mut store2, sample_request("b", RiskLevel::Level1))
        .expect("submit b");
    let appr_b = approval_id_of(&path2);
    {
        let c = rusqlite::Connection::open(&path2).expect("fixture conn");
        c.busy_timeout(std::time::Duration::from_secs(2)).unwrap();
        c.execute(
            "UPDATE approvals SET expires_at = '2020-01-01T00:00:00Z'",
            [],
        )
        .expect("stamp past expiry");
    }
    gw.approve(&mut store2, &appr_b).expect("approve-expired");
    let (status_b, ..) = queue_row(&path2, &appr_b).expect("queue row b");
    assert_eq!(
        status_b, "expired",
        "ActionExpired → expired (event-derived)"
    );
}

// ---- L1 RED #2 — resolve advances the status; the row leaves the OPEN queue (Q4) -------------

#[test]
fn test_approval_queue_status_advances_and_leaves_on_resolve() {
    // spec(§5.1 Approval / AG §14.3) — approve advances the row's status; a resolved approval
    // leaves the OPEN queue (status != awaiting_approval) but the row STAYS for history (Q4 —
    // status-marked, not deleted). status is derived from the EVENT TYPE, never the registry value.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();
    gw.submit_action(
        &mut store,
        sample_request("git.create_worktree", RiskLevel::Level2),
    )
    .expect("submit");
    let appr = approval_id_of(&path);
    gw.approve(&mut store, &appr).expect("approve");

    assert_eq!(queue_count(&path), 1, "the row stays for history (Q4)");
    let (status, ..) = queue_row(&path, &appr).expect("queue row");
    assert_eq!(
        status, "approved",
        "the resolved approval leaves the OPEN queue (status advanced from the event)"
    );
}

// ---- L1 RED #3 — sort_key orders risk-DESC then age (oldest-first) ---------------------------

#[test]
fn test_approval_queue_sort_key_risk_desc_then_age() {
    // spec(AG §8) — the Human Input Queue ranks risk-DESC then age-ASC; the projector populates
    // sort_key so a single `ORDER BY sort_key` yields that order.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();
    gw.submit_action(&mut store, sample_request("low", RiskLevel::Level1))
        .expect("submit low");
    gw.submit_action(&mut store, sample_request("high", RiskLevel::Level3))
        .expect("submit high");

    let conn = nexusopsd::eventstore::open_read_only(&path).unwrap();
    let ordered: Vec<i64> = {
        let mut stmt = conn
            .prepare("SELECT risk_level FROM proj_approval_queue ORDER BY sort_key")
            .unwrap();
        stmt.query_map([], |r| r.get::<_, i64>(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    };
    assert_eq!(
        ordered,
        vec![3, 1],
        "ORDER BY sort_key yields risk-DESC (3 before 1)"
    );
}

// ---- L1 RED #4 — rebuild reproduces identical proj_approval_queue state (§7.2) ---------------

#[test]
fn test_approval_queue_rebuild_equivalent() {
    // spec(§7.2) — rebuild() reproduces identical proj_approval_queue state: status re-derived
    // from the event stream, immutable fields re-read from the action_requests/approvals registry
    // rows (which rebuild preserves — they are NOT in REBUILD_TABLES). Deterministic fold.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = stub_gateway();
    gw.submit_action(&mut store, sample_request("a", RiskLevel::Level2))
        .expect("s1");
    let appr = approval_id_of(&path);
    gw.approve(&mut store, &appr).expect("approve"); // resolved row stays, status=approved
    gw.submit_action(&mut store, sample_request("b", RiskLevel::Level3))
        .expect("s2"); // a second, still-open approval

    let before = queue_fingerprint(&path);
    // guard against a vacuous pass (empty == empty): there must be rows to compare.
    assert_eq!(
        queue_count(&path),
        2,
        "two approval rows folded (one resolved, one open) before the rebuild"
    );
    store.rebuild_projections().unwrap();
    let after = queue_fingerprint(&path);
    assert_eq!(
        before, after,
        "full rebuild reproduces the incremental approval-queue state"
    );
}
