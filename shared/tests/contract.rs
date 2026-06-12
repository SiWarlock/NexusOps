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
            "denied", // 2.1b Option-A reconcile — terminal-by-nature (no forward edge, never executes)
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
    // MVP-live (LocalRunner + EventProjection + the local desktop-host Device, §16/Option A)
    assert!(!D::LocalRunner.is_deferred());
    assert!(!D::EventProjection.is_deferred());
    assert!(
        !D::Device.is_deferred(),
        "the local desktop-host Device is MVP-live (registered at cold-start, user-ruled Option A)"
    );
    // deferred — RemoteClient (+ the iOS multi-device/pairing dimension of Device)
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

// ---- Test (1.5 L2) — IPC wire contract values pinned (§6.1/§6.4) -------------

#[test]
fn test_ipc_contract_wire_values() {
    use nexusops_shared::ipc::{IpcErrorCode, ProjectionName};

    // §6.4 structured error codes — snake_case. 2.4 L1 adds `fencing_conflict` + `internal_error`
    // (lead-ruled Option A — the §17 failures need honest codes driving the THREE distinct §11.5
    // safety cards: fencing_conflict→never-auto-resolved hard-conflict (rule #6) · precondition_stale
    // →re-approvable · internal_error→fail-closed integrity; frozen here at 0.19.0, additive).
    check_values(
        IpcErrorCode::ALL,
        &[
            "version_skew",
            "frame_too_large",
            "unknown_method",
            "unauthorized_peer",
            "policy_denied",
            "precondition_stale",
            "fencing_conflict",
            "protocol_error",
            "internal_error",
        ],
    );
    // §6.1 projection names — PascalCase (match the ui's pinned get_projection literals + the
    // §7 registry labels; `UsageLedger` is canonical, not the ui's provisional `Usage`)
    check_values(
        ProjectionName::ALL,
        &[
            "ProjectActivity",
            "Session",
            "ApprovalQueue",
            "Worktree",
            "PullRequest",
            "PlanProgress",
            "ProjectGraph",
            "AgentTeam",
            "AuditTrail",
            "UsageLedger",
        ],
    );
}

// ---- 1.6a-L3 — Device/LocalRunner registration (§5.3/§16, Option B) -----------

#[test]
fn test_desktop_minted_id_newtypes() {
    use nexusops_shared::objects::{DesktopObjectKind, DeviceId, LocalRunnerId};

    // DeviceId mints `dev_<ULID>`, round-trips, and carries its DesktopObjectKind +
    // prefix as one truth (the `desktop_minted_id!` sibling of `minted_id!`).
    let dev = DeviceId::new();
    assert!(dev.as_str().starts_with("dev_"), "DeviceId carries dev_");
    assert_eq!(
        DeviceId::parse(dev.as_str()).unwrap(),
        dev,
        "DeviceId round-trips"
    );
    assert_eq!(DeviceId::KIND, DesktopObjectKind::Device);
    assert_eq!(DeviceId::PREFIX, "dev_");
    assert_eq!(
        DeviceId::KIND.id_prefix(),
        DeviceId::PREFIX,
        "KIND.id_prefix() == PREFIX (single source of truth)"
    );

    // LocalRunnerId mints `lr_<ULID>`, same contract.
    let lr = LocalRunnerId::new();
    assert!(lr.as_str().starts_with("lr_"), "LocalRunnerId carries lr_");
    assert_eq!(
        LocalRunnerId::parse(lr.as_str()).unwrap(),
        lr,
        "LocalRunnerId round-trips"
    );
    assert_eq!(LocalRunnerId::KIND, DesktopObjectKind::LocalRunner);
    assert_eq!(LocalRunnerId::PREFIX, "lr_");

    // fail-closed on a wrong prefix / malformed ULID body (§15)
    assert!(
        DeviceId::parse("lr_01ARZ3NDEKTSV4RRFFQ69G5FAV").is_err(),
        "wrong-prefix rejected"
    );
    assert!(
        DeviceId::parse("dev_not-a-ulid").is_err(),
        "bad-ULID body rejected"
    );
}

#[test]
fn test_frozen_22_excludes_desktop_ids() {
    use nexusops_shared::ids::IdKind;
    // the desktop ids key off DesktopObjectKind, NOT a new IdKind variant — so the frozen
    // 22-ID set stays exactly 22 and does NOT own dev_/lr_ (the contract guard, brief 027).
    assert_eq!(IdKind::ALL.len(), 22, "the 22-ID set is NOT expanded by L3");
    assert_eq!(
        IdKind::from_prefix("dev_"),
        None,
        "dev_ is not a frozen-22 prefix"
    );
    assert_eq!(
        IdKind::from_prefix("lr_"),
        None,
        "lr_ is not a frozen-22 prefix"
    );
}

#[test]
fn test_system_workspace_sentinel() {
    use nexusops_shared::ids::{WorkspaceId, SYSTEM_WORKSPACE_ID};
    // the reserved system-workspace sentinel for workspace-less System-actor lifecycle
    // events (bootstrap registration). It MUST parse as a valid ws_ id so the §15
    // fail-closed envelope parse still holds (NOT a nullable-column schema change).
    let sys = WorkspaceId::system();
    assert_eq!(
        sys.as_str(),
        SYSTEM_WORKSPACE_ID,
        "system() == the sentinel const"
    );
    assert!(
        WorkspaceId::parse(SYSTEM_WORKSPACE_ID).is_ok(),
        "the sentinel is a valid ws_ ULID (§15 parse still holds)"
    );
    assert_eq!(WorkspaceId::parse(SYSTEM_WORKSPACE_ID).unwrap(), sys);
}

#[test]
fn test_registration_payloads_wire_contract() {
    use nexusops_shared::events::{DeviceRegistered, LocalRunnerRegistered};
    use nexusops_shared::objects::{DeviceId, LocalRunnerId};

    // snake_case field name + round-trip (the SessionStarted payload precedent, §5.0).
    let dev = DeviceRegistered {
        device_id: DeviceId::new(),
    };
    let j = serde_json::to_value(&dev).unwrap();
    assert!(
        j.get("device_id").is_some(),
        "device_id is the snake_case wire field"
    );
    assert_eq!(
        serde_json::from_value::<DeviceRegistered>(j).unwrap(),
        dev,
        "DeviceRegistered round-trips"
    );

    let lr = LocalRunnerRegistered {
        local_runner_id: LocalRunnerId::new(),
    };
    let j = serde_json::to_value(&lr).unwrap();
    assert!(
        j.get("local_runner_id").is_some(),
        "local_runner_id is the snake_case wire field"
    );
    assert_eq!(
        serde_json::from_value::<LocalRunnerRegistered>(j).unwrap(),
        lr,
        "LocalRunnerRegistered round-trips"
    );

    // deny_unknown_fields — an extra key is rejected (reject-unknown end-to-end, §5.0/§15)
    let rogue = serde_json::json!({ "device_id": DeviceId::new().as_str(), "rogue": 1 });
    assert!(
        serde_json::from_value::<DeviceRegistered>(rogue).is_err(),
        "unknown field rejected (deny_unknown_fields)"
    );
}

// ---- 1.6c L2 — the §17 audit-integrity event (Option C "loud record") ---------

