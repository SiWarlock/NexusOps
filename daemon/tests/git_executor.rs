//! P5.2 (edges-020) — the `git.create_worktree` executor: the FIRST real edges FS/git MUTATION.
//!
//! `GitExecutor` (`ExecutorKind::Git`) handles `git.create_worktree` (edges-020) AND `git.create_branch`
//! (edges-021) by shelling to the **git CLI** (forbidden #6 — NEVER git2 for mutations) via an injected
//! `GitCli` seam and emits `WorktreeCreated`/`BranchCreated` through the in-txn §15 gate via the
//! edges-019 `EmittedEvent::Namespaced` bridge. `side_effect_applied: true` (a real FS/git change →
//! honest `ActionPartiallySucceeded` on a txn-B fault). `git.status`/`git.diff` (also
//! `ExecutorKind::Git`) delegate to the inner stub (served via the read path).
//!
//! **Test strategy** (note the §7.2 approve-path redaction interaction — see the Step-2.5 finding):
//!  * Tests 1-8 + 9a call `execute()` DIRECTLY with RAW inputs (no approve-path redaction).
//!  * 9a uses the REAL `SystemGitCli` over a hermetic temp repo → proves a real worktree is created.
//!  * 9b drives the full submit→approve→execute Gateway path with `FakeGitCli` + LOW-ENTROPY synthetic
//!    paths (so the §15 inputs-redaction backstop does not mask them) → proves approve-path reachability.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use git2::{Repository, RepositoryInitOptions, Signature};
use tempfile::{tempdir, TempDir};

use nexusops_shared::actions::{
    ActionRequest, RequesterType, ResourceRef, ResourceType, RiskLevel,
};
use nexusops_shared::catalog::ExecutorKind;
use nexusops_shared::events::{BranchCreated, WorktreeCreated};
use nexusops_shared::ids::{ActionRequestId, WorktreeId};
use nexusops_shared::status::ActionRequestStatus;
use nexusops_shared::time::Timestamp;
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{EventStore, PrefixRedactor};
use nexusopsd::gateway::executor::{ActionExecutor, CatalogExecutor, ExecutionOutcome};
use nexusopsd::gateway::policy::CatalogPolicy;
use nexusopsd::gateway::Gateway;
use nexusopsd::git::cli::{FakeGitCli, SystemGitCli};
use nexusopsd::git::executor::GitExecutor;
use nexusopsd::idgen::UlidGen;

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

fn temp_db() -> (TempDir, std::path::PathBuf) {
    let d = tempdir().unwrap();
    let p = d.path().join("nexusops.db");
    (d, p)
}

/// A `git.create_worktree` ActionRequest: operational params in `inputs`, the repo identity in a
/// `resource_ref` (the catalog `requires_resource_refs` precondition).
fn worktree_req(
    repo_path: &str,
    worktree_path: &str,
    branch: &str,
    base: Option<&str>,
    with_ref: bool,
) -> ActionRequest {
    let mut inputs = serde_json::json!({
        "repo_path": repo_path,
        "worktree_path": worktree_path,
        "branch_name": branch,
    });
    if let Some(b) = base {
        inputs["base_branch"] = serde_json::json!(b);
    }
    ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id: None,
        action_type: "git.create_worktree".to_string(),
        requester_type: RequesterType::User,
        requester_id: "u_local".to_string(),
        resource_refs: if with_ref {
            vec![ResourceRef {
                resource_type: ResourceType::Repo,
                id: "repo_x".to_string(),
                uri: None,
            }]
        } else {
            vec![]
        },
        inputs,
        risk_level: RiskLevel::Level2,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        created_at: Timestamp::parse(FIXED_TS).unwrap(),
    }
}

/// A bare `git.<verb>` req with a resource_ref (for the delegate-to-stub test).
fn git_verb_req(action_type: &str) -> ActionRequest {
    let mut req = worktree_req("/repo", "/repo/wt", "feature", None, true);
    req.action_type = action_type.to_string();
    req
}

fn gateway_with_git_executor(cli: Box<dyn nexusopsd::git::cli::GitCli>) -> Gateway {
    let mut catalog = CatalogExecutor::new();
    catalog.register(ExecutorKind::Git, Arc::new(GitExecutor::new(cli)));
    Gateway::new(Box::new(CatalogPolicy), Box::new(catalog))
}

