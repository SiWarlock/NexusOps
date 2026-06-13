//! P2.3 — the Gateway executor + preview + idempotency FRAMEWORK (realizing the §6.3 catalog's
//! named-only `preview_class`/`executor`/`idempotency_formula` bindings).
//!
//! **L1 (this layer):** idempotency-key derivation per the catalog `IdempotencyFormula` (catalog-
//! authoritative, NOT requester-supplied — the same recorded-not-trusted posture as 2.2's risk) +
//! dedup-on-submit (reuses the existing `ux_action_idem` UNIQUE index; no migration). The key for a
//! `FromInputs` action is a ONE-WAY hash over the RAW inputs (§15-safe: the raw inputs may carry a
//! secret, and the key persists unredacted into the indexed `idempotency_key` column — so it must be
//! irreversible; a redacted-input key would falsely dedup two actions that differ only in their
//! secret). L2 = the preview framework; L3 = the executor trait + dispatch.

use nexusops_shared::actions::{
    ActionPreview, ActionRequest, RequesterType, ResourceRef, ResourceType, RiskLevel,
};
use nexusops_shared::catalog;
use nexusops_shared::ids::{ActionRequestId, ProjectId};
use nexusops_shared::status::ActionRequestStatus;
use nexusops_shared::time::Timestamp;
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{EventStore, PrefixRedactor};
use nexusopsd::gateway::executor::{ActionExecutor, CatalogExecutor, ExecError, ExecutionOutcome};
use nexusopsd::gateway::idempotency::derive_key;
use nexusopsd::gateway::policy::CatalogPolicy;
use nexusopsd::gateway::preview::generate_preview;
use nexusopsd::gateway::Gateway;
use nexusopsd::idgen::UlidGen;
use serde_json::json;
use std::sync::{Arc, Mutex};

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

/// the production Gateway (catalog policy + the catalog-driven executor framework — L3).
fn catalog_gateway() -> Gateway {
    Gateway::new(Box::new(CatalogPolicy), Box::new(CatalogExecutor::new()))
}

/// an ActionRequest fixture with a given `action_type`, optional `project`, and `inputs`.
fn req_with(
    action_type: &str,
    project: Option<ProjectId>,
    inputs: serde_json::Value,
) -> ActionRequest {
    ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id: project,
        action_type: action_type.to_string(),
        requester_type: RequesterType::User,
        requester_id: "u_local".to_string(),
        resource_refs: vec![],
        inputs,
        risk_level: RiskLevel::Level0,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        created_at: Timestamp::parse("2026-06-11T00:00:00Z").unwrap(),
    }
}

/// an ActionRequest fixture with the given `action_type` + `resource_refs` carrying the given ids.
fn req_with_refs(action_type: &str, ref_ids: &[&str]) -> ActionRequest {
    let mut req = req_with(action_type, None, serde_json::json!({}));
    req.resource_refs = ref_ids
        .iter()
        .map(|id| ResourceRef {
            resource_type: ResourceType::Branch,
            id: id.to_string(),
            uri: None,
        })
        .collect();
    req
}

fn count(path: &std::path::Path, table: &str) -> i64 {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .expect("count")
}

/// the `idempotency_key` of the single action_requests row (None when NULL).
fn idem_key_of(path: &std::path::Path) -> Option<String> {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row("SELECT idempotency_key FROM action_requests", [], |r| {
        r.get::<_, Option<String>>(0)
    })
    .expect("a row")
}

fn requested_event_count(store: &EventStore) -> usize {
    store
        .read_all()
        .expect("read all events")
        .iter()
        .filter(|e| e.event_type == "ActionRequested")
        .count()
}

// ---- L1 RED #1 — FromInputs key is deterministic over {action_type, project_id, inputs} ---------

#[test]
fn idem_key_from_inputs_is_deterministic() {
    // spec(§6.3 FromInputs) — two requests with identical {action_type, project_id, inputs} derive
    // the SAME key (the differing action_request_id must NOT affect it); differing inputs differ.
    let entry =
        catalog::lookup("session.create").expect("session.create is catalogued (FromInputs)");
    let proj = Some(ProjectId::new());
    let inputs = serde_json::json!({ "name": "x", "n": 1 });

    let k1 = derive_key(
        &req_with("session.create", proj.clone(), inputs.clone()),
        &entry,
    );
    let k2 = derive_key(
        &req_with("session.create", proj.clone(), inputs.clone()),
        &entry,
    );
    assert!(k1.is_some(), "FromInputs derives a key");
    assert_eq!(k1, k2, "same logical FromInputs action → same key");

    let k3 = derive_key(
        &req_with(
            "session.create",
            proj.clone(),
            serde_json::json!({ "name": "y" }),
        ),
        &entry,
    );
    assert_ne!(k1, k3, "different inputs → different key");

    // canonicalization: key order in the inputs object must NOT change the key.
    let k4 = derive_key(
        &req_with(
            "session.create",
            proj,
            serde_json::json!({ "n": 1, "name": "x" }),
        ),
        &entry,
    );
    assert_eq!(k1, k4, "input key order is canonicalized (same key)");
}