#[test]
fn test_audit_integrity_violation_wire_contract() {
    use nexusops_shared::events::AuditIntegrityViolation;

    // {seq, reason} round-trips snake_case; the event-type name is a single source of truth.
    let v = AuditIntegrityViolation {
        seq: 42,
        reason: "event reconstruction failed".to_string(),
    };
    let j = serde_json::to_value(&v).unwrap();
    assert_eq!(
        j.get("seq").and_then(|x| x.as_i64()),
        Some(42),
        "seq wire field"
    );
    assert!(j.get("reason").is_some(), "reason wire field");
    assert_eq!(
        serde_json::from_value::<AuditIntegrityViolation>(j).unwrap(),
        v,
        "AuditIntegrityViolation round-trips"
    );
    // deny_unknown_fields (reject-unknown, §5.0/§15)
    let rogue = serde_json::json!({ "seq": 1, "reason": "x", "extra": true });
    assert!(
        serde_json::from_value::<AuditIntegrityViolation>(rogue).is_err(),
        "unknown field rejected"
    );
    // the event-type name has ONE home (used by the emit path + the audit projector)
    assert_eq!(
        AuditIntegrityViolation::EVENT_TYPE,
        "AuditIntegrityViolation"
    );
}

// ---- 1.7 L2 — the §15 quarantine-divert event (SensitiveOutputRedacted) ----------------

#[test]
fn test_sensitive_output_redacted_wire_contract() {
    use nexusops_shared::events::SensitiveOutputRedacted;

    // {original_event_type, reason, detector} round-trips snake_case; content-free (§15).
    let v = SensitiveOutputRedacted {
        original_event_type: "SessionStarted".to_string(),
        reason: "high-confidence secret could not be safely bounded".to_string(),
        detector: "entropy-kv".to_string(),
    };
    let j = serde_json::to_value(&v).unwrap();
    assert!(
        j.get("original_event_type").is_some(),
        "original_event_type wire field"
    );
    assert!(j.get("reason").is_some(), "reason wire field");
    assert!(j.get("detector").is_some(), "detector wire field");
    assert_eq!(
        serde_json::from_value::<SensitiveOutputRedacted>(j).unwrap(),
        v,
        "SensitiveOutputRedacted round-trips"
    );
    // deny_unknown_fields (reject-unknown, §5.0/§15)
    let rogue = serde_json::json!({
        "original_event_type": "X", "reason": "y", "detector": "z", "extra": true
    });
    assert!(
        serde_json::from_value::<SensitiveOutputRedacted>(rogue).is_err(),
        "unknown field rejected"
    );
    // the event-type name has ONE home (the daemon emit path + the audit projector)
    assert_eq!(
        SensitiveOutputRedacted::EVENT_TYPE,
        "SensitiveOutputRedacted"
    );
}

#[test]
fn test_contract_version_bumped_for_sensitive_output_redacted() {
    // 0.15.0 = the 2.1a action-contract freeze (§6.2 models + 9 enums + gateway IDs + Timestamp);
    // 0.16.0 = the 2.1b ActionExecution* event family + ActionAck; 0.17.0 = the 2.1c PlanAck
    // submit_action_plan wire type (O-3); 0.18.0 = the 2.2 ActionTypeCatalog + PolicyDecision
    // extension — all additive (§5.0). (The canonical version pin is `test_contract_version_bumped_0_20_0`.)
    assert_eq!(nexusops_shared::CONTRACT_VERSION, "0.20.0");
}

// ---- P2.1c L2 — the §6.1 PlanAck wire type (§2.5-seam snapshot, O-3) -----------------------

fn sample_plan_ack() -> nexusops_shared::ipc::PlanAck {
    use nexusops_shared::ipc::{PlanAck, PlanStepAck};
    use nexusops_shared::status::ActionRequestStatus;
    PlanAck {
        plan_id: "aplan_01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
        steps: vec![PlanStepAck {
            step_id: "s1".to_string(),
            action_request_id: "act_01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
            status: ActionRequestStatus::AwaitingApproval,
        }],
    }
}

#[test]
fn test_plan_ack_field_names_snapshot() {
    // spec(§6.1) — §2.5-seam GatewayPort wire-surface freeze guard: a field added/removed/renamed on
    // PlanAck / PlanStepAck fails this snapshot. The expected sets ARE the checked-in freeze.
    expect_fields(&sample_plan_ack(), &["plan_id", "steps"]);
    expect_fields(
        &sample_plan_ack().steps[0],
        &["step_id", "action_request_id", "status"],
    );
    // reject-unknown end-to-end (§5.0/§15 fail-closed)
    let rogue = serde_json::json!({
        "plan_id": "aplan_x", "steps": [], "extra": true
    });
    assert!(
        serde_json::from_value::<nexusops_shared::ipc::PlanAck>(rogue).is_err(),
        "unknown field rejected"
    );
}

// =====================================================================================
// Phase 2.1a — §6.2 action-contract freeze (NEW: shared/src/{actions,time,gateway_ids}.rs)
// The §6.2 Gateway core data model + its enums + the gateway platform IDs + Timestamp.
// Binding field sets: ARCHITECTURE.md Appendix A (rows ActionRequest/ActionPlan/Approval/
// ActionResult/ResourceRef·EvidenceRef·PolicyDecision) + DATA_MODEL §2.9 DDL. AG §9.x is
// ORIGIN/RATIONALE (richer) — frozen to the reconciled binding set (drops flagged Step-9).
// =====================================================================================

// the wire field-name set (top-level keys) of a serialized model
fn field_names<T: Serialize>(v: &T) -> BTreeSet<String> {
    serde_json::to_value(v)
        .unwrap()
        .as_object()
        .expect("a contract model must serialize to a JSON object")
        .keys()
        .cloned()
        .collect()
}

fn expect_fields<T: Serialize>(v: &T, expected: &[&str]) {
    let got = field_names(v);
    let exp: BTreeSet<String> = expected.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        got, exp,
        "model field-name set drifted from the frozen snapshot"
    );
}

// ---- sample constructors (fully populated; Options = Some so the snapshot sees every key) --

fn sample_resource_ref() -> nexusops_shared::actions::ResourceRef {
    use nexusops_shared::actions::{ResourceRef, ResourceType};
    ResourceRef {
        resource_type: ResourceType::Repo,
        id: "repo_01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
        uri: Some("file:///Users/x/repo".to_string()),
    }
}

fn sample_evidence_ref() -> nexusops_shared::actions::EvidenceRef {
    use nexusops_shared::actions::{EvidenceConfidence, EvidenceRef, EvidenceType};
    EvidenceRef {
        evidence_type: EvidenceType::ArchitectureAnchor,
        id: "§6.2".to_string(),
        label: "ARCHITECTURE §6.2 pins the Gateway core data model".to_string(),
        confidence: Some(EvidenceConfidence::Exact),
    }
}

fn sample_action_preview() -> nexusops_shared::actions::ActionPreview {
    use nexusops_shared::actions::{ActionPreview, RiskLevel};
    use nexusops_shared::ids::ActionRequestId;
    use nexusops_shared::time::Timestamp;
    ActionPreview {
        action_request_id: ActionRequestId::new(),
        generated_at: Timestamp::parse("2026-06-11T08:59:53.972Z").unwrap(),
        risk_level: RiskLevel::Level2,
        risk_reasons: vec!["writes to the working tree".to_string()],
        summary: "create a worktree".to_string(),
        changed_resources: vec![sample_resource_ref()],
        cannot_preview_reason: Some("n/a".to_string()),
    }
}

