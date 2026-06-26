//! P4.7/086 — the dev-client smoke `build_*_request` helpers (the 083 live-validation chain). Pure
//! arg→ActionRequest constructors for EXISTING audited actions (no new mutation surface): the right
//! action_type / inputs / resource_refs / catalog-derived identity, + fail-closed on a missing required
//! arg. The CLI dispatch glue + the live IPC round-trip are the manual-validation harness (the tool IS
//! the validator); these unit tests pin the safety-relevant request SHAPE (LESSON 49/60/61/63).
//!
//! dev-client-gated (the smoke harness is OUT of the production binary) → this test compiles+runs ONLY
//! under `--features dev-client` (the existing CI-rot residual: the default CI line doesn't exercise it).
#![cfg(feature = "dev-client")]

use nexusops_shared::actions::ResourceType;
use nexusopsd::smoke::{
    build_create_pr_request, build_integration_connect_request, build_merge_pr_request,
    build_set_live_writes_request, build_submit_review_request,
};

fn args(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

/// the inputs object's string value for `key`.
fn input_str<'a>(req: &'a nexusops_shared::actions::ActionRequest, key: &str) -> Option<&'a str> {
    req.inputs.get(key).and_then(|v| v.as_str())
}

// ---- 1. integration.connect (LESSON 49 registration-only — POINTER, no token) -------------------

#[test]
fn build_integration_connect_request_shape() {
    // spec(LESSON 49) — integration.connect: inputs {provider, keychain_ref POINTER, account}; NO token
    // field; requires_resource_refs=false (the connection identity is the inputs, not a resource_ref).
    let req = build_integration_connect_request(&args(&[
        "--provider",
        "github",
        "--keychain-ref",
        "nexusops/github/octocat",
        "--account",
        "octocat",
    ]))
    .expect("builds");
    assert_eq!(req.action_type, "integration.connect");
    assert_eq!(input_str(&req, "provider"), Some("github"));
    assert_eq!(
        input_str(&req, "keychain_ref"),
        Some("nexusops/github/octocat")
    );
    assert_eq!(input_str(&req, "account"), Some("octocat"));
    assert!(
        req.inputs.get("token").is_none(),
        "registration-only: NO token in inputs (LESSON 49)"
    );
}

// ---- 2. integration.set_live_writes -------------------------------------------------------------

#[test]
fn build_set_live_writes_request_shape() {
    // spec(083 governance) — integration.set_live_writes: inputs {connection_id, enabled: bool}.
    let req = build_set_live_writes_request(&args(&[
        "--connection",
        "conn_gh_octocat",
        "--enabled",
        "true",
    ]))
    .expect("builds");
    assert_eq!(req.action_type, "integration.set_live_writes");
    assert_eq!(input_str(&req, "connection_id"), Some("conn_gh_octocat"));
    assert_eq!(
        req.inputs.get("enabled").and_then(|v| v.as_bool()),
        Some(true),
        "enabled is a JSON bool"
    );

    // the GOVERNANCE-toggle footgun pin: `--enabled false` must round-trip to JSON `false` (an enable can
    // never mis-parse from a "false"); `--enabled true` → JSON `true`. The one CLI parse worth pinning.
    let off = build_set_live_writes_request(&args(&["--connection", "c", "--enabled", "false"]))
        .expect("builds");
    assert_eq!(
        off.inputs.get("enabled").and_then(|v| v.as_bool()),
        Some(false),
        "false → JSON false, never enabling"
    );
}

// ---- 3. github.create_pr (LESSON 63 — audited identity is the envelope project_id) --------------

#[test]
fn build_create_pr_request_shape() {
    // spec(LESSON 63) — github.create_pr: inputs {head, base, title, body?}; the audited create TARGET is
    // the envelope project_id (the executor's resolve_repo_target reads req.project_id), + a project_id
    // resource_ref to satisfy the catalog requires_resource_refs.
    let req = build_create_pr_request(&args(&[
        "--project",
        "proj_01ARZ3NDEKTSV4RRFFQ69G5FAV",
        "--head",
        "feature/x",
        "--base",
        "main",
        "--title",
        "My PR",
        "--body",
        "the description",
    ]))
    .expect("builds");
    assert_eq!(req.action_type, "github.create_pr");
    assert_eq!(input_str(&req, "head"), Some("feature/x"));
    assert_eq!(input_str(&req, "base"), Some("main"));
    assert_eq!(input_str(&req, "title"), Some("My PR"));
    assert_eq!(input_str(&req, "body"), Some("the description"));
    // the audited create target rides the envelope project_id (resolve_repo_target) ...
    assert_eq!(
        req.project_id.as_ref().map(|p| p.as_str()),
        Some("proj_01ARZ3NDEKTSV4RRFFQ69G5FAV")
    );
    // ... + a Project resource_ref carries the same id (the catalog requires_resource_refs).
    let rref = req.resource_refs.first().expect("a resource_ref");
    assert_eq!(rref.resource_type, ResourceType::Project);
    assert_eq!(rref.id, "proj_01ARZ3NDEKTSV4RRFFQ69G5FAV");
}