fn worktree_created_events(store: &EventStore) -> Vec<WorktreeCreated> {
    store
        .read_all()
        .unwrap()
        .iter()
        .filter(|e| e.event_type == "WorktreeCreated")
        .map(|e| serde_json::from_str(&e.payload_json).expect("WorktreeCreated parses"))
        .collect()
}

/// the single approvals row's approval_id (the submit→approve flow, mirrors tests/executor.rs).
fn approval_id_of(path: &Path) -> String {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row("SELECT approval_id FROM approvals", [], |r| r.get(0))
        .expect("an approval")
}

// hermetic git repo (for the real-CLI test 9a) ------------------------------
fn init_repo(path: &Path) -> Repository {
    let mut opts = RepositoryInitOptions::new();
    opts.initial_head("main");
    Repository::init_opts(path, &opts).expect("init repo")
}

fn commit_file(repo: &Repository, name: &str, content: &str) {
    let workdir = repo.workdir().expect("non-bare repo");
    fs::write(workdir.join(name), content).expect("write file");
    let mut index = repo.index().expect("index");
    index.add_path(Path::new(name)).expect("add path");
    index.write().expect("write index");
    let tree_oid = index.write_tree().expect("write tree");
    let tree = repo.find_tree(tree_oid).expect("find tree");
    let sig = Signature::now("Test", "test@example.com").expect("sig");
    repo.commit(Some("HEAD"), &sig, &sig, "commit", &tree, &[])
        .expect("commit");
}

// ---- 1. forbidden #6: mutation via the git CLI ----------------------------

#[test]
fn test_create_worktree_invokes_git_cli_add() {
    // spec(forbidden #6): the worktree is created by the canonical (option-before-operand)
    // `git worktree add -b <branch> <path> [<base>]`, run in the repo cwd, via the INJECTED CLI runner
    // (never git2).
    // with a base branch:
    let fake = FakeGitCli::succeeding();
    let invocations = fake.invocations();
    let exec = GitExecutor::new(Box::new(fake));
    let _ = exec.execute(&worktree_req(
        "/repo",
        "/repo/wt",
        "feature",
        Some("main"),
        true,
    ));
    let inv = invocations.lock().unwrap();
    assert_eq!(inv.len(), 1, "exactly one git invocation");
    assert_eq!(
        inv[0].0,
        vec!["worktree", "add", "-b", "feature", "/repo/wt", "main"],
        "git worktree add -b <branch> <path> <base>"
    );
    assert_eq!(inv[0].1, Path::new("/repo"), "run in the repo cwd");
    drop(inv);

    // without a base branch — the trailing base operand is absent (mutation-detecting).
    let fake = FakeGitCli::succeeding();
    let invocations = fake.invocations();
    let exec = GitExecutor::new(Box::new(fake));
    let _ = exec.execute(&worktree_req("/repo", "/repo/wt", "feature", None, true));
    assert_eq!(
        invocations.lock().unwrap()[0].0,
        vec!["worktree", "add", "-b", "feature", "/repo/wt"],
        "no base → no trailing operand"
    );
}

// ---- 2. emission ----------------------------------------------------------

#[test]
fn test_create_worktree_emits_worktree_created() {
    // spec(§6.3/§7.2): success → exactly one WorktreeCreated whose path/branch_name/base_branch match
    // inputs + a freshly-minted `wt_` worktree_id.
    let exec = GitExecutor::new(Box::new(FakeGitCli::succeeding()));
    let payload = match exec.execute(&worktree_req(
        "/repo",
        "/repo/wt",
        "feature",
        Some("main"),
        true,
    )) {
        ExecutionOutcome::Succeeded { emitted_events, .. } => {
            assert_eq!(emitted_events.len(), 1);
            match &emitted_events[0] {
                nexusopsd::gateway::executor::EmittedEvent::Namespaced {
                    event_type,
                    payload_json,
                } => {
                    assert_eq!(*event_type, "WorktreeCreated");
                    serde_json::from_str::<WorktreeCreated>(payload_json).expect("parses")
                }
                _ => panic!("expected a Namespaced WorktreeCreated"),
            }
        }
        ExecutionOutcome::Failed(e) => panic!("expected Succeeded, got Failed: {e}"),
        ExecutionOutcome::FailedWithEvents { detail, .. } => {
            panic!("git executor never returns FailedWithEvents, got: {detail}")
        }
    };
    assert_eq!(payload.path, "/repo/wt");
    assert_eq!(payload.branch_name, "feature");
    assert_eq!(payload.base_branch.as_deref(), Some("main"));
    assert!(
        WorktreeId::parse(payload.worktree_id.as_str()).is_ok(),
        "a freshly-minted, valid wt_ worktree_id"
    );
    assert!(payload.worktree_id.as_str().starts_with("wt_"));
}

