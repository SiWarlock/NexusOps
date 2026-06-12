//! The Claude hook-receiver ingress + the `MutationIntercept`→Gateway routing (brief 043, INV-SEC-1).
//!
//! L2 (this layer): the **hook-receiver ingress** — parse a `PreToolUse` payload + the pure
//! `tool_name (+ permission_mode) → agent-mutation action_type` mapping, **fail-closed** (malformed /
//! unmapped / non-`default` mode → Deny; never an un-adjudicated allow). The daemon-wired
//! `PreToolUse` hook (042's generated settings, matcher `"*"`) pipes the payload on stdin; the hook
//! fires on EVERY tool call (per-call interception is sound — hooks-guide.md), so the daemon
//! adjudicates each call.
//!
//! L3 adds the routing (build an agent-mutation `ActionRequest` → the EXISTING Gateway `submit_action`
//! → policy/approval → an **adjudication-only** verdict — the action terminates at the verdict, NO
//! daemon executor runs the tool). L4 adds the O-13 coverage-gap compensation (the §9.1 matrix
//! disposition — MCP / Task subagent / background subagent DENY two-layer + the launch
//! `permissions.deny` hard-block) + the params-sensitive deny-rules.
//!
//! **Safety #9** — the receiver reads the STRUCTURED hook payload, never the PTY. **Safety #10 /
//! O-13** — a non-`default` permission mode is denied (interception isn't guaranteed outside it).

use serde::Deserialize;

/// The subset of the Claude `PreToolUse` hook payload the receiver consumes. The daemon-wired hook
/// pipes the FULL JSON on stdin; the extra fields (`cwd`/`transcript_path`/`hook_event_name`/…) are
/// ignored — a tolerant read of the semi-trusted hook input, extracting only what the adjudication
/// needs. The 4 fields are REQUIRED: a payload missing any (or with a wrong type) fails to parse →
/// Deny (fail-closed — never construct an allow from garbage).
#[derive(Debug, Deserialize)]
pub struct HookPayload {
    /// the tool about to run. The interceptable DIRECT tools (`Bash`/`Write`/`Edit`/`Read`/…) map to
    /// an `agent.*` action_type; `mcp__<server>__<tool>` (MCP) and `Task` (subagent) are RECOGNIZED
    /// categories but **DENIED** (the lead-ratified coverage disposition — MCP BestEffort / subagent
    /// NotGuaranteed can't be reliably hook-intercepted; L4 formalizes the §9.1-matrix reason + the
    /// launch `permissions.deny` hard-block). An unknown tool is denied too (fail-closed).
    pub tool_name: String,
    /// the tool's input params — carried VERBATIM (any JSON value; no L2 shape validation). The L3
    /// routing redacts it into `ActionRequest.inputs`; the L4 deny-rules read it. L2 never persists it.
    pub tool_input: serde_json::Value,
    /// the Claude session id — **opaque at L2** (no adjudication logic depends on its value); the
    /// per-session `decision_sink` binding to the live session is the P4 transport.
    pub session_id: String,
    /// the session's permission mode — MUST be `default` (O-13 #10); any other mode → Deny.
    pub permission_mode: String,
}

/// Why the receiver denied a tool call WITHOUT adjudicating it (fail-closed — never a silent allow).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DenyReason {
    /// `permission_mode != "default"` — interception is only guaranteed in default mode (O-13 #10).
    NonDefaultMode,
    /// the tool is not a recognized interceptable DIRECT mutation: an unknown tool, OR a not-Direct
    /// category (MCP / `Task` subagent) which L4 denies via the §9.1 coverage matrix + the launch
    /// `permissions.deny` baseline. Conservative: anything not a covered direct tool is denied.
    UnmappedTool,
    /// the hook payload did not parse (truncated / missing a required field / wrong type).
    Malformed,
}

/// Parse a raw `PreToolUse` hook payload (the JSON the daemon-wired hook pipes on stdin). A parse
/// failure (truncated, missing a required field, wrong type) → [`DenyReason::Malformed`] (fail-closed
/// — the receiver never constructs an allow from un-parseable input).
pub fn parse_payload(json: &str) -> Result<HookPayload, DenyReason> {
    serde_json::from_str(json).map_err(|_| DenyReason::Malformed)
}

/// Map a parsed hook payload to its agent-mutation `action_type` (a catalog `agent.*` key — L1), or a
/// typed [`DenyReason`] (fail-closed). **The mode gate precedes the tool map** (O-13 #10 — a
/// non-`default` mode is a deny regardless of the tool). The DIRECT, hook-interceptable tools map to
/// their action_type; everything else (MCP / `Task` subagent / unknown) → [`DenyReason::UnmappedTool`]
/// (L4 formalizes the per-channel coverage-matrix disposition + the launch `permissions.deny`
/// hard-block — defense-in-depth). A returned action_type is the catalog key the L3 routing resolves
/// via `catalog::lookup`; a drift (a key not in the catalog) is itself fail-closed (lookup → None →
/// the policy denies it).
pub fn map_to_action_type(payload: &HookPayload) -> Result<&'static str, DenyReason> {
    if payload.permission_mode != "default" {
        return Err(DenyReason::NonDefaultMode);
    }
    match payload.tool_name.as_str() {
        "Bash" => Ok("agent.bash"),
        "Write" | "Edit" | "MultiEdit" | "NotebookEdit" => Ok("agent.file_edit"),
        "Read" | "Glob" | "Grep" => Ok("agent.file_read"),
        // MCP (`mcp__*`) / `Task` (subagent) / any unknown tool → the conservative L2 deny; L4 gives
        // the §9.1-matrix reason (BestEffort / NotGuaranteed / Unsupported) + the launch hard-block.
        _ => Err(DenyReason::UnmappedTool),
    }
}
