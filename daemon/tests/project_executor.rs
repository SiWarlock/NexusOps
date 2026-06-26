//! P5.1 (edges-019) — the `project.rescan` executor: `ProjectExecutor` (`ExecutorKind::Project`)
//! runs the detection engine (`detect_git` + `detect_workflow`) over a scan path read from
//! `req.inputs["path"]` and emits a `ProjectRescanned` event through the Gateway's in-txn §15
//! redaction gate — with the git `remote_url` credential userinfo stripped AT THE EMIT SOURCE.
//! This is the FIRST real edges Gateway mutator-path Action.
//!
//! **Unit tests** pin the §15 `strip_userinfo` helper (rule #5 — the load-bearing security pin).
//! **Integration tests** drive `submit_action` end-to-end (risk-0 auto-execute → `CatalogExecutor`
//! dispatch → `ProjectExecutor::execute` → `ProjectRescanned` appended atomic with `ActionSucceeded`).
//! Fixtures are hermetic (`tempfile` + `git2::Repository::init` + programmatic commits/remotes — no
//! shelling to `git`, no committed fixture repos), matching `tests/detect.rs`.
//!
//! The assertions read the PERSISTED `ProjectRescanned` event (or the `ExecutionOutcome`), NOT the
//! internal `EmittedEvent` bridge variant — so they hold regardless of the Step-2.5 bridge shape.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use git2::{Repository, RepositoryInitOptions, Signature};
use tempfile::tempdir;

use nexusops_shared::actions::{ActionRequest, RequesterType, RiskLevel};
use nexusops_shared::catalog::ExecutorKind;
use nexusops_shared::events::ProjectRescanned;
use nexusops_shared::ids::ActionRequestId;
use nexusops_shared::status::ActionRequestStatus;
use nexusops_shared::time::Timestamp;
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{EventStore, PrefixRedactor};
use nexusopsd::gateway::executor::{
    ActionExecutor, CatalogExecutor, EmittedEvent, ExecutionOutcome,
};
use nexusopsd::gateway::policy::CatalogPolicy;
use nexusopsd::gateway::Gateway;
use nexusopsd::idgen::UlidGen;
use nexusopsd::project::executor::{project_name_from_path, strip_userinfo, ProjectExecutor};

const FIXED_TS: &str = "2026-06-11T00:00:00Z";

// ---- harness helpers -------------------------------------------------------

fn open(path: &Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(FixedClock::new(FIXED_TS)),
        Box::new(PrefixRedactor),
    )
    .expect("open event store")
}

/// Init a fresh repo at `path` with `main` as the initial branch (deterministic, host-independent).
fn init_repo(path: &Path) -> Repository {
    let mut opts = RepositoryInitOptions::new();
    opts.initial_head("main");
    Repository::init_opts(path, &opts).expect("init repo")
}

/// Commit a single file so HEAD/`main` exists (an unborn HEAD has no branch shorthand).
fn commit_file(repo: &Repository, name: &str, content: &str) -> git2::Oid {
    let workdir = repo.workdir().expect("non-bare repo");
    fs::write(workdir.join(name), content).expect("write file");
    let mut index = repo.index().expect("index");
    index.add_path(Path::new(name)).expect("add path");
    index.write().expect("write index");
    let tree_oid = index.write_tree().expect("write tree");
    let tree = repo.find_tree(tree_oid).expect("find tree");
    let sig = Signature::now("Test", "test@example.com").expect("sig");
    let parents: Vec<git2::Commit> = repo
        .head()
        .ok()
        .and_then(|h| h.target())
        .and_then(|oid| repo.find_commit(oid).ok())
        .into_iter()
        .collect();
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, "commit", &tree, &parent_refs)
        .expect("commit")
}

/// A UI/IPC `project.rescan` ActionRequest carrying the scan path in `inputs["path"]`.
fn project_rescan_req(path: &str) -> ActionRequest {
    ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id: None,
        action_type: "project.rescan".to_string(),
        requester_type: RequesterType::User, // UI/IPC
        requester_id: "u_local".to_string(),
        resource_refs: vec![], // project.rescan: requires_resource_refs = false
        inputs: serde_json::json!({ "path": path }),
        risk_level: RiskLevel::Level0,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        created_at: Timestamp::parse(FIXED_TS).unwrap(),
    }
}