// ---- 3. partial-success contract ------------------------------------------

#[test]
fn test_create_worktree_side_effect_applied_true() {
    // spec(§17): a real FS worktree was created → side_effect_applied: true, so a txn-B append fault
    // yields ActionPartiallySucceeded (the honest divergence), NOT a clean rollback.
    let exec = GitExecutor::new(Box::new(FakeGitCli::succeeding()));
    match exec.execute(&worktree_req("/repo", "/repo/wt", "feature", None, true)) {
        ExecutionOutcome::Succeeded {
            side_effect_applied,
            ..
        } => assert!(side_effect_applied, "a real FS worktree was created"),
        ExecutionOutcome::Failed(e) => panic!("expected Succeeded, got Failed: {e}"),
        ExecutionOutcome::FailedWithEvents { detail, .. } => {
            panic!("git executor never returns FailedWithEvents, got: {detail}")
        }
    }
}

// ---- 4. CLI failure → Failed, no event ------------------------------------

#[test]
fn test_create_worktree_cli_failure_is_failed_no_event() {
    // spec(fail-before-event): a CLI failure — non-zero exit OR a spawn error — → Failed, NO
    // WorktreeCreated (no phantom record).
    for cli in [FakeGitCli::failing(), FakeGitCli::spawn_error()] {
        let exec = GitExecutor::new(Box::new(cli));
        match exec.execute(&worktree_req("/repo", "/repo/wt", "feature", None, true)) {
            ExecutionOutcome::Failed(_) => {} // expected
            ExecutionOutcome::FailedWithEvents { detail, .. } => {
                panic!("git executor never returns FailedWithEvents, got: {detail}")
            }
            ExecutionOutcome::Succeeded { emitted_events, .. } => panic!(
                "expected Failed; got Succeeded with {} events",
                emitted_events.len()
            ),
        }
    }
}

// ---- 5. fail-closed input guard -------------------------------------------

#[test]
fn test_create_worktree_missing_inputs_failed() {
    // spec(fail-closed): absent/blank repo_path OR worktree_path OR branch → Failed, the CLI runner is
    // NEVER invoked (no partial git invocation).
    let fake = FakeGitCli::succeeding();
    let inv = fake.invocations();
    let exec = GitExecutor::new(Box::new(fake));
    // (a) blank repo_path (the cwd) · (b) blank worktree_path · (c) blank branch
    for (repo, wt, branch) in [
        ("   ", "/repo/wt", "feature"),
        ("/repo", "   ", "feature"),
        ("/repo", "/repo/wt", ""),
    ] {
        assert!(
            matches!(
                exec.execute(&worktree_req(repo, wt, branch, None, true)),
                ExecutionOutcome::Failed(_)
            ),
            "blank input ({repo:?},{wt:?},{branch:?}) fails closed"
        );
    }
    assert_eq!(
        inv.lock().unwrap().len(),
        0,
        "the CLI runner is NEVER invoked on a fail-closed input"
    );
}

