//! Brief 066 (CAT-1) — the Codex `PreToolUse` interception: parse + normalize the Codex hook envelope
//! INTO the EXISTING 4.0b-2 daemon `intercept`→Gateway adjudication loop (LESSON §26 generalized /
//! §42), and translate the daemon verdict back to Codex's `PreToolUse` output. **Swap ONLY the I/O
//! envelope** — the Gateway adjudication (submit_action → decision_sink → wait-class → verdict) is
//! harness-agnostic and reused UNCHANGED; the divergence is the PARSER (the tool classifier).
//!
//! **Defense-in-depth (research §4.3 / the 🔴 INV-SEC-1 nuance):** Codex's docs call `PreToolUse` *"a
//! guardrail, not a complete enforcement boundary"* — so the hook is the **adjudication+audit** channel
//! and `--sandbox workspace-write` (the OS-enforcement boundary, [`super::launch::CodexLaunchConfig`])
//! is the **containment** layer. Both together = INV-SEC-1 for Codex.
//!
//! **Safety #9** — the receiver reads the STRUCTURED hook stdin, never the PTY. **PIN-1 (the
//! by-construction crux):** the `harness` discriminator is STAMPED by the trusted hook binary
//! ([`normalize_to_intercept_params`]) — [`CodexHookPayload`] has NO `harness` field, so a Codex stdin
//! that tries to spoof one is dropped at parse and CANNOT influence the daemon classifier.

use serde::Deserialize;

use crate::harness::claude::intercept::{disposition, DenyReason, Disposition};
use crate::harness::{coverage_of, Harness, MutationChannel, MutationCoverage};

use super::parse::classify_tool;
use super::status::CodexToolKind;

/// The subset of the Codex `PreToolUse` hook stdin the adjudicator consumes (research §4.1:
/// `{turn_id, tool_name, tool_use_id, tool_input{command,…}}`). A tolerant read — extra fields are
/// ignored. **PIN-1:** there is intentionally **NO `harness` field** here — the discriminator is the
/// trusted hook binary's, stamped in [`normalize_to_intercept_params`], NEVER read from agent stdin.
/// `tool_name` is REQUIRED (a payload missing it fails to parse → [`DenyReason::Malformed`] → deny).
#[derive(Debug, Deserialize)]
pub struct CodexHookPayload {
    /// the Codex tool about to run (research §4.1: `"Bash" | "apply_patch" | <MCP name>`, OR the
    /// rollout-vocabulary `shell`/`local_shell`/`exec_command`). The daemon — never the hook — is the
    /// trusted classifier of this RAW name ([`map_codex_to_action_type`]).
    pub tool_name: String,
    /// the tool's input params — carried VERBATIM (any JSON; the daemon pipeline §15-redacts at rest).
    #[serde(default)]
    pub tool_input: serde_json::Value,
    /// the Codex turn id — opaque (correlation only; no adjudication logic depends on it).
    #[serde(default)]
    pub turn_id: String,
    /// the Codex tool-use id — opaque (correlation only).
    #[serde(default)]
    pub tool_use_id: String,
}

/// The Codex hook's CLIENT-side fail-closed reason (content-free §15) — emitted when the conduit denies
/// by default (no daemon / unreachable / a read past the timeout / a non-allow verdict).
pub const CODEX_FAIL_CLOSED_REASON: &str =
    "NexusOps: denied (or interception unavailable — fail-closed)";

/// Parse a raw Codex `PreToolUse` stdin payload. A parse failure (truncated / missing `tool_name` /
/// wrong type) → [`DenyReason::Malformed`] (fail-closed — never construct an allow from garbage).
pub fn parse_codex_payload(json: &str) -> Result<CodexHookPayload, DenyReason> {
    serde_json::from_str(json).map_err(|_| DenyReason::Malformed)
}

