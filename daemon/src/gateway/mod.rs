//! The Action Gateway — the single, audited mutator (INV-SEC-1 chokepoint, §6/§6.1/§6.2/§15).
//!
//! Every state mutation flows through here as a typed, risk-classified, approved `ActionRequest`
//! recorded as an immutable event (forbidden #2). The foundation (L1) is the durable registry
//! schema (`action_requests`/`approvals`, MIGRATION_7) + the **R-9 transition guards** for the
//! frozen §5.1 ActionRequest(15)/Approval(10) machines. **L2 (the chokepoint):** `submit_action`
//! runs the staged pipeline (normalize → resolve → policy-STUB → preview-STUB → approval) — each
//! transition's `{durable-row write + authoritative ActionExecution* event}` is ONE atomic txn on
//! the single write-actor (via [`crate::eventstore::EventStore::gateway_txn`]), through the §15
//! redaction gate; **fail-closed** (a failed event-write rolls back the row → no ack, no side
//! effect). The Gateway emits events ONLY via the gateway-txn append path (forbidden #2/#3) —
//! never writes the DB directly. `approve`/`deny`/`preview` + execution → L3.

pub mod approval;
pub mod circuit_breaker;
pub mod executor;
pub mod idempotency;
pub mod pipeline;
pub mod plan;
pub mod policy;
pub mod precondition;
pub mod preview;
pub mod recovery;
pub mod request;
pub mod session_executor;

use serde::Serialize;

use nexusops_shared::actions::ActionRequest;
use nexusops_shared::event_envelope::{Sensitivity, SourceType, Visibility};
use nexusops_shared::ids::WorkspaceId;

use crate::eventstore::{AppendIntent, EventStoreError};

/// Typed Gateway-pipeline failures (Q5 — daemon-internal; the IPC layer maps these to the §6.4
/// `IpcErrorCode` set). `AuditWriteFailed` wraps any [`EventStoreError`] from the gateway txn (a
/// failed authoritative-event append OR a registry-row write) — the fail-closed signal: the audited
/// mutation could not durably + validly persist, so nothing is acknowledged.
#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    #[error("illegal {machine} transition: {from} → {to}")]
    IllegalTransition {
        machine: &'static str,
        from: String,
        to: String,
    },
    #[error("audit-write failed (§15/§17 fail-closed): {0}")]
    AuditWriteFailed(#[from] EventStoreError),
    #[error("approval expired")]
    ApprovalExpired,
    #[error("not found: {0}")]
    NotFound(String),
    #[error("policy denied the action")]
    PolicyDenied,
    /// the action's resource lease is NOT live for THIS action (expired OR superseded — held by
    /// another owner) at execute time (§17 / safety rule #6, 1.4 Option-B). The mutation is refused;
    /// the action is recorded terminal-`failed` with `ActionError::FencingConflict` (the hard-conflict
    /// surface) and the caller gets this typed error (→ §6.4 `fencing_conflict`). **NEVER auto-resolved**
    /// — a human resolves the conflict + re-submits. The recorded ActionFailed COMMITS before this Err.
    #[error(
        "fencing conflict: the action's resource lease is not live (stale token, §17 rule #6)"
    )]
    FencingConflict,
    /// the action's live source CHANGED between preview/approval and execute (§6.2 AG-16.4 / §7.2):
    /// executing now would apply a DIFFERENT mutation than was approved. The action is recorded
    /// terminal-`failed` with `ActionError::StalePrecondition` (+ a regenerated preview) and the caller
    /// gets this typed error (→ §6.4 `precondition_stale`, the re-approvable stale card). A fresh
    /// approval cycle is required (re-submit). The recorded ActionFailed COMMITS before this Err.
    #[error("stale precondition: the action's live source changed since approval (§6.2 AG-16.4)")]
    StalePrecondition,
    /// a `submit_action_plan` carried an `approval_mode` the Gateway does not accept from a proposer
    /// (2.1c: `Blocked` — a policy-ASSIGNED outcome, not a valid submitted mode; 2.2 owns it). Fail
    /// closed: never open phantom `awaiting_approval` steps with no approval object (Step-2.5 Mod 1).
    #[error("unsupported approval_mode for a submitted plan: {0}")]
    UnsupportedApprovalMode(String),
    /// the policy returned a decision 2.1b does not route yet (the stub is require-approval-for-all;
    /// the allow→queue/execute, deny, downgrade, needs-more-context routing is 2.2). Fail-closed —
    /// distinct from `PolicyDenied` (which means the policy actively denied), and never a panic in
    /// the write-actor.
    #[error("policy decision not supported until 2.2: {0}")]
    UnsupportedPolicyDecision(String),
    #[error("gateway serialization failed: {0}")]
    Serialize(String),
    /// P4.0b-2c (§17/§15 #5, RULED B) — the daemon-wide audit-backbone circuit-breaker is LATCHED
    /// (systemic audit failure: N consecutive faults or a clearly-unrecoverable class). The single
    /// audited mutator does NOT operate when it cannot audit: every mutation fail-closed-denies WITHOUT
    /// attempting an audit-write, until an operator restart. Reads stay live. Maps to §6.4
    /// `internal_error` (the loud distinguishable signal is the durable systemic alarm + the latched
    /// breaker state, not the generic wire code).
    #[error("audit backbone down (§17 systemic — daemon quiesced, mutations refused, latched)")]
    AuditBackboneDown,
}

/// Map a registry-row rusqlite error → fail-closed `AuditWriteFailed` (a failed row write in the
/// gateway txn means the audited mutation didn't persist → roll back).
pub(crate) fn db_err(e: rusqlite::Error) -> GatewayError {
    GatewayError::AuditWriteFailed(EventStoreError::Write(e))
}