#[test]
fn test_create_worktree_rejects_dash_leading_operand() {
    // spec(INV-SEC-1 / argument-injection): a git operand starting with `-` (which git would parse as
    // an OPTION, e.g. base `--no-checkout` silently altering the create → audit/reality divergence) is
    // rejected fail-closed, the CLI runner NEVER invoked. Covers worktree_path / branch_name / base.
    let fake = FakeGitCli::succeeding();
    let inv = fake.invocations();
    let exec = GitExecutor::new(Box::new(fake));
    for (repo, wt, branch, base) in [
        ("/repo", "-rf", "feature", None),
        ("/repo", "/repo/wt", "-x", None),
        ("/repo", "/repo/wt", "feature", Some("--no-checkout")),
    ] {
        match exec.execute(&worktree_req(repo, wt, branch, base, true)) {
            ExecutionOutcome::Failed(_) => {}
            ExecutionOutcome::FailedWithEvents { detail, .. } => {
                panic!("git executor never returns FailedWithEvents, got: {detail}")
            }
            ExecutionOutcome::Succeeded { .. } => {
                panic!("a dash-leading operand ({wt:?},{branch:?},{base:?}) must fail closed")
            }
        }
    }
    assert_eq!(
        inv.lock().unwrap().len(),
        0,
        "the CLI runner is NEVER invoked when an operand is rejected"
    );
}

// ---- 6. forbidden #6 structural pin ---------------------------------------

#[test]
fn test_git_executor_no_git2_mutation() {
    // spec(forbidden #6): the git-MUTATION files (executor.rs + the cli.rs seam) perform the worktree
    // mutation via the git CLI, NEVER a git2 API. Structural pin: neither source uses a `git2::` path
    // (the word "git2" may appear only in a forbidden-#6 comment). The READ backend (detect/reads/
    // precedence) legitimately uses git2 reads and is deliberately NOT grepped here.
    for file in ["/src/git/executor.rs", "/src/git/cli.rs"] {
        let src = fs::read_to_string(format!("{}{file}", env!("CARGO_MANIFEST_DIR")))
            .unwrap_or_else(|_| panic!("read {file}"));
        assert!(
            !src.contains("git2::"),
            "git{file} must call NO git2 API — mutations go through the git CLI (forbidden #6)"
        );
    }
}

// ---- 7. catalog precondition ----------------------------------------------

#[test]
fn test_create_worktree_requires_resource_ref() {
    // spec(§6.3): the catalog `requires_resource_refs` precondition (the repo identity) is enforced —
    // no resource_ref → Failed.
    let exec = GitExecutor::new(Box::new(FakeGitCli::succeeding()));
    assert!(matches!(
        exec.execute(&worktree_req("/repo", "/repo/wt", "feature", None, false)),
        ExecutionOutcome::Failed(_)
    ));
}

// ---- 8. shared ExecutorKind::Git dispatch — status/diff still delegate -----

#[test]
fn test_git_status_diff_still_delegate_to_stub() {
    // spec(§6.3): git.status/git.diff dispatched to GitExecutor still delegate to the inner stub
    // (no-op success, NO event) AFTER the create_branch arm lands — served via the read path, not
    // handled here. (git.create_branch is now a real mutator — see the create_branch tests below.)
    let exec = GitExecutor::new(Box::new(FakeGitCli::succeeding()));
    for verb in ["git.status", "git.diff"] {
        match exec.execute(&git_verb_req(verb)) {
            ExecutionOutcome::Succeeded { emitted_events, .. } => {
                assert!(emitted_events.is_empty(), "{verb} emits no event (stub)");
            }
            ExecutionOutcome::Failed(e) => panic!("{verb} should stub-succeed, got Failed: {e}"),
            ExecutionOutcome::FailedWithEvents { detail, .. } => {
                panic!("{verb} should stub-succeed, got FailedWithEvents: {detail}")
            }
        }
    }
}

// ---- 9a. REAL git CLI over a hermetic repo (direct execute, raw inputs) ----

#[test]
fn test_create_worktree_real_cli_creates_worktree() {
    // spec(forbidden #6, end-to-end real CLI): SystemGitCli over a real temp repo actually creates a
    // worktree directory on disk + emits WorktreeCreated with the real path. Direct execute() with RAW
    // inputs (the approve-path §15 redaction would mask the high-entropy temp path — see test 9b).
    let repo_dir = tempdir().unwrap();
    let repo = init_repo(repo_dir.path());
    commit_file(&repo, "README.md", "hi");
    let wt_path = repo_dir.path().join("wt-feature");

    let exec = GitExecutor::new(Box::new(SystemGitCli));
    let outcome = exec.execute(&worktree_req(
        repo_dir.path().to_str().unwrap(),
        wt_path.to_str().unwrap(),
        "feature",
        None,
        true,
    ));
    assert!(
        matches!(outcome, ExecutionOutcome::Succeeded { .. }),
        "the real git worktree add succeeded"
    );
    assert!(wt_path.is_dir(), "a real worktree directory exists on disk");
    assert!(
        wt_path.join(".git").exists(),
        "the worktree has its .git link"
    );
}

