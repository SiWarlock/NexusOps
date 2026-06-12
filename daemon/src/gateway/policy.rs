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

/// 043 (L4) — the agent-mutation policy: wraps [`CatalogPolicy`] with the O-13 **params-sensitive
/// deny-rules**. For an agent-mutation action (`agent.*`) whose params match a known-dangerous pattern
/// ([`deny_rule_match`]) it returns `Deny` OUTRIGHT — never reaching approval (the REDUNDANT O-13 layer
/// over the PRIMARY require-approval-by-default control). For every other action (the non-agent MVP
/// types AND a non-dangerous agent mutation) it DELEGATES to [`CatalogPolicy`] unchanged — the catalog-authoritative
/// base risk is the floor; the deny-rules only RAISE to a deny, never lower (the §15 catalog-risk
/// invariant holds). This is the Gateway's production policy when supervising agents.
pub struct AgentMutationPolicy;

impl PolicyEngine for AgentMutationPolicy {
    fn decide(&self, req: &ActionRequest) -> PolicyDecision {
        if is_agent_mutation(&req.action_type) {
            if let Some(reason) = deny_rule_match(&req.action_type, &req.inputs) {
                return decision(
                    PolicyDecisionStatus::Deny,
                    format!("agent-mutation deny-rule: {reason}"),
                );
            }
        }
        CatalogPolicy.decide(req)
    }
}

/// Whether an `action_type` is an agent-mutation family member (043 — the `agent.*` adjudication set).
fn is_agent_mutation(action_type: &str) -> bool {
    catalog::AGENT_MUTATION_ACTION_TYPES.contains(&action_type)
}

/// The O-13 params-sensitive deny-rule predicate (043 L4) — a CONSERVATIVE, PURE starter set, NOT a
/// complete sandbox (the PRIMARY control is require-approval-by-default; this is the REDUNDANT O-13
/// layer that denies the OBVIOUS catastrophic patterns OUTRIGHT, before they reach the human). Returns
/// `Some(reason)` on a match. **Errs toward NO false-positives** — a benign command must NOT be denied
/// (it falls through to require-approval); a missed dangerous variant is acceptable (the human still
/// sees it via approval). Today only `agent.bash` (the unbounded-blast-radius tool) has rules; the
/// `agent.file_edit`/`file_read` path-based rules (secret-file / out-of-tree write) are a flagged
/// extension. Extensible — pin every addition with a test.
pub fn deny_rule_match(action_type: &str, inputs: &serde_json::Value) -> Option<String> {
    if action_type != "agent.bash" {
        return None;
    }
    let command = inputs.get("command").and_then(|c| c.as_str())?;
    bash_deny_rule(command)
}

/// The Bash-command deny-rule patterns (pure; token/substring heuristics). Conservative — matches the
/// clear catastrophic SHAPES only (broad-path `rm -rf`, force-push, `curl|sh`, `--dangerously-*`).
fn bash_deny_rule(command: &str) -> Option<String> {
    let c = command.trim();
    if rm_rf_broad(c) {
        return Some(
            "recursive force-delete of a broad path (rm -rf /, ~, /*, $HOME, *)".to_string(),
        );
    }
    if c.contains("git push") && (c.contains("--force") || has_short_flag(c, 'f')) {
        return Some("git force-push (remote history overwrite)".to_string());
    }
    if (c.contains("curl") || c.contains("wget")) && pipes_to_shell(c) {
        return Some("piping a downloaded script into a shell (curl|sh)".to_string());
    }
    if c.contains("--dangerously") {
        return Some("a --dangerously-* flag".to_string());
    }
    None
}

/// A recursive+force `rm` targeting a BROAD path (catastrophic). Token-precise to avoid a false-positive
/// on a SCOPED delete (`rm -rf ./build` → no broad token → not matched).
fn rm_rf_broad(c: &str) -> bool {
    if !c.split_whitespace().any(|t| t == "rm") {
        return false;
    }
    // recursive AND force, in ANY flag form: short (`-r`/`-f`/combined `-rf`/`-fr`) or long
    // (`--recursive`/`--force`) — so `rm -rf /`, `rm -r --force /`, `rm --recursive --force /` all match.
    let recursive = has_short_flag(c, 'r') || c.contains("--recursive");
    let force = has_short_flag(c, 'f') || c.contains("--force");
    if !(recursive && force) {
        return false;
    }
    if c.contains("--no-preserve-root") {
        return true;
    }
    // a broad target as a whitespace-delimited token (NOT a scoped path like ./build or /abs/specific).
    c.split_whitespace()
        .any(|t| matches!(t, "/" | "/*" | "~" | "~/" | "$HOME" | "*"))
}

/// Whether the command pipes into a shell interpreter (`… | sh` / `… | bash` / `… | zsh`). Strips
/// spaces so `curl x | sh` and `curl x|sh` both match. Imprecise toward FALSE-POSITIVES (e.g. a pipe
/// into a `sh*`-prefixed program name) — acceptable: a false-positive is a safe deny, and a missed
/// interpreter (`| python`, process-substitution) still reaches require-approval (the primary control).
fn pipes_to_shell(c: &str) -> bool {
    let norm = c.replace(' ', "");
    norm.contains("|sh") || norm.contains("|bash") || norm.contains("|zsh")
}

/// Whether a short flag char appears in any `-…` (NOT `--…`) token (e.g. `-f`, or combined `-rf`).
fn has_short_flag(c: &str, flag: char) -> bool {
    c.split_whitespace()
        .any(|t| t.starts_with('-') && !t.starts_with("--") && t.contains(flag))
}