// ---- 4. github.merge_pr (LESSON 60 SHA-pin + LESSON 63 Repo resource_ref) ------------------------

#[test]
fn build_merge_pr_request_shape() {
    // spec(LESSON 60/63) — github.merge_pr: inputs {pr_number (number), sha (anti-race pin), merge_method};
    // the audited target is a Repo resource_ref (resolve_pr_target — never inputs).
    let req = build_merge_pr_request(&args(&[
        "--repo",
        "repo_acme",
        "--pr",
        "42",
        "--sha",
        "9fceb02",
        "--method",
        "squash",
    ]))
    .expect("builds");
    assert_eq!(req.action_type, "github.merge_pr");
    assert_eq!(
        req.inputs.get("pr_number").and_then(|v| v.as_u64()),
        Some(42),
        "pr_number is a number"
    );
    assert_eq!(input_str(&req, "sha"), Some("9fceb02"));
    assert_eq!(input_str(&req, "merge_method"), Some("squash"));
    let rref = req.resource_refs.first().expect("a Repo resource_ref");
    assert_eq!(rref.resource_type, ResourceType::Repo);
    assert_eq!(
        rref.id, "repo_acme",
        "the AUDITED repo target (082/LESSON 63)"
    );
}

// ---- 5. github.submit_review (LESSON 61 — commit_id pin + Repo resource_ref) ---------------------

#[test]
fn build_submit_review_request_shape() {
    // spec(LESSON 61) — github.submit_review: inputs {pr_number, commit_id (reviewed-head pin), event, body?};
    // a Repo resource_ref (resolve_pr_target).
    let req = build_submit_review_request(&args(&[
        "--repo",
        "repo_acme",
        "--pr",
        "42",
        "--sha",
        "9fceb02",
        "--event",
        "approve",
        "--body",
        "LGTM",
    ]))
    .expect("builds");
    assert_eq!(req.action_type, "github.submit_review");
    assert_eq!(
        req.inputs.get("pr_number").and_then(|v| v.as_u64()),
        Some(42)
    );
    assert_eq!(
        input_str(&req, "commit_id"),
        Some("9fceb02"),
        "the reviewed-head SHA pins the verdict"
    );
    assert_eq!(input_str(&req, "event"), Some("approve"));
    assert_eq!(input_str(&req, "body"), Some("LGTM"));
    let rref = req.resource_refs.first().expect("a Repo resource_ref");
    assert_eq!(rref.resource_type, ResourceType::Repo);
    assert_eq!(rref.id, "repo_acme");
}

// ---- 6. fail-closed on a missing required arg ---------------------------------------------------

#[test]
fn build_request_missing_required_arg_errors() {
    // spec(fail-closed CLI parse) — a missing REQUIRED arg → a typed CLI Err, NEVER a malformed/partial
    // ActionRequest submitted to the daemon.
    // merge-pr without --sha (the anti-race pin is required).
    assert!(build_merge_pr_request(&args(&[
        "--repo",
        "repo_acme",
        "--pr",
        "42",
        "--method",
        "squash"
    ]))
    .is_err());
    // create-pr (valid project) without --title.
    assert!(build_create_pr_request(&args(&[
        "--project",
        "proj_01ARZ3NDEKTSV4RRFFQ69G5FAV",
        "--head",
        "f",
        "--base",
        "main"
    ]))
    .is_err());
    // create-pr with a MALFORMED --project → a CLI parse error (not a confusing daemon execute-time error).
    assert!(build_create_pr_request(&args(&[
        "--project",
        "not-a-prof-id",
        "--head",
        "f",
        "--base",
        "main",
        "--title",
        "t"
    ]))
    .is_err());
    // submit-review without --event.
    assert!(build_submit_review_request(&args(&[
        "--repo",
        "repo_acme",
        "--pr",
        "42",
        "--sha",
        "abc"
    ]))
    .is_err());
    // integration.connect without --keychain-ref.
    assert!(build_integration_connect_request(&args(&[
        "--provider",
        "github",
        "--account",
        "octocat"
    ]))
    .is_err());
    // merge-pr with a non-numeric --pr → typed error (never a malformed pr_number).
    assert!(build_merge_pr_request(&args(&[
        "--repo",
        "repo_acme",
        "--pr",
        "not-a-number",
        "--sha",
        "abc",
        "--method",
        "merge"
    ]))
    .is_err());
    // set-live-writes with a NON-BOOL --enabled → typed error (the governance toggle can't be ambiguous).
    assert!(
        build_set_live_writes_request(&args(&["--connection", "c", "--enabled", "maybe"])).is_err()
    );
}