// ---- 9b. e2e Gateway approve-path reachability (FakeGitCli, low-entropy) ---

#[test]
fn test_create_worktree_e2e_via_submit_action_approve() {
    // spec(§6.3 reachability): submit `git.create_worktree` (risk-2) → AwaitingApproval (gate holds) →
    // approve → execute → WorktreeCreated persisted. FakeGitCli + LOW-ENTROPY synthetic paths so the
    // §7.2 approve-path §15 inputs-redaction does not mask them (a high-entropy real temp path WOULD
    // be masked — the Step-2.5 finding; this test proves the approval-path WIRING, not path fidelity).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gateway_with_git_executor(Box::new(FakeGitCli::succeeding()));

    let ack = gw
        .submit_action(
            &mut store,
            worktree_req("/repo", "/repo/wt", "feature", None, true),
        )
        .expect("submit");
    assert_eq!(
        ack.status,
        ActionRequestStatus::AwaitingApproval,
        "risk-2 holds at the approval gate (no auto-execute)"
    );
    assert_eq!(
        worktree_created_events(&store).len(),
        0,
        "no WorktreeCreated before approval"
    );

    gw.approve(&mut store, &approval_id_of(&path))
        .expect("approve drives execute");
    let events = worktree_created_events(&store);
    assert_eq!(
        events.len(),
        1,
        "approve → execute → WorktreeCreated persisted"
    );
    assert_eq!(
        events[0].path, "/repo/wt",
        "low-entropy path survives redaction"
    );
    assert_eq!(events[0].branch_name, "feature");
    assert!(
        events[0].worktree_id.as_str().starts_with("wt_"),
        "the approve path mints a valid wt_ id too"
    );
}

// ============================================================================
// edges-021 — the git.create_branch arm (extends GitExecutor)
// ============================================================================

/// A `git.create_branch` ActionRequest: `branch_name` (+ optional `base` start-point) + the repo cwd
/// in `inputs`, the repo identity in a `resource_ref`.
fn branch_req(repo_path: &str, branch: &str, base: Option<&str>, with_ref: bool) -> ActionRequest {
    let mut inputs = serde_json::json!({
        "repo_path": repo_path,
        "branch_name": branch,
    });
    if let Some(b) = base {
        inputs["base"] = serde_json::json!(b);
    }
    ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id: None,
        action_type: "git.create_branch".to_string(),
        requester_type: RequesterType::User,
        requester_id: "u_local".to_string(),
        resource_refs: if with_ref {
            vec![ResourceRef {
                resource_type: ResourceType::Repo,
                id: "repo_x".to_string(),
                uri: None,
            }]
        } else {
            vec![]
        },
        inputs,
        risk_level: RiskLevel::Level2,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        created_at: Timestamp::parse(FIXED_TS).unwrap(),
    }
}

fn branch_created_events(store: &EventStore) -> Vec<BranchCreated> {
    store
        .read_all()
        .unwrap()
        .iter()
        .filter(|e| e.event_type == "BranchCreated")
        .map(|e| serde_json::from_str(&e.payload_json).expect("BranchCreated parses"))
        .collect()
}

