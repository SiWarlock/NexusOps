//! `proj_approval_queue` projector (§7 / §2.3; AG §8 / §14.3) — the Human Input Queue read model.
//!
//! Folds the Action Gateway's approval-lifecycle events into `proj_approval_queue`. The row's
//! MUTABLE `status` is derived from the EVENT TYPE alone (`ActionApprovalRequested`→
//! `awaiting_approval`, `ActionApproved`→`approved`, `ActionDenied`→`denied`, `ActionExpired`→
//! `expired`); the IMMUTABLE fields (`risk_level`, `requester_type`/`requester_id`, `project_id`,
//! `expires_at`) are **sibling-read** from the `action_requests`/`approvals` registry rows in the
//! SAME txn (rebuild-safe — those never change post-submit; the `graph`-reads-`object_refs`
//! precedent generalized, Q8). A resolved approval keeps its row (status-marked) but leaves the
//! OPEN queue (the open-queue query filters `status='awaiting_approval'`, Q4). `sort_key` ranks
//! risk-DESC then age-ASC (AG §8 — `format!("{}_{}", 4 - risk, requested_at)`).
//!
//! FORBIDDEN: deriving the row's MUTABLE `status` from a registry row's CURRENT value — only the
//! event drives status; else a rebuild that reads a later registry state diverges from the
//! incremental fold (the rebuild-equivalence break this projector exists to avoid).

use rusqlite::{params, Transaction};

use nexusops_shared::event_envelope::EventEnvelope;
use nexusops_shared::events::{
    ActionApprovalRequested, ActionApproved, ActionDenied, ActionExpired,
};

use super::{ProjectionError, Projector};

pub struct ApprovalQueueProjector;

impl Projector for ApprovalQueueProjector {
    fn name(&self) -> &'static str {
        "approval_queue"
    }

    fn apply(&self, tx: &Transaction, env: &EventEnvelope) -> Result<(), ProjectionError> {
        // status is derived ONLY from the event type (never a registry value — rebuild-safety).
        let status = if env.event_type == ActionApprovalRequested::EVENT_TYPE {
            "awaiting_approval"
        } else if env.event_type == ActionApproved::EVENT_TYPE {
            "approved"
        } else if env.event_type == ActionDenied::EVENT_TYPE {
            "denied"
        } else if env.event_type == ActionExpired::EVENT_TYPE {
            "expired"
        } else {
            return Ok(()); // not an approval-lifecycle event — nothing to fold
        };

        // every approval event carries the `approval_id` envelope FK; without it there is no queue
        // row to address (defensive — the Gateway always sets it on these events).
        let approval_id = match env.approval_id.as_deref() {
            Some(a) => a,
            None => return Ok(()),
        };

        if status == "awaiting_approval" {
            open_row(tx, env, approval_id)
        } else {
            advance_status(tx, approval_id, status, env.seq)
        }
    }
}

/// UPSERT the OPEN queue row when an approval is requested. The immutable fields are sibling-read
/// (Q8): from `action_requests` for a single action OR a plan STEP (`action_request_id` present);
/// from `action_plans` for a plan-LEVEL approve-all approval (`action_request_id` NULL → the plan's
/// `overall_risk` + submit context). `plan_id`/`expires_at` come from `approvals`. The UPSERT is
/// idempotent under replay/rebuild (same `approval_id` PK).
fn open_row(
    tx: &Transaction,
    env: &EventEnvelope,
    approval_id: &str,
) -> Result<(), ProjectionError> {
    // the parent plan + expiry come from the approvals row (plan_id NULL for a standalone action).
    // A missing sibling row is a true integrity break (the Gateway wrote it in this very txn / it
    // is durable for a rebuild) → propagate as a Db error (fail-closed), never a silent default.
    let (plan_id, expires_at): (Option<String>, Option<String>) = tx.query_row(
        "SELECT plan_id, expires_at FROM approvals WHERE approval_id = ?1",
        [approval_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;

    let action_request_id = env.action_request_id.as_ref().map(|a| a.as_str());
    // immutable, never-change-post-submit identity/risk fields — sibling-read in the same txn.
    let (risk_level, requester_type, requester_id, project_id): (
        i64,
        String,
        String,
        Option<String>,
    ) = match action_request_id {
        // single action OR a plan step → from action_requests (the step's own risk + requester).
        Some(ar) => tx.query_row(
            "SELECT risk_level, requester_type, requester_id, project_id \
             FROM action_requests WHERE action_request_id = ?1",
            [ar],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?,
        // plan-LEVEL approve-all approval → from action_plans (overall_risk + the plan submitter).
        None => {
            let pid = plan_id.as_deref().ok_or_else(|| {
                ProjectionError::Decode(
                    "plan-level approval row carries no plan_id (integrity)".to_string(),
                )
            })?;
            tx.query_row(
                "SELECT overall_risk, requester_type, requester_id, project_id \
                 FROM action_plans WHERE plan_id = ?1",
                [pid],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )?
        }
    };

    let requested_at = env.occurred_at.as_str();
    // risk-DESC then age-ASC in ONE lexical ORDER BY: `4 - risk` flips the rank (risk 4 → "0_",
    // risk 0 → "4_") so a plain ascending sort puts the highest-risk oldest item first (AG §8).
    let sort_key = format!("{}_{}", 4 - risk_level, requested_at);

    tx.execute(
        "INSERT INTO proj_approval_queue \
         (approval_id, action_request_id, plan_id, project_id, session_id, agent_team_id, \
          risk_level, status, requester_type, requester_id, preview_summary, requested_at, \
          expires_at, sort_key, updated_at_seq) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,'awaiting_approval',?8,?9,NULL,?10,?11,?12,?13) \
         ON CONFLICT(approval_id) DO UPDATE SET \
           action_request_id=excluded.action_request_id, plan_id=excluded.plan_id, \
           project_id=excluded.project_id, session_id=excluded.session_id, \
           agent_team_id=excluded.agent_team_id, risk_level=excluded.risk_level, \
           status='awaiting_approval', requester_type=excluded.requester_type, \
           requester_id=excluded.requester_id, preview_summary=excluded.preview_summary, \
           requested_at=excluded.requested_at, expires_at=excluded.expires_at, \
           sort_key=excluded.sort_key, updated_at_seq=excluded.updated_at_seq",
        params![
            approval_id,
            action_request_id,
            plan_id,
            project_id,
            env.session_id.as_ref().map(|s| s.as_str()),
            env.agent_team_id.as_ref().map(|a| a.as_str()),
            risk_level,
            requester_type,
            requester_id,
            requested_at,
            expires_at,
            sort_key,
            env.seq,
        ],
    )?;
    Ok(())
}

/// Advance a resolved approval's status (approved/denied/expired). The row STAYS for history (Q4)
/// but leaves the OPEN queue. The row already exists (`ActionApprovalRequested` always precedes the
/// resolution in `seq` order, on both the append + rebuild paths); a 0-row UPDATE — the
/// open-request event was degraded/skipped — is a correct silent no-op (no row to mark).
fn advance_status(
    tx: &Transaction,
    approval_id: &str,
    status: &str,
    seq: i64,
) -> Result<(), ProjectionError> {
    tx.execute(
        "UPDATE proj_approval_queue SET status = ?2, updated_at_seq = ?3 WHERE approval_id = ?1",
        params![approval_id, status, seq],
    )?;
    Ok(())
}