// ---- L1 RED #2 — NaturalResourceRef key over {action_type, sorted resource_ref ids} ------------

#[test]
fn idem_key_natural_resource_ref() {
    // spec(§6.3 NaturalResourceRef) — the key is over {action_type, SORTED resource_ref ids}; ref
    // order doesn't matter; differing refs differ.
    let entry =
        catalog::lookup("git.create_branch").expect("git.create_branch is catalogued (NaturalRef)");
    let k1 = derive_key(
        &req_with_refs("git.create_branch", &["res_a", "res_b"]),
        &entry,
    );
    let k2 = derive_key(
        &req_with_refs("git.create_branch", &["res_b", "res_a"]),
        &entry,
    );
    assert!(k1.is_some(), "NaturalResourceRef derives a key");
    assert_eq!(
        k1, k2,
        "resource-ref order does not affect the key (sorted)"
    );

    let k3 = derive_key(
        &req_with_refs("git.create_branch", &["res_a", "res_c"]),
        &entry,
    );
    assert_ne!(k1, k3, "different resource refs → different key");
}

// ---- L1 RED #3 — a None-formula action derives no key (NULL column) ----------------------------

#[test]
fn idem_formula_none_yields_no_key() {
    // spec(§6.3 None) — a None-formula action (project.rescan) derives no key, and its persisted
    // action_requests.idempotency_key is NULL.
    let entry = catalog::lookup("project.rescan").expect("project.rescan is catalogued (None)");
    assert_eq!(
        derive_key(
            &req_with("project.rescan", None, serde_json::json!({})),
            &entry
        ),
        None,
        "None formula → no derived key"
    );

    // and it persists as NULL through submit (project.rescan is risk-0 → auto-executes).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gateway();
    gw.submit_action(
        &mut store,
        req_with("project.rescan", None, serde_json::json!({})),
    )
    .expect("submit");
    assert_eq!(
        idem_key_of(&path),
        None,
        "the persisted idempotency_key is NULL"
    );
}

// ---- L1 RED #4 — a duplicate keyed submit dedups to the original (at-most-one) -----------------

#[test]
fn duplicate_submit_dedups_to_original() {
    // spec(§6.3 idempotency / at-most-one) — a second submit whose derived key already exists returns
    // the ORIGINAL action's id + status, creates NO 2nd action_requests row + NO 2nd ActionRequested.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gateway();
    let proj = Some(ProjectId::new());
    let inputs = serde_json::json!({ "name": "x" });

    // session.create is FromInputs + risk-2 (require_approval → rests at awaiting_approval).
    let ack1 = gw
        .submit_action(
            &mut store,
            req_with("session.create", proj.clone(), inputs.clone()),
        )
        .expect("first submit");
    let ack2 = gw
        .submit_action(&mut store, req_with("session.create", proj, inputs))
        .expect("duplicate submit dedups (does not error)");

    assert_eq!(
        ack2.action_request_id, ack1.action_request_id,
        "the duplicate submit returns the ORIGINAL action_request_id"
    );
    assert_eq!(
        ack2.status, ack1.status,
        "the dedup reply carries the original action's current status"
    );
    assert_eq!(
        count(&path, "action_requests"),
        1,
        "no 2nd action_requests row"
    );
    assert_eq!(
        requested_event_count(&store),
        1,
        "no 2nd ActionRequested event"
    );
}

// ---- L1 RED #5 — a None-formula action is never deduped (fresh each submit) --------------------

#[test]
fn none_formula_resubmit_creates_fresh() {
    // spec(§6.3 None) — only keyed actions dedup; a None-formula action (project.rescan) submitted
    // twice creates two DISTINCT actions (the partial index ignores NULL keys).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gateway();

    let ack1 = gw
        .submit_action(
            &mut store,
            req_with("project.rescan", None, serde_json::json!({})),
        )
        .expect("first submit");
    let ack2 = gw
        .submit_action(
            &mut store,
            req_with("project.rescan", None, serde_json::json!({})),
        )
        .expect("second submit");

    assert_ne!(
        ack1.action_request_id, ack2.action_request_id,
        "a None-formula action is never deduped — a fresh action each time"
    );
    assert_eq!(
        count(&path, "action_requests"),
        2,
        "two distinct actions persisted"
    );
}

// ---- L1 RED #6 — the derived key OVERRIDES any requester-supplied idempotency_key ---------------

