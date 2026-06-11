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
    for col in ["action_request_id", "status", "created_at"] {
        assert!(ap[col].1, "approvals.{col} must be NOT NULL");
    }

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

/// the staged-pipeline default: a stub-policy Gateway (require-approval for all, until 2.2).
fn stub_gateway() -> Gateway {
    Gateway::new(Box::new(nexusopsd::gateway::policy::StubPolicy))
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
