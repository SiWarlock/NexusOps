//! The staged Action Gateway pipeline (§6/§6.1). L2: `submit_action` (intake → awaiting_approval).
//! L3 adds `approve`/`deny`/`preview_action` + execution completion.

use nexusops_shared::actions::{
    ActionError, ActionPlan, ActionPreview, ActionRequest, ApprovalMode, PolicyDecisionStatus,
    RequiredApprover, ResourceType, RiskLevel,
};
use nexusops_shared::catalog;
use nexusops_shared::events::{
    PullRequestMerged, PullRequestSynced, ReviewSubmitted, ReviewSynced, WorktreeCreated,
};
use nexusops_shared::gateway_ids::ApprovalId;
use nexusops_shared::ipc::{
    ActionAck, DeltaKind, PlanAck, PlanStepAck, ProjectionDelta, ProjectionName,
};
use nexusops_shared::status::{ActionRequest as ARStatus, Approval as ApprovalStatus};
use nexusops_shared::time::Timestamp;

use crate::eventstore::{EventStore, GatewayTxn};
use crate::gateway::executor::{EmittedEvent, ExecutionOutcome};
use crate::gateway::precondition::PreconditionStatus;
use crate::gateway::{
    approval, db_err, enum_wire, idempotency, lease_err, plan, preview, request, Gateway,
    GatewayError,
};
use crate::locks::{FencingToken, LeaseError, LeaseKind, OwnerId, ResourceId};
use crate::projections::{deltas_for_event, EventDeltaIds};

/// The TTL (seconds) of an action's execution lease (2.4 L3). A bound, conservative window for the
/// synchronous stub execute; the real long-running executors (Phase 3+) renew it via the heartbeat seam.
const FENCE_TTL_SECS: i64 = 300;

/// The routed outcome of `submit_action`'s decision txn (2.2): either a terminal ack (the action
/// rests at `awaiting_approval`), or a risk-0 `allow` action queued for the SEPARATE execute step
/// (the executor runs OFF the write-actor txn — LESSON §16; 2.3 moves it off-thread).
enum Routed {
    /// the require-approval path: the ack is final (no execution without a human approve).
    Done(ActionAck),
    /// the risk-0 `allow` auto-execute path: run the executor + completion txn after commit.
    /// Boxed — `ActionRequest` is much larger than `ActionAck` (clippy::large_enum_variant).
    Execute(Box<ActionRequest>),
}

/// The outcome of an `approve_single` decision txn (2.4 + 043): an EXPIRED approval (lapsed past
/// `expires_at`, never executes); an ADJUDICATED action (043 — an adjudication-only action that rests
/// at `Approved`, the verdict, with NO daemon execution); or a QUEUED action (the normal path — the
/// executor runs after commit).
enum ApproveOutcome {
    Expired,
    Adjudicated,
    Queued,
}

/// The shape of an approval the gateway is about to resolve (the 2.1c approve/deny dispatch).
enum ApprovalTarget {
    /// a single-action or per-step approval (`action_request_id` set) → the 2.1b single path.
    Single,
    /// a plan-level approve-all approval (`action_request_id` NULL) → cascade over the plan's
    /// non-critical steps. Carries the `plan_id`.
    Plan(String),
}

/// the well-known approver for the 2.1b stub decisions (RequiredApprover::current_user). 2.2/the
/// real auth surface resolves the actual `decided_by` from the IPC peer identity.
const STUB_DECIDER: &str = "current_user";

/// Whether an `action_type` is an **adjudication-only** agent mutation (043): its §6.3 catalog
/// executor is [`ExecutorKind::Adjudication`]. Such an action TERMINATES at the policy/approval
/// verdict — the daemon adjudicates + audits, the **agent** runs the tool; NO daemon executor ever
/// runs (the verdict is the side-effect, gated on the audit event, §15 #5 / INV-SEC-1). Both
/// `submit_action` (a risk-0 `allow` rests at `PolicyDecided`) and `approve` (a human-approved action
/// rests at `Approved`) consult this to skip the `queued`→`executing` execution path. An uncatalogued
/// type is NOT adjudication-only (it is denied upstream); fail-closed (`None` → false).
fn is_adjudication(action_type: &str) -> bool {
    catalog::lookup(action_type).map(|e| e.executor) == Some(catalog::ExecutorKind::Adjudication)
}

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

/// D4b — map a gateway `EmittedEvent` (+ the action `req` for the envelope/resource ids) to its
/// projection-delta nudges, via the SHARED [`deltas_for_event`] (ONE mapping for both paths, §51). The
/// gateway path is the one that HAS the payload — so it supplies the PAYLOAD ids (`worktree_id`/`pr_id`),
/// where the Command::Append path is payload-agnostic. `session_id` comes from the typed `SessionStarted`
/// variant; `project_id` from `req`. `pr_id` = `{repo_id}#{pr_number}` (repo_id = the action's Repo
/// resource_ref) to match the `proj_pull_request` row PK — absent → coarse `None` (the workspace re-reads).
fn emitted_event_deltas(req: &ActionRequest, ev: &EmittedEvent) -> Vec<ProjectionDelta> {
    let project_id = req.project_id.as_ref().map(|p| p.as_str().to_string());
    match ev {
        EmittedEvent::SessionStarted { session_id, .. } => deltas_for_event(
            "SessionStarted",
            &EventDeltaIds {
                session_id: Some(session_id.as_str().to_string()),
                project_id,
                ..Default::default()
            },
        ),
        EmittedEvent::Namespaced {
            event_type,
            payload_json,
        } => {
            let mut ids = EventDeltaIds {
                project_id,
                ..Default::default()
            };
            if *event_type == WorktreeCreated::EVENT_TYPE {
                if let Ok(w) = serde_json::from_str::<WorktreeCreated>(payload_json) {
                    ids.worktree_id = Some(w.worktree_id.as_str().to_string());
                }
            } else if *event_type == PullRequestSynced::EVENT_TYPE {
                if let Ok(p) = serde_json::from_str::<PullRequestSynced>(payload_json) {
                    let repo_id = req
                        .resource_refs
                        .iter()
                        .find(|r| r.resource_type == ResourceType::Repo)
                        .map(|r| r.id.clone());
                    ids.pr_id = repo_id.map(|r| format!("{r}#{}", p.pr_number));
                }
            } else if *event_type == PullRequestMerged::EVENT_TYPE {
                // D9 — same pr_id formula as PullRequestSynced: repo_id (the action's Repo ref) + the
                // payload's pr_number → the `{repo_id}#{pr_number}` row PK the PullRequestProjector folds.
                if let Ok(p) = serde_json::from_str::<PullRequestMerged>(payload_json) {
                    let repo_id = req
                        .resource_refs
                        .iter()
                        .find(|r| r.resource_type == ResourceType::Repo)
                        .map(|r| r.id.clone());
                    ids.pr_id = repo_id.map(|r| format!("{r}#{}", p.pr_number));
                }
            } else if *event_type == ReviewSynced::EVENT_TYPE {
                // review_id is self-contained in the payload (globally unique → the proj_review PK); no
                // sibling Repo-ref needed for the NUDGE (only the projector's repo_id column sibling-reads).
                if let Ok(r) = serde_json::from_str::<ReviewSynced>(payload_json) {
                    ids.review_id = Some(r.review_id.to_string());
                }
            } else if *event_type == ReviewSubmitted::EVENT_TYPE {
                // D10 — same as ReviewSynced: review_id self-contained in the payload (the proj_review PK).
                if let Ok(r) = serde_json::from_str::<ReviewSubmitted>(payload_json) {
                    ids.review_id = Some(r.review_id.to_string());
                }
            }
            deltas_for_event(event_type, &ids)
        }
    }
}

