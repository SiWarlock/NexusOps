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
use nexusopsd::git::cli::{FakeGitCli, GitCli, GitCliError, GitCliOutput, SystemGitCli};
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

// ============================================================================
// W1-git-stage (095) — the git.stage_hunk / git.unstage_hunk executor bodies.
// Re-derive the hunk from the live diff (the frozen position-only resource-ref carries LOCATION,
// not content), build a one-hunk patch, and apply it to the INDEX via `git apply --cached [-R]`
// with `git apply --check` fail-closed (§17 read↔mutate race guard). Contract-neutral; no domain
// event (LESSON 47 live-read cache). Real-repo fixtures (the create_worktree 9a real-CLI precedent).
// ============================================================================

use nexusops_shared::ipc::Hunk;
use nexusopsd::git::executor::WorktreePathResolver;

/// the §6.3 frozen hunk resource-ref separator (4.0b-ui1): `\x1f` (unit separator).
const US: char = '\u{1f}';

/// A [`WorktreePathResolver`] double — returns a fixed path (the hermetic repo) for any worktree_id,
/// or `None` (the unresolvable case). Production resolves `worktree_id → proj_worktree.path` over WAL.
struct FixedResolver(Option<std::path::PathBuf>);
impl WorktreePathResolver for FixedResolver {
    fn resolve(&self, _worktree_id: &str) -> Option<std::path::PathBuf> {
        self.0.clone()
    }
}

/// the frozen position-only hunk resource-ref id (4.0b-ui1):
/// `{worktree_id}\x1f{file}\x1f{old_start},{old_lines},{new_start},{new_lines}`.
fn hunk_ref_id(worktree_id: &str, file: &str, h: &Hunk) -> String {
    format!(
        "{worktree_id}{US}{file}{US}{},{},{},{}",
        h.old_start, h.old_lines, h.new_start, h.new_lines
    )
}

fn hunk_req(action_type: &str, ref_id: &str) -> ActionRequest {
    ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id: None,
        action_type: action_type.to_string(),
        requester_type: RequesterType::User,
        requester_id: "u_local".to_string(),
        resource_refs: vec![ResourceRef {
            resource_type: ResourceType::File,
            id: ref_id.to_string(),
            uri: None,
        }],
        inputs: serde_json::json!({}),
        risk_level: RiskLevel::Level2,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        created_at: Timestamp::parse(FIXED_TS).unwrap(),
    }
}

fn stage_executor(repo: &std::path::Path) -> GitExecutor {
    GitExecutor::new(Box::new(SystemGitCli))
        .with_worktree_resolver(Box::new(FixedResolver(Some(repo.to_path_buf()))))
}

/// a real repo with `f.txt` committed then modified → ONE unstaged hunk. Returns the dir + repo path +
/// the live hunk (positions computed via the get_diff backend `read_diff`, mirroring what the UI sends).
fn repo_with_unstaged_hunk() -> (TempDir, std::path::PathBuf, Hunk) {
    let dir = tempdir().unwrap();
    let repo = init_repo(dir.path());
    commit_file(&repo, "f.txt", "line1\nline2\nline3\n");
    fs::write(dir.path().join("f.txt"), "line1\nCHANGED\nline3\n").expect("modify f.txt");
    let path = dir.path().to_path_buf();
    let diff = nexusopsd::git::read_diff(&path, "f.txt").expect("read the live diff");
    let hunk = diff.hunks.into_iter().next().expect("one unstaged hunk");
    (dir, path, hunk)
}