#[test]
fn test_create_branch_invokes_git_cli_branch() {
    // spec(forbidden #6): the branch is created by `git branch <name> [<start-point>]` run in the repo
    // cwd, via the INJECTED CLI runner (never git2).
    // with a start-point:
    let fake = FakeGitCli::succeeding();
    let inv = fake.invocations();
    let exec = GitExecutor::new(Box::new(fake));
    let _ = exec.execute(&branch_req("/repo", "feature", Some("main"), true));
    assert_eq!(
        inv.lock().unwrap()[0].0,
        vec!["branch", "feature", "main"],
        "git branch <name> <start-point>"
    );
    assert_eq!(inv.lock().unwrap()[0].1, Path::new("/repo"), "repo cwd");

    // without a start-point → current HEAD:
    let fake = FakeGitCli::succeeding();
    let inv = fake.invocations();
    let exec = GitExecutor::new(Box::new(fake));
    let _ = exec.execute(&branch_req("/repo", "feature", None, true));
    assert_eq!(
        inv.lock().unwrap()[0].0,
        vec!["branch", "feature"],
        "no start-point → no trailing operand"
    );
}

#[test]
fn test_create_branch_emits_branch_created() {
    // spec(§6.3/§7.2): success → exactly one BranchCreated whose branch_name/base match inputs.
    let exec = GitExecutor::new(Box::new(FakeGitCli::succeeding()));
    let payload = match exec.execute(&branch_req("/repo", "feature", Some("main"), true)) {
        ExecutionOutcome::Succeeded { emitted_events, .. } => {
            assert_eq!(emitted_events.len(), 1);
            match &emitted_events[0] {
                nexusopsd::gateway::executor::EmittedEvent::Namespaced {
                    event_type,
                    payload_json,
                } => {
                    assert_eq!(*event_type, "BranchCreated");
                    serde_json::from_str::<BranchCreated>(payload_json).expect("parses")
                }
                _ => panic!("expected a Namespaced BranchCreated"),
            }
        }
        ExecutionOutcome::Failed(e) => panic!("expected Succeeded, got Failed: {e}"),
        ExecutionOutcome::FailedWithEvents { detail, .. } => {
            panic!("git executor never returns FailedWithEvents, got: {detail}")
        }
    };
    assert_eq!(payload.branch_name, "feature");
    assert_eq!(payload.base.as_deref(), Some("main"));
}

#[test]
fn test_create_branch_side_effect_applied_true() {
    // spec(§17): a real git branch mutation → side_effect_applied: true (honest ActionPartiallySucceeded
    // on a txn-B fault, not a clean rollback).
    let exec = GitExecutor::new(Box::new(FakeGitCli::succeeding()));
    match exec.execute(&branch_req("/repo", "feature", None, true)) {
        ExecutionOutcome::Succeeded {
            side_effect_applied,
            ..
        } => assert!(side_effect_applied, "a real branch was created"),
        ExecutionOutcome::Failed(e) => panic!("expected Succeeded, got Failed: {e}"),
        ExecutionOutcome::FailedWithEvents { detail, .. } => {
            panic!("git executor never returns FailedWithEvents, got: {detail}")
        }
    }
}

#[test]
fn test_create_branch_cli_failure_is_failed_no_event() {
    // spec(fail-before-event): a CLI failure (non-zero exit OR spawn error) → Failed, NO BranchCreated;
    // the CLI WAS reached (the failure is CLI-level, not an input/guard short-circuit).
    for make in [FakeGitCli::failing, FakeGitCli::spawn_error] {
        let fake = make();
        let inv = fake.invocations();
        let exec = GitExecutor::new(Box::new(fake));
        match exec.execute(&branch_req("/repo", "feature", None, true)) {
            ExecutionOutcome::Failed(_) => {}
            ExecutionOutcome::FailedWithEvents { detail, .. } => {
                panic!("git executor never returns FailedWithEvents, got: {detail}")
            }
            ExecutionOutcome::Succeeded { emitted_events, .. } => panic!(
                "expected Failed; got Succeeded with {} events",
                emitted_events.len()
            ),
        }
        assert_eq!(
            inv.lock().unwrap().len(),
            1,
            "the CLI was reached (a CLI-level failure, not an input short-circuit)"
        );
    }
}

#[test]
fn test_create_branch_missing_inputs_failed() {
    // spec(fail-closed): blank repo_path OR branch_name → Failed, the CLI runner is NEVER invoked.
    let fake = FakeGitCli::succeeding();
    let inv = fake.invocations();
    let exec = GitExecutor::new(Box::new(fake));
    for (repo, branch) in [("/repo", "   "), ("   ", "feature")] {
        assert!(
            matches!(
                exec.execute(&branch_req(repo, branch, None, true)),
                ExecutionOutcome::Failed(_)
            ),
            "blank input ({repo:?},{branch:?}) fails closed"
        );
    }
    assert_eq!(
        inv.lock().unwrap().len(),
        0,
        "CLI never invoked on fail-closed input"
    );
}

