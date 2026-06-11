//! The staged Action Gateway pipeline (§6/§6.1). L2: `submit_action` (intake → awaiting_approval).
//! L3 adds `approve`/`deny`/`preview_action` + execution completion.

use nexusops_shared::actions::{ActionRequest, PolicyDecisionStatus, RequiredApprover};
use nexusops_shared::gateway_ids::ApprovalId;
use nexusops_shared::ipc::ActionAck;
use nexusops_shared::status::{ActionRequest as ARStatus, Approval as ApprovalStatus};

use crate::eventstore::EventStore;
use crate::gateway::{approval, request, Gateway, GatewayError};

impl Gateway {
    /// `submit_action` (§6.1/AG §8) — run the staged single-action pipeline to `awaiting_approval`.
    /// ONE atomic gateway txn (fail-closed): persist the `action_requests` row at `Submitted` +
    /// emit `ActionRequested`; consult the policy stub (require-approval); advance
    /// Submitted→PolicyDecided→AwaitingApproval (R-9 guarded); open an `approvals` row + emit
    /// `ActionApprovalRequested`. Any event-write / row-write failure rolls the whole txn back — no
    /// row, no approval, no ack, no event (INV-SEC-1 / §15 / §17). **The requester-supplied
    /// `risk_level` is RECORDED for audit but NOT trusted for the decision in 2.1b** — the stub is
    /// risk-blind (require-approval-for-all), so an under-claimed risk cannot bypass the gate; 2.2
    /// makes risk catalog-authoritative.
    pub fn submit_action(
        &self,
        store: &mut EventStore,
        req: ActionRequest,
    ) -> Result<ActionAck, GatewayError> {
        store.gateway_txn(|gtx| {
            let now = gtx.now_rfc3339();
            let act_id = req.action_request_id.as_str().to_string();

            // 1. persist the intent at Submitted + emit ActionRequested (atomic: row + event).
            request::insert(gtx.tx(), &req, ARStatus::Submitted, &now)?;
            gtx.append(&request::action_requested_intent(&req, &now)?)?;

            // 2. policy decision (2.1b stub: always require_approval — risk-blind).
            let decision = self.policy().decide(&req);
            match decision.status {
                PolicyDecisionStatus::RequireApproval
                | PolicyDecisionStatus::RequireStepApproval => {
                    // 3. submitted → policy_decided → awaiting_approval (R-9 guarded, stepwise).
                    request::update_status(
                        gtx.tx(),
                        &act_id,
                        ARStatus::Submitted,
                        ARStatus::PolicyDecided,
                    )?;
                    request::update_status(
                        gtx.tx(),
                        &act_id,
                        ARStatus::PolicyDecided,
                        ARStatus::AwaitingApproval,
                    )?;

                    // 4. open an approval (awaiting the human) + emit ActionApprovalRequested.
                    let appr_id = ApprovalId::new();
                    approval::insert(
                        gtx.tx(),
                        &appr_id,
                        &act_id,
                        ApprovalStatus::AwaitingApproval,
                        &RequiredApprover::current_user(),
                        None, // no expiry on the stub submit; the expiry path is exercised at L3
                        &now,
                    )?;
                    gtx.append(&approval::approval_requested_intent(&req, &appr_id, &now)?)?;

                    Ok(ActionAck {
                        action_request_id: act_id,
                        status: ARStatus::AwaitingApproval,
                    })
                }
                // 2.2 implements the allow→queue/execute + deny→denied + downgrade routing; the
                // 2.1b stub is require-approval-for-all, so this arm is unreached today. Fail CLOSED
                // with an honest error (NOT a misleading `PolicyDenied`, NOT a write-actor panic) —
                // a future policy that returns an unrouted decision is rejected, never mis-routed.
                other => Err(GatewayError::UnsupportedPolicyDecision(format!(
                    "{other:?}"
                ))),
            }
        })
    }
}
