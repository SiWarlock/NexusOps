//! The §6.2 AG-16.4 / §7.2 stale-precondition re-read seam (2.4 L4).
//!
//! After lock + fencing (L3) + BEFORE execute, the gateway RE-READS the action's live source (the
//! git worktree state, the PR state, the session state — whatever the action would mutate) and asks:
//! did it CHANGE since the action's preview/approval? A `Changed` answer means the approved mutation
//! is now STALE — executing it would apply a DIFFERENT mutation than the human approved (§6.2: never
//! execute a mutation different from what was approved). The gateway fails it closed (records
//! `ActionFailed(stale_precondition)` + regenerates the preview + refuses → a fresh approval cycle).
//!
//! 2.4 ships the SEAM + the stale→fail logic, exercised by a fake oracle. The REAL per-action_type
//! live-source re-reads (git2 worktree / octocrab PR / session host) land with their adapters
//! (Phase 5/7); the 2.4 production default ([`NullPreconditionOracle`]) reports `Unchanged` for all.

use nexusops_shared::actions::ActionRequest;

/// The result of re-reading an action's live source (§6.2 AG-16.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreconditionStatus {
    /// the live source is unchanged since preview/approval → the approved mutation is still valid.
    Unchanged,
    /// the live source CHANGED since preview/approval → the approved mutation is stale (§17 / §6.2).
    Changed,
}

/// Re-reads an action's live source after lock + before execute (§6.2/§7.2). A 2.4 framework seam:
/// the real git2 / octocrab / session-host re-reads land Phase 5/7; 2.4 fakes it. Injected on the
/// [`crate::gateway::Gateway`] (sibling to the policy + executor seams).
pub trait PreconditionOracle: Send + Sync {
    /// Re-read `req`'s live source and report whether it changed since the action's preview/approval.
    fn recheck(&self, req: &ActionRequest) -> PreconditionStatus;
}

/// The 2.4 PRODUCTION default — the real per-action_type live-source re-read is not built yet
/// (Phase 5/7), so the precondition is treated as `Unchanged` (execute proceeds; the seam is wired,
/// the real re-read swaps in later). The gateway's `with_precondition` constructor injects a real or
/// fake oracle; `Gateway::new` defaults to this.
pub struct NullPreconditionOracle;

impl PreconditionOracle for NullPreconditionOracle {
    fn recheck(&self, _req: &ActionRequest) -> PreconditionStatus {
        PreconditionStatus::Unchanged
    }
}