#[test]
fn idem_key_overrides_requester_supplied() {
    // spec(§15 recorded-not-trusted) — the gateway DERIVES the key (catalog formula + inputs) and
    // IGNORES any requester-supplied idempotency_key. A proposer must not control the dedup key: a
    // chosen key could suppress a victim's action via a forced collision, or evade dedup entirely
    // (the same recorded-not-trusted posture as 2.2's risk reconcile).
    let entry = catalog::lookup("session.create").unwrap();
    let proj = Some(ProjectId::new());
    let inputs = serde_json::json!({ "name": "x" });
    let derived = derive_key(
        &req_with("session.create", proj.clone(), inputs.clone()),
        &entry,
    );

    // (a) a FromInputs action: the requester's bogus key is replaced by the catalog-derived key.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gateway();
    let mut req = req_with("session.create", proj, inputs);
    req.idempotency_key = Some("attacker-supplied-key".to_string());
    gw.submit_action(&mut store, req).expect("submit");
    assert_eq!(
        idem_key_of(&path),
        derived,
        "the persisted key is the catalog-derived key, not the requester's claim"
    );

    // (b) a None-formula action: a requester-supplied key cannot inject a dedup key (stays NULL).
    let (_d2, path2) = temp_db();
    let mut store2 = open(&path2);
    let gw2 = catalog_gateway();
    let mut req2 = req_with("project.rescan", None, serde_json::json!({}));
    req2.idempotency_key = Some("attacker-supplied-key".to_string());
    gw2.submit_action(&mut store2, req2).expect("submit");
    assert_eq!(
        idem_key_of(&path2),
        None,
        "a None-formula action ignores the requester-supplied key → NULL"
    );
}

// =================================================================================================
// L2 — the preview framework: dispatch by the catalog preview_class → a typed ActionPreview rendered
// into the FROZEN flat envelope; cannot_preview_reason + risk-escalation when the real dry-run
// capability is absent (2.3 has no namespace adapters); persist preview_json.
// =================================================================================================

/// the `preview_json` of the single action_requests row (None when NULL).
fn preview_json_of(path: &std::path::Path) -> Option<String> {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row("SELECT preview_json FROM action_requests", [], |r| {
        r.get::<_, Option<String>>(0)
    })
    .expect("a row")
}

fn ts() -> Timestamp {
    Timestamp::parse("2026-06-11T00:00:00Z").unwrap()
}

// ---- L2 RED #7 — generate_preview dispatches by the catalog preview_class --------------------------

#[test]
fn preview_dispatches_by_catalog_class() {
    // spec(§6.2/§6.3) — generate_preview dispatches by the catalog preview_class → a class-specific
    // summary + changed_resources, NOT the 2.1b stub text. Two different classes → different summaries.
    let git_entry = catalog::lookup("git.create_worktree").unwrap(); // Git class
    let diff_entry = catalog::lookup("git.diff").unwrap(); // Diff class
    let git_req = req_with_refs("git.create_worktree", &["wt_1"]);
    let diff_req = req_with_refs("git.diff", &["repo_1"]);

    let git_preview = generate_preview(&git_req, &git_entry, ts());
    let diff_preview = generate_preview(&diff_req, &diff_entry, ts());

    assert!(
        !git_preview.summary.to_lowercase().contains("stub"),
        "the rendered summary is class-specific, not the 2.1b stub text"
    );
    assert_ne!(
        git_preview.summary, diff_preview.summary,
        "dispatch by class → distinct class-specific summaries"
    );
    assert!(
        diff_preview.summary.to_lowercase().contains("diff"),
        "the Diff-class summary reflects its class"
    );
    assert!(
        git_preview.summary.to_lowercase().contains("git"),
        "the Git-class summary reflects its class"
    );
    assert_eq!(
        diff_preview.changed_resources, diff_req.resource_refs,
        "changed_resources are populated from the action's resource_refs"
    );
}

// ---- L2 RED #8 — an impossible preview sets cannot_preview_reason + escalates the preview risk -----