/// Classify a RAW Codex `tool_name` → its agent-mutation `action_type` (a §6.3 `agent.*` catalog key),
/// or a typed [`DenyReason`] (fail-closed). Classification is by SEMANTICS (LESSON §42, reusing
/// [`classify_tool`]) — the shell family (`exec_command`/`shell`/`local_shell`/`bash` + the
/// PreToolUse-layer `"Bash"`) → `agent.bash`; `apply_patch` → `agent.file_edit`; an MCP name →
/// `agent.mcp_tool`. The §9.1 coverage matrix (`Harness::Codex`) drives [`disposition`]: Codex
/// direct + MCP are `Guaranteed` → Adjudicate; an un-classified tool (`Other`/unknown) →
/// [`DenyReason::CoverageGap`] (conservative deny-unknown — the 043 posture; the sandbox backs any miss).
pub fn map_codex_to_action_type(tool_name: &str) -> Result<&'static str, DenyReason> {
    // `classify_tool` is case-insensitive (3.3c) — the PreToolUse layer presents `"Bash"`, the rollout
    // vocabulary is lowercase (`shell`/`exec_command`); one normalized contract serves both callers.
    let (channel, action_type) = match classify_tool(tool_name) {
        CodexToolKind::ShellExec => (MutationChannel::DirectToolUse, "agent.bash"),
        CodexToolKind::FilePatch => (MutationChannel::DirectToolUse, "agent.file_edit"),
        CodexToolKind::McpTool => (MutationChannel::McpTool, "agent.mcp_tool"),
        // an un-classified / benign-non-mutation tool (update_plan / request_user_input / unknown) is
        // NOT a mapped mutation channel → conservative deny-unknown (the sandbox is the backstop).
        CodexToolKind::Other => return Err(DenyReason::CoverageGap),
    };
    // the §9.1 matrix is the source of truth (a missing cell — impossible, all 8 defined — fails
    // closed to Unsupported → Deny). Codex direct/MCP are Guaranteed → Adjudicate.
    let coverage = coverage_of(Harness::Codex, channel).unwrap_or(MutationCoverage::Unsupported);
    match disposition(coverage, "default") {
        Disposition::Adjudicate => Ok(action_type),
        Disposition::Deny => Err(DenyReason::CoverageGap),
    }
}

/// Normalize a parsed Codex hook payload + the daemon `session_id` (from `NEXUSOPS_SESSION_ID`) into the
/// SAME `intercept` RPC params the Claude hook sends — so the daemon adjudication loop is reused
/// UNCHANGED. The fields: the RAW codex `tool_name` (the daemon is the trusted classifier — never a
/// hook-supplied action_type), the verbatim `tool_input`, the `session_id` correlation tag,
/// `permission_mode="default"` (Codex has no modes → the O-13 #10 gate is a no-op for Codex), and
/// **`harness="codex"` STAMPED HERE (PIN-1)** — the trusted-binary discriminator that selects the Codex
/// classifier in [`crate::harness::claude::intercept::map_to_action_type`]. The agent cannot set it: it
/// is a literal, not read from `payload`.
pub fn normalize_to_intercept_params(
    payload: &CodexHookPayload,
    session_id: &str,
) -> serde_json::Value {
    serde_json::json!({
        "tool_name": payload.tool_name,
        "tool_input": payload.tool_input,
        "session_id": session_id,
        "permission_mode": "default",
        // PIN-1: the discriminator is the trusted hook binary's (the `--harness codex` branch), a
        // LITERAL — never `payload.harness` (CodexHookPayload has no such field). A spoofed `harness`
        // in the agent's stdin is dropped at parse and cannot reach here.
        "harness": "codex",
    })
}

/// Codex's `PreToolUse` ALLOW output (research §4.1 — identical shape to Claude's hookSpecificOutput).
pub fn codex_allow_output() -> serde_json::Value {
    serde_json::json!({
        "hookSpecificOutput": { "hookEventName": "PreToolUse", "permissionDecision": "allow" }
    })
}

/// Codex's `PreToolUse` DENY output (blocks the tool call; research §4.1). The reason rides
/// `permissionDecisionReason` (content-free §15).
pub fn codex_deny_output(reason: &str) -> serde_json::Value {
    serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason,
        }
    })
}

/// The fail-closed verdict→output mapping: ONLY `Ok(true)` (a daemon Allow) opens the tool; an
/// `Ok(false)` (a daemon Deny) AND any transport `Err` (no daemon / unreachable / a read past the
/// timeout / a parse failure) → the deny output. The conduit NEVER silently allows (§15 / LESSON
/// §26/§30 — the daemon-side receiver deny is the PRIMARY control; this conduit defaults to deny).
pub fn codex_verdict_output(adjudicated: &std::io::Result<bool>) -> serde_json::Value {
    match adjudicated {
        Ok(true) => codex_allow_output(),
        _ => codex_deny_output(CODEX_FAIL_CLOSED_REASON),
    }
}