/// The production policy (catalog-driven) + a `CatalogExecutor` with `ProjectExecutor` registered for
/// `ExecutorKind::Project` (the real edges registration the production `main.rs` mirrors).
fn gateway_with_project_executor() -> Gateway {
    let mut catalog = CatalogExecutor::new();
    catalog.register(
        ExecutorKind::Project,
        Arc::new(ProjectExecutor::new(Box::new(FixedClock::new(FIXED_TS)))),
    );
    Gateway::new(Box::new(CatalogPolicy), Box::new(catalog))
}

/// Every persisted `ProjectRescanned` event payload, parsed.
fn rescanned_events(store: &EventStore) -> Vec<ProjectRescanned> {
    store
        .read_all()
        .unwrap()
        .iter()
        .filter(|e| e.event_type == "ProjectRescanned")
        .map(|e| serde_json::from_str(&e.payload_json).expect("ProjectRescanned payload parses"))
        .collect()
}

/// All persisted event payloads concatenated — the raw-log sweep for the credential-leak assertion
/// (proves the token never reaches the immutable log via ANY field, not just the typed `remote_url`).
fn all_payloads(store: &EventStore) -> String {
    store
        .read_all()
        .unwrap()
        .iter()
        .map(|e| e.payload_json.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

// ---- §15 strip_userinfo unit tests (the load-bearing security pin) ---------

#[test]
fn test_strip_userinfo_https_with_creds() {
    // spec(§15 rule #5): credential userinfo (`user:token@`) stripped AT SOURCE — the Redactor is
    // the backstop only (a generic URL password has no prefix; LESSON §13).
    assert_eq!(
        strip_userinfo("https://user:token@github.com/o/r"),
        "https://github.com/o/r"
    );
}

#[test]
fn test_strip_userinfo_https_no_creds_unchanged() {
    // spec(§15): a URL with no userinfo is byte-identical out — no false mutation.
    assert_eq!(
        strip_userinfo("https://github.com/o/r"),
        "https://github.com/o/r"
    );
}

#[test]
fn test_strip_userinfo_scp_ssh_unchanged() {
    // spec(§15): scp-style ssh (no scheme) — `git@` is the ssh USERNAME, not a secret → left intact.
    assert_eq!(strip_userinfo("git@github.com:o/r"), "git@github.com:o/r");
}

#[test]
fn test_strip_userinfo_ssh_scheme_with_user() {
    // spec(§15): Q2 ruling — strip userinfo from ANY scheme-bearing URL (a token can ride the bare
    // username slot, e.g. `https://ghp_xxx@host`; we cannot safely distinguish it from a benign
    // ssh user without a brittle allowlist → strip uniformly). scp-style (no scheme) stays intact.
    assert_eq!(strip_userinfo("ssh://git@host/o/r"), "ssh://host/o/r");
}

#[test]
fn test_strip_userinfo_at_in_password() {
    // spec(§15): the authority userinfo is delimited by the LAST `@` IN THE AUTHORITY (a password may
    // itself contain `@`). A naive FIRST-`@` strip would leak `ss@github.com/o/r` — a PARTIAL
    // credential leak that reads as "stripped" (worse than no strip). Pins authority-scoped + the
    // correct (last-`@`-in-authority) delimiter; also covers `@`-in-PATH (no authority `@`) for free.
    assert_eq!(
        strip_userinfo("https://user:p@ss@github.com/o/r"),
        "https://github.com/o/r"
    );
}

#[test]
fn test_strip_userinfo_bare_token_in_username_slot() {
    // spec(§15): the stated Q2 threat model — a GitHub PAT rides the BARE username slot with NO
    // password (`https://ghp_TOKEN@host`, no colon). The most common real-world leak form; a strip
    // that only handled `user:token@` would miss it. The whole userinfo (incl. a colon-less token) is
    // stripped via the last-`@`-in-authority delimiter.
    assert_eq!(
        strip_userinfo("https://ghp_SECRETTOKEN@github.com/o/r"),
        "https://github.com/o/r"
    );
}

// ---- 092 — friendly project name (basename of the scan path) --------------

#[test]
fn project_name_from_path_basename() {
    // spec(092) — name = the last non-empty path component (the lead's basename ruling), trailing-slash-
    // tolerant; a degenerate/empty basename → None (never a panic).
    assert_eq!(project_name_from_path("/a/b/c"), Some("c".to_string()));
    assert_eq!(project_name_from_path("/a/b/"), Some("b".to_string()));
    assert_eq!(project_name_from_path("/a/b///"), Some("b".to_string()));
    assert_eq!(project_name_from_path("repo"), Some("repo".to_string()));
    assert_eq!(project_name_from_path(""), None);
    assert_eq!(project_name_from_path("/"), None);
    assert_eq!(
        project_name_from_path("."),
        None,
        ". (cwd marker) is not a real name"
    );
    assert_eq!(project_name_from_path(".."), None);
}

#[test]
fn rescan_emits_name_from_scan_path() {
    // spec(§7.1 / 092) — the ProjectExecutor emits name = Some(basename(inputs.path)). The non-git path
    // (the project-brain sidecar, repo_root=None) STILL gets a name — the exact user case. Inspect the raw
    // persisted payload via JSON (name is the field under test → RED-before-the-field, no struct coupling).
    let store_dir = tempdir().unwrap();
    let mut store = open(&store_dir.path().join("nexusops.db"));
    let gw = gateway_with_project_executor();
    gw.submit_action(&mut store, project_rescan_req("/x/project-brain"))
        .expect("submit");
    let payload = store
        .read_all()
        .unwrap()
        .into_iter()
        .find(|e| e.event_type == "ProjectRescanned")
        .expect("a ProjectRescanned event")
        .payload_json;
    let v: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(
        v["name"],
        serde_json::json!("project-brain"),
        "name = the scan-path basename (non-git path still named)"
    );
}

// ---- end-to-end emission via submit_action --------------------------------

#[test]
fn test_project_rescan_emits_project_rescanned() {
    // spec(§6.3/§7.2): project.rescan dispatches to ProjectExecutor → ActionSucceeded + EXACTLY one
    // ProjectRescanned whose ALL 11 fields map the fixture's detected state (both detect_git AND
    // detect_workflow). The fixture sets non-default workflow signals (.claude/ + a plan file → an
    // uncommitted dirty tree) so the mapping is exercised, not just the false/None defaults.
    let repo_dir = tempdir().unwrap();
    let repo = init_repo(repo_dir.path());
    commit_file(&repo, "README.md", "hi"); // clean baseline → branch `main`
    repo.remote("origin", "https://github.com/example/repo.git")
        .unwrap();
    // non-default workflow signals, left UNCOMMITTED → cc_crew + plan_file detected AND is_dirty.
    fs::create_dir(repo_dir.path().join(".claude")).unwrap();
    fs::write(repo_dir.path().join(".claude").join("settings.json"), "{}").unwrap();
    fs::write(repo_dir.path().join("IMPLEMENTATION_PLAN.md"), "# plan").unwrap();

    let store_dir = tempdir().unwrap();
    let mut store = open(&store_dir.path().join("nexusops.db"));
    let gw = gateway_with_project_executor();

    let ack = gw
        .submit_action(
            &mut store,
            project_rescan_req(repo_dir.path().to_str().unwrap()),
        )
        .expect("submit");
    assert_eq!(
        ack.status,
        ActionRequestStatus::Succeeded,
        "risk-0 project.rescan auto-executes to succeeded"
    );

    let events = rescanned_events(&store);
    assert_eq!(events.len(), 1, "EXACTLY one ProjectRescanned emitted");
    let ev = &events[0];
    // detect_git → ProjectRescanned (6 fields)
    assert!(ev.is_git, "the fixture is a git repo");
    // repo_root is PRESENT post-pipeline; its EXACT value is pinned pre-redaction in
    // `test_project_rescan_payload_maps_all_fields`. Here the §15 JSON-value backstop (2.0-SEC L3,
    // LESSON §13) masks the random high-entropy tempdir component of the path — a recall-envelope
    // false positive on a RANDOM test path (production repo paths are low-entropy). The backstop
    // firing here confirms §15 is live on the edges event payload (defense-in-depth).
    assert!(
        ev.repo_root.is_some(),
        "repo_root present (exact value pinned pre-redaction in the unit test)"
    );
    assert_eq!(
        ev.remote_url.as_deref(),
        Some("https://github.com/example/repo.git"),
        "the credential-free origin is carried verbatim"
    );
    assert_eq!(ev.branch.as_deref(), Some("main"), "the current branch");
    assert!(!ev.detached);
    assert!(
        ev.is_dirty,
        "the uncommitted .claude/ + plan file make the tree dirty"
    );
    // detect_workflow → ProjectRescanned (4 fields)
    assert!(!ev.workflow_pack, "no .scaffolding/manifest.json");
    assert!(ev.cc_crew, ".claude/ present");
    assert!(ev.plan_file.is_some(), "IMPLEMENTATION_PLAN.md present");
    assert!(!ev.brain, "no .brain marker");
    // the executor-stamped scan time (1 field)
    assert_eq!(
        ev.scanned_at.as_str(),
        FIXED_TS,
        "scanned_at is stamped from the injected Clock (deterministic)"
    );
}

#[test]
fn test_project_rescan_payload_maps_all_fields() {
    // spec(§6.3/§7.2): the executor maps detect_git (6) + detect_workflow (4) + the Clock stamp (1)
    // into ALL 11 ProjectRescanned fields. Asserted on the PRE-redaction emitted payload (direct
    // executor call) so the EXACT repo_root path is checked without the §15 backstop (which masks the
    // high-entropy random tempdir component in the end-to-end path — see test 5).
    let repo_dir = tempdir().unwrap();
    let repo = init_repo(repo_dir.path());
    commit_file(&repo, "README.md", "hi");
    repo.remote("origin", "https://github.com/example/repo.git")
        .unwrap();
    fs::create_dir(repo_dir.path().join(".claude")).unwrap();
    fs::write(repo_dir.path().join(".claude").join("settings.json"), "{}").unwrap();
    fs::write(repo_dir.path().join("IMPLEMENTATION_PLAN.md"), "# plan").unwrap();

    let exec = ProjectExecutor::new(Box::new(FixedClock::new(FIXED_TS)));
    let ev = match exec.execute(&project_rescan_req(repo_dir.path().to_str().unwrap())) {
        ExecutionOutcome::Succeeded {
            emitted_events,
            side_effect_applied,
            ..
        } => {
            assert!(!side_effect_applied);
            assert_eq!(emitted_events.len(), 1);
            match &emitted_events[0] {
                EmittedEvent::Namespaced {
                    event_type,
                    payload_json,
                } => {
                    assert_eq!(*event_type, "ProjectRescanned");
                    serde_json::from_str::<ProjectRescanned>(payload_json)
                        .expect("ProjectRescanned payload parses")
                }
                _ => panic!("expected a Namespaced ProjectRescanned event"),
            }
        }
        ExecutionOutcome::Failed(e) => panic!("expected Succeeded, got Failed: {e}"),
        ExecutionOutcome::FailedWithEvents { detail, .. } => {
            panic!("project.rescan never returns FailedWithEvents, got: {detail}")
        }
    };
    // detect_git → 6
    assert!(ev.is_git);
    assert_eq!(
        ev.repo_root.as_deref(),
        repo_dir.path().canonicalize().unwrap().to_str(),
        "repo_root = the canonicalized workdir (exact, pre-redaction)"
    );
    assert_eq!(
        ev.remote_url.as_deref(),
        Some("https://github.com/example/repo.git")
    );
    assert_eq!(ev.branch.as_deref(), Some("main"));
    assert!(!ev.detached);
    assert!(
        ev.is_dirty,
        "the uncommitted .claude/ + plan file make the tree dirty"
    );
    // detect_workflow → 4
    assert!(!ev.workflow_pack, "no .scaffolding/manifest.json");
    assert!(ev.cc_crew, ".claude/ present");
    assert!(ev.plan_file.is_some(), "IMPLEMENTATION_PLAN.md present");
    assert!(!ev.brain, "no .brain marker");
    // Clock stamp → 1
    assert_eq!(ev.scanned_at.as_str(), FIXED_TS);
}

#[test]
fn test_project_rescan_emitted_remote_url_stripped() {
    // spec(§15): a repo whose origin embeds credentials → the emitted remote_url is stripped AND the
    // token never reaches ANY persisted event payload (the end-to-end security pin, distinct from the
    // unit strip test).
    let repo_dir = tempdir().unwrap();
    let repo = init_repo(repo_dir.path());
    commit_file(&repo, "README.md", "hi");
    repo.remote("origin", "https://alice:ghp_SECRETTOKEN@github.com/o/r.git")
        .unwrap();

    let store_dir = tempdir().unwrap();
    let mut store = open(&store_dir.path().join("nexusops.db"));
    let gw = gateway_with_project_executor();
    gw.submit_action(
        &mut store,
        project_rescan_req(repo_dir.path().to_str().unwrap()),
    )
    .expect("submit");

    let events = rescanned_events(&store);
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].remote_url.as_deref(),
        Some("https://github.com/o/r.git"),
        "userinfo stripped at source"
    );
    let log = all_payloads(&store);
    assert!(
        !log.contains("ghp_SECRETTOKEN"),
        "the credential NEVER reaches the immutable event log"
    );
    assert!(!log.contains("alice:ghp_SECRETTOKEN"));
}