#[test]
fn preview_impossible_sets_reason_and_escalates_risk() {
    // spec(§6.2 "preview-impossible escalates risk + sets cannotPreviewReason") — a preview whose real
    // dry-run capability is absent in 2.3 (no namespace adapter) sets cannot_preview_reason, escalates
    // the PREVIEW risk_level ABOVE the catalog-authoritative risk, and records a risk_reasons entry.
    // The escalation is on the preview envelope ONLY (it never lowers the policy risk).
    //
    // FORWARD-NOTE: in 2.3 EVERY preview is impossible (no adapter) — a 2.3 transient. When a phase
    // lands its real adapter (Phase 3/5/7/8), git.create_worktree becomes genuinely previewable;
    // RE-TARGET this test to a still-unpreviewable class then (do NOT delete the impossible-preview
    // contract — it must hold for any genuinely-unpreviewable action).
    let entry = catalog::lookup("git.create_worktree").unwrap(); // catalog risk-2
    let req = req_with_refs("git.create_worktree", &["wt_1"]);
    let preview = generate_preview(&req, &entry, ts());

    assert!(
        preview.cannot_preview_reason.is_some(),
        "a preview the 2.3 framework cannot dry-run sets cannot_preview_reason"
    );
    assert!(
        !preview.risk_reasons.is_empty(),
        "a risk_reasons entry explains the escalation"
    );
    assert!(
        preview.risk_level > entry.locked_risk,
        "the preview risk is escalated ABOVE the catalog-authoritative risk (envelope-only)"
    );
}

// ---- L2 RED #9 — preview_action persists the generated preview to the row -------------------------

#[test]
fn preview_persists_to_row() {
    // spec(§7.2) — preview_action persists the generated preview to action_requests.preview_json (the
    // durable preview source) and it round-trips on request::load.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gateway();
    // git.create_worktree is risk-2 → require_approval → rests at awaiting_approval (the previewable
    // flow: a human previews before approving).
    let req = req_with_refs("git.create_worktree", &["wt_1"]);
    let id = req.action_request_id.as_str().to_string();
    gw.submit_action(&mut store, req).expect("submit");

    let preview = gw.preview_action(&mut store, &id).expect("preview");
    assert_eq!(
        preview.action_request_id.as_str(),
        id,
        "the preview is for the action"
    );

    let stored = preview_json_of(&path).expect("preview_json persisted to the row");
    let parsed: ActionPreview =
        serde_json::from_str(&stored).expect("preview_json round-trips to ActionPreview");
    assert_eq!(
        parsed.action_request_id.as_str(),
        id,
        "the persisted preview round-trips with the action id"
    );
    assert_eq!(
        parsed.summary, preview.summary,
        "the persisted preview matches the returned preview"
    );
}

// =================================================================================================
// L3 — the ActionExecutor framework: CatalogExecutor dispatch by ExecutorKind to side-effect-free
// per-namespace stubs + validate(requires_resource_refs) + the §7.2 read-source split (auto =
// in-memory reconciled req; approve = durable redacted row). INV-SEC-1-critical.
// =================================================================================================

/// a 40-char high-entropy `KEY=value` secret the §15 PrefixRedactor masks at rest — used to
/// distinguish RAW (in-memory) from REDACTED (durable-row) inputs the executor is handed.
const SECRET: &str = "Zx9Kq2Lm7Wp4Rn1Vc6Bt3Hy8Fd5Gj0Aa2Ss4Dd";

/// the `status` of the single action_requests row.
fn action_status(path: &std::path::Path) -> Option<String> {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row("SELECT status FROM action_requests", [], |r| r.get(0))
        .ok()
}

/// the `Action*` event types in the log.
fn action_event_types(store: &EventStore) -> Vec<String> {
    store
        .read_all()
        .expect("read all events")
        .iter()
        .filter(|e| e.event_type.starts_with("Action"))
        .map(|e| e.event_type.clone())
        .collect()
}

/// the approval_id of the single approvals row.
fn approval_id_of(path: &std::path::Path) -> String {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row("SELECT approval_id FROM approvals", [], |r| r.get(0))
        .expect("an approval")
}

/// a test-double executor that RECORDS the `inputs` of each `req` it executes — so a test can observe
/// WHICH ActionRequest the §7.2 read-source handed the executor (in-memory raw vs durable redacted).
struct RecordingExecutor {
    seen_inputs: Arc<Mutex<Vec<serde_json::Value>>>,
}
impl RecordingExecutor {
    fn new() -> Self {
        Self {
            seen_inputs: Arc::new(Mutex::new(vec![])),
        }
    }
    fn inputs_log(&self) -> Arc<Mutex<Vec<serde_json::Value>>> {
        self.seen_inputs.clone()
    }
}
impl ActionExecutor for RecordingExecutor {
    fn validate(&self, _req: &ActionRequest) -> Result<(), ExecError> {
        Ok(())
    }
    fn execute(&self, req: &ActionRequest) -> ExecutionOutcome {
        self.seen_inputs.lock().unwrap().push(req.inputs.clone());
        ExecutionOutcome::Succeeded {
            changed_resources: req.resource_refs.clone(),
            detail: "recorded".to_string(),
            side_effect_applied: false,
            emitted_events: vec![],
        }
    }
    fn preview(&self, req: &ActionRequest, generated_at: Timestamp) -> ActionPreview {
        ActionPreview {
            action_request_id: req.action_request_id.clone(),
            generated_at,
            risk_level: req.risk_level,
            risk_reasons: vec![],
            summary: "recording".to_string(),
            changed_resources: vec![],
            cannot_preview_reason: None,
        }
    }
}

