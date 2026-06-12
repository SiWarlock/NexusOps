//! ActionPlan durable grouping (§6.2, O-3) — the thin `action_plans` row helper.
//!
//! A bundled `ActionPlan` is a GROUPING over `action_requests` (each step is a full `ActionRequest`
//! with `plan_id` set, emitting its own `Action*` events), NOT a new event type (Q7). `action_plans`
//! carries the plan metadata + the plan-submit IMMUTABLE context (`project_id`/`requester_*`,
//! derived from the steps — all share the submitter) so the `proj_approval_queue` projector can
//! populate a plan-level approval row WITHOUT a step cross-join (Step-2.5 Flag 2). Inserted inside
//! the ONE atomic plan gateway txn (fail-closed — the whole plan persists or nothing).

use nexusops_shared::actions::ActionPlan;

use crate::eventstore::GatewayTxn;
use crate::gateway::{db_err, enum_int, enum_wire, GatewayError};

/// INSERT the `action_plans` grouping row. `project_id`/`requester_*` are the plan-submit context
/// (taken by the caller from the plan's first step — all steps of a submitted plan share the
/// submitter; an empty plan is rejected before this runs). Called inside the plan gateway txn.
pub(crate) fn insert(
    gtx: &GatewayTxn,
    plan: &ActionPlan,
    project_id: Option<&str>,
    requester_type: &str,
    requester_id: &str,
    created_at: &str,
) -> Result<(), GatewayError> {
    gtx.tx()
        .execute(
            "INSERT INTO action_plans \
             (plan_id, project_id, requester_type, requester_id, title, overall_risk, \
              approval_mode, created_at) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            rusqlite::params![
                plan.plan_id.as_str(),
                project_id,
                requester_type,
                requester_id,
                plan.title,
                enum_int(&plan.overall_risk)?,
                enum_wire(&plan.approval_mode)?,
                created_at,
            ],
        )
        .map_err(db_err)?;
    Ok(())
}