#[test]
fn test_project_rescan_non_git_path() {
    // spec(§9): a non-git path → ProjectRescanned{is_git:false, repo_root:None}, ActionSucceeded —
    // degraded detection, NEVER an Err/panic ("basic project w/ no git still works").
    let plain_dir = tempdir().unwrap(); // empty, non-git
    let store_dir = tempdir().unwrap();
    let mut store = open(&store_dir.path().join("nexusops.db"));
    let gw = gateway_with_project_executor();

    let ack = gw
        .submit_action(
            &mut store,
            project_rescan_req(plain_dir.path().to_str().unwrap()),
        )
        .expect("submit");
    assert_eq!(ack.status, ActionRequestStatus::Succeeded);

    let events = rescanned_events(&store);
    assert_eq!(events.len(), 1);
    assert!(!events[0].is_git);
    assert_eq!(events[0].repo_root, None);
}

// ---- fail-closed input handling + the read-only side-effect contract ------

#[test]
fn test_project_rescan_missing_path_input_fails() {
    // spec(§15/fail-closed): no/blank path input → ExecutionOutcome::Failed, NO event, NEVER a scan of
    // the daemon's own cwd. Direct executor call (the outcome is the surface; the path guard runs
    // BEFORE any detect call).
    let exec = ProjectExecutor::new(Box::new(FixedClock::new(FIXED_TS)));

    // (a) absent "path" key.
    let mut req_absent = project_rescan_req("");
    req_absent.inputs = serde_json::json!({});
    assert!(
        matches!(exec.execute(&req_absent), ExecutionOutcome::Failed(_)),
        "an absent path input fails closed"
    );

    // (b) present-but-blank path.
    let req_blank = project_rescan_req("   ");
    assert!(
        matches!(exec.execute(&req_blank), ExecutionOutcome::Failed(_)),
        "a blank path input fails closed (never defaults to cwd)"
    );
}

