//! Frozen projection-row contracts (§7 / §5.0) — the typed read-model rows the UI/Brain consume.
//!
//! The FIRST frozen projection-row is [`ApprovalQueueRow`] (P4.0b-ui2, the ②-mini): the
//! `proj_approval_queue` read model the §11.5 human-approval card renders, typed in `shared/` so the
//! safety-critical approval path is served as a contract, not loose JSON (Fork B, pin #2). The other
//! projection-row reconciles (SessionRow / ProjectActivityRow / PullRequestRow / AuditEventRow) follow
//! this pattern. `schemars` → versioned JSON-Schema → generated Zod/Pydantic (§5.0); reject-unknown.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::actions::{PolicyDecision, RequesterType, RiskLevel};
use crate::status::Approval;

/// One open row of the `proj_approval_queue` read model (§7 / §11.5) — an approval awaiting a human
/// decision. The bookkeeping `sort_key`/`updated_at_seq` columns are NOT on the wire row (internal —
/// the UI sorts off `risk_level` + `requested_at`). The load-bearing typed fields: `risk_level`
/// (the authoritative §6.3 risk) + `policy_decision` (the frozen §6.2 decision, persisted §15-redacted
/// at approval-open in C1; `None` for a plan-level approve-all). `status`/`requester_type` are the
/// frozen §5.1/§6.2 enums (reject-unknown — no loose JSON on the approval path). `action_request_id`
/// is `None` for a plan-level approve-all approval. Optionals serialize as explicit `null` (no
/// `skip_serializing_if`) so the §2.5-seam field-name snapshot is stable (LESSON §15 trap 3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)] // reject-unknown end-to-end (§5.0/§15 fail-closed)
pub struct ApprovalQueueRow {
    pub approval_id: String,
    pub action_request_id: Option<String>,
    pub plan_id: Option<String>,
    pub project_id: Option<String>,
    pub session_id: Option<String>,
    pub agent_team_id: Option<String>,
    pub risk_level: RiskLevel,
    pub status: Approval,
    pub requester_type: RequesterType,
    pub requester_id: String,
    pub preview_summary: Option<String>,
    pub requested_at: String,
    pub expires_at: Option<String>,
    pub policy_decision: Option<PolicyDecision>,
}