#[test]
fn test_create_branch_rejects_dash_leading_operand() {
    // spec(INV-SEC-1 / argument-injection, the edges-020 standing requirement): a branch_name/base
    // starting with `-` (which git would parse as an OPTION, e.g. `--force`) is rejected fail-closed,
    // the CLI runner NEVER invoked — the SHARED guard now covers both git arms.
    let fake = FakeGitCli::succeeding();
    let inv = fake.invocations();
    let exec = GitExecutor::new(Box::new(fake));
    for (branch, base) in [("--force", None), ("feature", Some("--orphan"))] {
        match exec.execute(&branch_req("/repo", branch, base, true)) {
            ExecutionOutcome::Failed(_) => {}
            ExecutionOutcome::FailedWithEvents { detail, .. } => {
                panic!("git executor never returns FailedWithEvents, got: {detail}")
            }
            ExecutionOutcome::Succeeded { .. } => {
                panic!("a dash-leading operand ({branch:?},{base:?}) must fail closed")
            }
        }
    }
    assert_eq!(
        inv.lock().unwrap().len(),
        0,
        "CLI never invoked when an operand is rejected"
    );
}

#[test]
fn test_create_branch_repo_path_dash_not_guarded() {
    // spec(INV-SEC-1 boundary): repo_path is the cwd (`Command::current_dir`), NOT a git arg → it is
    // intentionally EXEMPT from the arg-injection guard. A `-`-leading repo_path PASSES the guard
    // (reaches the CLI as the cwd) — proving the exemption is real, so a future author must not add
    // repo_path to the operand list "for symmetry".
    let fake = FakeGitCli::succeeding();
    let inv = fake.invocations();
    let exec = GitExecutor::new(Box::new(fake));
    match exec.execute(&branch_req("--evil", "feature", None, true)) {
        ExecutionOutcome::Succeeded { .. } => {}
        ExecutionOutcome::Failed(e) => {
            panic!("repo_path is exempt from the operand guard; got Failed: {e}")
        }
        ExecutionOutcome::FailedWithEvents { detail, .. } => {
            panic!("git executor never returns FailedWithEvents, got: {detail}")
        }
    }
    assert_eq!(
        inv.lock().unwrap()[0].1,
        Path::new("--evil"),
        "repo_path rides as the cwd (current_dir), not a git arg"
    );
}

#[test]
fn test_create_branch_requires_resource_ref() {
    // spec(§6.3): the catalog requires_resource_refs precondition is enforced — no resource_ref → Failed.
    let exec = GitExecutor::new(Box::new(FakeGitCli::succeeding()));
    assert!(matches!(
        exec.execute(&branch_req("/repo", "feature", None, false)),
        ExecutionOutcome::Failed(_)
    ));
}

#[test]
fn test_create_branch_e2e_via_submit_action_approve() {
    // spec(§6.3 reachability): submit git.create_branch (risk-2) → AwaitingApproval (gate holds, no
    // event) → approve → BranchCreated persisted. FakeGitCli + LOW-ENTROPY paths (§7.2 redaction).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = gateway_with_git_executor(Box::new(FakeGitCli::succeeding()));

    let ack = gw
        .submit_action(
            &mut store,
            branch_req("/repo", "feature", Some("main"), true),
        )
        .expect("submit");
    assert_eq!(
        ack.status,
        ActionRequestStatus::AwaitingApproval,
        "risk-2 holds at the approval gate"
    );
    assert_eq!(
        branch_created_events(&store).len(),
        0,
        "no event before approval"
    );

    gw.approve(&mut store, &approval_id_of(&path))
        .expect("approve drives execute");
    let events = branch_created_events(&store);
    assert_eq!(
        events.len(),
        1,
        "approve → execute → BranchCreated persisted"
    );
    assert_eq!(events[0].branch_name, "feature");
    assert_eq!(events[0].base.as_deref(), Some("main"));
}