// ---- L3 RED #10 — CatalogExecutor dispatches by ExecutorKind to a per-namespace stub --------------

#[test]
fn catalog_executor_dispatches_by_executor_kind() {
    // spec(§6.3 ExecutorKind) — CatalogExecutor::execute routes by the catalog ExecutorKind to a
    // per-namespace stub; the (side-effect-free) Succeeded outcome NAMES the namespace (distinct per
    // namespace) + carries changed_resources from the req (Q7).
    let ex = CatalogExecutor::new();
    let git = ex.execute(&req_with_refs("git.create_worktree", &["wt_1"])); // ExecutorKind::Git
    let gh = ex.execute(&req_with_refs("github.create_pr", &["pr_1"])); // ExecutorKind::Github
    let (git_detail, git_changed) = match git {
        ExecutionOutcome::Succeeded {
            detail,
            changed_resources,
            ..
        } => (detail, changed_resources),
        ExecutionOutcome::Failed(e) => panic!("the git stub should succeed, got {e}"),
        ExecutionOutcome::FailedWithEvents { detail, .. } => {
            panic!("the git stub never returns FailedWithEvents, got: {detail}")
        }
    };
    let gh_detail = match gh {
        ExecutionOutcome::Succeeded { detail, .. } => detail,
        ExecutionOutcome::Failed(e) => panic!("the github stub should succeed, got {e}"),
        ExecutionOutcome::FailedWithEvents { detail, .. } => {
            panic!("the github stub never returns FailedWithEvents, got: {detail}")
        }
    };
    assert_ne!(git_detail, gh_detail, "distinct per-namespace dispatch");
    assert!(
        git_detail.to_lowercase().contains("git"),
        "the git stub names its namespace: {git_detail}"
    );
    assert!(
        gh_detail.to_lowercase().contains("github"),
        "the github stub names its namespace: {gh_detail}"
    );
    assert_eq!(
        git_changed.len(),
        1,
        "changed_resources carries the req's resource refs (Q7)"
    );
}

// ---- L3 RED #11 — validate rejects a missing required resource_ref → ActionFailed (fail-closed) ----

#[test]
fn validate_rejects_missing_required_resource_ref() {
    // spec(§6.3 requires_resource_refs; fail-closed) — an action whose catalog entry is
    // requires_resource_refs=true but carries NO resource_ref → validate Err → the executor returns
    // Failed → the Gateway records ActionFailed (never a silent skip, never a panic). No success.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gateway(); // CatalogExecutor (validates at execute, Q5)
                                // git.create_worktree requires_resource_refs=true; submit with NO refs. risk-2 → awaiting.
    let req = req_with("git.create_worktree", None, serde_json::json!({}));
    gw.submit_action(&mut store, req).expect("submit");
    gw.approve(&mut store, &approval_id_of(&path))
        .expect("approve drives execute");

    assert_eq!(
        action_status(&path),
        Some("failed".to_string()),
        "validate failure → the action FAILS"
    );
    let types = action_event_types(&store);
    assert!(
        types.contains(&"ActionFailed".to_string()),
        "an ActionFailed event is recorded (never a silent skip)"
    );
    assert!(
        !types.contains(&"ActionSucceeded".to_string()),
        "the action does NOT succeed"
    );
}

// ---- L3 RED #12 — the risk-0 auto-execute path runs off the IN-MEMORY (raw) req (§7.2) -------------

#[test]
fn auto_execute_runs_off_in_memory_inputs() {
    // spec(§7.2 read-source) — the risk-0 auto-execute path hands the executor the IN-MEMORY reconciled
    // ActionRequest (raw inputs), NOT a re-read of the §15-redacted row. A RecordingExecutor proves the
    // executor sees the RAW secret (a redacted re-read would show [REDACTED]); the row stays redacted.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let spy = RecordingExecutor::new();
    let seen = spy.inputs_log();
    let gw = Gateway::new(Box::new(CatalogPolicy), Box::new(spy));
    // brain.ask is risk-0 → allow → auto-executes; carry a secret in inputs.
    let req = req_with(
        "brain.ask",
        None,
        serde_json::json!({ "env": format!("API_SECRET={SECRET}") }),
    );
    gw.submit_action(&mut store, req).expect("auto-execute");

    let captured = seen.lock().unwrap();
    assert_eq!(captured.len(), 1, "the executor ran once (auto-execute)");
    assert!(
        captured[0].to_string().contains(SECRET),
        "the auto-execute path gives the executor the RAW in-memory inputs (not the redacted row)"
    );
    // the row at rest is still redacted (the §15 gate held independently of the executor read-source)
    let conn = nexusopsd::eventstore::open_read_only(&path).unwrap();
    let row_inputs: Option<String> = conn
        .query_row("SELECT inputs_json FROM action_requests", [], |r| r.get(0))
        .unwrap();
    assert!(
        !row_inputs.unwrap().contains(SECRET),
        "the persisted row is redacted at rest (§15)"
    );
}

