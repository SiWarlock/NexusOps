//! §6.2/§6.3 preview framework (2.3 L2) — render a dry-run [`ActionPreview`] by dispatching on the
//! catalog `preview_class`, into the **frozen flat envelope** (`summary` + `changed_resources` + the
//! escalation fields; no new model fields, CONTRACT stays 0.18.0).
//!
//! # 2.3 TRANSIENT — every preview is structural-only (escalates)
//!
//! 2.3 ships the preview FRAMEWORK (the dispatch + the escalation logic) but **no real namespace
//! adapter** — `git/`, `integrations/`, `harness/`, `brainclient/` are absent (Phase 3/5/7/8). So no
//! class can produce a *real* dry-run yet: each renders a class-specific structural `summary` the
//! human can read, but ALSO sets [`ActionPreview::cannot_preview_reason`] (naming the owning phase)
//! and escalates the **preview** `risk_level` one level above the catalog risk + records a
//! `risk_reasons` entry. **This all-escalate behavior is a 2.3 TRANSIENT, not a permanent invariant**
//! — as each phase lands its adapter it replaces that class's impossible arm with a real dry-run,
//! dropping the per-class escalation. The escalation is on the **preview envelope ONLY**: it never
//! touches `action_requests.risk_level` / the policy / approval / the risk-0 auto-execute re-gate.

use nexusops_shared::actions::{ActionPreview, ActionRequest, RiskLevel};
use nexusops_shared::catalog::{ActionTypeCatalogEntry, ExecutorKind, PreviewClass};
use nexusops_shared::time::Timestamp;

/// Generate the dry-run preview for `req` from its catalog `entry` (§6.2/§6.3). Dispatches by
/// `entry.preview_class` for the class-specific `summary`; sets `cannot_preview_reason` + escalates
/// the preview risk because the real per-namespace adapter is absent in 2.3 (see the module
/// 2.3-TRANSIENT note). `changed_resources` come from the action's (already §15-redacted) resource
/// refs.
pub fn generate_preview(
    req: &ActionRequest,
    entry: &ActionTypeCatalogEntry,
    generated_at: Timestamp,
) -> ActionPreview {
    let n_refs = req.resource_refs.len();
    let summary = class_summary(entry.preview_class, &req.action_type, n_refs);
    // 2.3 TRANSIENT: no real adapter → a structural-only preview. NAME the owning phase so it's
    // actionable + later distinguishable from a genuinely-unpreviewable action.
    let cannot = format!(
        "the {} preview for `{}` needs the {} adapter (lands {}); 2.3 ships the preview framework only",
        class_label(entry.preview_class),
        req.action_type,
        namespace_label(entry.executor),
        owning_phase(entry.executor),
    );
    let escalated = escalate(entry.locked_risk);
    ActionPreview {
        action_request_id: req.action_request_id.clone(),
        generated_at,
        risk_level: escalated,
        risk_reasons: vec![format!(
            "preview unavailable (no real {} dry-run in 2.3) → risk shown one level above the catalog \
             risk {} for review caution (envelope-only; the policy risk is unchanged)",
            namespace_label(entry.executor),
            entry.locked_risk as u8,
        )],
        summary,
        changed_resources: req.resource_refs.clone(),
        cannot_preview_reason: Some(cannot),
    }
}

/// the class-specific structural summary (each names its class so the dispatch is observable).
fn class_summary(class: PreviewClass, action_type: &str, n_refs: usize) -> String {
    match class {
        PreviewClass::Command => {
            format!("command preview: would run `{action_type}` ({n_refs} resource ref(s))")
        }
        PreviewClass::Diff => {
            format!("diff preview: would show the changes `{action_type}` makes to {n_refs} resource(s)")
        }
        PreviewClass::Git => {
            format!("git preview: would perform `{action_type}` on {n_refs} resource(s)")
        }
        PreviewClass::Api => {
            format!("api preview: would call `{action_type}` ({n_refs} resource ref(s))")
        }
        PreviewClass::Session => {
            format!("session preview: would `{action_type}` on {n_refs} session resource(s)")
        }
        PreviewClass::Workflow => {
            format!("workflow preview: would run `{action_type}` ({n_refs} resource ref(s))")
        }
        PreviewClass::Rollback => {
            format!("rollback preview: would revert `{action_type}` ({n_refs} resource ref(s))")
        }
    }
}

/// the human-facing class label for the cannot-preview reason.
fn class_label(class: PreviewClass) -> &'static str {
    match class {
        PreviewClass::Command => "command",
        PreviewClass::Diff => "diff",
        PreviewClass::Git => "git",
        PreviewClass::Api => "api",
        PreviewClass::Session => "session",
        PreviewClass::Workflow => "workflow",
        PreviewClass::Rollback => "rollback",
    }
}

/// the namespace whose real executor adapter renders this class's dry-run. `pub(crate)` so the
/// executor framework (`CatalogExecutor`) names the same namespace in its stub-outcome detail.
pub(crate) fn namespace_label(executor: ExecutorKind) -> &'static str {
    match executor {
        ExecutorKind::Brain => "Project Brain",
        ExecutorKind::Project => "project scanner",
        ExecutorKind::Workflow => "workflow runtime",
        ExecutorKind::Plan => "plan/task integration",
        ExecutorKind::Session => "session host",
        ExecutorKind::Git => "git",
        ExecutorKind::Github => "GitHub",
        ExecutorKind::Linear => "Linear",
        ExecutorKind::Code => "code",
        ExecutorKind::Review => "review",
    }
}

/// the planned owning phase for a namespace's real adapter (per the §2.5 Track map: Phase 3 harness/
/// git/code/session/workflow, Phase 5 projects/GitHub/plan, Phase 7 PR-review/Linear, Phase 8 Brain).
/// Indicative — refined as each adapter lands; the label is advisory text. `pub(crate)` so the
/// executor framework names the same owning phase in its stub-outcome detail.
pub(crate) fn owning_phase(executor: ExecutorKind) -> &'static str {
    match executor {
        ExecutorKind::Git | ExecutorKind::Session | ExecutorKind::Code | ExecutorKind::Workflow => {
            "Phase 3"
        }
        ExecutorKind::Github | ExecutorKind::Plan | ExecutorKind::Project => "Phase 5",
        ExecutorKind::Linear | ExecutorKind::Review => "Phase 7",
        ExecutorKind::Brain => "Phase 8",
    }
}

/// escalate one risk level for the preview envelope (capped at 4 — never wraps). Envelope-only.
fn escalate(risk: RiskLevel) -> RiskLevel {
    match risk {
        RiskLevel::Level0 => RiskLevel::Level1,
        RiskLevel::Level1 => RiskLevel::Level2,
        RiskLevel::Level2 => RiskLevel::Level3,
        RiskLevel::Level3 | RiskLevel::Level4 => RiskLevel::Level4,
    }
}