fn sample_action_request() -> nexusops_shared::actions::ActionRequest {
    use nexusops_shared::actions::{ActionRequest, RequesterType, RiskLevel};
    use nexusops_shared::ids::{ActionRequestId, ProjectId};
    use nexusops_shared::status::ActionRequestStatus;
    use nexusops_shared::time::Timestamp;
    ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id: Some(ProjectId::new()),
        action_type: "git.create_worktree".to_string(),
        requester_type: RequesterType::User,
        requester_id: "u_local".to_string(),
        resource_refs: vec![sample_resource_ref()],
        inputs: serde_json::json!({ "branch": "feature/x" }),
        risk_level: RiskLevel::Level2,
        idempotency_key: Some("k-1".to_string()),
        fencing_token: Some(7),
        status: ActionRequestStatus::Submitted,
        preview: Some(sample_action_preview()),
        created_at: Timestamp::parse("2026-06-11T08:59:53.972Z").unwrap(),
    }
}

fn sample_action_dependency() -> nexusops_shared::actions::ActionDependency {
    use nexusops_shared::actions::ActionDependency;
    ActionDependency {
        step_id: "step-2".to_string(),
        depends_on_step_ids: vec!["step-1".to_string()],
    }
}

fn sample_action_plan_step() -> nexusops_shared::actions::ActionPlanStep {
    use nexusops_shared::actions::ActionPlanStep;
    use nexusops_shared::status::ActionRequestStatus;
    ActionPlanStep {
        step_id: "step-1".to_string(),
        label: "create the worktree".to_string(),
        action_request: sample_action_request(),
        required: true,
        can_skip: false,
        rollback_action_type: Some("git.delete_worktree".to_string()),
        status: ActionRequestStatus::Submitted,
    }
}

fn sample_action_plan() -> nexusops_shared::actions::ActionPlan {
    use nexusops_shared::actions::{ActionPlan, ApprovalMode, RiskLevel};
    use nexusops_shared::gateway_ids::ActionPlanId;
    ActionPlan {
        plan_id: ActionPlanId::new(),
        title: "set up the feature branch".to_string(),
        steps: vec![sample_action_plan_step()],
        dependencies: vec![sample_action_dependency()],
        overall_risk: RiskLevel::Level2,
        approval_mode: ApprovalMode::StepByStep,
    }
}

fn sample_approval() -> nexusops_shared::actions::Approval {
    use nexusops_shared::actions::{Approval, ApprovalScope, RequiredApprover, RiskLevel};
    use nexusops_shared::gateway_ids::{ActionPlanId, ApprovalId};
    use nexusops_shared::ids::ActionRequestId;
    use nexusops_shared::status::ApprovalStatus;
    use nexusops_shared::time::Timestamp;
    Approval {
        approval_id: ApprovalId::new(),
        action_request_id: Some(ActionRequestId::new()),
        plan_id: Some(ActionPlanId::new()),
        required_approver: RequiredApprover::current_user(),
        status: ApprovalStatus::Requested,
        risk_level: RiskLevel::Level2,
        scope: ApprovalScope::SingleAction,
        decided_by: Some("u_local".to_string()),
        decided_at: Some(Timestamp::parse("2026-06-11T09:00:00Z").unwrap()),
        expires_at: Some(Timestamp::parse("2026-06-11T10:00:00Z").unwrap()),
    }
}

fn sample_action_result() -> nexusops_shared::actions::ActionResult {
    use nexusops_shared::actions::{ActionResult, ActionResultStatus};
    use nexusops_shared::ids::{ActionRequestId, EventId};
    // a coherent fully-populated sample: a partial success DOES carry an error detail (so every
    // optional is Some → the field-name snapshot is robust regardless of skip behavior).
    ActionResult {
        action_request_id: ActionRequestId::new(),
        status: ActionResultStatus::PartiallySucceeded,
        created_resources: vec![sample_resource_ref()],
        changed_resources: vec![sample_resource_ref()],
        emitted_events: vec![EventId::new()],
        error: Some("worktree created; branch push failed".to_string()),
        rollback_available: true,
    }
}

fn sample_policy_decision() -> nexusops_shared::actions::PolicyDecision {
    use nexusops_shared::actions::{PolicyDecision, PolicyDecisionStatus, RequiredApprover};
    // fully-populated (every optional Some / non-empty) so the field-name snapshot sees every key.
    PolicyDecision {
        status: PolicyDecisionStatus::RequireApproval,
        reasons: vec!["risk >= 1 requires approval".to_string()],
        required_approvals: vec![RequiredApprover::current_user()],
        constraints: vec!["execute within the active worktree".to_string()],
        safer_alt: Some("github.create_pr_draft".to_string()),
    }
}

// ---- RED #1 — the §2.5-seam schema-snapshot guard (field-name set == checked-in) ----------

#[test]
fn test_action_model_field_names_snapshot() {
    // spec(§6.2) — §2.5-seam freeze guard: a field added/removed/renamed on any of the 10
    // §6.2 models fails this snapshot. The expected sets ARE the checked-in freeze.
    expect_fields(
        &sample_action_request(),
        &[
            "action_request_id",
            "project_id",
            "action_type",
            "requester_type",
            "requester_id",
            "resource_refs",
            "inputs",
            "risk_level",
            "idempotency_key",
            "fencing_token",
            "status",
            "preview",
            "created_at",
        ],
    );
    expect_fields(
        &sample_action_plan(),
        &[
            "plan_id",
            "title",
            "steps",
            "dependencies",
            "overall_risk",
            "approval_mode",
        ],
    );
    expect_fields(
        &sample_action_plan_step(),
        &[
            "step_id",
            "label",
            "action_request",
            "required",
            "can_skip",
            "rollback_action_type",
            "status",
        ],
    );
    expect_fields(
        &sample_action_dependency(),
        &["step_id", "depends_on_step_ids"],
    );
    expect_fields(
        &sample_action_preview(),
        &[
            "action_request_id",
            "generated_at",
            "risk_level",
            "risk_reasons",
            "summary",
            "changed_resources",
            "cannot_preview_reason",
        ],
    );
    expect_fields(
        &sample_approval(),
        &[
            "approval_id",
            "action_request_id",
            "plan_id",
            "required_approver",
            "status",
            "risk_level",
            "scope",
            "decided_by",
            "decided_at",
            "expires_at",
        ],
    );
    expect_fields(
        &sample_action_result(),
        &[
            "action_request_id",
            "status",
            "created_resources",
            "changed_resources",
            "emitted_events",
            "error",
            "rollback_available",
        ],
    );
    expect_fields(&sample_resource_ref(), &["type", "id", "uri"]);
    expect_fields(
        &sample_evidence_ref(),
        &["type", "id", "label", "confidence"],
    );
    // PolicyDecision gained its 2.2 fields (required_approvals/constraints/safer_alt) — see also
    // the dedicated P2.2 §2.5-seam snapshot `test_catalog_and_policy_decision_field_snapshot`.
    expect_fields(
        &sample_policy_decision(),
        &[
            "status",
            "reasons",
            "required_approvals",
            "constraints",
            "safer_alt",
        ],
    );
}

