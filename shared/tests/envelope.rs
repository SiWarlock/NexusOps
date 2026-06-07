//! Phase 1.1 (L1) — Event envelope + the 3 new enums (ARCHITECTURE §7.1).
//! source_type from EM §8 (docs/architecture/EVENT_MODEL_AND_AUDIT_TRAIL.md §8);
//! sensitivity from §7.1/§15 (EM §9); visibility from §7.1. Same Option-A freeze
//! mechanism as 0.5 — snake_case wire, reject-unknown (cross-language equality +
//! schema-diff gate are covered by the shared verify harness, which auto-includes
//! the new enums once they're in the schema bundle).

use std::collections::BTreeSet;

use serde::de::DeserializeOwned;
use serde::Serialize;

fn wire<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .unwrap()
        .as_str()
        .expect("contract enum must serialize to a JSON string")
        .to_string()
}

fn check_values<T>(all: &[T], expected: &[&str])
where
    T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug + Copy,
{
    let got: BTreeSet<String> = all.iter().map(wire).collect();
    let exp: BTreeSet<String> = expected.iter().map(|s| s.to_string()).collect();
    assert_eq!(got, exp, "value-set mismatch");
    assert_eq!(all.len(), expected.len(), "variant count mismatch");
    for v in all {
        let j = serde_json::to_value(v).unwrap();
        let back: T = serde_json::from_value(j.clone()).unwrap();
        assert_eq!(*v, back, "round-trip {j:?}");
    }
}

#[test]
fn test_source_type_values() {
    use nexusops_shared::event_envelope::SourceType;
    // EM §8 source model (15). Frozen closed (Option A reject-unknown); the EM doc
    // lists these as "Examples" — see Step-2.5 flag on closed-vs-open.
    check_values(
        SourceType::ALL,
        &[
            "desktop_ui",
            "local_daemon",
            "action_gateway",
            "terminal_supervisor",
            "claude_code_adapter",
            "codex_adapter",
            "git_executor",
            "github_syncer",
            "linear_syncer",
            "workflow_pack_runtime",
            "project_brain_indexer",
            "project_brain_retriever",
            "project_brain_action_planner",
            "usage_meter",
            "remote_relay",
        ],
    );
}

#[test]
fn test_sensitivity_values() {
    use nexusops_shared::event_envelope::Sensitivity;
    check_values(
        Sensitivity::ALL,
        &["public", "internal", "confidential", "secret", "restricted"],
    );
}

#[test]
fn test_visibility_values() {
    use nexusops_shared::event_envelope::Visibility;
    check_values(Visibility::ALL, &["user", "project", "workspace", "system"]);
}

#[test]
fn test_unknown_enum_value_rejected() {
    use nexusops_shared::event_envelope::{Sensitivity, SourceType, Visibility};
    assert!(serde_json::from_value::<SourceType>(serde_json::json!("not_a_source")).is_err());
    assert!(serde_json::from_value::<Sensitivity>(serde_json::json!("ultra_secret")).is_err());
    assert!(serde_json::from_value::<Visibility>(serde_json::json!("global")).is_err());
}

#[test]
fn test_envelope_round_trips_with_required_and_optional_fields() {
    use nexusops_shared::actor::ActorType;
    use nexusops_shared::event_envelope::{
        EventEnvelope, ObjectRef, Sensitivity, SourceType, Visibility,
    };
    use nexusops_shared::ids::{ActionRequestId, EventId, ProjectId, SessionId, WorkspaceId};

    let env = EventEnvelope {
        // required (§7.1)
        event_id: EventId::new(),
        seq: 42,
        event_type: "SessionStarted".to_string(),
        event_version: 1,
        occurred_at: "2026-06-07T21:00:00.000Z".to_string(),
        recorded_at: "2026-06-07T21:00:00.010Z".to_string(),
        workspace_id: WorkspaceId::new(),
        actor_type: ActorType::User,
        actor_id: "u_1".to_string(),
        source_type: SourceType::DesktopUi,
        source_id: "src_1".to_string(),
        correlation_id: "corr_1".to_string(),
        sensitivity: Sensitivity::Internal,
        payload_json: "{}".to_string(),
        schema_version: "event-envelope-v1".to_string(),
        // optional (§7.1)
        project_id: Some(ProjectId::new()),
        session_id: Some(SessionId::new()),
        agent_team_id: None,
        causation_id: None,
        action_request_id: Some(ActionRequestId::new()),
        approval_id: None,
        workflow_run_id: None,
        object_refs: vec![ObjectRef {
            object_type: "session".to_string(),
            object_id: "sess_x".to_string(),
        }],
        idempotency_key: Some("idem_1".to_string()),
        visibility: Some(Visibility::Project),
        payload_hash: None,        // reserved R-10
        previous_event_hash: None, // reserved R-10
    };

    let json = serde_json::to_value(&env).unwrap();
    let back: EventEnvelope = serde_json::from_value(json).unwrap();
    assert_eq!(env, back, "envelope must round-trip");
    // seq is the canonical order key, kept as an integer (§7.1)
    assert_eq!(back.seq, 42);
}

#[test]
fn test_contract_version_bumped_past_0_5() {
    // L1 extends the frozen contract → CONTRACT_VERSION advances to 0.6.0 (additive, semver minor).
    assert_eq!(nexusops_shared::CONTRACT_VERSION, "0.6.0");
}
