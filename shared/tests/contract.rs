//! Phase 0.5 shared-contract-freeze — Rust authority contract tests (RED first).
//! Value sets pinned from ARCHITECTURE §5.1 (LOCKED reconciliation, R-4..R-9) for
//! the status machines, §5.2 for IDs, §7.1/R-2 for the actor enum, §5.3 for the
//! desktop objects. (DATA_MODEL §4 is the older ROUGH-DRAFT 8-machine list and is
//! superseded by §5.1 — see Step-9 cross-doc flag.)

use std::collections::BTreeSet;

use serde::de::DeserializeOwned;
use serde::Serialize;

/// the snake_case wire string a contract enum serializes to
fn wire<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .unwrap()
        .as_str()
        .expect("contract enum must serialize to a JSON string")
        .to_string()
}

/// every variant serializes to its exact wire string, round-trips, and the value
/// set == `expected` (exact set + count).
fn check_values<T>(all: &[T], expected: &[&str])
where
    T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug + Copy,
{
    let got: BTreeSet<String> = all.iter().map(wire).collect();
    let exp: BTreeSet<String> = expected.iter().map(|s| s.to_string()).collect();
    assert_eq!(got, exp, "value-set mismatch");
    assert_eq!(all.len(), expected.len(), "variant count mismatch (dupes?)");
    for v in all {
        let json = serde_json::to_value(v).unwrap();
        let back: T = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(*v, back, "round-trip failed for {json:?}");
    }
}

/// terminal-state set for a status enum exposing `ALL` + `is_terminal()`.
macro_rules! check_terminal {
    ($ty:ty, $expected:expr) => {{
        let got: BTreeSet<String> = <$ty>::ALL
            .iter()
            .filter(|v| v.is_terminal())
            .map(|v| wire(v))
            .collect();
        let exp: BTreeSet<String> = $expected.iter().map(|s| s.to_string()).collect();
        assert_eq!(
            got, exp,
            concat!("terminal-set mismatch for ", stringify!($ty))
        );
    }};
}

// ---- Test 1 — every state-machine value present + serializes (§5.1) ----------

#[test]
fn test_every_state_machine_value_present_and_serializes() {
    use nexusops_shared::status::*;

    check_values(
        SessionStatus::ALL,
        &[
            "creating",
            "starting",
            "active",
            "thinking",
            "running_command",
            "editing_files",
            "running_tests",
            "waiting_on_permission",
            "waiting_on_human_input",
            "waiting_on_external_service",
            "idle",
            "stale",
            "changes_ready",
            "failed",
            "completed",
            "archived",
            "killed",
        ],
    );
    check_values(
        TaskStatus::ALL,
        &[
            "unassigned",
            "queued",
            "assigned",
            "ready",
            "in_progress",
            "blocked",
            "needs_clarification",
            "in_review",
            "changes_ready",
            "pr_opened",
            "needs_review",
            "requested_changes",
            "done",
            "deferred",
            "merged",
            "closed",
            "abandoned",
        ],
    );
    check_values(
        WorktreeGitStatus::ALL,
        &[
            "clean",
            "dirty",
            "untracked_files",
            "conflicts",
            "behind_base",
            "ahead_of_base",
        ],
    );
    check_values(
        WorktreeOverlayStatus::ALL,
        &[
            "creating", "locked", "pr_open", "merged", "prunable", "deleted",
        ],
    );
    check_values(
        PullRequestStatus::ALL,
        &[
            "draft",
            "open",
            "checks_pending",
            "checks_failing",
            "needs_review",
            "changes_requested",
            "approved",
            "mergeable",
            "conflict",
            "merged",
            "closed",
        ],
    );
    check_values(
        WorkflowInstanceStatus::ALL,
        &[
            "not_detected",
            "pack_available",
            "needs_personalization",
            "personalization_in_progress",
            "generated_review_required",
            "active",
            "ready_for_team_run",
            "degraded",
            "drift_detected",
            "upgrade_available",
            "archived",
            "detached",
        ],
    );
    check_values(
        ProjectBrainStatus::ALL,
        &[
            "not_configured",
            "indexing",
            "ready",
            "partial_index",
            "stale",
            "graph_degraded",
            "transcript_ingestion_off",
            "transcript_ingestion_active",
            "reindex_required",
            "error",
        ],
    );
    check_values(
        ApprovalStatus::ALL,
        &[
            "requested",
            "previewed",
            "awaiting_approval",
            "approved",
            "denied",
            "edited",
            "auto_approved_by_policy",
            "expired",
            "cancelled",
            "escalated",
        ],
    );
    check_values(
        ActionRequestStatus::ALL,
        &[
            "submitted",
            "previewed",
            "policy_decided",
            "awaiting_approval",
            "approved",
            "denied",
            "queued",
            "executing",
            "succeeded",
            "failed",
            "partially_succeeded",
            "rolled_back",
            "rollback_failed",
            "cancelled",
            "expired",
        ],
    );
    check_values(
        AgentTeamStatus::ALL,
        &[
            "draft",
            "starting",
            "active",
            "waiting_on_human",
            "blocked",
            "reconciling_outputs",
            "completed",
            "failed",
            "archived",
        ],
    );
}

