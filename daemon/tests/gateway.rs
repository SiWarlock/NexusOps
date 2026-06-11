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
    // Denied is a sink in the guard (no legal outgoing) — pins the §5.1-terminal reconcile
    // (status.rs doesn't mark Denied terminal; the guard treats it as one — Step-9 flag).
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