// ---- RED #2 — RequesterType wire values (§6.2 / R-2 / §15) --------------------------------

#[test]
fn test_requester_type_values() {
    use nexusops_shared::actions::RequesterType;
    check_values(
        RequesterType::ALL,
        &[
            "user",
            "project_brain",
            "agent_session",
            "workflow_pack",
            "system_policy",
            "remote_client",
        ],
    );
    // reject-unknown at the parse boundary (§15 fail-closed)
    assert!(serde_json::from_value::<RequesterType>(serde_json::json!("nope")).is_err());
}

// ---- RED #3 — RequesterType → ActorType binding map (Appendix A / R-2) --------------------

#[test]
fn test_requester_type_maps_to_actor_type() {
    use nexusops_shared::actions::RequesterType;
    use nexusops_shared::actor::ActorType;
    // the 3 aliases (request-time → audit actor)
    assert_eq!(
        RequesterType::AgentSession.to_actor_type(),
        ActorType::SessionAdapter
    );
    assert_eq!(
        RequesterType::WorkflowPack.to_actor_type(),
        ActorType::WorkflowRuntime
    );
    assert_eq!(
        RequesterType::SystemPolicy.to_actor_type(),
        ActorType::AutomationPolicy
    );
    // the 3 straight-throughs
    assert_eq!(RequesterType::User.to_actor_type(), ActorType::User);
    assert_eq!(
        RequesterType::ProjectBrain.to_actor_type(),
        ActorType::ProjectBrain
    );
    assert_eq!(
        RequesterType::RemoteClient.to_actor_type(),
        ActorType::RemoteClient
    );
}

// ---- RED #4 — RiskLevel is integer 0..=4 (§6.2 / AG §7) -----------------------------------

#[test]
fn test_risk_level_wire_is_0_to_4() {
    use nexusops_shared::actions::RiskLevel;
    // exactly 5 variants, declaration order == ascending integer
    assert_eq!(RiskLevel::ALL.len(), 5);
    for (i, level) in RiskLevel::ALL.iter().enumerate() {
        assert_eq!(
            serde_json::to_value(level).unwrap(),
            serde_json::json!(i as i64),
            "RiskLevel serializes to its integer rank"
        );
    }
    // Ord (for the §6.3 risk-range comparisons, e.g. >= 1, == 4)
    assert!(RiskLevel::Level0 < RiskLevel::Level4);
    // out-of-range integers are rejected at the parse boundary (§15 fail-closed)
    assert!(serde_json::from_value::<RiskLevel>(serde_json::json!(5)).is_err());
    assert!(serde_json::from_value::<RiskLevel>(serde_json::json!(-1)).is_err());
    assert!(serde_json::from_value::<RiskLevel>(serde_json::json!("2")).is_err());
}

// ---- RED #5 — the 4 closed approval/policy/result enums (§6.2 / Appendix A) ---------------

#[test]
fn test_closed_enum_wire_values() {
    use nexusops_shared::actions::{
        ActionResultStatus, ApprovalMode, ApprovalScope, PolicyDecisionStatus,
    };
    check_values(
        ApprovalScope::ALL,
        &["single_action", "plan", "policy_grant"],
    );
    check_values(
        ApprovalMode::ALL,
        &["approve_all", "step_by_step", "mixed", "blocked"],
    );
    check_values(
        PolicyDecisionStatus::ALL,
        &[
            "allow",
            "require_approval",
            "require_step_approval",
            "deny",
            "downgrade",
            "needs_more_context",
        ],
    );
    check_values(
        ActionResultStatus::ALL,
        &["succeeded", "failed", "partially_succeeded", "cancelled"],
    );
    // reject-unknown on each (§15)
    assert!(serde_json::from_value::<ApprovalScope>(serde_json::json!("x")).is_err());
    assert!(serde_json::from_value::<ApprovalMode>(serde_json::json!("x")).is_err());
    assert!(serde_json::from_value::<PolicyDecisionStatus>(serde_json::json!("x")).is_err());
    assert!(serde_json::from_value::<ActionResultStatus>(serde_json::json!("x")).is_err());
}

// ---- RED #6 — ResourceType(20) + EvidenceType(11) + EvidenceConfidence(4) (§6.2) ----------

#[test]
fn test_resource_and_evidence_enum_values() {
    use nexusops_shared::actions::{EvidenceConfidence, EvidenceType, ResourceType};
    // ResourceType — the AG §9.8 set, frozen VERBATIM (20 values; the §6.2/Appendix-A "21"
    // discrepancy is a Step-9 cross-doc note, NOT a 21st invented value).
    check_values(
        ResourceType::ALL,
        &[
            "project",
            "repo",
            "worktree",
            "branch",
            "file",
            "diff",
            "session",
            "agent_team",
            "plan_task",
            "implementation_plan",
            "workflow_pack",
            "workflow_instance",
            "workflow_command",
            "linear_issue",
            "github_issue",
            "pull_request",
            "execution_profile",
            "memory_source",
            "decision",
            "artifact",
        ],
    );
    check_values(
        EvidenceType::ALL,
        &[
            "project_brain_evidence_item",
            "file_anchor",
            "architecture_anchor",
            "plan_task",
            "session_episode",
            "commit",
            "pr_check",
            "terminal_event",
            "ticket",
            "workflow_manifest",
            "user_instruction",
        ],
    );
    check_values(
        EvidenceConfidence::ALL,
        &["exact", "likely", "loose", "unverified"],
    );
}

// ---- RED #7 — the new gateway platform IDs (RULED Option A; IdKind stays 22) --------------

#[test]
fn test_gateway_minted_id_newtypes() {
    use nexusops_shared::gateway_ids::{ActionPlanId, ApprovalId, GatewayObjectKind};
    // ApprovalId mints `appr_<ULID>`, round-trips, single-truth KIND/PREFIX (the
    // `desktop_minted_id!` sibling — RULED Option A, non-cross-product Gateway objects).
    let appr = ApprovalId::new();
    assert!(
        appr.as_str().starts_with("appr_"),
        "ApprovalId carries appr_"
    );
    assert_eq!(ApprovalId::parse(appr.as_str()).unwrap(), appr);
    assert_eq!(ApprovalId::KIND, GatewayObjectKind::Approval);
    assert_eq!(ApprovalId::PREFIX, "appr_");
    assert_eq!(ApprovalId::KIND.id_prefix(), ApprovalId::PREFIX);

    let plan = ActionPlanId::new();
    assert!(
        plan.as_str().starts_with("aplan_"),
        "ActionPlanId carries aplan_"
    );
    assert_eq!(ActionPlanId::parse(plan.as_str()).unwrap(), plan);
    assert_eq!(ActionPlanId::KIND, GatewayObjectKind::ActionPlan);
    assert_eq!(ActionPlanId::PREFIX, "aplan_");

    // fail-closed: wrong prefix / malformed ULID body (§15)
    assert!(ApprovalId::parse("aplan_01ARZ3NDEKTSV4RRFFQ69G5FAV").is_err());
    assert!(ApprovalId::parse("appr_not-a-ulid").is_err());

    // GatewayObjectKind catalog
    assert_eq!(GatewayObjectKind::ALL.len(), 2);
    assert_eq!(GatewayObjectKind::Approval.id_prefix(), "appr_");
    assert_eq!(GatewayObjectKind::ActionPlan.id_prefix(), "aplan_");
}