// ---- Test 2 — terminal states marked (§5.1 bold) ----------------------------

#[test]
fn test_terminal_states_marked() {
    use nexusops_shared::status::*;
    check_terminal!(SessionStatus, ["failed", "completed", "archived", "killed"]);
    check_terminal!(TaskStatus, ["merged", "closed", "abandoned"]);
    check_terminal!(WorktreeGitStatus, [] as [&str; 0]); // git axis has no terminal
    check_terminal!(WorktreeOverlayStatus, ["deleted"]);
    check_terminal!(PullRequestStatus, ["merged", "closed"]);
    check_terminal!(WorkflowInstanceStatus, ["archived", "detached"]);
    check_terminal!(ProjectBrainStatus, ["error"]);
    check_terminal!(
        ApprovalStatus,
        [
            "approved",
            "denied",
            "edited",
            "auto_approved_by_policy",
            "expired",
            "cancelled",
            "escalated"
        ]
    );
    check_terminal!(
        ActionRequestStatus,
        [
            "succeeded",
            "failed",
            "partially_succeeded",
            "rolled_back",
            "rollback_failed",
            "cancelled",
            "expired"
        ]
    );
    check_terminal!(AgentTeamStatus, ["completed", "failed", "archived"]);
}

// ---- Test 3 — 22 IDs present, prefixes total + unique (§5.2) -----------------

#[test]
fn test_all_22_ids_present_with_prefixes() {
    use nexusops_shared::ids::IdKind;

    assert_eq!(IdKind::ALL.len(), 22, "exactly 22 shared IDs");

    // the 6 external (native-valued) kinds carry NO prefix
    let external = [
        IdKind::BranchName,
        IdKind::CommitSha,
        IdKind::ArchitectureAnchor,
        IdKind::LinearIssueId,
        IdKind::GithubIssueNumber,
        IdKind::PrNumber,
    ];
    for k in external {
        assert_eq!(k.prefix(), None, "{k:?} is external → no ULID prefix");
    }

    // the 16 platform-minted kinds + their canonical ULID prefixes
    let minted: &[(IdKind, &str)] = &[
        (IdKind::WorkspaceId, "ws_"),
        (IdKind::ProjectId, "proj_"),
        (IdKind::RepoId, "repo_"),
        (IdKind::WorktreeId, "wt_"),
        (IdKind::SessionId, "sess_"),
        (IdKind::AgentTeamId, "team_"),
        (IdKind::ExecutionProfileId, "prof_"),
        (IdKind::WorkflowPackId, "pack_"),
        (IdKind::WorkflowInstanceId, "wfi_"),
        (IdKind::WorkflowCommandId, "cmd_"),
        (IdKind::ImplementationPlanId, "plan_"),
        (IdKind::PlanTaskId, "task_"),
        (IdKind::ActionRequestId, "act_"),
        (IdKind::EventId, "evt_"),
        (IdKind::ArtifactId, "artf_"),
        (IdKind::EvidenceItemId, "evid_"),
    ];
    assert_eq!(minted.len(), 16);

    let mut seen = BTreeSet::new();
    for (kind, prefix) in minted {
        assert_eq!(kind.prefix(), Some(*prefix), "{kind:?} prefix");
        assert_eq!(IdKind::from_prefix(prefix), Some(*kind), "{prefix} → kind");
        assert!(seen.insert(*prefix), "prefix {prefix} is not unique");
    }

    // totality: every kind is either external (no prefix) or platform-minted (prefix)
    for k in IdKind::ALL {
        assert_eq!(
            k.prefix().is_none(),
            external.contains(k),
            "{k:?}: prefix presence must match external-ness"
        );
    }
}

#[test]
fn test_minted_id_newtype_carries_prefix_and_parses() {
    use nexusops_shared::ids::{IdKind, SessionId};
    let id = SessionId::new();
    assert!(
        id.as_str().starts_with("sess_"),
        "minted SessionId carries sess_"
    );
    assert_eq!(SessionId::KIND, IdKind::SessionId);
    let parsed = SessionId::parse(id.as_str()).expect("round-trip parse");
    assert_eq!(parsed, id);
    // wrong-prefix / malformed value is rejected (fail-closed, §15)
    assert!(SessionId::parse("wt_01ARZ3NDEKTSV4RRFFQ69G5FAV").is_err());
    assert!(SessionId::parse("sess_not-a-ulid").is_err());
}

/// mint → prefix → parse round-trip for every platform-minted newtype.
macro_rules! check_minted {
    ($ty:ty, $prefix:literal) => {{
        let id = <$ty>::new();
        assert!(
            id.as_str().starts_with($prefix),
            concat!(stringify!($ty), " must carry ", $prefix)
        );
        assert_eq!(<$ty>::parse(id.as_str()).unwrap(), id, "round-trip parse");
        // PREFIX const, KIND.prefix(), and the literal must be one single truth
        assert_eq!(<$ty>::PREFIX, $prefix, "PREFIX const agrees");
        assert_eq!(
            <$ty>::KIND.prefix(),
            Some(<$ty>::PREFIX),
            "KIND.prefix() == PREFIX"
        );
    }};
}