/// run real git in `repo`, return stdout (test helper to inspect the index state).
fn git_stdout(repo: &std::path::Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .expect("git runs");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn git_stage_hunk_applies_to_index() {
    // spec(§6.3) — stage_hunk re-derives the hunk from the live diff + applies it to the INDEX
    // (`git apply --cached`): the index now carries the hunk; the unstaged diff no longer shows it.
    let (_dir, path, hunk) = repo_with_unstaged_hunk();
    let exec = stage_executor(&path);
    let outcome = exec.execute(&hunk_req(
        "git.stage_hunk",
        &hunk_ref_id("wt_abc", "f.txt", &hunk),
    ));
    assert!(
        matches!(
            outcome,
            ExecutionOutcome::Succeeded {
                side_effect_applied: true,
                ..
            }
        ),
        "the hunk staged (a real index change)"
    );
    assert!(
        git_stdout(&path, &["diff", "--cached", "--", "f.txt"]).contains("CHANGED"),
        "the hunk is now staged to the index"
    );
    assert!(
        !git_stdout(&path, &["diff", "--", "f.txt"]).contains("CHANGED"),
        "the unstaged diff no longer shows the staged hunk"
    );
}

#[test]
fn git_unstage_hunk_reverse_applies() {
    // spec(§6.3) — unstage_hunk reverse-applies the STAGED hunk to the index (`git apply --cached -R`):
    // the index no longer carries it. The positions come from the staged diff (index vs HEAD, same here).
    let (_dir, path, hunk) = repo_with_unstaged_hunk();
    git_stdout(&path, &["add", "f.txt"]); // stage it directly first
    assert!(
        git_stdout(&path, &["diff", "--cached", "--", "f.txt"]).contains("CHANGED"),
        "precondition: the hunk is staged"
    );
    let exec = stage_executor(&path);
    match exec.execute(&hunk_req(
        "git.unstage_hunk",
        &hunk_ref_id("wt_abc", "f.txt", &hunk),
    )) {
        ExecutionOutcome::Succeeded {
            side_effect_applied,
            emitted_events,
            ..
        } => {
            assert!(
                side_effect_applied,
                "the hunk unstaged (a real index change)"
            );
            assert!(
                emitted_events.is_empty(),
                "no domain event on unstage either (LESSON 47 live-read cache)"
            );
        }
        _ => panic!("expected Succeeded"),
    }
    assert!(
        !git_stdout(&path, &["diff", "--cached", "--", "f.txt"]).contains("CHANGED"),
        "the index no longer carries the unstaged hunk"
    );
}

#[test]
fn git_stage_hunk_race_changed_file_fails_closed() {
    // spec(§17 read↔mutate) — the file CHANGED since the UI read it (a prepended line SHIFTS the hunk
    // positions) → the resource-ref positions match NO live hunk → Failed, NO index change. The daemon
    // never stages a phantom/divergent hunk.
    let (_dir, path, hunk) = repo_with_unstaged_hunk();
    let ref_id = hunk_ref_id("wt_abc", "f.txt", &hunk); // captured at "read time"
    fs::write(path.join("f.txt"), "NEWTOP\nline1\nCHANGED\nline3\n").expect("race-edit f.txt");
    let exec = stage_executor(&path);
    assert!(
        matches!(
            exec.execute(&hunk_req("git.stage_hunk", &ref_id)),
            ExecutionOutcome::Failed(_)
        ),
        "the hunk no longer matches the live diff → fail-closed"
    );
    assert!(
        git_stdout(&path, &["diff", "--cached", "--name-only"])
            .trim()
            .is_empty(),
        "no index change on a race-failed stage"
    );
}

#[test]
fn git_stage_hunk_no_matching_hunk_fails() {
    // spec(§17) — the resource-ref positions match NO live hunk (the displayed hunk is gone) → Failed.
    let (_dir, path, _hunk) = repo_with_unstaged_hunk();
    let ref_id = format!("wt_abc{US}f.txt{US}999,1,999,1");
    let exec = stage_executor(&path);
    assert!(
        matches!(
            exec.execute(&hunk_req("git.stage_hunk", &ref_id)),
            ExecutionOutcome::Failed(_)
        ),
        "no live hunk at those positions → fail-closed"
    );
}

#[test]
fn git_stage_hunk_malformed_ref_or_missing_target_fails() {
    // spec(§6.3 precondition + LESSON 63) — no resource_ref / a malformed `\x1f` id / non-numeric
    // positions → Failed BEFORE any git call (the CLI is never invoked).
    let fake = FakeGitCli::succeeding();
    let inv = fake.invocations();
    let exec = GitExecutor::new(Box::new(fake)).with_worktree_resolver(Box::new(FixedResolver(
        Some(std::path::PathBuf::from("/repo")),
    )));
    // (a) no resource_ref → requires_resource_refs fails.
    let mut no_ref = hunk_req("git.stage_hunk", "");
    no_ref.resource_refs = vec![];
    assert!(matches!(exec.execute(&no_ref), ExecutionOutcome::Failed(_)));
    // (b) a malformed id (no `\x1f` separators).
    assert!(matches!(
        exec.execute(&hunk_req("git.stage_hunk", "not-a-hunk-ref")),
        ExecutionOutcome::Failed(_)
    ));
    // (c) non-numeric positions.
    let bad = format!("wt_abc{US}f.txt{US}a,b,c,d");
    assert!(matches!(
        exec.execute(&hunk_req("git.stage_hunk", &bad)),
        ExecutionOutcome::Failed(_)
    ));
    assert_eq!(
        inv.lock().unwrap().len(),
        0,
        "the CLI is NEVER invoked on a malformed ref / missing target"
    );
}

#[test]
fn git_stage_hunk_rejects_dash_file_operand() {
    // spec(LESSON 45) — a leading-`-` file operand is rejected fail-closed (defense-in-depth; the CLI is
    // never invoked) even though `git … -- <file>` disambiguates — the standing external-mutator guard.
    let fake = FakeGitCli::succeeding();
    let inv = fake.invocations();
    let exec = GitExecutor::new(Box::new(fake)).with_worktree_resolver(Box::new(FixedResolver(
        Some(std::path::PathBuf::from("/repo")),
    )));
    let ref_id = format!("wt_abc{US}-rf{US}1,3,1,3");
    assert!(matches!(
        exec.execute(&hunk_req("git.stage_hunk", &ref_id)),
        ExecutionOutcome::Failed(_)
    ));
    assert_eq!(
        inv.lock().unwrap().len(),
        0,
        "the CLI is NEVER invoked on a rejected dash file operand"
    );
}

#[test]
fn git_stage_hunk_structural_reason_no_stderr() {
    // spec(§15) — a forced git failure → the persisted `ActionFailed` reason is a STRUCTURAL class,
    // NEVER raw git stderr (a path/diff can carry secrets).
    let exec = GitExecutor::new(Box::new(FakeGitCli::failing())).with_worktree_resolver(Box::new(
        FixedResolver(Some(std::path::PathBuf::from("/repo"))),
    ));
    let ref_id = format!("wt_abc{US}f.txt{US}1,3,1,3");
    match exec.execute(&hunk_req("git.stage_hunk", &ref_id)) {
        ExecutionOutcome::Failed(reason) => assert!(
            !reason.contains("simulated git failure"),
            "the reason is structural, NOT raw git stderr (§15): {reason}"
        ),
        _ => panic!("expected Failed on a git CLI failure"),
    }
}

#[test]
fn git_stage_hunk_emits_no_event() {
    // spec(LESSON 47) — a successful stage emits NO domain event (the worktree git-axis is a live-read
    // cache; the UI re-reads get_diff); `side_effect_applied=true` (a real index change).
    let (_dir, path, hunk) = repo_with_unstaged_hunk();
    let exec = stage_executor(&path);
    match exec.execute(&hunk_req(
        "git.stage_hunk",
        &hunk_ref_id("wt_abc", "f.txt", &hunk),
    )) {
        ExecutionOutcome::Succeeded {
            side_effect_applied,
            emitted_events,
            ..
        } => {
            assert!(side_effect_applied, "a real index change");
            assert!(
                emitted_events.is_empty(),
                "no domain event — the worktree git-axis is a live-read cache (LESSON 47)"
            );
        }
        _ => panic!("expected Succeeded"),
    }
}

#[test]
fn git_stage_hunk_unresolvable_worktree_fails() {
    // spec(§6.1 get_diff posture) — an UNRESOLVABLE worktree_id (the resolver returns None — proj_worktree
    // not yet populated / unknown id) → Failed (NotFound-class), the CLI is NEVER invoked.
    let fake = FakeGitCli::succeeding();
    let inv = fake.invocations();
    let exec =
        GitExecutor::new(Box::new(fake)).with_worktree_resolver(Box::new(FixedResolver(None)));
    let ref_id = format!("wt_unknown{US}f.txt{US}1,3,1,3");
    assert!(matches!(
        exec.execute(&hunk_req("git.stage_hunk", &ref_id)),
        ExecutionOutcome::Failed(_)
    ));
    assert_eq!(
        inv.lock().unwrap().len(),
        0,
        "the CLI is NEVER invoked when the worktree is unresolvable"
    );
}

/// A per-command [`GitCli`] double: `git diff` SUCCEEDS with a canned matching-hunk diff; `git apply
/// --check` FAILS (the §17 GUARD #2 race — a concurrent index change between the diff-read and the
/// apply). Records whether the REAL `git apply` (no `--check`) was ever invoked — it must NOT be.
struct CheckFailsGitCli {
    diff_out: String,
    real_apply_called: Arc<std::sync::atomic::AtomicBool>,
}
impl GitCli for CheckFailsGitCli {
    fn run(&self, args: &[String], _cwd: &Path) -> Result<GitCliOutput, GitCliError> {
        let is_apply = args.first().map(|a| a == "apply").unwrap_or(false);
        let is_check = args.iter().any(|a| a == "--check");
        if is_apply && !is_check {
            self.real_apply_called
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }
        if args.first().map(|a| a == "diff").unwrap_or(false) {
            Ok(GitCliOutput {
                success: true,
                stdout: self.diff_out.clone(),
                stderr: String::new(),
            })
        } else if is_apply && is_check {
            // the §17 GUARD #2: the patch no longer applies cleanly to the index.
            Ok(GitCliOutput {
                success: false,
                stdout: String::new(),
                stderr: "error: patch does not apply".to_string(),
            })
        } else {
            Ok(GitCliOutput {
                success: true,
                stdout: String::new(),
                stderr: String::new(),
            })
        }
    }
}

#[test]
fn git_stage_hunk_apply_check_failure_fails_closed_no_apply() {
    // spec(§17 GUARD #2 + §15) — a positions-MATCH but `git apply --check` FAILS (a concurrent index
    // change) → Failed, the REAL `git apply` is NEVER invoked (no index mutation), and the reason is a
    // STRUCTURAL class — NOT the raw git stderr ("patch does not apply").
    let diff_out = "diff --git a/f.txt b/f.txt\n\
        index 1111111..2222222 100644\n\
        --- a/f.txt\n\
        +++ b/f.txt\n\
        @@ -1,3 +1,3 @@\n \
        line1\n\
        -line2\n\
        +CHANGED\n \
        line3\n"
        .to_string();
    let real_apply_called = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let exec = GitExecutor::new(Box::new(CheckFailsGitCli {
        diff_out,
        real_apply_called: real_apply_called.clone(),
    }))
    .with_worktree_resolver(Box::new(FixedResolver(Some(std::path::PathBuf::from(
        "/repo",
    )))));
    let ref_id = format!("wt_abc{US}f.txt{US}1,3,1,3");
    match exec.execute(&hunk_req("git.stage_hunk", &ref_id)) {
        ExecutionOutcome::Failed(reason) => assert!(
            !reason.contains("patch does not apply"),
            "the §15 reason is structural, NOT raw git stderr: {reason}"
        ),
        _ => panic!("a failed --check must fail closed"),
    }
    assert!(
        !real_apply_called.load(std::sync::atomic::Ordering::SeqCst),
        "the REAL `git apply` is NEVER invoked after a failed --check (no index mutation)"
    );
}