// ---- L3 RED #13 — the approve path runs off the DURABLE (redacted) row (§7.2 canonical) ------------

#[test]
fn approve_path_executes_off_durable_row() {
    // spec(§7.2 "rows canonical for execution") — the approve→execute path loads the action from the
    // DURABLE row (request::load) and executes off it; the in-memory inputs are gone at approve-time
    // (possibly post-restart). The RecordingExecutor sees the REDACTED row inputs, not the raw secret.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let spy = RecordingExecutor::new();
    let seen = spy.inputs_log();
    let gw = Gateway::new(Box::new(CatalogPolicy), Box::new(spy));
    // git.create_worktree is risk-2 → require_approval → awaiting; carry a secret in inputs.
    let req = req_with(
        "git.create_worktree",
        None,
        serde_json::json!({ "env": format!("API_SECRET={SECRET}") }),
    );
    gw.submit_action(&mut store, req).expect("submit");
    gw.approve(&mut store, &approval_id_of(&path))
        .expect("approve drives execute");

    let captured = seen.lock().unwrap();
    assert_eq!(captured.len(), 1, "the executor ran once (after approval)");
    assert!(
        !captured[0].to_string().contains(SECRET),
        "the approve path executes off the durable REDACTED row (§7.2 canonical), not the raw inputs"
    );
}

// ---- L3 RED #14 — the executor is invoked ONLY via the gated Gateway paths (INV-SEC-1) -------------

#[test]
fn executor_only_reachable_via_gateway() {
    // spec(§15 INV-SEC-1) — the executor is invoked ONLY from the gated paths: NOT on submit of a
    // require-approval action (the gate holds), only AFTER approve. A RecordingExecutor counts calls.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let spy = RecordingExecutor::new();
    let seen = spy.inputs_log();
    let gw = Gateway::new(Box::new(CatalogPolicy), Box::new(spy));
    // git.create_worktree is risk-2 → require_approval → awaiting (the gate holds).
    let req = req_with_refs("git.create_worktree", &["wt_1"]);
    gw.submit_action(&mut store, req).expect("submit");
    assert_eq!(
        seen.lock().unwrap().len(),
        0,
        "submit of a require-approval action does NOT execute (the gate holds)"
    );

    gw.approve(&mut store, &approval_id_of(&path))
        .expect("approve");
    assert_eq!(
        seen.lock().unwrap().len(),
        1,
        "the executor runs exactly once, only after the approval gate"
    );
}

// =================================================================================================
// P4.0b-R1a — the executor REGISTRATION SEAM: CatalogExecutor unit-struct → a per-ExecutorKind
// registry (register()-or-stub dispatch), preserving the INV-SEC-1 Adjudication guard BEFORE
// dispatch + the §6.3 requires_resource_refs precondition. Behavior-preserving (R1a registers no
// handler in production). Serves 4.0b-2's SessionExecutor + edges P5/P7.
// =================================================================================================

/// A spy [`ActionExecutor`] for the registration-seam tests: counts execute/rollback calls and
/// returns DISTINCTIVE outcomes (`regspy:*` detail) so a test can tell "the registered handler ran"
/// from "the fallback stub ran". A handler does NOT re-run the catalog precondition (the registry's
/// `resolve` owns it — Step-2.5 #1), so `validate` is a permissive `Ok`.
struct RegSpy {
    executed: Arc<Mutex<u32>>,
    rolled_back: Arc<Mutex<u32>>,
}
impl RegSpy {
    fn new() -> Self {
        Self {
            executed: Arc::new(Mutex::new(0)),
            rolled_back: Arc::new(Mutex::new(0)),
        }
    }
    fn exec_count(&self) -> Arc<Mutex<u32>> {
        self.executed.clone()
    }
    fn rollback_count(&self) -> Arc<Mutex<u32>> {
        self.rolled_back.clone()
    }
}
impl ActionExecutor for RegSpy {
    fn validate(&self, _req: &ActionRequest) -> Result<(), ExecError> {
        Ok(())
    }
    fn execute(&self, req: &ActionRequest) -> ExecutionOutcome {
        *self.executed.lock().unwrap() += 1;
        ExecutionOutcome::Succeeded {
            changed_resources: req.resource_refs.clone(),
            detail: "regspy:execute".to_string(),
            side_effect_applied: false,
            emitted_events: vec![],
        }
    }
    fn preview(&self, req: &ActionRequest, generated_at: Timestamp) -> ActionPreview {
        ActionPreview {
            action_request_id: req.action_request_id.clone(),
            generated_at,
            risk_level: req.risk_level,
            risk_reasons: vec![],
            summary: "regspy".to_string(),
            changed_resources: vec![],
            cannot_preview_reason: None,
        }
    }
    fn rollback(&self, _req: &ActionRequest) -> ExecutionOutcome {
        *self.rolled_back.lock().unwrap() += 1;
        ExecutionOutcome::Succeeded {
            changed_resources: vec![],
            detail: "regspy:rollback".to_string(),
            side_effect_applied: false,
            emitted_events: vec![],
        }
    }
}

