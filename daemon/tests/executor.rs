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
use nexusopsd::gateway::executor::StubExecutor;
use nexusopsd::gateway::idempotency::derive_key;
use nexusopsd::gateway::policy::CatalogPolicy;
use nexusopsd::gateway::preview::generate_preview;
use nexusopsd::gateway::Gateway;
use nexusopsd::idgen::UlidGen;

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

/// the production Gateway (catalog policy + stub executor — real executor framework lands L3).
fn catalog_gateway() -> Gateway {
    Gateway::new(Box::new(CatalogPolicy), Box::new(StubExecutor))
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