#[test]
fn test_idkind_still_22() {
    use nexusops_shared::ids::IdKind;
    // RULED Option A — the gateway ids key off GatewayObjectKind, NOT new IdKind variants;
    // the frozen 22-ID cross-product set stays EXACTLY 22 and owns neither new prefix.
    assert_eq!(
        IdKind::ALL.len(),
        22,
        "the 22-ID set is NOT expanded by 2.1a"
    );
    assert_eq!(IdKind::from_prefix("appr_"), None);
    assert_eq!(IdKind::from_prefix("aplan_"), None);
}

#[test]
fn test_gateway_prefixes_dont_collide() {
    use nexusops_shared::gateway_ids::GatewayObjectKind;
    use nexusops_shared::ids::IdKind;
    use nexusops_shared::objects::DesktopObjectKind;
    // appr_/aplan_ collide with none of the 16 IdKind prefixes nor the 4 desktop prefixes.
    let mut taken: BTreeSet<String> = BTreeSet::new();
    for k in IdKind::ALL {
        if let Some(p) = k.prefix() {
            taken.insert(p.to_string());
        }
    }
    for k in DesktopObjectKind::ALL {
        taken.insert(k.id_prefix().to_string());
    }
    for k in GatewayObjectKind::ALL {
        assert!(
            !taken.contains(k.id_prefix()),
            "gateway prefix {} collides with an existing id prefix",
            k.id_prefix()
        );
    }
}

// ---- RED #8 — Timestamp newtype: transparent string + RFC3339 + format date-time ---------

#[test]
fn test_timestamp_newtype_format_and_rfc3339() {
    use nexusops_shared::time::Timestamp;
    // a valid RFC3339 parses + serializes transparently (the wire value is the bare string)
    let ts = Timestamp::parse("2026-06-11T08:59:53.972Z").unwrap();
    assert_eq!(ts.as_str(), "2026-06-11T08:59:53.972Z");
    assert_eq!(
        serde_json::to_value(&ts).unwrap(),
        serde_json::json!("2026-06-11T08:59:53.972Z"),
        "Timestamp serializes transparently (a bare string, not a wrapper object)"
    );
    // offset form (non-Z) also valid
    assert!(Timestamp::parse("2026-06-11T08:59:53+02:00").is_ok());
    // garbage is rejected (fail-closed parse, §15)
    assert!(Timestamp::parse("not-a-timestamp").is_err());
    assert!(Timestamp::parse("2026-13-40T99:99:99Z").is_err());
    assert!(Timestamp::parse("2026-06-11").is_err()); // date only, no time
                                                      // the offset is MANDATORY (RFC3339): the common naive ISO-8601 / SQL-CURRENT_TIMESTAMP form
                                                      // with no Z/±hh:mm is rejected — pins the UTC-explicitness boundary (Lessons §2/§5).
    assert!(Timestamp::parse("2026-06-11T12:00:00").is_err());
    // schemars emits string + format: date-time (so generated Zod/Pydantic carry the format)
    let schema = serde_json::to_value(schemars::schema_for!(Timestamp)).unwrap();
    assert_eq!(schema.get("type").and_then(|t| t.as_str()), Some("string"));
    assert_eq!(
        schema.get("format").and_then(|f| f.as_str()),
        Some("date-time")
    );
}

// ---- RED #9 — ActionRequest round-trips (required + all-optional-None) + reject-unknown ---

#[test]
fn test_action_request_round_trips_with_required_and_optional_fields() {
    use nexusops_shared::actions::{ActionRequest, RequesterType, RiskLevel};
    use nexusops_shared::status::ActionRequestStatus;
    use nexusops_shared::time::Timestamp;
    // full sample round-trips
    let full = sample_action_request();
    let j = serde_json::to_value(&full).unwrap();
    assert_eq!(
        serde_json::from_value::<ActionRequest>(j.clone()).unwrap(),
        full,
        "full ActionRequest round-trips"
    );
    // every optional = None round-trips
    let minimal = ActionRequest {
        action_request_id: nexusops_shared::ids::ActionRequestId::new(),
        project_id: None,
        action_type: "project.rescan".to_string(),
        requester_type: RequesterType::User,
        requester_id: "u_local".to_string(),
        resource_refs: vec![],
        inputs: serde_json::json!({}),
        risk_level: RiskLevel::Level1,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        created_at: Timestamp::parse("2026-06-11T08:59:53Z").unwrap(),
    };
    let jm = serde_json::to_value(&minimal).unwrap();
    assert_eq!(
        serde_json::from_value::<ActionRequest>(jm).unwrap(),
        minimal,
        "all-optional-None ActionRequest round-trips"
    );
    // deny_unknown_fields — an extra key fails closed (reject-unknown, §5.0/§15)
    let mut rogue = j.as_object().unwrap().clone();
    rogue.insert("rogue".to_string(), serde_json::json!(1));
    assert!(
        serde_json::from_value::<ActionRequest>(serde_json::Value::Object(rogue)).is_err(),
        "unknown field rejected (deny_unknown_fields)"
    );
}

// ---- deny_unknown_fields holds on the NESTED structs too (§5.0/§15 reject-unknown surface) ----

/// each type round-trips its own sample, and fails closed when an unknown key is injected.
macro_rules! assert_rejects_unknown {
    ($ty:ty, $sample:expr) => {{
        let v = serde_json::to_value(&$sample).unwrap();
        // clean round-trip without the rogue key
        assert!(
            serde_json::from_value::<$ty>(v.clone()).is_ok(),
            concat!(stringify!($ty), " round-trips its own sample")
        );
        let mut rogue = v.as_object().unwrap().clone();
        rogue.insert("rogue".to_string(), serde_json::json!(true));
        assert!(
            serde_json::from_value::<$ty>(serde_json::Value::Object(rogue)).is_err(),
            concat!(
                stringify!($ty),
                " rejects an unknown field (deny_unknown_fields, §5.0/§15)"
            )
        );
    }};
}

#[test]
fn test_nested_structs_reject_unknown_fields() {
    use nexusops_shared::actions::{ActionResult, EvidenceRef, ResourceRef};
    // the §15 reject-unknown parse boundary is wired on every model, not just ActionRequest's top
    // level — pin it on a representative spread (a leaf, a labeled leaf, a result envelope).
    assert_rejects_unknown!(ResourceRef, sample_resource_ref());
    assert_rejects_unknown!(EvidenceRef, sample_evidence_ref());
    assert_rejects_unknown!(ActionResult, sample_action_result());
    // 2.2 — the catalog entry + the extended PolicyDecision carry deny_unknown_fields too.
    assert_rejects_unknown!(
        nexusops_shared::catalog::ActionTypeCatalogEntry,
        sample_catalog_entry()
    );
    assert_rejects_unknown!(
        nexusops_shared::actions::PolicyDecision,
        sample_policy_decision()
    );
}