/// the `detail` of a `Succeeded` outcome (panics on `Failed`, naming the kind under test).
fn succeeded_detail(outcome: ExecutionOutcome, ctx: &str) -> String {
    match outcome {
        ExecutionOutcome::Succeeded { detail, .. } => detail,
        ExecutionOutcome::Failed(e) => panic!("{ctx}: expected Succeeded, got Failed({e})"),
        ExecutionOutcome::FailedWithEvents { detail, .. } => {
            panic!("{ctx}: expected Succeeded, got FailedWithEvents({detail})")
        }
    }
}

// ---- R1a #1 — register() then execute() dispatches to the registered handler ----------------------

#[test]
fn register_then_execute_dispatches_to_handler() {
    // spec(§6.3) — a registered handler for the resolved ExecutorKind runs; the fallback stub does NOT.
    let mut ex = CatalogExecutor::new();
    let spy = Arc::new(RegSpy::new());
    let execs = spy.exec_count();
    ex.register(catalog::ExecutorKind::Brain, spy.clone()); // brain.ask → ExecutorKind::Brain

    let detail = succeeded_detail(
        ex.execute(&req_with("brain.ask", None, json!({}))),
        "dispatch",
    );
    assert_eq!(
        detail, "regspy:execute",
        "the registered Brain handler ran (not the stub)"
    );
    assert_eq!(
        *execs.lock().unwrap(),
        1,
        "the handler's execute ran exactly once"
    );
}

// ---- R1a #2 — an unregistered kind falls back to today's structured stub (behavior-preserving) -----

#[test]
fn unregistered_kind_falls_back_to_stub() {
    // spec(§6.3) — an empty registry reproduces today's per-namespace stub: "would execute … via the
    // {ns} adapter …", side_effect_applied=false. Unchanged for every unregistered kind.
    let ex = CatalogExecutor::new();
    let out = ex.execute(&req_with("brain.ask", None, json!({})));
    let (detail, side_effect) = match out {
        ExecutionOutcome::Succeeded {
            detail,
            side_effect_applied,
            ..
        } => (detail, side_effect_applied),
        ExecutionOutcome::Failed(e) => {
            panic!("the unregistered stub should succeed, got Failed({e})")
        }
        ExecutionOutcome::FailedWithEvents { detail, .. } => {
            panic!("the unregistered stub never returns FailedWithEvents, got: {detail}")
        }
    };
    assert!(
        detail.contains("would execute"),
        "the stub names the would-execute provenance: {detail}"
    );
    assert!(
        detail.to_lowercase().contains("brain"),
        "the stub names the namespace: {detail}"
    );
    assert!(!side_effect, "the stub applies NO durable side effect");
}

// ---- R1a #3 — incremental: only the registered kind is live ---------------------------------------

#[test]
fn incremental_registration_only_registered_kind_live() {
    // spec(§6.3) — registering kind A (Brain) does NOT make kind B (Git) live; B still stubs.
    let mut ex = CatalogExecutor::new();
    let spy = Arc::new(RegSpy::new());
    let execs = spy.exec_count();
    ex.register(catalog::ExecutorKind::Brain, spy.clone());

    // git.create_worktree → ExecutorKind::Git (unregistered) → stub.
    let detail = succeeded_detail(
        ex.execute(&req_with_refs("git.create_worktree", &["wt_1"])),
        "git",
    );
    assert!(
        detail.contains("would execute"),
        "Git is unregistered → stub: {detail}"
    );
    assert_ne!(
        detail, "regspy:execute",
        "the Brain handler must NOT run for a Git action"
    );
    assert_eq!(
        *execs.lock().unwrap(),
        0,
        "the Brain handler never ran for a Git action"
    );
}

// ---- R1a #4 — the INV-SEC-1 Adjudication guard fires BEFORE dispatch, even if a handler exists -----