/// Map a non-`Held` lease error → fail-closed `GatewayError` (2.4 L3). A lease-table DB error is an
/// `AuditWriteFailed` (the audited mutation can't proceed safely); `NotHolder`/`TokenOverflow` are
/// logic errors that fail closed. (`Held` is adjudicated by the caller — fencing-conflict vs self-renew
/// — and never reaches here.)
pub(crate) fn lease_err(e: crate::locks::LeaseError) -> GatewayError {
    match e {
        crate::locks::LeaseError::Db(inner) => {
            GatewayError::AuditWriteFailed(EventStoreError::Write(inner))
        }
        other => GatewayError::Serialize(format!("lease error: {other}")),
    }
}

/// A closed contract enum → its exact snake_case wire string (for a `TEXT` registry column). A
/// serde failure preserves its context (vs collapsing to a generic message).
pub(crate) fn enum_wire<T: Serialize>(v: &T) -> Result<String, GatewayError> {
    let j = serde_json::to_value(v).map_err(|e| GatewayError::Serialize(e.to_string()))?;
    j.as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| GatewayError::Serialize("enum did not render to a wire string".to_string()))
}

/// `RiskLevel` (and any integer-wire newtype) → its integer (for the `risk_level` column).
pub(crate) fn enum_int<T: Serialize>(v: &T) -> Result<i64, GatewayError> {
    let j = serde_json::to_value(v).map_err(|e| GatewayError::Serialize(e.to_string()))?;
    j.as_i64()
        .ok_or_else(|| GatewayError::Serialize("value did not render to an integer".to_string()))
}

/// Build an `ActionExecution*` [`AppendIntent`] for `req`: identity on the envelope columns (§7.1)
/// — `correlation_id == action_request_id` (ties the event family), the requester's **mapped audit
/// actor** (R-2 `requester_type → actor_type`), `source = action_gateway`. The payload carries only
/// the event-specific delta. `workspace_id` uses the system sentinel in 2.1b (project→workspace
/// resolution lands with Phase-5 projects — Step-9 flag). Events go ONLY through the gateway-txn
/// `.append()` so this passes the §15 redaction gate (forbidden #2/#3).
pub(crate) fn gateway_event_intent(
    req: &ActionRequest,
    event_type: &str,
    payload_json: String,
    occurred_at: &str,
    approval_id: Option<String>,
) -> AppendIntent {
    AppendIntent {
        event_type: event_type.to_string(),
        event_version: 1,
        occurred_at: occurred_at.to_string(),
        // 2.1b: the action's true workspace (project→workspace) resolves with Phase-5 projects;
        // until then gateway events scope to the reserved system workspace (Step-9 flag).
        workspace_id: WorkspaceId::system(),
        actor_type: req.requester_type.to_actor_type(),
        actor_id: req.requester_id.clone(),
        source_type: SourceType::ActionGateway,
        source_id: "action_gateway".to_string(),
        correlation_id: req.action_request_id.as_str().to_string(),
        sensitivity: Sensitivity::Internal,
        payload_json,
        schema_version: "event-envelope-v1".to_string(),
        idempotency_key: None,
        project_id: req.project_id.clone(),
        session_id: None,
        agent_team_id: None,
        visibility: Some(Visibility::Project),
        action_request_id: Some(req.action_request_id.clone()),
        approval_id,
        causation_id: None,
    }
}

/// The Action Gateway — holds the injected policy engine + executor (both stubbed in 2.1b; 2.2
/// swaps the policy, 2.3 the executor) + the §6.2 stale-precondition oracle (2.4 L4; defaults to the
/// no-op [`precondition::NullPreconditionOracle`]). The pipeline methods (`submit_action`/`approve`/
/// `deny`/`preview_action`) live in [`pipeline`].
pub struct Gateway {
    policy: Box<dyn policy::PolicyEngine>,
    executor: Box<dyn executor::ActionExecutor>,
    precondition: Box<dyn precondition::PreconditionOracle>,
}

impl Gateway {
    /// The production constructor — defaults the §6.2 precondition oracle to the no-op
    /// [`precondition::NullPreconditionOracle`] (the real per-action_type live-source re-read lands
    /// Phase 5/7). Every existing call site keeps this 2-arg form.
    pub fn new(
        policy: Box<dyn policy::PolicyEngine>,
        executor: Box<dyn executor::ActionExecutor>,
    ) -> Self {
        Self {
            policy,
            executor,
            precondition: Box::new(precondition::NullPreconditionOracle),
        }
    }

    /// A full constructor that takes all three seams explicitly (2.4 L4) — use this to inject a
    /// specific precondition oracle (a real Phase-5/7 oracle, or a test fake) instead of the
    /// [`precondition::NullPreconditionOracle`] that [`Gateway::new`] defaults to; `policy`/`executor`
    /// are supplied the same as in `new`.
    pub fn with_precondition(
        policy: Box<dyn policy::PolicyEngine>,
        executor: Box<dyn executor::ActionExecutor>,
        precondition: Box<dyn precondition::PreconditionOracle>,
    ) -> Self {
        Self {
            policy,
            executor,
            precondition,
        }
    }

    /// the injected policy engine (the pipeline submodule consults it).
    pub(crate) fn policy(&self) -> &dyn policy::PolicyEngine {
        self.policy.as_ref()
    }

    /// the injected stale-precondition oracle (the pipeline re-checks it after lock, before execute).
    pub(crate) fn precondition(&self) -> &dyn precondition::PreconditionOracle {
        self.precondition.as_ref()
    }

    /// the injected executor (the pipeline runs it BETWEEN the approve + completion txns).
    pub(crate) fn executor(&self) -> &dyn executor::ActionExecutor {
        self.executor.as_ref()
    }
}