#[test]
fn test_all_minted_newtypes() {
    use nexusops_shared::ids::*;
    check_minted!(WorkspaceId, "ws_");
    check_minted!(ProjectId, "proj_");
    check_minted!(RepoId, "repo_");
    check_minted!(WorktreeId, "wt_");
    check_minted!(SessionId, "sess_");
    check_minted!(AgentTeamId, "team_");
    check_minted!(ExecutionProfileId, "prof_");
    check_minted!(WorkflowPackId, "pack_");
    check_minted!(WorkflowInstanceId, "wfi_");
    check_minted!(WorkflowCommandId, "cmd_");
    check_minted!(ImplementationPlanId, "plan_");
    check_minted!(PlanTaskId, "task_");
    check_minted!(ActionRequestId, "act_");
    check_minted!(EventId, "evt_");
    check_minted!(ArtifactId, "artf_");
    check_minted!(EvidenceItemId, "evid_");
}

#[test]
fn test_external_id_newtypes_keep_native_values() {
    use nexusops_shared::ids::{BranchName, GithubIssueNumber, IdKind, PrNumber};
    // external IDs are NOT re-minted — they wrap their native provider value (§5.2)
    assert_eq!(PrNumber(84).0, 84);
    assert_eq!(PrNumber::KIND, IdKind::PrNumber);
    assert_eq!(GithubIssueNumber(7).0, 7);
    assert_eq!(BranchName("feature/x".into()).0, "feature/x");
    assert_eq!(BranchName::KIND.prefix(), None);
}

// ---- Test 4 — actor enum == R-2 set (§7.1) ----------------------------------

#[test]
fn test_actor_enum_matches_r2() {
    use nexusops_shared::actor::ActorType;
    check_values(
        ActorType::ALL,
        &[
            "user",
            "project_brain",
            "action_gateway",
            "workflow_runtime",
            "local_runner",
            "session_adapter",
            "integration_syncer",
            "system",
            "remote_client",
            "automation_policy",
        ],
    );
}

// ---- Test 5 — desktop objects defined + deferred marked (§5.3) ---------------

#[test]
fn test_desktop_objects_defined_and_deferred_marked() {
    use nexusops_shared::objects::DesktopObjectKind as D;
    assert_eq!(D::ALL.len(), 4);
    // MVP-live
    assert!(!D::LocalRunner.is_deferred());
    assert!(!D::EventProjection.is_deferred());
    // dormant iOS scaffolding
    assert!(D::Device.is_deferred());
    assert!(D::RemoteClient.is_deferred());
    // identity prefixes (DATA_MODEL §6)
    assert_eq!(D::LocalRunner.id_prefix(), "lr_");
    assert_eq!(D::EventProjection.id_prefix(), "eprj_");
    assert_eq!(D::Device.id_prefix(), "dev_");
    assert_eq!(D::RemoteClient.id_prefix(), "rc_");
}

// ---- Test 6 — ExecutionProfile HELD, not frozen (guardrail 1 / cat-4) --------

#[test]
fn test_execution_profile_held_not_frozen() {
    // ExecutionProfile's runtime states could be reshaped by the cat-4 SDK-vs-PTY
    // + ≥6/15 credit-pool drain → re-frozen in 0.5b. The hold must be DELIBERATE
    // (a marker), not silently missing.
    let marker = nexusops_shared::EXECUTION_PROFILE_STATUS_HELD;
    assert!(!marker.is_empty(), "hold marker must explain itself");
    assert!(marker.contains("0.5b"), "marker names the follow-up slice");
}

// ---- Test 7 — unknown value rejected at the parse boundary (§0.5 / §15) ------

#[test]
fn test_unknown_value_rejected() {
    use nexusops_shared::actor::ActorType;
    use nexusops_shared::ids::IdKind;
    use nexusops_shared::status::SessionStatus;

    assert!(serde_json::from_value::<SessionStatus>(serde_json::json!("not_a_status")).is_err());
    assert!(serde_json::from_value::<ActorType>(serde_json::json!("not_an_actor")).is_err());
    assert_eq!(IdKind::from_prefix("zzz_"), None);
    // near-miss prefixes (missing trailing underscore / wrong) also fail-closed
    assert_eq!(IdKind::from_prefix("task"), None);
    assert_eq!(IdKind::from_prefix("sess"), None);
    assert_eq!(IdKind::from_prefix(""), None);
}

// ---- Test 9 — schema artifact matches the Rust authority (CI diff gate, §5.0) -

#[test]
fn test_schema_artifact_matches_rust() {
    let generated = nexusops_shared::schema::emit_schema_json();
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/contracts/schema/nexusops-contract.schema.json"
    );
    let checked_in = std::fs::read_to_string(path)
        .expect("checked-in schema present (run `cargo run --bin emit_schema`)");
    // exact byte comparison (emit_schema_json guarantees the trailing newline) so
    // even whitespace drift is caught by the gate.
    assert_eq!(
        generated, checked_in,
        "schema drift — regenerate with `cargo run --bin emit_schema` and commit"
    );
}
