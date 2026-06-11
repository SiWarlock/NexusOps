//! The policy-engine seam (§6.2/§15). 2.2 owns the real risk→decision engine; 2.1b ships a STUB
//! so the chokepoint + its approval gate are test-first before the risk engine exists.

use nexusops_shared::actions::{ActionRequest, PolicyDecision, PolicyDecisionStatus, RiskLevel};
use nexusops_shared::catalog;

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
            // the 2.2 PolicyDecision fields — empty for the require-approval-for-all stub.
            required_approvals: vec![],
            constraints: vec![],
            safer_alt: None,
        }
    }
}

/// 2.2 — the catalog-driven policy engine (the production policy; INV-SEC-1). Resolves each action's
/// risk from the §6.3 [`catalog`] (AUTHORITATIVE), **never** the requester-supplied `risk_level`
/// (recorded, not trusted — §15). The risk → decision mapping (Q4, AG §7/§12):
/// - **risk-0** → `allow` (the auto-execute-eligible read/inspect/propose-only set);
/// - **risk 1/2/3** → `require_approval` (the MVP default-confirm posture);
/// - **risk-4** (critical) → `require_step_approval` (never broad automation);
/// - an `action_type` **absent from the catalog** → `deny` (fail-closed, §15 — never a default-allow).
///
/// The **null-schema floor** (§6.3/OQ-WP-5): an action whose params carry no typed schema
/// (`params_schema_present == false`) can NEVER resolve to `allow` — defense-in-depth that holds even
/// if a future catalog edit lowered its risk (the MVP's only such type, `workflow.command.invoke`, is
/// already risk-4). The `downgrade`/`needs_more_context` statuses exist in the enum but have no MVP
/// trigger (a later policy slice). The decision's `required_approvals`/`constraints`/`safer_alt` are
/// left empty in 2.2 (the pipeline resolves the approver as `current_user`; populating the decision's
/// own approver list is a later policy-surface concern).
pub struct CatalogPolicy;

impl PolicyEngine for CatalogPolicy {
    fn decide(&self, req: &ActionRequest) -> PolicyDecision {
        let Some(entry) = catalog::lookup(&req.action_type) else {
            return decision(
                PolicyDecisionStatus::Deny,
                format!(
                    "action_type '{}' is not in the §6.3 catalog — fail-closed deny (§15)",
                    req.action_type
                ),
            );
        };
        // the §6.3/OQ-WP-5 null-schema floor: no typed params schema → never auto-allow. The floor
        // only needs to act on the `Level0` arm — `Allow` is the sole status it must suppress;
        // risk 1-3 already `require_approval` and risk-4 already `require_step_approval`, so a
        // null-schema entry at any non-zero risk is approval-gated regardless (the floor is a no-op
        // there). `floored` therefore intentionally guards only `Level0`.
        let floored = !entry.params_schema_present;
        let status = match entry.locked_risk {
            RiskLevel::Level0 if floored => PolicyDecisionStatus::RequireApproval,
            RiskLevel::Level0 => PolicyDecisionStatus::Allow,
            RiskLevel::Level1 | RiskLevel::Level2 | RiskLevel::Level3 => {
                PolicyDecisionStatus::RequireApproval
            }
            RiskLevel::Level4 => PolicyDecisionStatus::RequireStepApproval,
        };
        decision(
            status,
            format!(
                "catalog risk {} for '{}'",
                entry.locked_risk as u8, req.action_type
            ),
        )
    }
}

/// Build a [`PolicyDecision`] with the given status + a single reason. The 2.2 fields
/// (`required_approvals`/`constraints`/`safer_alt`) are left empty (see [`CatalogPolicy`]).
fn decision(status: PolicyDecisionStatus, reason: String) -> PolicyDecision {
    PolicyDecision {
        status,
        reasons: vec![reason],
        required_approvals: vec![],
        constraints: vec![],
        safer_alt: None,
    }
}
