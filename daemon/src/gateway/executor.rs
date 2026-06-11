//! The `ActionExecutor` seam (§6.2/§6.3). 2.3 owns the real per-action-type adapters (git CLI,
//! octocrab, …) + the typed preview classes; 2.1b ships a STUB so the lifecycle completes
//! end-to-end and the INV-SEC-1 pin holds before real execution exists.
//!
//! **The executor runs OFF the write-actor txn** — the Gateway invokes [`ActionExecutor::execute`]
//! BETWEEN the `ActionApproved` txn and the `ActionStarted`/`ActionSucceeded` txn, never inside a
//! `gateway_txn` (2.3's slow/blocking git-CLI/octocrab executors must not starve the single
//! writer). The stub is instant, but the seam is structurally distinct so 2.3 swaps stub→real +
//! moves it off-thread without re-architecting `approve`.

use nexusops_shared::actions::{ActionPreview, ActionRequest};
use nexusops_shared::time::Timestamp;

/// The outcome of running an action's executor (§6.2 ActionResult, simplified for 2.1b). 2.3 adds
/// created/changed resources + the structured error taxonomy.
pub enum ExecutionOutcome {
    Succeeded,
    /// the executor failed; the message is recorded on `ActionFailed` (structured taxonomy → 2.4)
    Failed(String),
}

/// Runs (or previews) an action's side effect (§6.2/§6.3). The Gateway invokes `execute` BETWEEN
/// the `ActionApproved` and the `ActionStarted`/`ActionSucceeded` transitions — never inside a
/// write-actor txn. `preview` produces a dry-run envelope (the 6 typed per-class previews → 2.3).
pub trait ActionExecutor: Send + Sync {
    fn execute(&self, req: &ActionRequest) -> ExecutionOutcome;
    fn preview(&self, req: &ActionRequest, generated_at: Timestamp) -> ActionPreview;
}

/// 2.1b STUB — **NO real side effect**. `execute` records a "would-execute" (returns `Succeeded`,
/// driving ActionStarted→ActionSucceeded); `preview` returns a minimal envelope. 2.3 swaps in the
/// real per-action-type adapters + the typed preview classes.
pub struct StubExecutor;

impl ActionExecutor for StubExecutor {
    fn execute(&self, _req: &ActionRequest) -> ExecutionOutcome {
        // would-execute: no FS/git/network side effect in 2.1b — the lifecycle completes so the
        // chokepoint + its events are test-first before the real adapters (2.3) exist.
        ExecutionOutcome::Succeeded
    }

    fn preview(&self, req: &ActionRequest, generated_at: Timestamp) -> ActionPreview {
        ActionPreview {
            action_request_id: req.action_request_id.clone(),
            generated_at,
            risk_level: req.risk_level,
            risk_reasons: vec![],
            summary: format!(
                "(stub preview) {} — would execute; the typed preview classes land in 2.3",
                req.action_type
            ),
            changed_resources: req.resource_refs.clone(),
            cannot_preview_reason: None,
        }
    }
}
