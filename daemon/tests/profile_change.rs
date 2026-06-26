//! P5.3b/085 — piece 4: the `session.profile_change` executor body (§15 #8 no-silent-account-hop). The
//! SessionExecutor's profile-rebind arm: a REGISTERED target → an audited `SessionProfileChanged` swap event;
//! an absent/unparseable/unregistered target → Failed (no mint, no hop). The UI/IPC-only requester gate +
//! the catalog risk-2 approval gate are pinned in `tests/policy.rs` + `shared/tests/contract.rs`.

use std::sync::Arc;

use nexusopsd::gateway::executor::{ActionExecutor, EmittedEvent, ExecutionOutcome};
use nexusopsd::gateway::session_executor::SessionExecutor;
use nexusopsd::session::{spawn_supervisor_task, FakeLauncher, SupervisorHandle};

use nexusops_shared::actions::{
    ActionRequest, RequesterType, ResourceRef, ResourceType, RiskLevel,
};
use nexusops_shared::events::SessionProfileChanged;
use nexusops_shared::harness::HarnessCapabilities;
use nexusops_shared::ids::{ActionRequestId, ExecutionProfileId, SessionId};
use nexusops_shared::status::ActionRequestStatus;
use nexusops_shared::time::Timestamp;

fn full_caps() -> HarnessCapabilities {
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

/// An in-memory [`ProfileLookup`] double — a set of registered ids (the §15 #8 fail-closed-on-unknown gate).
struct FakeProfileLookup {
    default: ExecutionProfileId,
    known: std::collections::HashSet<String>,
}
impl FakeProfileLookup {
    fn with(known: &ExecutionProfileId) -> Self {
        let mut set = std::collections::HashSet::new();
        set.insert(known.as_str().to_string());
        Self {
            default: ExecutionProfileId::new(),
            known: set,
        }
    }
}
impl nexusopsd::profiles::ProfileLookup for FakeProfileLookup {
    fn default_id(&self) -> Result<ExecutionProfileId, nexusopsd::profiles::ProfileError> {
        Ok(self.default.clone())
    }
    fn exists(&self, id: &ExecutionProfileId) -> Result<bool, nexusopsd::profiles::ProfileError> {
        Ok(self.known.contains(id.as_str()))
    }
}

/// Build a SessionExecutor over a NON-LIVE FakeLauncher + a real (unbounded) supervisor + the given lookup.
/// Returns the executor + the shutdown/join keep-alives (drop them only at the end so the supervisor stays up).
fn session_executor_with(
    lookup: Box<dyn nexusopsd::profiles::ProfileLookup>,
) -> (
    SessionExecutor,
    tokio::sync::watch::Sender<bool>,
    tokio::task::JoinHandle<()>,
) {
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let (join, handle): (_, SupervisorHandle) = spawn_supervisor_task(
        shutdown_rx,
        Arc::new(nexusopsd::decisions::DecisionRegistry::new()),
        Box::new(nexusopsd::session::NullSessionDeathSink),
        Arc::new(nexusopsd::terminal::NoopScrollbackStore),
    );
    let exec = SessionExecutor::new(Box::new(FakeLauncher::new(full_caps())), handle, lookup);
    (exec, shutdown_tx, join)
}

/// a session.profile_change request: the AUDITED target session is the resource_ref; the NEW profile is
/// `inputs.execution_profile_id` (the `new_profile` arg, omitted when `None` to test the absent path).
fn profile_change_req(session: &SessionId, new_profile: Option<&str>) -> ActionRequest {
    let inputs = match new_profile {
        Some(p) => serde_json::json!({ "execution_profile_id": p }),
        None => serde_json::json!({}),
    };
    ActionRequest {
        action_request_id: ActionRequestId::new(),
        project_id: None,
        action_type: "session.profile_change".to_string(),
        requester_type: RequesterType::User,
        requester_id: "u_local".to_string(),
        resource_refs: vec![ResourceRef {
            resource_type: ResourceType::Session,
            id: session.as_str().to_string(),
            uri: None,
        }],
        inputs,
        risk_level: RiskLevel::Level2,
        idempotency_key: None,
        fencing_token: None,
        status: ActionRequestStatus::Submitted,
        preview: None,
        created_at: Timestamp::parse("2026-06-25T00:00:00Z").unwrap(),
    }
}

#[tokio::test]
async fn profile_change_to_registered_records_swap() {
    // spec(§15 #8 recorded-not-silent) — a REGISTERED target profile → Succeeded with an audited
    // SessionProfileChanged carrying {session_id, new execution_profile_id}.
    let session = SessionId::new();
    let new_profile = ExecutionProfileId::new();
    let (exec, _sd, _join) = session_executor_with(Box::new(FakeProfileLookup::with(&new_profile)));

    let outcome = exec.execute(&profile_change_req(&session, Some(new_profile.as_str())));
    match outcome {
        ExecutionOutcome::Succeeded {
            emitted_events,
            side_effect_applied,
            ..
        } => {
            assert!(
                !side_effect_applied,
                "the event IS the record (no external re-bind side effect)"
            );
            assert_eq!(emitted_events.len(), 1);
            match &emitted_events[0] {
                EmittedEvent::Namespaced {
                    event_type,
                    payload_json,
                } => {
                    assert_eq!(*event_type, SessionProfileChanged::EVENT_TYPE);
                    let p: SessionProfileChanged = serde_json::from_str(payload_json).unwrap();
                    assert_eq!(p.session_id, session, "the swap records the target session");
                    assert_eq!(
                        p.execution_profile_id, new_profile,
                        "and the NEW (registered) profile"
                    );
                }
                _ => panic!("expected a Namespaced SessionProfileChanged event"),
            }
        }
        _ => panic!("a registered target should succeed"),
    }
}

#[tokio::test]
async fn profile_change_unknown_or_absent_fails_closed_no_hop() {
    // spec(§15 #8 — no account-hop, no mint) — an unregistered / unparseable / ABSENT target profile →
    // Failed BEFORE any swap (no SessionProfileChanged emitted).
    let registered = ExecutionProfileId::new();
    let session = SessionId::new();
    let (exec, _sd, _join) = session_executor_with(Box::new(FakeProfileLookup::with(&registered)));

    // (a) a WELL-FORMED but UNREGISTERED target → Failed (fail-closed-on-unknown, no mint).
    let unknown = ExecutionProfileId::new();
    assert!(matches!(
        exec.execute(&profile_change_req(&session, Some(unknown.as_str()))),
        ExecutionOutcome::Failed(_)
    ));
    // (b) an UNPARSEABLE target id → Failed.
    assert!(matches!(
        exec.execute(&profile_change_req(&session, Some("not-a-prof-id"))),
        ExecutionOutcome::Failed(_)
    ));
    // (c) an ABSENT target (no inputs.execution_profile_id) → Failed (a change has NO default fallback).
    assert!(matches!(
        exec.execute(&profile_change_req(&session, None)),
        ExecutionOutcome::Failed(_)
    ));
}