// =====================================================================================
// P2.2 L1 — the §6.3 ActionTypeCatalog contract (shared/) + the PolicyDecision extension.
// BINDING: ARCHITECTURE Appendix A row ActionTypeCatalog (§6.3 — per action_type: params schema,
// locked risk 0-4, required preview class, idempotency-key formula, executor, resource_refs
// required) + the §6.3 LOCKED MVP 22-type set + AG §7 (risk defs) + §12 (PolicyDecision).
// =====================================================================================

// ---- L1 RED #1 — the catalog covers the §6.3 MVP set, each entry binding -------------------

#[test]
fn test_action_type_catalog_covers_mvp_set() {
    // spec(§6.3) — every §6.3 MVP action_type has a binding catalog entry (a locked risk 0-4 +
    // preview class + executor + the flags). The closed 22-type set; presence + a valid risk is the
    // contract (the AS-BUILT per-type values are recorded in Appendix-A + reviewed at Step-2.5).
    use nexusops_shared::actions::RiskLevel;
    use nexusops_shared::catalog::{lookup, MVP_ACTION_TYPES};

    assert_eq!(
        MVP_ACTION_TYPES.len(),
        22,
        "the §6.3 LOCKED MVP set is 22 types"
    );
    for at in MVP_ACTION_TYPES {
        let e = lookup(at).unwrap_or_else(|| panic!("catalog missing the MVP type {at}"));
        assert!(
            RiskLevel::ALL.contains(&e.locked_risk),
            "{at} has a locked risk in 0..=4"
        );
    }
    // the §6.3/OQ-WP-5 null-input_schema floor type is present + flagged (params_schema_present=false)
    let wci = lookup("workflow.command.invoke").expect("workflow.command.invoke in the catalog");
    assert!(
        !wci.params_schema_present,
        "workflow.command.invoke carries a null input_schema (the §6.3 approval floor)"
    );
    // pin the two LOAD-BEARING anchors of the AS-BUILT risk table (not just reflexive membership):
    // the sole critical (the safety-pin anchor) + a read-only auto-execute-eligible type.
    assert_eq!(
        wci.locked_risk,
        RiskLevel::Level4,
        "workflow.command.invoke is the critical (risk-4) anchor — never auto-execute / approve-all"
    );
    assert_eq!(
        lookup("git.status").unwrap().locked_risk,
        RiskLevel::Level0,
        "a read-only type is risk-0 (auto-execute eligible)"
    );
}

// ---- L1 RED #2 — an unknown / deferred action_type fails closed (no default entry) ----------

#[test]
fn test_catalog_lookup_unknown_type_fails_closed() {
    // spec(§15 fail-closed) — an action_type not in the closed catalog → None (a typed not-found),
    // NEVER a default entry. A deferred high-risk type (force_push/merge/delete, AG §28.3) is
    // simply absent — the policy denies it, never default-allows.
    use nexusops_shared::catalog::lookup;
    for unknown in [
        "git.force_push",
        "git.merge",
        "git.delete_worktree",
        "not.a.real.action",
        "",
    ] {
        assert!(
            lookup(unknown).is_none(),
            "an unknown/deferred type `{unknown}` must fail closed (None), not default-allow"
        );
    }
}

// ---- L1 RED #3 — the catalog entry + PolicyDecision-extension field snapshots (§2.5-seam) ----

fn sample_catalog_entry() -> nexusops_shared::catalog::ActionTypeCatalogEntry {
    // the lookup result IS the entry; sample a concrete one for the field-name snapshot.
    nexusops_shared::catalog::lookup("git.create_worktree").expect("git.create_worktree entry")
}

#[test]
fn test_catalog_and_policy_decision_field_snapshot() {
    // spec(§6.3 / §6.2) — §2.5-seam freeze guards. A field added/removed/renamed on the catalog
    // entry or the extended PolicyDecision fails this snapshot. The expected sets ARE the freeze.
    expect_fields(
        &sample_catalog_entry(),
        &[
            "locked_risk",
            "preview_class",
            "idempotency_formula",
            "executor",
            "requires_resource_refs",
            "params_schema_present",
        ],
    );
    expect_fields(
        &sample_policy_decision(),
        &[
            "status",
            "reasons",
            "required_approvals",
            "constraints",
            "safer_alt",
        ],
    );
}

#[test]
fn test_contract_version_bumped_0_18_0() {
    // 0.17.0 = the 2.1c PlanAck; 0.18.0 = the 2.2 ActionTypeCatalog (+ its enums) + the
    // PolicyDecision extension (required_approvals/constraints/safer_alt) — additive (§5.0).
    // (The CURRENT canonical version pin is `test_contract_version_bumped_0_20_0`.)
    assert_eq!(nexusops_shared::CONTRACT_VERSION, "0.20.0");
}

// =====================================================================================
// Phase 2.4 L1 — §17 failure-mode CONTRACT additions (freeze; CONTRACT 0.18.0 → 0.19.0):
//   • a NEW `ActionPartiallySucceeded` event in the §7.1 ActionExecution* family
//     (the §17 "side effect applied but the terminal event could not be written" record);
//   • a structured `ActionError` taxonomy on `ActionFailed` (replacing the 2.1b
//     `ActionFailed{error: String}`) — the typed execute-error set L2-L5 emit;
//   • the §5.1 rollback `can_transition` edges (pinned daemon-side in tests/gateway.rs).
// BINDING: ARCHITECTURE §7.1 (EventTypeRegistry) + §17 (failure-mode contract) + §5.1.
// NO behavior — L2-L5 consume these. ActionError is internally-tagged on `kind` (the
// `ServerFrame` precedent, §6.4) so the §5.0 3-way verify stays exact (LESSON §15 trap 2).
// =====================================================================================

// ---- 2.4 L1 RED #1 — the ActionPartiallySucceeded event payload (§17 side-effect-applied record) ----

#[test]
fn test_action_partially_succeeded_wire_contract() {
    // spec(§7.1) — the §17 "side effect applied but the terminal event could not be written" record,
    // added to the ActionExecution* family. Carries a redaction-safe STRUCTURAL `reason` only (never
    // row/payload content, §15 — mirrors AuditIntegrityViolation); snake_case; round-trips;
    // deny_unknown_fields; ONE EVENT_TYPE home (no new bare string-literal in the emit/projector dup set).
    use nexusops_shared::events::ActionPartiallySucceeded;
    let v = ActionPartiallySucceeded {
        reason: "side effect applied; terminal ActionSucceeded event write failed".to_string(),
    };
    let j = serde_json::to_value(&v).unwrap();
    assert!(j.get("reason").is_some(), "reason wire field");
    assert_eq!(
        serde_json::from_value::<ActionPartiallySucceeded>(j).unwrap(),
        v,
        "ActionPartiallySucceeded round-trips"
    );
    let rogue = serde_json::json!({ "reason": "x", "extra": true });
    assert!(
        serde_json::from_value::<ActionPartiallySucceeded>(rogue).is_err(),
        "unknown field rejected (deny_unknown_fields, §5.0/§15)"
    );
    assert_eq!(
        ActionPartiallySucceeded::EVENT_TYPE,
        "ActionPartiallySucceeded"
    );
}

// ---- 2.4 L1 RED #2 — the structured ActionError taxonomy (the §17 typed execute-error set) ----