impl Gateway {
    /// `submit_action` (§6.1/AG §8) — run the staged single-action pipeline. ONE atomic gateway txn
    /// (fail-closed): persist the `action_requests` row at `Submitted` + emit `ActionRequested`;
    /// consult the catalog-driven policy (2.2); advance Submitted→PolicyDecided→AwaitingApproval
    /// (R-9 guarded) + open an `approvals` row + emit `ActionApprovalRequested` for a
    /// require_approval / require_step_approval decision. Any event-write / row-write failure rolls
    /// the whole txn back — no row, no approval, no ack, no event (INV-SEC-1 / §15 / §17). **The
    /// requester-supplied `risk_level` is RECORDED for audit but NOT trusted** — it is reconciled to
    /// the §6.3 catalog risk before persist (so an under-claimed risk can never lower scrutiny). The
    /// risk-0 `allow` auto-execute path (no approval) is L3; an uncatalogued type fails closed (deny).
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
        mut req: ActionRequest,
        deltas: &mut Vec<ProjectionDelta>,
    ) -> Result<ActionAck, GatewayError> {
        // 2.2 Q5 + 2.3 L1 — reconcile the recorded risk AND derive the idempotency key from the
        // AUTHORITATIVE §6.3 catalog BEFORE persist, so the audit trail + the dedup key record the
        // TRUE values, not the proposer's claim (§15 recorded-not-trusted). Policy-independent. An
        // uncatalogued type resolves to neither (it is DENIED by the policy below — nothing persists).
        if let Some(entry) = catalog::lookup(&req.action_type) {
            req.risk_level = entry.locked_risk;
            // OVERWRITE any requester-supplied idempotency_key with the catalog-derived one: a proposer
            // must not control the dedup key (a chosen key could force a collision to suppress a
            // victim's action, or evade dedup). `None`-formula actions derive `None` (never deduped).
            req.idempotency_key = idempotency::derive_key(&req, &entry);
        }

        let routed = store.gateway_txn(|gtx| -> Result<Routed, GatewayError> {
            let now = gtx.now_rfc3339();
            let act_id = req.action_request_id.as_str().to_string();

            // 2.3 L1 dedup-on-submit: if a keyed action with this idempotency key already exists,
            // REPLAY the original (at-most-one execution) — no 2nd row, no 2nd ActionRequested, no 2nd
            // execution. The `ux_action_idem` UNIQUE index is the backstop (a racing insert → fail-
            // closed AuditWriteFailed); on the single write-actor this lookup is authoritative.
            if let Some(key) = &req.idempotency_key {
                if let Some((orig_id, orig_status)) =
                    request::find_by_idempotency_key(gtx.tx(), key)?
                {
                    return Ok(Routed::Done(ActionAck {
                        action_request_id: orig_id,
                        status: orig_status,
                    }));
                }
            }

            // 1. persist the intent at Submitted (inputs §15-redacted at rest) + emit ActionRequested
            //    (carrying the reconciled authoritative risk). plan_id = None — single action (2.1c).
            request::insert(gtx, &req, ARStatus::Submitted, None, &now)?;
            gtx.append(&request::action_requested_intent(&req, &now)?)?;

            // 2. the catalog-driven policy decision (2.2): risk resolved from the §6.3 catalog.
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
                        gtx,
                        &appr_id,
                        Some(&act_id),
                        None, // a single action has no parent plan (2.1c)
                        ApprovalStatus::AwaitingApproval,
                        &RequiredApprover::current_user(),
                        Some(&decision), // ②-mini: persist the authoritative PolicyDecision (§15-redacted)
                        None, // no expiry on submit; the expiry path is exercised via approve (2.1c)
                        &now,
                    )?;
                    gtx.append(&approval::approval_requested_intent(&req, &appr_id, &now)?)?;
                    // the queue row was just folded in-band → nudge subscribers (publish post-commit).
                    deltas.push(approval_queue_delta(appr_id.as_str()));

                    Ok(Routed::Done(ActionAck {
                        action_request_id: act_id,
                        status: ARStatus::AwaitingApproval,
                    }))
                }
                // a policy `deny` is a ROUTED outcome. **043 L5 / A1 — record-then-deny:** a blocked
                // AGENT-MUTATION attempt (a deny-rule hit, e.g. `rm -rf /`) is AUDITED, never silently
                // rolled back (audit-integrity — the forensic record of a blocked dangerous attempt).
                // It COMMITS Submitted→PolicyDecided→Denied + an `ActionDenied{approval_id: None,
                // reason}` (no approval object — the policy denied at submit), then the Deny verdict
                // (the agent is blocked regardless). **GATED via `is_adjudication`, which is the
                // agent-mutation set** (the `agent.*` family is the ONLY `ExecutorKind::Adjudication`
                // catalog group — the lead's A1 boundary). A NON-agent / uncatalogued deny keeps the
                // existing fail-closed `Err(PolicyDenied)`-rollback (no behavior change beyond the agent
                // path). (If a future non-agent adjudication type is ever catalogued, re-evaluate this
                // guard — auditing its denial is harmless, but the boundary should stay intentional.)
                // A denial-audit-write fault rolls the whole txn back → `Err` → Deny (fail-closed).
                PolicyDecisionStatus::Deny if is_adjudication(&req.action_type) => {
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
                        ARStatus::Denied,
                    )?;
                    let reason = decision
                        .reasons
                        .first()
                        .map(String::as_str)
                        .unwrap_or("policy denied");
                    gtx.append(&approval::policy_denied_intent(&req, reason, &now)?)?;
                    Ok(Routed::Done(ActionAck {
                        action_request_id: act_id,
                        status: ARStatus::Denied,
                    }))
                }
                // a non-agent / uncatalogued policy `deny`: surface the honest PolicyDenied (→ §6.4
                // policy_denied), never the misleading UnsupportedPolicyDecision. Terminal — nothing
                // executes; the txn rolls back (the existing behavior, unchanged).
                PolicyDecisionStatus::Deny => Err(GatewayError::PolicyDenied),
                // 2.2 INV-SEC-1 — the FIRST no-human-approval execution path: a risk-0 `allow` action
                // auto-executes (submitted → policy_decided → queued → … → succeeded) with NO approval
                // row + NO ActionApprovalRequested. Gated STRICTLY on `allow` AND catalog-risk-0 — the
                // pipeline RE-VERIFIES risk-0 here (defense-in-depth: a policy bug returning `allow`
                // for a non-zero risk must NEVER open the auto-queue; the §14 invariant extends to "no
                // non-zero / non-allow auto-queue").
                PolicyDecisionStatus::Allow => {
                    if req.risk_level != RiskLevel::Level0 {
                        return Err(GatewayError::UnsupportedPolicyDecision(format!(
                            "policy returned `allow` for catalog-risk-{} '{}' — refusing to \
                             auto-queue a non-risk-0 action (INV-SEC-1)",
                            req.risk_level as u8, req.action_type
                        )));
                    }
                    // submitted → policy_decided (the auto-allow decision; NO approval object).
                    request::update_status(
                        gtx.tx(),
                        &act_id,
                        ARStatus::Submitted,
                        ARStatus::PolicyDecided,
                    )?;
                    // 043 adjudication-only: an agent-mutation action TERMINATES at the verdict — the
                    // daemon adjudicated `allow`, the AGENT runs the tool; NO daemon executor. The risk-0
                    // `allow` rests at PolicyDecided (the audit ActionRequested, already appended above,
                    // gates the Allow verdict — §15 #5). It NEVER enters queued→executing (the
                    // `CatalogExecutor` fail-safe Adjudication arm is the defense-in-depth backstop).
                    if is_adjudication(&req.action_type) {
                        return Ok(Routed::Done(ActionAck {
                            action_request_id: act_id,
                            status: ARStatus::PolicyDecided,
                        }));
                    }
                    // policy_decided → queued (the non-adjudication risk-0 auto-execute edge).
                    request::update_status(
                        gtx.tx(),
                        &act_id,
                        ARStatus::PolicyDecided,
                        ARStatus::Queued,
                    )?;
                    // hand the (reconciled) request to the SEPARATE execute step after commit.
                    Ok(Routed::Execute(Box::new(req.clone())))
                }
                // `downgrade` / `needs_more_context` have no MVP trigger → genuinely unrouted;
                // fail-closed (never mis-routed, never a write-actor panic).
                other @ (PolicyDecisionStatus::Downgrade
                | PolicyDecisionStatus::NeedsMoreContext) => Err(
                    GatewayError::UnsupportedPolicyDecision(format!("{other:?}")),
                ),
            }
        })?;

        match routed {
            Routed::Done(ack) => Ok(ack),
            // the risk-0 allow auto-execute path: run the executor + the completion txn (off the
            // decision txn that already committed submitted→policy_decided→queued — LESSON §16).
            // §7.2 read-source: the executor runs off the IN-MEMORY reconciled `req` (raw inputs) —
            // NOT a re-read of the §15-redacted row (a redaction FP must not break a legit
            // execution while the row stays redacted at rest). Pinned by tests/executor.rs #12.
            Routed::Execute(req) => self.execute(store, &req, deltas),
        }
    }

    /// `submit_action_plan` (§6.1/AG §8, O-3) — the bundled-plan intake. ONE atomic gateway txn
    /// (fail-closed — the WHOLE plan persists or nothing): insert the `action_plans` grouping row;
    /// for each step insert its `action_requests` row (`plan_id` set; inputs §15-row-redacted) +
    /// emit `ActionRequested` + advance Submitted→PolicyDecided→AwaitingApproval (R-9 guarded,
    /// reusing the 2.1b helpers); open the approval object(s) per `approval_mode` (Q3) + emit
    /// `ActionApprovalRequested`. Any step failure rolls the whole txn back (no partial plan,
    /// INV-SEC-1/§15/§17). The plan is a durable GROUPING, NOT a new event type (Q7).
    pub fn submit_action_plan(
        &self,
        store: &mut EventStore,
        plan: ActionPlan,
    ) -> Result<PlanAck, GatewayError> {
        self.submit_action_plan_collecting(store, plan, &mut Vec::new())
    }

    /// `submit_action_plan` accumulating the `proj_approval_queue` subscribe-delta(s) into `deltas`
    /// (Q6) — the write-actor publishes them post-commit. Same pipeline as
    /// [`Gateway::submit_action_plan`].
    pub(crate) fn submit_action_plan_collecting(
        &self,
        store: &mut EventStore,
        plan: ActionPlan,
        deltas: &mut Vec<ProjectionDelta>,
    ) -> Result<PlanAck, GatewayError> {
        // fail-closed up front, BEFORE the txn (Step-2.5 Mod 1): `Blocked` is a policy-ASSIGNED
        // outcome (2.2 owns it), never a valid proposer-submitted mode — rejecting it avoids
        // opening phantom `awaiting_approval` steps with no approval object. An empty plan is also
        // rejected (nothing to group / no submitter context to record).
        if plan.approval_mode == ApprovalMode::Blocked {
            return Err(GatewayError::UnsupportedApprovalMode("blocked".to_string()));
        }
        if plan.steps.is_empty() {
            return Err(GatewayError::UnsupportedApprovalMode(
                "a submitted plan must carry at least one step".to_string(),
            ));
        }
        // 2.2 #11 — every step's action_type MUST be in the §6.3 catalog; an uncatalogued step
        // rejects the WHOLE plan fail-closed (PolicyDenied), consistent with the single-action
        // unknown→deny + whole-plan atomicity. NEVER admit an uncatalogued step: it has no
        // authoritative risk (the §11.5 exclusion can't be evaluated on it) and no executor binding,
        // and falling back to the proposer's claimed risk would reopen the claim-0-into-approve-all
        // bypass via the unknown-type door.
        if plan
            .steps
            .iter()
            .any(|s| catalog::lookup(&s.action_request.action_type).is_none())
        {
            return Err(GatewayError::PolicyDenied);
        }
        store.gateway_txn(|gtx| {
            let now = gtx.now_rfc3339();
            let plan_id = plan.plan_id.as_str().to_string();
            // the plan-submit IMMUTABLE context — all steps of a submitted plan share the submitter
            // (recorded on `action_plans` so the projector populates a plan-level queue row without a
            // step cross-join, Step-2.5 Flag 2).
            let first = &plan.steps[0].action_request;
            let project_id = first.project_id.as_ref().map(|p| p.as_str());
            let requester_type_wire = enum_wire(&first.requester_type)?;

            // 1. the action_plans grouping row.
            plan::insert(
                gtx,
                &plan,
                project_id,
                &requester_type_wire,
                &first.requester_id,
                &now,
            )?;

            // 2. each step: persist + ActionRequested + advance to awaiting_approval (R-9 guarded).
            let mut step_acks = Vec::with_capacity(plan.steps.len());
            for step in &plan.steps {
                // 2.2 Q5 — reconcile each step's recorded risk to the AUTHORITATIVE §6.3 catalog risk
                // before persist, so `action_requests.risk_level` (which `load_covered_steps`' SQL
                // reads for the approve-all cascade) + the ActionRequested event carry the TRUE risk,
                // not the proposer's claim. Every step is catalogued (checked above) → lookup is Some.
                let mut req = step.action_request.clone();
                if let Some(entry) = catalog::lookup(&req.action_type) {
                    req.risk_level = entry.locked_risk;
                }
                let act_id = req.action_request_id.as_str().to_string();
                request::insert(gtx, &req, ARStatus::Submitted, Some(&plan_id), &now)?;
                gtx.append(&request::action_requested_intent(&req, &now)?)?;
                // plan steps never auto-execute in 2.2 (no plan-step allow path) — each routes to
                // approval per `approval_mode`; advance submitted → policy_decided → awaiting_approval.
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
                step_acks.push(PlanStepAck {
                    step_id: step.step_id.clone(),
                    action_request_id: act_id,
                    status: ARStatus::AwaitingApproval,
                });
            }

            // 3. open the approval object(s) per approval_mode (Q3) + emit ActionApprovalRequested.
            self.open_plan_approvals(gtx, &plan, &plan_id, &now, deltas)?;

            Ok(PlanAck {
                plan_id,
                steps: step_acks,
            })
        })
    }

    /// Open the approval object(s) for a submitted plan per `approval_mode` (Q3). `StepByStep`/
    /// `Mixed` → one per-step approval each (`Mixed` is a safe over-approximation in 2.1c, marker).
    /// `ApproveAll` → ONE plan-level approval over the NON-critical steps + a SEPARATE per-step
    /// approval for EACH critical (risk-4) step — **critical is never in approve-all** (§6.2/§11.5,
    /// the safety pin). 2.2 keys `critical` off the AUTHORITATIVE §6.3 catalog risk (not the
    /// requester-supplied `risk_level`), so an under-claimed catalog-risk-4 step is still excluded.
    fn open_plan_approvals(
        &self,
        gtx: &GatewayTxn,
        plan: &ActionPlan,
        plan_id: &str,
        now: &str,
        deltas: &mut Vec<ProjectionDelta>,
    ) -> Result<(), GatewayError> {
        match plan.approval_mode {
            ApprovalMode::StepByStep | ApprovalMode::Mixed => {
                for step in &plan.steps {
                    self.open_step_approval(gtx, &step.action_request, plan_id, now, deltas)?;
                }
            }
            ApprovalMode::ApproveAll => {
                // §11.5 + P4.0b-ui1 — a step is NON-standing-grantable (excluded from approve-all) if
                // its AUTHORITATIVE §6.3 catalog entry is risk-4 (critical) OR `!standing_grant_eligible`
                // (the destructive `git.discard_hunk`). BOTH disjuncts (defense-in-depth; the invariant
                // test pins risk-4⇒!eligible so they can't drift). Keys off the catalog, NOT the
                // requester-supplied `risk_level` (recorded, not trusted — an under-claim still excludes).
                // Every step is catalogued (rejected upfront otherwise), so lookup is Some.
                let is_non_standing_grantable = |s: &nexusops_shared::actions::ActionPlanStep| {
                    catalog::lookup(&s.action_request.action_type)
                        .map(|e| e.locked_risk == RiskLevel::Level4 || !e.standing_grant_eligible)
                        == Some(true)
                };
                // ONE plan-level approval over the standing-grant-eligible steps (opened only if any
                // exist — an all-excluded plan gets only per-step approvals, no empty plan-level one).
                if plan.steps.iter().any(|s| !is_non_standing_grantable(s)) {
                    let first = &plan.steps[0].action_request;
                    let appr_id = ApprovalId::new();
                    approval::insert(
                        gtx,
                        &appr_id,
                        None, // plan-level: no single action (scope=Plan)
                        Some(plan_id),
                        ApprovalStatus::AwaitingApproval,
                        &RequiredApprover::current_user(),
                        None, // ②-mini: plan-level policy_decision = NULL (Q2; sourcing is a follow-on)
                        None,
                        now,
                    )?;
                    gtx.append(&approval::plan_approval_requested_intent(
                        plan_id,
                        &appr_id,
                        first.requester_type,
                        &first.requester_id,
                        first.project_id.as_ref(),
                        now,
                    )?)?;
                    deltas.push(approval_queue_delta(appr_id.as_str()));
                }
                // each non-standing-grantable step (risk-4 OR !standing_grant_eligible) gets its OWN
                // per-step approval — never folded into approve-all.
                for step in plan.steps.iter().filter(|s| is_non_standing_grantable(s)) {
                    self.open_step_approval(gtx, &step.action_request, plan_id, now, deltas)?;
                }
            }
            // unreachable: Blocked is rejected before the txn (Mod 1). Fail closed if ever reached.
            ApprovalMode::Blocked => {
                return Err(GatewayError::UnsupportedApprovalMode("blocked".to_string()))
            }
        }
        Ok(())
    }

    /// Open ONE per-step approval (a single step of a plan): an `approvals` row tying the step's
    /// `action_request_id` + the `plan_id` + emit a per-step `ActionApprovalRequested`.
    fn open_step_approval(
        &self,
        gtx: &GatewayTxn,
        req: &ActionRequest,
        plan_id: &str,
        now: &str,
        deltas: &mut Vec<ProjectionDelta>,
    ) -> Result<(), GatewayError> {
        let appr_id = ApprovalId::new();
        approval::insert(
            gtx,
            &appr_id,
            Some(req.action_request_id.as_str()),
            Some(plan_id),
            ApprovalStatus::AwaitingApproval,
            &RequiredApprover::current_user(),
            None, // ②-mini: per-step (StepByStep) policy_decision = NULL (Q2 follow-on, like plan-level)
            None, // expires_at
            now,
        )?;
        gtx.append(&approval::approval_requested_intent(req, &appr_id, now)?)?;
        deltas.push(approval_queue_delta(appr_id.as_str()));
        Ok(())
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

    /// `approve` accumulating the `proj_approval_queue` subscribe-delta(s) into `deltas` (Q6).
    /// Dispatches on the approval shape (Q3): a per-step / single-action approval (action_request_id
    /// set) runs the 2.1b single path; a plan-level approve-all approval (action_request_id NULL,
    /// plan_id set) CASCADES over the plan's non-critical steps (§6.2/§11.5 — critical is never
    /// cascaded). Same `step_id?` §6.1 surface (accepted at the IPC boundary; 2.1c resolves the whole
    /// approval, so step-targeting within a plan-level approval is a later refinement).
    pub(crate) fn approve_collecting(
        &self,
        store: &mut EventStore,
        approval_id: &str,
        deltas: &mut Vec<ProjectionDelta>,
    ) -> Result<ActionAck, GatewayError> {
        match self.approval_target(store, approval_id)? {
            ApprovalTarget::Single => self.approve_single(store, approval_id, deltas),
            ApprovalTarget::Plan(plan_id) => {
                self.approve_plan_cascade(store, approval_id, &plan_id, deltas)
            }
        }
    }

    /// Peek an approval's shape (a read-only txn) to dispatch single-action vs plan-level cascade.
    /// (A second small load in the chosen path is acceptable for 2.1c — preview's read-txn precedent;
    /// a fold-into-one optimization is a Carry-forward note.)
    fn approval_target(
        &self,
        store: &mut EventStore,
        approval_id: &str,
    ) -> Result<ApprovalTarget, GatewayError> {
        store.gateway_txn(|gtx| -> Result<ApprovalTarget, GatewayError> {
            let appr = approval::load(gtx.tx(), approval_id)?;
            match (appr.action_request_id.is_some(), appr.plan_id) {
                (true, _) => Ok(ApprovalTarget::Single),
                (false, Some(plan_id)) => Ok(ApprovalTarget::Plan(plan_id)),
                (false, None) => Err(GatewayError::NotFound(format!(
                    "approval {approval_id} ties to neither an action nor a plan"
                ))),
            }
        })
    }

    /// `approve` a single-action / per-step approval (the 2.1b path): the decision txn emits
    /// ActionApproved and transitions AwaitingApproval→Approved→Queued (R-9 guarded), then — if not
    /// expired — the execute step runs. An approval past its `expires_at` → `ActionExpired`, never
    /// executes (§17).
    fn approve_single(
        &self,
        store: &mut EventStore,
        approval_id: &str,
        deltas: &mut Vec<ProjectionDelta>,
    ) -> Result<ActionAck, GatewayError> {
        // --- decision txn: returns (the loaded action, the ApproveOutcome) ---
        let (req, outcome) = store.gateway_txn(
            |gtx| -> Result<(ActionRequest, ApproveOutcome), GatewayError> {
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
                // dispatched here only for a single/per-step approval → action_request_id is Some.
                let act_id = appr.action_request_id.ok_or_else(|| {
                    GatewayError::NotFound(format!(
                        "approval {approval_id} has no action_request_id"
                    ))
                })?;
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
                    return Ok((req, ApproveOutcome::Expired));
                }

                // approve: stamp the decision, transition both machines, emit ActionApproved.
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
                deltas.push(approval_queue_delta(approval_id)); // queue row → approved
                                                                // 043 adjudication-only: an agent-mutation action TERMINATES at Approved (the verdict) —
                                                                // the daemon adjudicated `approve`, the AGENT runs the tool; NO daemon executor, NO
                                                                // Approved→Queued. The ActionApproved (committed above) gates the Allow verdict (§15 #5).
                if is_adjudication(&req.action_type) {
                    return Ok((req, ApproveOutcome::Adjudicated));
                }
                request::update_status(gtx.tx(), &act_id, ARStatus::Approved, ARStatus::Queued)?;
                Ok((req, ApproveOutcome::Queued))
            },
        )?;

        match outcome {
            ApproveOutcome::Expired => {
                return Ok(ActionAck {
                    action_request_id: req.action_request_id.as_str().to_string(),
                    status: ARStatus::Expired,
                })
            }
            // an adjudication-only action rests at Approved (the Allow verdict) — NO daemon executor.
            ApproveOutcome::Adjudicated => {
                return Ok(ActionAck {
                    action_request_id: req.action_request_id.as_str().to_string(),
                    status: ARStatus::Approved,
                })
            }
            ApproveOutcome::Queued => {}
        }
        // --- execute step (structurally separate; the executor runs OFF the txn) ---
        // NOTE (Q6 corollary, →2.4): the decision txn above ALREADY committed the queue-row status
        // change (awaiting_approval→approved) + pushed its delta into `deltas`. If `execute`'s
        // completion txn then errors, `approve_single` returns Err, so the write-actor's
        // `publish_after_commit` (result.is_ok() gate) SUPPRESSES that already-durable delta — the
        // subscriber is briefly stale until its next reconnect/`get_projection` (the lag-resync
        // policy covers it; the stub executor never fails, so it's unreached in 2.1c). 2.4 reconciles
        // the same two-txn boundary for crash recovery; the missed-nudge folds in there.
        // §7.2 read-source: `req` here is the DURABLE row (`request::load` in the decision txn above) —
        // §7.2 deems the row canonical for execution (the in-memory inputs are gone at approve-time,
        // possibly post-restart). It carries the §15-redacted inputs; the real-input-fidelity concern
        // (a redaction FP breaking a legit real execution) is owned when real executors land (later
        // phases). Pinned by tests/executor.rs #13.
        self.execute(store, &req, deltas)
    }

    /// `approve` a PLAN-LEVEL approve-all approval (O-3 cascade) — drive EVERY non-critical step of
    /// the plan (critical/risk-4 is never cascaded; it keeps its own per-step approval). The decision
    /// txn marks the plan-level approval Approved + queues every covered step (each emitting its own
    /// `ActionApproved` tied to that step's action_request_id + the SHARED plan approval_id); then
    /// each step's executor runs (stub, off-txn) + its completion txn. An expired plan-level approval
    /// → `ActionExpired` per covered step, none execute (§17). The decision is one atomic txn.
    fn approve_plan_cascade(
        &self,
        store: &mut EventStore,
        approval_id: &str,
        plan_id: &str,
        deltas: &mut Vec<ProjectionDelta>,
    ) -> Result<ActionAck, GatewayError> {
        let (steps, expired) =
            store.gateway_txn(|gtx| -> Result<(Vec<ActionRequest>, bool), GatewayError> {
                let now = gtx.now_rfc3339();
                let appr = approval::load(gtx.tx(), approval_id)?;
                if appr.status != ApprovalStatus::AwaitingApproval {
                    return Err(GatewayError::IllegalTransition {
                        machine: "Approval",
                        from: format!("{:?}", appr.status),
                        to: "decided".to_string(),
                    });
                }
                // the covered steps: the plan's NON-critical steps still awaiting (criticals carry
                // their own per-step approvals, untouched by the plan-level approve — the safety pin).
                // INVARIANT (2.1c): a plan-level approval is opened (L2) ONLY when ≥1 non-critical
                // step exists, and on the single write-actor those steps are still `awaiting_approval`
                // at cascade time → `reqs` is non-empty. If 2.3+ widens the filter or a step can be
                // cancelled mid-plan, an empty cascade would vacuously mark the approval Approved with
                // no step events — guard it then (Step-9 flag).
                let reqs = load_covered_steps(gtx, plan_id)?;

                // §17 expiry on the plan-level approval → expire every covered step, none execute.
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
                    for req in &reqs {
                        request::update_status(
                            gtx.tx(),
                            req.action_request_id.as_str(),
                            ARStatus::AwaitingApproval,
                            ARStatus::Expired,
                        )?;
                        gtx.append(&approval::expired_intent(req, approval_id, &now)?)?;
                    }
                    deltas.push(approval_queue_delta(approval_id));
                    return Ok((reqs, true));
                }

                // approve: mark the plan-level approval Approved + queue every covered step.
                approval::update_status(
                    gtx.tx(),
                    approval_id,
                    ApprovalStatus::AwaitingApproval,
                    ApprovalStatus::Approved,
                )?;
                approval::record_decision(gtx.tx(), approval_id, Some(STUB_DECIDER), &now)?;
                for req in &reqs {
                    let ar = req.action_request_id.as_str();
                    request::update_status(
                        gtx.tx(),
                        ar,
                        ARStatus::AwaitingApproval,
                        ARStatus::Approved,
                    )?;
                    gtx.append(&approval::approved_intent(
                        req,
                        approval_id,
                        Some(STUB_DECIDER),
                        &now,
                    )?)?;
                    request::update_status(gtx.tx(), ar, ARStatus::Approved, ARStatus::Queued)?;
                }
                deltas.push(approval_queue_delta(approval_id));
                Ok((reqs, false))
            })?;

        if expired {
            return Ok(ActionAck {
                action_request_id: approval_id.to_string(),
                status: ARStatus::Expired,
            });
        }
        // execute each covered step (stub; off-txn) → ActionStarted + ActionSucceeded per step.
        // NOTE (cascade crash-recovery, →2.4): the decision txn above atomically queued ALL covered
        // steps; this loop then runs each step's executor + completion txn SEPARATELY. A mid-loop
        // failure (or a crash) strands the *remaining* steps at `queued` — the MULTI-STEP form of the
        // single-action two-txn gap documented on `execute` below, and materially worse (N orphaned
        // rows, not 1). Benign in 2.1c (the stub executor never fails); 2.4's orphaned-`queued`
        // reconciliation by idempotency key MUST cover the cascade case, not just the single step.
        for req in &steps {
            self.execute(store, req, deltas)?;
        }
        // the ack reflects the cascade: `action_request_id` carries the plan-level approval_id (a
        // plan approve has no single action — Step-9 field-reuse note), status the overall outcome.
        Ok(ActionAck {
            action_request_id: approval_id.to_string(),
            status: ARStatus::Succeeded,
        })
    }

    /// Record a §17 fencing conflict as a terminal `ActionFailed(FencingConflict)` (2.4 L3) — the
    /// durable audit + the §11.5 hard-conflict surface. Its own gateway txn so it **COMMITS** before the
    /// caller propagates `Err(GatewayError::FencingConflict)` — the recorded `failed` state must survive
    /// the Err (never let the Err roll back the audit record). The action is at `executing` (txn-A) when
    /// this runs → the `Executing → Failed` edge is legal.
    fn record_fencing_conflict(
        &self,
        store: &mut EventStore,
        req: &ActionRequest,
    ) -> Result<(), GatewayError> {
        let act_id = req.action_request_id.as_str().to_string();
        store.gateway_txn(|gtx| -> Result<(), GatewayError> {
            let now = gtx.now_rfc3339();
            request::update_status(gtx.tx(), &act_id, ARStatus::Executing, ARStatus::Failed)?;
            gtx.append(&request::failed_intent(
                req,
                ActionError::FencingConflict,
                &now,
            )?)?;
            Ok(())
        })
    }

    /// Record a §6.2 stale-precondition as a terminal `ActionFailed(StalePrecondition)` (2.4 L4) — and
    /// REGENERATE the preview FIRST (before the fail transition) so the failed action's `preview_json`
    /// reflects the CHANGED live state (the audit/UI shows what changed for the human's fresh decision).
    /// Its own gateway txn so it **COMMITS** before the caller propagates `Err(StalePrecondition)` — the
    /// recorded `failed` state + the regenerated preview must survive the Err. The preview regen is
    /// STRUCTURAL-only in 2.4 (the real live-source diff-regen lands with real previews, Phase 5/7). The
    /// action is at `executing` (txn-A) → the `Executing → Failed` edge is legal.
    fn record_stale_precondition(
        &self,
        store: &mut EventStore,
        req: &ActionRequest,
    ) -> Result<(), GatewayError> {
        let act_id = req.action_request_id.as_str().to_string();
        store.gateway_txn(|gtx| -> Result<(), GatewayError> {
            let now = gtx.now_rfc3339();
            // regenerate the preview FIRST + persist through the §15 redaction gate (rule #3). The
            // action was submitted+approved so it is catalogued; a (defensive) uncatalogued lookup skips
            // the regen and still records the failure.
            if let Some(entry) = catalog::lookup(&req.action_type) {
                let generated_at =
                    Timestamp::parse(&now).map_err(|e| GatewayError::Serialize(e.to_string()))?;
                let preview = preview::generate_preview(req, &entry, generated_at);
                let preview_json = gtx.redact_row(
                    &serde_json::to_string(&preview)
                        .map_err(|e| GatewayError::Serialize(e.to_string()))?,
                )?;
                request::update_preview(gtx.tx(), &act_id, &preview_json)?;
            }
            // terminal: executing → failed + ActionFailed(StalePrecondition).
            request::update_status(gtx.tx(), &act_id, ARStatus::Executing, ARStatus::Failed)?;
            gtx.append(&request::failed_intent(
                req,
                ActionError::StalePrecondition,
                &now,
            )?)?;
            Ok(())
        })
    }

    /// Run the executor for an approved+queued action, then record the outcome. The executor is
    /// invoked OUTSIDE any write-actor txn (2.3's git-CLI/octocrab executors move off-thread here).
    ///
    /// **2.4 — the completion is SPLIT into two txns** so a fail-closed audit-write leaves the action
    /// durably `executing` (reconciled on restart by L5), never stuck pre-execution:
    /// **txn-A** `queued → executing` + `ActionStarted` (COMMITS) → the executor runs (off the
    /// write-actor — so `ActionStarted` precedes the side effect, §17.1) → **txn-B** `executing →
    /// succeeded/failed` + the terminal event. A terminal-event write failure (§15/§17) rolls txn-B
    /// back → the action STAYS `executing` → L5; never acked succeeded. If a real side effect was
    /// applied but its event is unwritable → `ActionPartiallySucceeded` best-effort (§17 / L2).
    fn execute(
        &self,
        store: &mut EventStore,
        req: &ActionRequest,
        deltas: &mut Vec<ProjectionDelta>,
    ) -> Result<ActionAck, GatewayError> {
        let act_id = req.action_request_id.as_str().to_string();

        // §14 crash sim (cfg-gated, test-only — compiled out of release): abort BEFORE the
        // executing-commit → leaves a `queued` orphan for L5's crash-reconcile.
        #[cfg(feature = "fault-injection")]
        if crate::fault::take_if(crate::fault::FaultPoint::BeforeExecutingTxn) {
            return Err(GatewayError::Serialize(
                "injected §14 crash (BeforeExecutingTxn)".to_string(),
            ));
        }

        // txn-A: queued → executing + ActionStarted. Commits FIRST so the action is durably `executing`
        // before the side effect / terminal write — a crash or a fail-closed terminal write then leaves
        // a reconcilable `executing` orphan (L5), not a lost pre-execution slot. (`WHERE status=queued`
        // makes a stale slot a typed NotFound, never a silent double-execute.)
        store.gateway_txn(|gtx| -> Result<(), GatewayError> {
            let now = gtx.now_rfc3339();
            request::update_status(gtx.tx(), &act_id, ARStatus::Queued, ARStatus::Executing)?;
            gtx.append(&request::started_intent(req, &now)?)?;
            Ok(())
        })?;

        // L3 — fencing (§17 / safety rule #6, 1.4 Option-B). A resource-bearing action holds a
        // `resource_mutation` lease over its primary resource (owner = the action). Acquire the lock →
        // bind the minted token (L5 re-derives authority from it) → re-check `validate_held` AFTER lock
        // + BEFORE execute. A lease NOT live for THIS action (held by another owner, OR a stale bound
        // token) → ActionFailed(fencing_conflict): recorded (COMMITS) THEN refused with the typed Err
        // (never auto-resolved — rule #6). Risk-0 / no-resource actions hold NO lease → skipped.
        // (Future-TODO: PRIMARY resource_ref ONLY — a multi-resource action leaves `resource_refs[1..]`
        //  unfenced; fence ALL refs when multi-resource actions land. The MVP catalog is ~single-resource.)
        if let Some(rref) = req.resource_refs.first() {
            let resource_id = ResourceId(rref.id.clone());
            let owner = OwnerId(act_id.clone());
            let kind = LeaseKind::resource_mutation();
            let token = match store.gw_acquire_lease(&resource_id, &kind, &owner, FENCE_TTL_SECS) {
                Ok(lease) => {
                    let bound = i64::try_from(lease.fencing_token.0).map_err(|_| {
                        GatewayError::Serialize("fencing token overflow".to_string())
                    })?;
                    store
                        .gateway_txn(|gtx| request::bind_fencing_token(gtx.tx(), &act_id, bound))?;
                    lease.fencing_token
                }
                // self-Held (a re-execution of our OWN live lease — the L5 path) → idempotent renew-like:
                // the action's bound token (carried on the durable-row `req`) is the authority, re-checked
                // by validate_held below. A self-Held with NO valid recorded token (a crash between txn-A
                // and the bind → NULL/invalid column) is an inconsistent state we cannot prove authority
                // for, and we CANNOT re-acquire (the slot is live-held) → **fail closed (fence) EXPLICITLY**,
                // never proceed with a guessed/zero token. 2.4's single-execute never hits self-Held; an L5
                // re-execute loads `req` from the row with the bound token.
                Err(LeaseError::Held { owner_id }) if owner_id == act_id => {
                    match req
                        .fencing_token
                        .and_then(|t| u64::try_from(t).ok())
                        .filter(|&t| t > 0)
                    {
                        Some(t) => FencingToken(t),
                        None => {
                            self.record_fencing_conflict(store, req)?;
                            return Err(GatewayError::FencingConflict);
                        }
                    }
                }
                // a LIVE lease held by ANOTHER owner → superseded → fencing conflict.
                Err(LeaseError::Held { .. }) => {
                    self.record_fencing_conflict(store, req)?;
                    return Err(GatewayError::FencingConflict);
                }
                Err(e) => return Err(lease_err(e)),
            };
            // §6.2/§17: validate_held AFTER lock + BEFORE execute → false (expired/superseded) → fence.
            if !store
                .gw_validate_lease(&resource_id, &kind, &owner, token)
                .map_err(lease_err)?
            {
                self.record_fencing_conflict(store, req)?;
                return Err(GatewayError::FencingConflict);
            }
        }

        // L4 — stale-precondition re-check (§6.2 AG-16.4 / §7.2). AFTER lock+fencing (L3), BEFORE
        // execute: re-read the action's live source; if it CHANGED since preview/approval, the approved
        // mutation is stale → do NOT execute a DIFFERENT mutation than was approved. Regenerate the
        // preview (the changed state) + record ActionFailed(stale_precondition) (COMMITS) + refuse with
        // the typed Err → a fresh approval cycle (re-submit). 2.4 fails on ANY change (conservative MVP;
        // the finer §6.2 "only if the previewed diff/resource changed" needs real preview-diffing,
        // Phase 5/7). The Null production oracle reports Unchanged → this is a no-op until the real
        // per-action_type live-source re-read lands.
        if self.precondition().recheck(req) == PreconditionStatus::Changed {
            self.record_stale_precondition(store, req)?;
            return Err(GatewayError::StalePrecondition);
        }

        // §14 crash sim (cfg-gated, test-only — compiled out of release): abort AFTER the
        // executing-commit (txn-A) + BEFORE the terminal txn → leaves an `executing` orphan for L5.
        #[cfg(feature = "fault-injection")]
        if crate::fault::take_if(crate::fault::FaultPoint::BeforeTerminalTxn) {
            return Err(GatewayError::Serialize(
                "injected §14 crash (BeforeTerminalTxn)".to_string(),
            ));
        }

        // the side effect (2.4 stubs: NONE) runs BETWEEN txn-A and txn-B (off the write-actor, §16).
        let outcome = self.executor().execute(req);

        // txn-B: executing → succeeded/failed + the terminal event. A fail-closed audit-write here
        // (§15/§17 — the terminal event can't be written) rolls txn-B back → the action STAYS
        // `executing` (txn-A committed) → L5 reconciles; NEVER acked succeeded.
        // (Two-txn delta corollary, 2.1c→2.4: on a txn-B failure the post-commit publish is Err-gated →
        // the outcome delta is suppressed while the row is durably `executing`; a subscriber sees
        // `executing` until lag-resync — benign, the durable row is the source of truth.)
        let terminal = store.gateway_txn(|gtx| -> Result<ActionAck, GatewayError> {
            let now = gtx.now_rfc3339();
            match &outcome {
                // the executor's changed_resources/detail (Q7) are daemon-internal — recorded on the
                // §6.2 ActionResult in a later phase; the ActionSucceeded event payload stays empty.
                ExecutionOutcome::Succeeded { emitted_events, .. } => {
                    request::update_status(
                        gtx.tx(),
                        &act_id,
                        ARStatus::Executing,
                        ARStatus::Succeeded,
                    )?;
                    gtx.append(&request::succeeded_intent(req, &now)?)?;
                    // PIN a/b (P4.0b-1) — append the executor's additional in-txn events (the
                    // session.create `SessionStarted`, carrying the §15 #8 `execution_profile_id`)
                    // ATOMIC with `ActionSucceeded`: a write failure `?`-propagates → txn-B rolls back →
                    // the action stays `executing` → L5 (INV-SEC-1 — never an unaudited session record).
                    for ev in emitted_events {
                        gtx.append(&request::emitted_event_intent(req, ev, &now)?)?;
                    }
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
                    // the executor's free-string failure is the ExecutorError variant of the structured
                    // taxonomy (the §17-specific variants land at L3-L5). No info lost.
                    gtx.append(&request::failed_intent(
                        req,
                        ActionError::ExecutorError {
                            message: err.clone(),
                        },
                        &now,
                    )?)?;
                    Ok(ActionAck {
                        action_request_id: act_id.clone(),
                        status: ARStatus::Failed,
                    })
                }
                // P7.1 (edges-023) — a FAILED action that ALSO emits structured observation events (a
                // TERMINAL non-auth `*SyncFailed`). Record `ActionFailed` (the terminal event — the action
                // IS a failure) AND append the emitted_events in the SAME txn-B, ATOMIC: a §17 sync-failure
                // record is durable iff the failure is. A write failure `?`-propagates → txn-B rolls back →
                // the action stays `executing` → L5 (and `side_effect_applied()` is false for this variant
                // → the outer Err arm rolls back cleanly, never `ActionPartiallySucceeded`).
                ExecutionOutcome::FailedWithEvents {
                    detail,
                    emitted_events,
                } => {
                    request::update_status(
                        gtx.tx(),
                        &act_id,
                        ARStatus::Executing,
                        ARStatus::Failed,
                    )?;
                    gtx.append(&request::failed_intent(
                        req,
                        ActionError::ExecutorError {
                            message: detail.clone(),
                        },
                        &now,
                    )?)?;
                    for ev in emitted_events {
                        gtx.append(&request::emitted_event_intent(req, ev, &now)?)?;
                    }
                    Ok(ActionAck {
                        action_request_id: act_id.clone(),
                        status: ARStatus::Failed,
                    })
                }
            }
        });

        match terminal {
            Ok(ack) => {
                // D4b — post-commit / fire-and-forget: nudge the projections the COMMITTED emitted events
                // fed, via the SHARED `deltas_for_event` mapping (the §51 one-list discipline, identical to
                // the Command::Append path). Built AFTER the txn-B closure committed → a parse/build here can
                // NEVER affect the fail-closed commit; published by `publish_after_commit` (result.is_ok()-
                // gated, double-gating with the commit). The FINDING fix: production SessionStarted (gateway
                // emitted, NOT Command::Append) now nudges Session+ProjectActivity+ProjectGraph.
                let emitted: &[EmittedEvent] = match &outcome {
                    ExecutionOutcome::Succeeded { emitted_events, .. }
                    | ExecutionOutcome::FailedWithEvents { emitted_events, .. } => emitted_events,
                    ExecutionOutcome::Failed(_) => &[],
                };
                for ev in emitted {
                    deltas.extend(emitted_event_deltas(req, ev));
                }
                Ok(ack)
            }
            // txn-B could not write the terminal event (§15/§17 fail-closed). The next step depends on
            // whether a DURABLE side effect was applied that can no longer be un-done:
            Err(audit_err) => {
                if !outcome.side_effect_applied() {
                    // no durable change was applied — a side-effect-free `Succeeded` (every 2.4 stub) or
                    // any `Failed` (which by definition applied nothing) → clean rollback → the action
                    // stays `executing` → L5 reconciles. Fail closed (never acked succeeded).
                    return Err(audit_err);
                }
                // a real side effect WAS applied but its success event is lost → record the divergence
                // BEST-EFFORT in its own txn-C: `ActionPartiallySucceeded` (the loud, consumer-visible
                // audit-integrity record). If txn-C ALSO can't write (audit fully broken) → return the
                // original AuditWriteFailed + stay `executing` (→ L5): NEVER claim a partial-success the
                // audit log does not hold (the ultimate fail-closed).
                store
                    .gateway_txn(|gtx| -> Result<ActionAck, GatewayError> {
                        let now = gtx.now_rfc3339();
                        request::update_status(
                            gtx.tx(),
                            &act_id,
                            ARStatus::Executing,
                            ARStatus::PartiallySucceeded,
                        )?;
                        gtx.append(&request::partially_succeeded_intent(req, &now)?)?;
                        Ok(ActionAck {
                            action_request_id: act_id.clone(),
                            status: ARStatus::PartiallySucceeded,
                        })
                    })
                    .map_err(|_txn_c_failed| audit_err)
            }
        }
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

    /// `deny` accumulating the `proj_approval_queue` subscribe-delta into `deltas` (Q6). Dispatches
    /// (Q3): a per-step / single-action approval denies its one action; a plan-level approve-all
    /// approval denies EVERY covered non-critical step (each terminal; the executor never runs).
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
            // stamp the plan/single-level approval Denied (terminal); the executor is NEVER invoked.
            approval::update_status(
                gtx.tx(),
                approval_id,
                ApprovalStatus::AwaitingApproval,
                ApprovalStatus::Denied,
            )?;
            approval::record_decision(gtx.tx(), approval_id, Some(STUB_DECIDER), &now)?;

            // the actions to deny: the single action (per-step/single approval) OR every covered
            // non-critical step (a plan-level approve-all approval). The ack's `action_request_id`
            // carries the single action's id (2.1b behavior preserved) or the plan-level approval_id.
            let (reqs, ack_id) = match (&appr.action_request_id, &appr.plan_id) {
                (Some(act_id), _) => (vec![request::load(gtx.tx(), act_id)?], act_id.clone()),
                (None, Some(plan_id)) => {
                    (load_covered_steps(gtx, plan_id)?, approval_id.to_string())
                }
                (None, None) => {
                    return Err(GatewayError::NotFound(format!(
                        "approval {approval_id} ties to neither an action nor a plan"
                    )))
                }
            };
            for req in &reqs {
                request::update_status(
                    gtx.tx(),
                    req.action_request_id.as_str(),
                    ARStatus::AwaitingApproval,
                    ARStatus::Denied,
                )?;
                gtx.append(&approval::denied_intent(req, approval_id, reason, &now)?)?;
            }
            deltas.push(approval_queue_delta(approval_id)); // queue row → denied
            Ok(ActionAck {
                action_request_id: ack_id,
                status: ARStatus::Denied,
            })
        })
    }

    /// `preview_action` (§6.1) — a dry-run preview of an action (§6.2/§6.3). Renders the typed preview
    /// by dispatching on the catalog `preview_class` ([`preview::generate_preview`]) and PERSISTS it to
    /// `action_requests.preview_json` (the §7.2 durable preview source) through the §15 redaction gate
    /// (rule #3). 2.3 has no real namespace adapter, so every preview is structural-only — it sets
    /// `cannot_preview_reason` + escalates the PREVIEW risk (envelope-only; the policy risk is
    /// untouched). Runs in a `gateway_txn` (it now writes `preview_json`).
    pub fn preview_action(
        &self,
        store: &mut EventStore,
        action_request_id: &str,
    ) -> Result<ActionPreview, GatewayError> {
        store.gateway_txn(|gtx| -> Result<ActionPreview, GatewayError> {
            let now = gtx.now_rfc3339();
            let req = request::load(gtx.tx(), action_request_id)?;
            // the §6.3 catalog entry drives the preview class. An existing row is always catalogued
            // (submit denies uncatalogued) → defensive fail-closed if somehow absent.
            let entry = catalog::lookup(&req.action_type).ok_or_else(|| {
                GatewayError::NotFound(format!(
                    "action_type '{}' is not in the §6.3 catalog (preview)",
                    req.action_type
                ))
            })?;
            let generated_at =
                Timestamp::parse(&now).map_err(|e| GatewayError::Serialize(e.to_string()))?;
            let preview = preview::generate_preview(&req, &entry, generated_at);
            // persist preview_json through the §15 redaction gate (rule #3 — every persisted payload
            // through the Redactor; a no-op today since the preview derives from the already-redacted
            // row, but uniform defense-in-depth for non-row-derived preview content later).
            let preview_json = gtx.redact_row(
                &serde_json::to_string(&preview)
                    .map_err(|e| GatewayError::Serialize(e.to_string()))?,
            )?;
            request::update_preview(gtx.tx(), action_request_id, &preview_json)?;
            Ok(preview)
        })
    }
}

/// Load the plan's steps covered by a plan-level approve-all approval: the NON-critical
/// (risk != 4) steps still `awaiting_approval`. Criticals carry their own per-step approvals and
/// are NEVER cascaded (§6.2/§11.5 — the safety pin). The `risk_level <> 4` reads the PERSISTED risk,
/// which 2.2 reconciled to the AUTHORITATIVE §6.3 catalog risk at submit — so a proposer who
/// under-claims a catalog-risk-4 step as risk-0 still can't slip it into the cascade. Reads via the
/// open gateway txn.
fn load_covered_steps(gtx: &GatewayTxn, plan_id: &str) -> Result<Vec<ActionRequest>, GatewayError> {
    let ids: Vec<String> = {
        let mut stmt = gtx
            .tx()
            .prepare(
                "SELECT action_request_id FROM action_requests \
                 WHERE plan_id = ?1 AND risk_level <> 4 AND status = 'awaiting_approval' \
                 ORDER BY action_request_id",
            )
            .map_err(db_err)?;
        let rows = stmt
            .query_map([plan_id], |r| r.get::<_, String>(0))
            .map_err(db_err)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(db_err)?);
        }
        out
    };
    ids.iter().map(|id| request::load(gtx.tx(), id)).collect()
}
