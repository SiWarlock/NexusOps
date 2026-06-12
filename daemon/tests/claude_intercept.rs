//! Brief 043 — the Claude `MutationIntercept`→Gateway interception (INV-SEC-1), tests.
//!
//! L2 (this file's first block): the daemon **hook-receiver ingress** — parsing a `PreToolUse`
//! payload + the **pure `tool_name (+ permission_mode) → agent-mutation action_type`** mapping,
//! **fail-closed** (malformed / unmapped / non-`default` mode → Deny; never an un-adjudicated allow).
//! L3 (the routing → Gateway + the adjudication verdict) + L4 (the coverage-gap crux + deny-rules)
//! land in their own blocks. Per-call interception is sound (the `*`-matcher PreToolUse hook fires on
//! EVERY tool call — claude-code-guide / hooks-guide.md); the coverage-gap compensation (mcp/Task/bg
//! DENY two-layer + the `permissions.deny` baseline) is L4.

use nexusopsd::harness::claude::intercept::{map_to_action_type, parse_payload, DenyReason};

// ---- L2 helpers ---------------------------------------------------------------------------------

/// a well-formed `PreToolUse` hook payload JSON (the daemon-wired hook pipes this on stdin).
fn payload_json(tool_name: &str, permission_mode: &str) -> String {
    serde_json::json!({
        "tool_name": tool_name,
        "tool_input": { "command": "ls -la" },
        "session_id": "sess_01ARZ3NDEKTSV4RRFFQ69G5FAV",
        "permission_mode": permission_mode,
    })
    .to_string()
}

// ---- 043 L2 RED #5 — the tool_name → agent-mutation action_type mapping (DirectToolUse) ----------

#[test]
fn test_hook_payload_maps_tool_to_action_type() {
    // spec(§9.1) — the receiver maps each interceptable DIRECT Claude tool (default mode) to its
    // agent-mutation action_type: Bash→agent.bash, the file-write family→agent.file_edit, the
    // read family→agent.file_read. (MCP/Task are the L4 coverage-gap DENYs — NOT mapped here.)
    let cases: &[(&str, &str)] = &[
        ("Bash", "agent.bash"),
        ("Write", "agent.file_edit"),
        ("Edit", "agent.file_edit"),
        ("MultiEdit", "agent.file_edit"),
        ("NotebookEdit", "agent.file_edit"),
        ("Read", "agent.file_read"),
        ("Glob", "agent.file_read"),
        ("Grep", "agent.file_read"),
    ];
    for (tool, expected) in cases {
        let payload = parse_payload(&payload_json(tool, "default")).expect("well-formed payload");
        assert_eq!(
            map_to_action_type(&payload),
            Ok(*expected),
            "{tool} → {expected}"
        );
    }
}

// ---- 043 L2 RED #6 — an unmapped tool fails closed (never an un-adjudicated allow) ---------------

#[test]
fn test_unknown_tool_fail_closed_deny() {
    // spec(§15 #1 / §9.1) — a tool the receiver does NOT recognize as an interceptable direct
    // mutation → Deny (UnmappedTool), never a silent allow. Covers a genuinely-unknown tool AND the
    // not-Direct categories L2 denies conservatively (Task subagent, MCP) — L4 formalizes WHY via the
    // §9.1 coverage matrix; here the invariant is "anything not a covered direct tool is denied".
    for tool in [
        "Task",
        "mcp__codegraph__search",
        "WebFetch",
        "SomeBrandNewTool",
        "",
    ] {
        let payload = parse_payload(&payload_json(tool, "default")).expect("well-formed payload");
        assert_eq!(
            map_to_action_type(&payload),
            Err(DenyReason::UnmappedTool),
            "an unmapped tool `{tool}` must fail closed (Deny), never an un-adjudicated allow"
        );
    }
}

// ---- 043 L2 RED #7 — a non-default permission mode fails closed (O-13 #10) -----------------------

#[test]
fn test_non_default_mode_fail_closed_deny() {
    // spec(§9.1 / O-13 #10) — interception is only guaranteed in `default` permission mode; ANY other
    // mode (acceptEdits/bypassPermissions/plan) → Deny, even for an otherwise-interceptable tool. The
    // mode check precedes the tool map (a non-default mode is a deny regardless of the tool).
    for mode in ["acceptEdits", "bypassPermissions", "plan", "", "Default"] {
        let payload = parse_payload(&payload_json("Bash", mode)).expect("well-formed payload");
        assert_eq!(
            map_to_action_type(&payload),
            Err(DenyReason::NonDefaultMode),
            "permission_mode `{mode}` (≠ default) must fail closed (Deny) — O-13 #10"
        );
    }
}

// ---- 043 L2 RED #8 — a malformed hook payload fails closed -----------------------------------

#[test]
fn test_malformed_payload_deny() {
    // spec(§15 fail-closed) — a hook payload that does not parse (truncated JSON, missing the required
    // fields, wrong types) → Deny (Malformed), never construct an un-adjudicated allow from garbage.
    for bad in [
        "{ not valid json",           // truncated
        "{}",                         // missing every field
        r#"{ "tool_name": "Bash" }"#, // missing session_id / permission_mode
        r#"{ "tool_name": 42, "tool_input": {}, "session_id": "s", "permission_mode": "default" }"#, // wrong type
        "", // empty
    ] {
        assert_eq!(
            parse_payload(bad).err(),
            Some(DenyReason::Malformed),
            "a malformed payload `{bad}` must fail closed (Deny)"
        );
    }
}