#[test]
fn test_project_rescan_dispatches_to_registered_executor() {
    // spec(§6.3 reachability): the ProjectRescanned event proves the REAL executor ran — a bare
    // CatalogExecutor (no Project handler) takes the side-effect-free stub path and emits NONE.
    let repo_dir = tempdir().unwrap();
    let repo = init_repo(repo_dir.path());
    commit_file(&repo, "README.md", "hi");

    // registered → emits ProjectRescanned.
    let sd1 = tempdir().unwrap();
    let mut store1 = open(&sd1.path().join("nexusops.db"));
    let gw_reg = gateway_with_project_executor();
    gw_reg
        .submit_action(
            &mut store1,
            project_rescan_req(repo_dir.path().to_str().unwrap()),
        )
        .expect("submit");
    assert_eq!(
        rescanned_events(&store1).len(),
        1,
        "the registered ProjectExecutor emitted ProjectRescanned"
    );

    // NOT registered → the stub path still succeeds but emits NO ProjectRescanned.
    let sd2 = tempdir().unwrap();
    let mut store2 = open(&sd2.path().join("nexusops.db"));
    let gw_stub = Gateway::new(Box::new(CatalogPolicy), Box::new(CatalogExecutor::new()));
    let ack = gw_stub
        .submit_action(
            &mut store2,
            project_rescan_req(repo_dir.path().to_str().unwrap()),
        )
        .expect("submit");
    assert_eq!(
        ack.status,
        ActionRequestStatus::Succeeded,
        "the stub succeeds (no side effect)"
    );
    assert_eq!(
        rescanned_events(&store2).len(),
        0,
        "the stub emits NO ProjectRescanned (proves dispatch went to the real executor when registered)"
    );
}

#[test]
fn test_project_rescan_side_effect_applied_false() {
    // spec(§17): detection is READ-ONLY → side_effect_applied == false, so a txn-B append failure
    // rolls back cleanly (the action stays `executing` → L5), NOT ActionPartiallySucceeded.
    let repo_dir = tempdir().unwrap();
    init_repo(repo_dir.path());
    let exec = ProjectExecutor::new(Box::new(FixedClock::new(FIXED_TS)));

    match exec.execute(&project_rescan_req(repo_dir.path().to_str().unwrap())) {
        ExecutionOutcome::Succeeded {
            side_effect_applied,
            emitted_events,
            ..
        } => {
            assert!(
                !side_effect_applied,
                "read-only detection applies no durable external side effect"
            );
            assert_eq!(
                emitted_events.len(),
                1,
                "exactly one event (ProjectRescanned) is carried for the in-txn append"
            );
        }
        ExecutionOutcome::Failed(e) => panic!("expected Succeeded, got Failed: {e}"),
        ExecutionOutcome::FailedWithEvents { detail, .. } => {
            panic!("project.rescan never returns FailedWithEvents, got: {detail}")
        }
    }
}