#[test]
fn test_action_error_taxonomy_wire_contract() {
    // spec(§7.1) — the typed execute-error taxonomy replacing the 2.1b `ActionFailed{error:String}`.
    // Internally-tagged on `kind` (the ServerFrame precedent): 5 variants — 4 unit (audit_write_failed
    // / stale_precondition / fencing_conflict / unknown_outcome) + executor_error carrying a free
    // `message`. Each serializes to its `kind`, round-trips; an unknown kind fails closed (§15).
    use nexusops_shared::actions::ActionError;
    let cases: &[(ActionError, &str)] = &[
        (ActionError::AuditWriteFailed, "audit_write_failed"),
        (ActionError::StalePrecondition, "stale_precondition"),
        (ActionError::FencingConflict, "fencing_conflict"),
        (ActionError::UnknownOutcome, "unknown_outcome"),
        (
            ActionError::ExecutorError {
                message: "boom".to_string(),
            },
            "executor_error",
        ),
    ];
    for (variant, kind) in cases {
        let j = serde_json::to_value(variant).unwrap();
        assert_eq!(
            j.get("kind").and_then(|k| k.as_str()),
            Some(*kind),
            "ActionError serializes its `kind` discriminant"
        );
        assert_eq!(
            &serde_json::from_value::<ActionError>(j).unwrap(),
            variant,
            "ActionError round-trips"
        );
    }
    // executor_error carries its free message alongside the discriminant
    let ee = serde_json::to_value(&ActionError::ExecutorError {
        message: "boom".to_string(),
    })
    .unwrap();
    assert_eq!(ee.get("message").and_then(|m| m.as_str()), Some("boom"));
    // an unknown discriminant fails closed (§15 reject-unknown)
    assert!(
        serde_json::from_value::<ActionError>(serde_json::json!({ "kind": "nope" })).is_err(),
        "unknown ActionError kind rejected"
    );
    // the struct variant's required field is enforced: executor_error WITHOUT its message fails closed
    // (a missing required field is not a silent default — §15 fail-closed parse).
    assert!(
        serde_json::from_value::<ActionError>(serde_json::json!({ "kind": "executor_error" }))
            .is_err(),
        "executor_error without its required message rejected"
    );
}

// ---- 2.4 L1 RED #3 — ActionFailed now carries the structured ActionError (the contract change) ----

#[test]
fn test_action_failed_carries_structured_error() {
    // spec(§7.1) — `ActionFailed.error` changed `String → ActionError` (the 2.4 typed taxonomy). The
    // top-level field set stays {error}; each variant round-trips on the event payload. The 2.1b
    // free-string failure maps to `ExecutorError{message}` (no information lost).
    use nexusops_shared::actions::ActionError;
    use nexusops_shared::events::ActionFailed;
    let v = ActionFailed {
        error: ActionError::FencingConflict,
    };
    let j = serde_json::to_value(&v).unwrap();
    let exp: BTreeSet<String> = ["error"].iter().map(|s| s.to_string()).collect();
    assert_eq!(
        field_names(&v),
        exp,
        "ActionFailed top-level field set is {{error}}"
    );
    assert_eq!(
        j["error"]["kind"].as_str(),
        Some("fencing_conflict"),
        "the structured error nests under `error`"
    );
    assert_eq!(
        serde_json::from_value::<ActionFailed>(j).unwrap(),
        v,
        "ActionFailed round-trips a unit variant"
    );
    let ee = ActionFailed {
        error: ActionError::ExecutorError {
            message: "validate: missing required resource_ref".to_string(),
        },
    };
    let je = serde_json::to_value(&ee).unwrap();
    assert_eq!(
        serde_json::from_value::<ActionFailed>(je).unwrap(),
        ee,
        "ActionFailed round-trips the executor_error variant (the 2.1b String home)"
    );
    let rogue = serde_json::json!({ "error": { "kind": "fencing_conflict" }, "extra": 1 });
    assert!(
        serde_json::from_value::<ActionFailed>(rogue).is_err(),
        "unknown field rejected (deny_unknown_fields)"
    );
}

// ---- 2.4 L1 RED #4 — the §2.5-seam field-name snapshot for the changed/new event family ----

#[test]
fn test_action_failure_family_field_snapshot() {
    // spec(§7.1) — §2.5-seam freeze guard for the 2.4 contract additions: a field added/removed/
    // renamed on ActionPartiallySucceeded / ActionFailed / ActionError fails this snapshot. The
    // expected sets ARE the checked-in freeze.
    use nexusops_shared::actions::ActionError;
    use nexusops_shared::events::{ActionFailed, ActionPartiallySucceeded};
    expect_fields(
        &ActionPartiallySucceeded {
            reason: "x".to_string(),
        },
        &["reason"],
    );
    expect_fields(
        &ActionFailed {
            error: ActionError::UnknownOutcome,
        },
        &["error"],
    );
    // the data-carrying variant's internally-tagged wire shape is {kind, message}; a unit variant {kind}.
    expect_fields(
        &ActionError::ExecutorError {
            message: "x".to_string(),
        },
        &["kind", "message"],
    );
    expect_fields(&ActionError::AuditWriteFailed, &["kind"]);
}

// ---- 2.4 L1 RED #5 — CONTRACT_VERSION bumped to 0.19.0 (the additive event/taxonomy change) ----

#[test]
fn test_contract_version_bumped_0_19_0() {
    // 0.18.0 = the 2.2 catalog + PolicyDecision extension; 0.19.0 = the 2.4 §17 contract additions
    // (the ActionPartiallySucceeded event + the structured ActionError on ActionFailed) — additive (§5.0).
    // (The CURRENT canonical version pin is `test_contract_version_bumped_0_20_0`.)
    assert_eq!(nexusops_shared::CONTRACT_VERSION, "0.20.0");
}

// =====================================================================================
// Phase 3.1 L1 — §9.1 HarnessAdapter contract freeze (NEW: shared/src/harness.rs) +
// the §7.1 TelemetrySampled event. The next §2.5-seam shared-contract freeze (line 138
// lists the HarnessAdapter normalized types as a shared-contract model → snapshot-pinned).
// Freezes the NORMALIZED RETURN TYPES both the Claude (3.2) + Codex (3.3) adapters map
// INTO: TelemetrySample / MetricQuality / TranscriptRef / HarnessCapabilities + the
// TelemetrySampled telemetry-observation event. (ResumeResult is DAEMON-INTERNAL, NOT frozen
// here — resume/survival is a Phase-4 §8/§17 design; freezing it now would collide with the
// ui's provisional ResumeMode at P4 = a breaking reshape the §2.5-seam discipline prevents.)
// NormalizedStatus is NOT a new type —
// it is the frozen §5.1 `Session` machine (status.rs:45). CONTRACT 0.19.0 → 0.20.0, additive.
// Binding: ARCHITECTURE §9.1 + Appendix A rows (HarnessAdapter normalized types / capabilities)
// + §7.1 (the new event). The HarnessAdapter TRAIT + coverage matrix are daemon-only (Q5) —
// pinned in daemon/src/harness/mod.rs, not here.
// =====================================================================================

// ---- sample constructors (fully populated; Options = Some so the snapshot sees every key) --

fn sample_telemetry_sample() -> nexusops_shared::harness::TelemetrySample {
    use nexusops_shared::harness::{MetricQuality, TelemetrySample};
    TelemetrySample {
        tokens_in: 1_200,
        tokens_out: 340,
        context_pct: Some(42.5),
        cost_estimate: 0.0187,
        metric_quality: MetricQuality::Exact,
    }
}