#[test]
fn adjudication_refused_before_dispatch_even_if_registered() {
    // spec(§15 / LESSON 26) — the load-bearing safety pin. An adjudication-only action is fail-closed-
    // refused BEFORE the handler lookup: even with a handler registered for ExecutorKind::Adjudication,
    // execute returns Failed and the handler is NEVER called (the agent runs the tool, not the daemon).
    let mut ex = CatalogExecutor::new();
    let spy = Arc::new(RegSpy::new());
    let execs = spy.exec_count();
    ex.register(catalog::ExecutorKind::Adjudication, spy.clone()); // a (mis-)registered adjudication handler

    let out = ex.execute(&req_with("agent.bash", None, json!({}))); // agent.bash → ExecutorKind::Adjudication
    match out {
        ExecutionOutcome::Failed(e) => assert!(
            e.contains("adjudication"),
            "the refusal names the INV-SEC-1 reason: {e}"
        ),
        ExecutionOutcome::FailedWithEvents { detail, .. } => {
            panic!("an adjudication action must be a plain Failed refusal, got FailedWithEvents({detail})")
        }
        ExecutionOutcome::Succeeded { detail, .. } => {
            panic!("an adjudication action must be REFUSED, got Succeeded({detail})")
        }
    }
    assert_eq!(
        *execs.lock().unwrap(),
        0,
        "the registered Adjudication handler was NEVER called (guard fires before dispatch)"
    );
}

// ---- R1a #5 — the requires_resource_refs precondition survives the refactor (registered or not) -----

#[test]
fn requires_resource_refs_precondition_survives() {
    // spec(§6.3) — git.create_worktree requires_resource_refs=true; carrying NONE → Failed
    // (MissingResourceRef), uniformly — the precondition runs BEFORE dispatch, so a registered handler
    // is never reached and the failure is identical with or without one.
    let no_refs = || req_with("git.create_worktree", None, json!({}));

    // (a) empty registry → Failed.
    let bare = CatalogExecutor::new();
    match bare.execute(&no_refs()) {
        ExecutionOutcome::Failed(e) => {
            assert!(e.contains("resource_ref"), "names the precondition: {e}")
        }
        ExecutionOutcome::FailedWithEvents { detail, .. } => {
            panic!("the precondition fails as a plain Failed, got FailedWithEvents({detail})")
        }
        ExecutionOutcome::Succeeded { detail, .. } => {
            panic!("missing resource_ref must Fail, got {detail}")
        }
    }

    // (b) with a Git handler registered → STILL Failed, and the handler never ran (precondition first).
    let mut with_handler = CatalogExecutor::new();
    let spy = Arc::new(RegSpy::new());
    let execs = spy.exec_count();
    with_handler.register(catalog::ExecutorKind::Git, spy.clone());
    match with_handler.execute(&no_refs()) {
        ExecutionOutcome::Failed(e) => assert!(
            e.contains("resource_ref"),
            "precondition still fails-closed: {e}"
        ),
        ExecutionOutcome::FailedWithEvents { detail, .. } => {
            panic!("the precondition fails as a plain Failed, got FailedWithEvents({detail})")
        }
        ExecutionOutcome::Succeeded { detail, .. } => {
            panic!("precondition must short-circuit dispatch, got {detail}")
        }
    }
    assert_eq!(
        *execs.lock().unwrap(),
        0,
        "the precondition runs BEFORE dispatch — the handler never ran"
    );
}

// ---- R1a #6 — rollback delegates to the registered handler, else the fail-closed default -----------

#[test]
fn rollback_delegates_to_handler_else_default() {
    // spec(§6.2 rollback seam) — a registered handler's rollback is delegated to; an unregistered kind
    // returns the existing fail-closed default Failed (never a false "rolled back").
    let req = req_with("brain.ask", None, json!({}));

    // registered → the handler's rollback runs.
    let mut ex = CatalogExecutor::new();
    let spy = Arc::new(RegSpy::new());
    let rbs = spy.rollback_count();
    ex.register(catalog::ExecutorKind::Brain, spy.clone());
    let detail = succeeded_detail(ex.rollback(&req), "rollback-delegate");
    assert_eq!(
        detail, "regspy:rollback",
        "rollback delegated to the registered handler"
    );
    assert_eq!(*rbs.lock().unwrap(), 1, "the handler's rollback ran once");

    // unregistered → the fail-closed default Failed.
    let bare = CatalogExecutor::new();
    match bare.rollback(&req) {
        ExecutionOutcome::Failed(_) => {}
        ExecutionOutcome::FailedWithEvents { detail, .. } => {
            panic!("rollback's default is a plain Failed, never FailedWithEvents, got: {detail}")
        }
        ExecutionOutcome::Succeeded { detail, .. } => {
            panic!("an unregistered rollback must fail-close (default Failed), got Succeeded({detail})")
        }
    }
}
