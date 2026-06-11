//! The policy-engine seam (§6.2/§15). 2.2 owns the real risk→decision engine; 2.1b ships a STUB
//! so the chokepoint + its approval gate are test-first before the risk engine exists.

use nexusops_shared::actions::{ActionRequest, PolicyDecision, PolicyDecisionStatus};

/// Decides whether an action may proceed, needs approval, or is denied (§6.2 / AG §12). The
/// Gateway holds a `Box<dyn PolicyEngine>` so 2.2 swaps in the catalog-driven risk engine without
/// touching the pipeline.
pub trait PolicyEngine: Send + Sync {
    fn decide(&self, req: &ActionRequest) -> PolicyDecision;
}

/// 2.1b STUB — **risk-blind, require-approval-for-all**. The conservative pre-2.2 posture: nothing
/// auto-executes, the approval gate is ALWAYS live, so a requester's under-claimed `risk_level`
/// cannot bypass approval (it is recorded for audit, not trusted for the decision). 2.2 replaces
/// this with the §6.3-catalog risk engine (risk-0 → `allow`, ranges resolved by resource state).
pub struct StubPolicy;

impl PolicyEngine for StubPolicy {
    fn decide(&self, _req: &ActionRequest) -> PolicyDecision {
        PolicyDecision {
            status: PolicyDecisionStatus::RequireApproval,
            reasons: vec![
                "2.1b stub policy: every action requires approval until the 2.2 risk engine"
                    .to_string(),
            ],
        }
    }
}