fn sample_transcript_ref() -> nexusops_shared::harness::TranscriptRef {
    use nexusops_shared::harness::TranscriptRef;
    TranscriptRef {
        path: "~/.claude/projects/x/sess_01ARZ3NDEKTSV4RRFFQ69G5FAV.jsonl".to_string(),
        hash: "sha256:abc123".to_string(),
        is_in_place: true,
    }
}

fn sample_harness_capabilities() -> nexusops_shared::harness::HarnessCapabilities {
    use nexusops_shared::harness::HarnessCapabilities;
    // every flag true so the field-name snapshot sees all 10 keys (PRD HARN-5).
    HarnessCapabilities {
        supports_terminal: true,
        supports_resume: true,
        supports_transcript_read: true,
        supports_tool_call_parsing: true,
        supports_usage_metadata: true,
        supports_context_metadata: true,
        supports_command_injection: true,
        supports_subagents: true,
        supports_hooks: true,
        supports_cloud_tasks: true,
    }
}

fn sample_telemetry_sampled() -> nexusops_shared::events::TelemetrySampled {
    use nexusops_shared::events::TelemetrySampled;
    TelemetrySampled {
        sample: sample_telemetry_sample(),
        model: Some("claude-opus-4-8".to_string()),
        execution_profile_id: Some("prof_01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string()),
    }
}

// ---- 3.1 L1 RED #1 — the §2.5-seam normalized-type field-name snapshot (§9.1) ----

#[test]
fn test_harness_normalized_type_field_names_snapshot() {
    // spec(§9.1) — §2.5-seam freeze guard (line 138 lists the HarnessAdapter normalized types as a
    // shared-contract model). A field added/removed/renamed on any normalized type fails this
    // snapshot. The expected sets ARE the checked-in freeze. (HarnessCapabilities has its own
    // dedicated 10-field pin in `test_harness_capabilities_ten_fields`.)
    expect_fields(
        &sample_telemetry_sample(),
        &[
            "tokens_in",
            "tokens_out",
            "context_pct",
            "cost_estimate",
            "metric_quality",
        ],
    );
    expect_fields(&sample_transcript_ref(), &["path", "hash", "is_in_place"]);
    // (ResumeResult is daemon-internal — NOT in the shared freeze; see the section header.)
    // the telemetry-observation event: identity (session_id/project_id/occurred_at) is on the
    // envelope columns; the payload carries only the rollup dims the envelope lacks.
    expect_fields(
        &sample_telemetry_sampled(),
        &["sample", "model", "execution_profile_id"],
    );
}

// ---- 3.1 L1 RED #2 — MetricQuality wire values + reject-unknown (§9.1/§7.2/§11.7) ----

#[test]
fn test_metric_quality_wire_values() {
    // spec(§9.1) — `metric_quality` is carried on ALL telemetry; the UsageMeter degrades off it
    // (§11.7 — "exact" must never render over partially-estimated data). snake_case wire; reject-unknown.
    use nexusops_shared::harness::MetricQuality;
    check_values(MetricQuality::ALL, &["exact", "estimated", "unavailable"]);
    assert!(serde_json::from_value::<MetricQuality>(serde_json::json!("nope")).is_err());
}

// ---- 3.1 L1 RED #3 — the TelemetrySampled event wire contract (§7.1/§5.0/§15) ----

#[test]
fn test_telemetry_sampled_wire_contract() {
    // spec(§7.1) — EventTypeRegistry single-home + reject-unknown. Round-trips snake_case; an extra
    // key fails closed (deny_unknown_fields); the event-type name has ONE home (the EVENT_TYPE const,
    // the AuditIntegrityViolation precedent — adds no new bare string-literal to the emit/projector dup set).
    use nexusops_shared::events::TelemetrySampled;
    let v = sample_telemetry_sampled();
    let j = serde_json::to_value(&v).unwrap();
    assert_eq!(
        serde_json::from_value::<TelemetrySampled>(j).unwrap(),
        v,
        "TelemetrySampled round-trips"
    );
    let mut rogue = serde_json::to_value(&v)
        .unwrap()
        .as_object()
        .unwrap()
        .clone();
    rogue.insert("rogue".to_string(), serde_json::json!(1));
    assert!(
        serde_json::from_value::<TelemetrySampled>(serde_json::Value::Object(rogue)).is_err(),
        "unknown field rejected (deny_unknown_fields, §5.0/§15)"
    );
    assert_eq!(TelemetrySampled::EVENT_TYPE, "TelemetrySampled");
}

// ---- 3.1 L1 RED #4 — NormalizedStatus IS the frozen §5.1 Session machine (no forked enum) ----

#[test]
fn test_normalized_status_is_session() {
    // spec(§9.1) — status.rs:45: the §9.1 adapters map INTO `Session` (the driver-agnostic
    // normalized vocabulary). NormalizedStatus must BE that frozen 17-state machine, not a fork.
    use nexusops_shared::harness::NormalizedStatus;
    use nexusops_shared::status::Session;
    // type identity: a `Session` IS a `NormalizedStatus` (the re-export), so this compiles only if
    // they are the same type — a forked enum would not (a compile-time anti-fork guard).
    fn _identity(s: Session) -> NormalizedStatus {
        s
    }
    // and it is exactly the 17 Session states (the §5.1 freeze), not a subset/superset.
    assert_eq!(
        NormalizedStatus::ALL.len(),
        17,
        "NormalizedStatus == the 17-state Session machine"
    );
    assert_eq!(NormalizedStatus::ALL, Session::ALL);
}

// ---- 3.1 L1 RED #5 — CONTRACT_VERSION bumped to 0.20.0 (the additive §9.1 freeze) ----

#[test]
fn test_contract_version_bumped_0_20_0() {
    // 0.19.0 = the 2.4 §17 additions; 0.20.0 = the 3.1 §9.1 HarnessAdapter normalized-type freeze
    // (TelemetrySample/MetricQuality/TranscriptRef/HarnessCapabilities + the TelemetrySampled event)
    // — additive, no frozen type reshaped (§5.0). (ResumeResult is daemon-internal — NOT frozen here.)
    assert_eq!(nexusops_shared::CONTRACT_VERSION, "0.20.0");
}

// ---- 3.1 L1 RED #7 — HarnessCapabilities pins exactly the 10 PRD HARN-5 fields ----

#[test]
fn test_harness_capabilities_ten_fields() {
    // spec(§9.1) — Appendix A "HarnessCapabilities — 10 fields (PRD HARN-5)". The count AND the names
    // are the freeze; a flag added/removed/renamed fails here. Drives per-capability UI degradation.
    let caps = sample_harness_capabilities();
    expect_fields(
        &caps,
        &[
            "supports_terminal",
            "supports_resume",
            "supports_transcript_read",
            "supports_tool_call_parsing",
            "supports_usage_metadata",
            "supports_context_metadata",
            "supports_command_injection",
            "supports_subagents",
            "supports_hooks",
            "supports_cloud_tasks",
        ],
    );
    assert_eq!(
        field_names(&caps).len(),
        10,
        "exactly 10 capability fields (PRD HARN-5)"
    );
}
