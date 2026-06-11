//! The staged Action Gateway pipeline (§6/§6.1). L2: `submit_action` (intake → awaiting_approval).
//! L3 adds `approve`/`deny`/`preview_action` + execution completion.

use nexusops_shared::actions::{
    ActionPreview, ActionRequest, PolicyDecisionStatus, RequiredApprover,
};
use nexusops_shared::gateway_ids::ApprovalId;
use nexusops_shared::ipc::{ActionAck, DeltaKind, ProjectionDelta, ProjectionName};
use nexusops_shared::status::{ActionRequest as ARStatus, Approval as ApprovalStatus};
use nexusops_shared::time::Timestamp;

use crate::eventstore::EventStore;
use crate::gateway::executor::ExecutionOutcome;
use crate::gateway::{approval, request, Gateway, GatewayError};

/// the well-known approver for the 2.1b stub decisions (RequiredApprover::current_user). 2.2/the
/// real auth surface resolves the actual `decided_by` from the IPC peer identity.
const STUB_DECIDER: &str = "current_user";

/// The `proj_approval_queue` subscribe-delta for a touched approval row (§6.1 subscribe, Q6): a
/// "something changed" Upsert nudge keyed by `approval_id` (the subscriber re-reads the row via
/// `get_projection`, consistent with the lag-resync policy). The Gateway pipeline accumulates these
/// into a `&mut Vec`; the write-actor publishes them **after commit** (forbidden #3 — a reader
/// never back-pressures the writer).
pub(crate) fn approval_queue_delta(approval_id: &str) -> ProjectionDelta {
    ProjectionDelta {
        projection: ProjectionName::ApprovalQueue,
        kind: DeltaKind::Upsert,
        row: None,
        id: Some(approval_id.to_string()),
    }
}

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
        // public 2.1b signature, unchanged — collects+discards the subscribe-deltas. The write-actor
        // path (`submit_action_collecting`) keeps them to publish post-commit (Q6).
        self.submit_action_collecting(store, req, &mut Vec::new())
    }

    /// `submit_action` accumulating the `proj_approval_queue` subscribe-delta(s) it produces into
    /// `deltas` (Q6) — the write-actor publishes them AFTER the txn commits. Same pipeline as
    /// [`Gateway::submit_action`]; only the delta accumulation differs.
    pub(crate) fn submit_action_collecting(
        &self,
        store: &mut EventStore,
        req: ActionRequest,
        deltas: &mut Vec<ProjectionDelta>,
    ) -> Result<ActionAck, GatewayError> {
        store.gateway_txn(|gtx| {
            let now = gtx.now_rfc3339();
            let act_id = req.action_request_id.as_str().to_string();

            // 1. persist the intent at Submitted (inputs §15-redacted at rest) + emit ActionRequested.
            request::insert(gtx, &req, ARStatus::Submitted, &now)?;
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
                    // the queue row was just folded in-band → nudge subscribers (publish post-commit).
                    deltas.push(approval_queue_delta(appr_id.as_str()));

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

    /// `approve` (§6.1/AG §8.8) — resolve an awaiting approval. The **decision txn** (emits
    /// ActionApproved and transitions the action AwaitingApproval→Approved→Queued, R-9 guarded,
    /// atomic) runs first; then — if not expired — the **execute step** runs structurally SEPARATELY
    /// (the executor OFF the write-actor txn, then a completion txn for ActionStarted plus
    /// ActionSucceeded/Failed). An approval past its `expires_at` → `ActionExpired`, the executor is
    /// NEVER invoked (§17). Fail-closed throughout.
    pub fn approve(
        &self,
        store: &mut EventStore,
        approval_id: &str,
    ) -> Result<ActionAck, GatewayError> {
        // public 2.1b signature, unchanged (collects+discards deltas — Q6).
        self.approve_collecting(store, approval_id, &mut Vec::new())
    }

    /// `approve` accumulating the `proj_approval_queue` subscribe-delta(s) into `deltas` (Q6) — the
    /// decision txn advances the approval row's status, so the write-actor nudges subscribers
    /// post-commit. Same pipeline as [`Gateway::approve`].
    pub(crate) fn approve_collecting(
        &self,
        store: &mut EventStore,
        approval_id: &str,
        deltas: &mut Vec<ProjectionDelta>,
    ) -> Result<ActionAck, GatewayError> {
        // --- decision txn: returns (the loaded action, queued?) — queued=false means expired ---
        let (req, queued) =
            store.gateway_txn(|gtx| -> Result<(ActionRequest, bool), GatewayError> {
                let now = gtx.now_rfc3339();
                let appr = approval::load(gtx.tx(), approval_id)?;
                // the approval must still be awaiting a decision — re-deciding a resolved approval
                // (already approved/denied/expired — all terminal) is rejected up front.
                if appr.status != ApprovalStatus::AwaitingApproval {
                    return Err(GatewayError::IllegalTransition {
                        machine: "Approval",
                        from: format!("{:?}", appr.status),
                        to: "decided".to_string(),
                    });
                }
                let act_id = appr.action_request_id.clone();
                let req = request::load(gtx.tx(), &act_id)?;

                // §17 expiry: an approval past expires_at lapses → ActionExpired, never executes.
                // RFC3339-`Z` strings sort lexically == chronologically (LESSON §5) — so the
                // expires_at a real caller writes (2.1c sets one; the 2.1b stub leaves it NULL) MUST
                // be a `Z`-suffix UTC string, not a numeric `+00:00` offset (which would mis-sort).
                if appr
                    .expires_at
                    .as_deref()
                    .is_some_and(|exp| exp < now.as_str())
                {
                    approval::update_status(
                        gtx.tx(),
                        approval_id,
                        ApprovalStatus::AwaitingApproval,
                        ApprovalStatus::Expired,
                    )?;
                    request::update_status(
                        gtx.tx(),
                        &act_id,
                        ARStatus::AwaitingApproval,
                        ARStatus::Expired,
                    )?;
                    gtx.append(&approval::expired_intent(&req, approval_id, &now)?)?;
                    deltas.push(approval_queue_delta(approval_id)); // queue row → expired
                    return Ok((req, false));
                }

                // approve: stamp the decision, transition both machines, emit ActionApproved, queue.
                approval::update_status(
                    gtx.tx(),
                    approval_id,
                    ApprovalStatus::AwaitingApproval,
                    ApprovalStatus::Approved,
                )?;
                approval::record_decision(gtx.tx(), approval_id, Some(STUB_DECIDER), &now)?;
                request::update_status(
                    gtx.tx(),
                    &act_id,
                    ARStatus::AwaitingApproval,
                    ARStatus::Approved,
                )?;
                gtx.append(&approval::approved_intent(
                    &req,
                    approval_id,
                    Some(STUB_DECIDER),
                    &now,
                )?)?;
                request::update_status(gtx.tx(), &act_id, ARStatus::Approved, ARStatus::Queued)?;
                deltas.push(approval_queue_delta(approval_id)); // queue row → approved
                Ok((req, true))
            })?;

        if !queued {
            return Ok(ActionAck {
                action_request_id: req.action_request_id.as_str().to_string(),
                status: ARStatus::Expired,
            });
        }
        // --- execute step (structurally separate; the executor runs OFF the txn) ---
        // NOTE (Q6 corollary, →2.4): the decision txn above ALREADY committed the queue-row status
        // change (awaiting_approval→approved) + pushed its delta into `deltas`. If `execute`'s
        // completion txn then errors, `approve_collecting` returns Err, so the write-actor's
        // `publish_after_commit` (result.is_ok() gate) SUPPRESSES that already-durable delta — the
        // subscriber is briefly stale until its next reconnect/`get_projection` (the lag-resync
        // policy covers it; the stub executor never fails, so it's unreached in 2.1c). 2.4 reconciles
        // the same two-txn boundary for crash recovery; the missed-nudge folds in there.
        self.execute(store, &req)
    }

    /// Run the executor for an approved+queued action, then record the outcome. The executor is
    /// invoked OUTSIDE any write-actor txn (2.3's git-CLI/octocrab executors move off-thread here);
    /// the ActionStarted + ActionSucceeded/Failed transitions land in one atomic completion txn.
    fn execute(
        &self,
        store: &mut EventStore,
        req: &ActionRequest,
    ) -> Result<ActionAck, GatewayError> {
        // the side effect (stub: none) runs BETWEEN the decision txn and the completion txn.
        let outcome = self.executor().execute(req);
        let act_id = req.action_request_id.as_str().to_string();
        store.gateway_txn(|gtx| -> Result<ActionAck, GatewayError> {
            let now = gtx.now_rfc3339();
            // 2.1b: `approve`'s decision txn + this completion txn both run inside ONE
            // `Command::GatewayApprove` on the single write-actor → fully serialized, so no second
            // approve can race this Queued→Executing slot. The remaining gap is CRASH recovery — a
            // crash between the two txns strands the action at `queued` (harmless with the no-side-
            // effect stub; with a real executor, 2.4 reconciles orphaned queued/executing actions
            // by idempotency key + adds the fencing guard, §17/Q6). The `WHERE status=queued` guard
            // already makes a stale slot a typed `NotFound`, never a silent double-execute.
            request::update_status(gtx.tx(), &act_id, ARStatus::Queued, ARStatus::Executing)?;
            gtx.append(&request::started_intent(req, &now)?)?;
            match outcome {
                ExecutionOutcome::Succeeded => {
                    request::update_status(
                        gtx.tx(),
                        &act_id,
                        ARStatus::Executing,
                        ARStatus::Succeeded,
                    )?;
                    gtx.append(&request::succeeded_intent(req, &now)?)?;
                    Ok(ActionAck {
                        action_request_id: act_id.clone(),
                        status: ARStatus::Succeeded,
                    })
                }
                ExecutionOutcome::Failed(err) => {
                    request::update_status(
                        gtx.tx(),
                        &act_id,
                        ARStatus::Executing,
                        ARStatus::Failed,
                    )?;
                    gtx.append(&request::failed_intent(req, &err, &now)?)?;
                    Ok(ActionAck {
                        action_request_id: act_id.clone(),
                        status: ARStatus::Failed,
                    })
                }
            }
        })
    }

    /// `deny` (§6.1/AG §8.8) — deny an awaiting approval with a `reason`. ONE atomic txn: stamp the
    /// decision, transition Approval→Denied + action AwaitingApproval→Denied, emit `ActionDenied`.
    /// Terminal — the executor is NEVER invoked.
    pub fn deny(
        &self,
        store: &mut EventStore,
        approval_id: &str,
        reason: &str,
    ) -> Result<ActionAck, GatewayError> {
        // public 2.1b signature, unchanged (collects+discards deltas — Q6).
        self.deny_collecting(store, approval_id, reason, &mut Vec::new())
    }

    /// `deny` accumulating the `proj_approval_queue` subscribe-delta into `deltas` (Q6).
    pub(crate) fn deny_collecting(
        &self,
        store: &mut EventStore,
        approval_id: &str,
        reason: &str,
        deltas: &mut Vec<ProjectionDelta>,
    ) -> Result<ActionAck, GatewayError> {
        store.gateway_txn(|gtx| -> Result<ActionAck, GatewayError> {
            let now = gtx.now_rfc3339();
            let appr = approval::load(gtx.tx(), approval_id)?;
            if appr.status != ApprovalStatus::AwaitingApproval {
                return Err(GatewayError::IllegalTransition {
                    machine: "Approval",
                    from: format!("{:?}", appr.status),
                    to: "denied".to_string(),
                });
            }
            let act_id = appr.action_request_id.clone();
            let req = request::load(gtx.tx(), &act_id)?;

            approval::update_status(
                gtx.tx(),
                approval_id,
                ApprovalStatus::AwaitingApproval,
                ApprovalStatus::Denied,
            )?;
            approval::record_decision(gtx.tx(), approval_id, Some(STUB_DECIDER), &now)?;
            request::update_status(
                gtx.tx(),
                &act_id,
                ARStatus::AwaitingApproval,
                ARStatus::Denied,
            )?;
            gtx.append(&approval::denied_intent(&req, approval_id, reason, &now)?)?;
            deltas.push(approval_queue_delta(approval_id)); // queue row → denied
            Ok(ActionAck {
                action_request_id: act_id,
                status: ARStatus::Denied,
            })
        })
    }

    /// `preview_action` (§6.1) — a dry-run preview of an action (read-only). Returns the stub
    /// `ActionPreview` envelope (the 6 typed per-class previews → 2.3). Runs through `gateway_txn`
    /// for uniform row access; it performs no write, so the txn commits a no-op.
    pub fn preview_action(
        &self,
        store: &mut EventStore,
        action_request_id: &str,
    ) -> Result<ActionPreview, GatewayError> {
        store.gateway_txn(|gtx| -> Result<ActionPreview, GatewayError> {
            let now = gtx.now_rfc3339();
            let req = request::load(gtx.tx(), action_request_id)?;
            let generated_at =
                Timestamp::parse(&now).map_err(|e| GatewayError::Serialize(e.to_string()))?;
            Ok(self.executor().preview(&req, generated_at))
        })
    }
}
