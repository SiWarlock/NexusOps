//! Brief 066 — the CAT-1 Codex INV-SEC-1 interception (PreToolUse→Gateway + `--sandbox`
//! defense-in-depth), tests. RED-first.
//!
//! ARCHITECTURE §15 (INV-SEC-1 / single audited mutator — no FS/git/external mutation except via a
//! typed, policy+approval-gated, AUDITED Action), §9.1 (the `MutationIntercept`→Gateway routing + the
//! per-harness coverage matrix), §6/§6.3 (the agent-mutation `agent.*` catalog family +
//! `ExecutorKind::Adjudication`). Foundation: docs/planning/0.3-codex-schema-research.md §4.1
//! (the `PreToolUse` stdin/stdout grammar — `{turn_id,tool_name,tool_use_id,tool_input}` →
//! `hookSpecificOutput.permissionDecision:"allow"|"deny"`) / §4.3 (the approval×sandbox matrix +
//! the 🔴 INV-SEC-1 nuance: the hook is a guardrail, NOT a boundary → MUST layer `--sandbox`).
//!
//! **The reuse thesis (LESSON §26 generalized / §42):** the SECOND harness's interception REUSES the
//! Claude→Gateway adjudication loop by swapping ONLY the I/O envelope — the Codex `PreToolUse`
//! stdin/stdout normalizes INTO the SAME `route_intercept`→`submit_action`→decision_sink→verdict path;
//! the daemon adjudication stays harness-agnostic. The divergence is the PARSER (the tool classifier).
//!
//! **CAT-1 + NO LIVE AGENT** — driven via the REAL Gateway (`CatalogPolicy` + `StubExecutor`) + a temp
//! EventStore (the 043/4.0b-2 "FakeGateway" pattern); no `codex` is spawned (the binding condition —
//! the live drive + the LIVE interception/containment proof are the 0.1/0.3-HITL follow-on). The
//! no-live-spawn pin over the WHOLE codex dir (incl. the new `intercept.rs`) is
//! `codex_launch.rs::test_no_codex_spawn_in_slice`.

use std::path::{Path, PathBuf};

use nexusops_shared::status::ActionRequestStatus as AR;
use nexusopsd::clock::FixedClock;
use nexusopsd::eventstore::{EventStore, PrefixRedactor};
use nexusopsd::fault::{arm, FaultPoint};
use nexusopsd::gateway::executor::StubExecutor;
use nexusopsd::gateway::policy::CatalogPolicy;
use nexusopsd::gateway::Gateway;
use nexusopsd::harness::claude::intercept::{
    map_to_action_type, parse_payload, route_intercept, verdict_for_status, DenyReason,
    InterceptOutcome,
};
use nexusopsd::harness::codex::intercept::{
    codex_allow_output, codex_deny_output, codex_verdict_output, map_codex_to_action_type,
    normalize_to_intercept_params, parse_codex_payload,
};
use nexusopsd::harness::MutationVerdict;
use nexusopsd::idgen::UlidGen;

// ---- helpers ------------------------------------------------------------------------------------

const SESS: &str = "sess_01ARZ3NDEKTSV4RRFFQ69G5FAV";

/// a well-formed Codex `PreToolUse` stdin payload (research §4.1: `{turn_id, tool_name, tool_use_id,
/// tool_input{command,…}}`).
fn codex_stdin(tool_name: &str) -> String {
    serde_json::json!({
        "turn_id": "turn_7",
        "tool_name": tool_name,
        "tool_use_id": "call_42",
        "tool_input": { "command": "ls -la" },
    })
    .to_string()
}

fn temp_db() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nexusops.db");
    (dir, path)
}

fn open(path: &Path) -> EventStore {
    EventStore::open(
        path,
        Box::new(UlidGen),
        Box::new(FixedClock::new("2026-06-17T00:00:00Z")),
        Box::new(PrefixRedactor),
    )
    .expect("open event store")
}

/// the PRODUCTION Gateway (CatalogPolicy + the side-effect-free StubExecutor) — the "FakeGateway".
fn catalog_gw() -> Gateway {
    Gateway::new(Box::new(CatalogPolicy), Box::new(StubExecutor))
}

/// the single action_requests row's status (the temp DB has exactly one unless noted).
fn action_status(path: &Path) -> Option<String> {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row("SELECT status FROM action_requests", [], |r| r.get(0))
        .ok()
}

fn approval_id_of(path: &Path) -> String {
    let conn = nexusopsd::eventstore::open_read_only(path).expect("read-only conn");
    conn.query_row("SELECT approval_id FROM approvals", [], |r| r.get(0))
        .expect("approval_id")
}

/// the daemon-side parse of the hook-normalized `intercept` params → the `HookPayload` the UNCHANGED
/// `route_intercept` consumes (proves the envelope normalizes INTO the existing loop, not a fork).
fn route_codex(gw: &Gateway, store: &mut EventStore, tool_name: &str) -> InterceptOutcome {
    let payload = parse_codex_payload(&codex_stdin(tool_name)).expect("well-formed codex stdin");
    let params = normalize_to_intercept_params(&payload, SESS);
    let hook_payload =
        parse_payload(&params.to_string()).expect("normalized params parse as HookPayload");
    route_intercept(gw, store, &hook_payload)
}

// ---- RED #6 — the Codex tool classification (semantics → agent.*; un-classified → deny) ----------

#[test]
fn test_codex_tool_classification() {
    // spec(§6.3/§9.1) — classify by SEMANTICS not vendor names (LESSON §42): the shell family
    // (exec_command/shell/local_shell/bash + the PreToolUse-layer "Bash") → agent.bash; apply_patch →
    // agent.file_edit; an MCP name → agent.mcp_tool (Codex MCP is Guaranteed-interceptable per the
    // coverage matrix, UNLIKE Claude's BestEffort-deny). An un-classified tool → CoverageGap → deny
    // (conservative deny-unknown — the 043 posture; the sandbox backs up any miss).
    let mapped: &[(&str, &str)] = &[
        ("Bash", "agent.bash"),
        ("shell", "agent.bash"),
        ("local_shell", "agent.bash"),
        ("exec_command", "agent.bash"),
        ("bash", "agent.bash"),
        ("apply_patch", "agent.file_edit"),
        ("mcp__codegraph__search", "agent.mcp_tool"),
    ];
    for (tool, expected) in mapped {
        assert_eq!(
            map_codex_to_action_type(tool),
            Ok(*expected),
            "{tool} → {expected}"
        );
    }
    // un-classified / benign-non-mutation (update_plan/request_user_input/unknown) → conservative deny.
    for tool in [
        "update_plan",
        "request_user_input",
        "write_stdin",
        "SomeBrandNewTool",
        "",
    ] {
        assert_eq!(
            map_codex_to_action_type(tool),
            Err(DenyReason::CoverageGap),
            "an un-classified codex tool `{tool}` fails closed (CoverageGap deny-unknown)"
        );
    }
}

// ---- RED #5 — the Codex stdin envelope normalizes to the SAME `intercept` params ----------------

#[test]
fn test_codex_envelope_normalizes_to_intercept() {
    // spec(§9.1) — reuse-not-fork: the Codex `PreToolUse` stdin normalizes to the SAME `intercept` RPC
    // params shape the Claude hook sends — the RAW codex tool_name (the daemon is the trusted
    // classifier, never a hook-supplied action_type), the verbatim tool_input, the NEXUSOPS_SESSION_ID
    // correlation tag, permission_mode="default" (Codex has no modes → the O-13 gate is a no-op), and a
    // harness="codex" discriminator so the daemon classifier branches to the Codex semantics. The
    // result MUST parse as the daemon's `HookPayload` (the loop is harness-agnostic).
    let payload = parse_codex_payload(&codex_stdin("shell")).unwrap();
    let params = normalize_to_intercept_params(&payload, SESS);
    assert_eq!(
        params["tool_name"], "shell",
        "the RAW codex tool_name is forwarded (daemon classifies)"
    );
    assert_eq!(
        params["tool_input"]["command"], "ls -la",
        "tool_input forwarded verbatim"
    );
    assert_eq!(
        params["session_id"], SESS,
        "tagged with the daemon session correlation key"
    );
    assert_eq!(
        params["permission_mode"], "default",
        "Codex has no modes → default (gate no-op)"
    );
    assert_eq!(
        params["harness"], "codex",
        "the harness discriminator selects the Codex classifier"
    );
    // the load-bearing reuse assertion: the normalized envelope parses as the daemon HookPayload.
    parse_payload(&params.to_string()).expect("normalized params parse as the daemon HookPayload");
}

// ---- RED — the daemon classifier branches on the harness tag (Codex routes Codex semantics) ------

#[test]
fn test_harness_tag_routes_codex_classifier() {
    // spec(§9.1) — the daemon `map_to_action_type` (the UNCHANGED loop's classifier) branches on the
    // HookPayload.harness tag: a codex-tagged payload classifies the RAW codex tool_name with the Codex
    // semantics (apply_patch → agent.file_edit), where the DEFAULT (claude, absent tag) would deny it
    // (apply_patch is not a Claude tool → UnmappedTool). This is the "handler needs a harness
    // discriminator" Step-2.5 design (a codex-spoofed harness can only ever pick a CONSERVATIVE
    // classifier — risk is catalog-authoritative + daemon-side — never a silent allow).
    let codex = parse_payload(
        &normalize_to_intercept_params(
            &parse_codex_payload(&codex_stdin("apply_patch")).unwrap(),
            SESS,
        )
        .to_string(),
    )
    .unwrap();
    assert_eq!(
        map_to_action_type(&codex),
        Ok("agent.file_edit"),
        "codex tag → Codex classifier"
    );

    // the DEFAULT (claude) path is byte-unchanged: a payload without a harness tag still classifies as
    // Claude — apply_patch is not a Claude tool → UnmappedTool deny (the Claude path is untouched).
    let claude_default = parse_payload(
        &serde_json::json!({
            "tool_name": "apply_patch",
            "tool_input": {},
            "session_id": SESS,
            "permission_mode": "default",
        })
        .to_string(),
    )
    .unwrap();
    assert_eq!(
        map_to_action_type(&claude_default),
        Err(DenyReason::UnmappedTool),
        "no harness tag ⇒ Claude classifier (unchanged) — apply_patch is unknown to Claude"
    );
}

// ---- RED #2 — allow-passes: a verdict Allow → the Codex `permissionDecision:"allow"` grammar -------

#[test]
fn test_codex_hook_allow_passes() {
    // spec(§4.1) — an Allow verdict emits Codex's `PreToolUse` allow output (the §4.1 grammar, identical
    // shape to Claude's hookSpecificOutput). `codex_verdict_output(Ok(true))` is the allow output.
    let allow = codex_allow_output();
    assert_eq!(allow["hookSpecificOutput"]["hookEventName"], "PreToolUse");
    assert_eq!(allow["hookSpecificOutput"]["permissionDecision"], "allow");
    // the fail-closed verdict mapping: only Ok(true) → allow.
    assert_eq!(
        codex_verdict_output(&Ok(true))["hookSpecificOutput"]["permissionDecision"],
        "allow"
    );
}

// ---- RED #1 — deny-blocks: a verdict Deny → the Codex `permissionDecision:"deny"` grammar ----------

#[test]
fn test_codex_hook_deny_blocks() {
    // spec(§15 INV-SEC-1 / §4.1) — a Deny verdict emits Codex's deny output (blocks the tool call). The
    // reason is carried in `permissionDecisionReason` (content-free §15).
    let deny = codex_deny_output("denied by policy");
    assert_eq!(deny["hookSpecificOutput"]["hookEventName"], "PreToolUse");
    assert_eq!(deny["hookSpecificOutput"]["permissionDecision"], "deny");
    assert_eq!(
        deny["hookSpecificOutput"]["permissionDecisionReason"], "denied by policy",
        "the deny reason rides permissionDecisionReason"
    );
}

// ---- RED #3/#4 — FAIL-CLOSED: any error (no daemon / parse-fail / timeout) OR a non-allow → deny ---

#[test]
fn test_codex_hook_fail_closed() {
    // spec(§15 / LESSON §26/§30) — the conduit defaults to DENY: a transport Err (no daemon /
    // unreachable socket / a read past the timeout) AND an explicit Ok(false) (a daemon Deny verdict)
    // BOTH emit the deny output; only Ok(true) opens the tool. The hook NEVER silently allows.
    let no_daemon: std::io::Result<bool> = Err(std::io::Error::other("connection refused"));
    let timeout: std::io::Result<bool> = Err(std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        "read timed out",
    ));
    let denied: std::io::Result<bool> = Ok(false);
    for r in [no_daemon, timeout, denied] {
        assert_eq!(
            codex_verdict_output(&r)["hookSpecificOutput"]["permissionDecision"],
            "deny",
            "any error OR a non-allow verdict fails closed to deny"
        );
    }
    // a malformed Codex stdin → parse fails closed (Malformed) → the run-path emits deny. Covers:
    // truncated JSON, missing `tool_name`, missing-while-present-tool_input, WRONG-type tool_name, empty.
    for bad in [
        "{ not json",
        "{}",
        r#"{ "tool_input": {} }"#,
        r#"{ "tool_name": 42, "tool_input": {} }"#,
        "",
    ] {
        assert_eq!(
            parse_codex_payload(bad).err(),
            Some(DenyReason::Malformed),
            "a malformed codex stdin `{bad}` fails closed (Malformed → deny)"
        );
    }
}

// ---- RED — the FULL reuse: a Codex mutating tool routes through the REAL Gateway (no live agent) ---

#[test]
fn test_codex_mutating_tool_routes_through_gateway_then_allow() {
    // spec(§15 INV-SEC-1 / §6.2) — the load-bearing reuse: a Codex `apply_patch` (→ agent.file_edit,
    // risk-2) normalized into the EXISTING `route_intercept` loop rests at AwaitingApproval (the human
    // decides — adjudication-only, NO daemon executor); when approved it terminates at Approved and
    // verdict_for_status(Approved) → Allow. Same chokepoint as Claude — only the envelope differed.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gw();
    let outcome = route_codex(&gw, &mut store, "apply_patch");
    let action_request_id = match outcome {
        InterceptOutcome::AwaitingApproval { action_request_id } => action_request_id,
        _ => panic!("a codex mutating tool awaits the human's decision (AwaitingApproval)"),
    };
    assert!(!action_request_id.is_empty(), "carries the action id");
    assert_eq!(action_status(&path).as_deref(), Some("awaiting_approval"));
    let appr = approval_id_of(&path);
    gw.approve(&mut store, &appr).expect("approve");
    assert_eq!(
        action_status(&path).as_deref(),
        Some("approved"),
        "an approved adjudication rests at Approved — NEVER queued/executing/succeeded"
    );
    assert!(matches!(
        verdict_for_status(AR::Approved),
        MutationVerdict::Allow
    ));
}

// ---- RED — an un-classified Codex tool is denied through the loop (fail-closed, audited path) ------

#[test]
fn test_codex_unclassified_tool_denied_through_loop() {
    // spec(§15 fail-closed) — a Codex tool the classifier doesn't map (update_plan) routes to a
    // Resolved(Deny) — the agent is BLOCKED, no action row queued/executing (the deny is at the ingress,
    // before submit; the sandbox is the backstop for anything that slips the hook).
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gw();
    let outcome = route_codex(&gw, &mut store, "update_plan");
    assert!(
        matches!(
            outcome,
            InterceptOutcome::Resolved(MutationVerdict::Deny { .. })
        ),
        "an un-classified codex tool fails closed to Deny"
    );
}

// =================================================================================================
// RED #8 — the `--sandbox` DEFENSE-IN-DEPTH (the cat-1 pin). Codex's PreToolUse is a GUARDRAIL, not a
// boundary (research §4.3) → the hook ALONE is not INV-SEC-1. The OS-enforcement boundary is
// `--sandbox workspace-write` (USER-CONFIRMED 2026-06-15) scoped to {the worktree + per-profile
// approved extra WRITE paths}, network-off. Grammar (Context7, codex rust-v0.75.0): workspace-write
// reads ALL files (reads NOT confined); only WRITES scope to cwd + `[sandbox_workspace_write]
// .writable_roots`; `network_access=false`. So test the WRITE boundary + network-off + no-bypass; do
// NOT assert read-confinement (the mode doesn't provide it, and INV-SEC-1 governs MUTATION).
// =================================================================================================

#[test]
fn test_sandbox_is_inv_sec_layer() {
    use nexusopsd::harness::codex::launch::CodexLaunchConfig;

    // a DAEMON-resolved codex_home (the 3.3b security NIT — never an agent-supplied path) + the worktree
    // cwd + a per-profile user-approved extra WRITE path; the daemon hook receiver = `<exe> hook
    // --harness codex` (the nexusops-codex-gate command).
    let codex_home = Path::new("/Users/daemon/Library/Application Support/NexusOps/codex");
    let worktree = Path::new("/work/proj/.worktrees/feature-x");
    let approved = vec!["/Users/daemon/.pyenv/shims".to_string()];
    let receiver = "/usr/local/bin/nexusopsd hook --harness codex";
    let cfg = CodexLaunchConfig::build(codex_home, worktree, &approved, receiver);

    // (1) the WRITE-containment policy (USER-CONFIRMED): workspace-write + network OFF + writable_roots
    // scoped to {worktree + approved}, never arbitrary.
    assert_eq!(
        cfg.sandbox_mode(),
        "workspace-write",
        "the USER-CONFIRMED containment mode"
    );
    assert!(
        !cfg.network_access(),
        "network-off default (pinned explicitly, not relied-on)"
    );
    assert!(
        cfg.writable_roots()
            .iter()
            .any(|r| r == &worktree.to_string_lossy()),
        "the worktree is a writable root"
    );
    assert!(
        cfg.writable_roots()
            .iter()
            .any(|r| r == "/Users/daemon/.pyenv/shims"),
        "the per-profile user-approved extra path is a writable root"
    );
    assert!(
        cfg.writable_roots()
            .iter()
            .all(|r| r == &worktree.to_string_lossy() || approved.contains(r)),
        "writable_roots is EXACTLY the worktree plus approved paths — never arbitrary"
    );

    // (2) NO bypass — never `--yolo`/`--dangerously-bypass-approvals-and-sandbox`/`--full-auto`
    // (workspace-write, not danger-full-access).
    assert!(
        !cfg.has_bypass(),
        "the containment config never bypasses the sandbox"
    );
    assert_ne!(cfg.sandbox_mode(), "danger-full-access");

    // (3) the hook-config wires PreToolUse → the nexusops-codex-gate command under the DAEMON-resolved
    // codex_home (+ a stable trusted_hash so Codex's tamper-check accepts the registered command).
    assert_eq!(
        cfg.codex_home(),
        codex_home,
        "codex_home is daemon-resolved, not agent-supplied"
    );
    assert!(
        cfg.references_receiver(receiver),
        "PreToolUse wired to the nexusops-codex-gate command"
    );
    assert!(
        !cfg.trusted_hash().is_empty(),
        "a [hooks.state] trusted_hash is registered for the command"
    );
    // the generated config document carries the sandbox + the PreToolUse hook wiring (the two layers).
    let doc = cfg.config_document();
    assert_eq!(doc["sandbox_mode"], "workspace-write");
    assert_eq!(doc["sandbox_workspace_write"]["network_access"], false);
    assert!(
        doc["hooks"]["PreToolUse"].is_array(),
        "PreToolUse hook is wired in the generated config (the adjudication ingress)"
    );
}

// ---- the no-bypass STRUCTURAL pin scoped to the new config generator -----------------------------

#[test]
fn test_codex_launch_config_no_bypass_literal() {
    // spec(§15) — the launch source emits NO bypass flag as a string literal (the codex_launch.rs #2
    // pin, extended to the NEW config generator surface). Grep the quoted form so doc-prose doesn't
    // false-alarm.
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/harness/codex/launch.rs"
    ))
    .unwrap();
    // grep the QUOTED forms (an actual emitted string literal), so the doc-comments that DOCUMENT the
    // forbidden flags/mode (backtick prose) don't false-alarm — only a real `push("…")`/return literal.
    for bypass in [
        "\"--yolo\"",
        "\"--dangerously-bypass",
        "\"--full-auto\"",
        "\"danger-full-access\"",
    ] {
        assert!(
            !src.contains(bypass),
            "launch source emits no {bypass} literal"
        );
    }
}

// =================================================================================================
// The 4 cat-1 security VERIFY-surface pins (Step-8 security-reviewer verifies these; PIN-1 is the
// REQUIRED ADD — the lead's sign-off condition for the B2 harness-discriminator design).
// =================================================================================================

// ---- PIN-1 (the by-construction crux) — the harness tag is trusted-binary-set, NOT agent-settable ---

#[test]
fn test_codex_harness_tag_not_agent_settable() {
    // spec(§15 INV-SEC-1, PIN-1) — the `harness` discriminator is the TRUSTED hook binary's, never
    // agent input. A Codex stdin that SPOOFS its own `harness` field (e.g. "claude", to dodge the
    // Codex classifier) is IGNORED: `CodexHookPayload` has NO `harness` field (the spoof is dropped at
    // parse), and `normalize_to_intercept_params` STAMPS "codex" (a literal). Routing therefore still
    // goes to the Codex classifier — the agent cannot flip the discriminator to weaken adjudication.
    let spoof = serde_json::json!({
        "turn_id": "t",
        "tool_name": "apply_patch",
        "tool_use_id": "c",
        "tool_input": { "command": "x" },
        "harness": "claude", // the spoof — MUST be ignored (CodexHookPayload has no such field)
    })
    .to_string();
    let payload =
        parse_codex_payload(&spoof).expect("parses — the spoofed harness field is dropped");
    let params = normalize_to_intercept_params(&payload, SESS);
    assert_eq!(
        params["harness"], "codex",
        "the tag is binary-stamped 'codex', NEVER the agent's spoofed 'claude'"
    );
    // and the stamped tag drives routing → the Codex classifier (apply_patch → agent.file_edit), NOT
    // the Claude classifier the spoof tried to force (which would UnmappedTool-deny apply_patch).
    let hook_payload = parse_payload(&params.to_string()).unwrap();
    assert_eq!(
        map_to_action_type(&hook_payload),
        Ok("agent.file_edit"),
        "the agent-spoofed tag did not flip the classifier — routing honors the binary-stamped 'codex'"
    );
}

// ---- PIN-2 — deny-unknown BOTH arms + ZERO Codex auto-allow (adversarial) ------------------------

#[test]
fn test_codex_zero_auto_allow_adversarial() {
    // spec(§15/§6.2, PIN-2) — ZERO Codex auto-allow: EVERY Codex-mapped tool is risk-2
    // (agent.bash/file_edit/mcp_tool) → AwaitingApproval (a human decides), NEVER Resolved(Allow) — no
    // risk-0 auto-allow path exists for Codex (UNLIKE Claude's agent.file_read/agent.todo_write). An
    // un-classified tool → Resolved(Deny). So no codex tool can EVER be allowed without a human.
    for tool in [
        "shell",
        "exec_command",
        "apply_patch",
        "mcp__codegraph__search",
    ] {
        let (_d, path) = temp_db();
        let mut store = open(&path);
        let gw = catalog_gw();
        assert!(
            matches!(
                route_codex(&gw, &mut store, tool),
                InterceptOutcome::AwaitingApproval { .. }
            ),
            "codex `{tool}` requires a human approval — NEVER an auto-allow"
        );
    }
    for tool in ["update_plan", "definitely_unknown_tool", ""] {
        let (_d, path) = temp_db();
        let mut store = open(&path);
        let gw = catalog_gw();
        assert!(
            matches!(
                route_codex(&gw, &mut store, tool),
                InterceptOutcome::Resolved(MutationVerdict::Deny { .. })
            ),
            "an un-classified codex `{tool}` fails closed to Deny (both arms: classifier + routed loop)"
        );
    }
}

// ---- PIN-4 — downstream genuinely SHARED/unchanged: audit-BEFORE-verdict fail-closed (§15 #5) ----

#[test]
fn test_codex_audit_before_verdict_fail_closed() {
    // spec(§15 #5, PIN-4) — the Codex envelope flows through the SAME audit-before-verdict gate: an
    // audit-write fault on the adjudication ActionRequested append rolls the submit back → Resolved(Deny),
    // never Allow/Await, and NO action row persists. Proves the shared loop's INV-SEC-1 core (submit/
    // decision_sink/verdict/audit-before-verdict) covers the Codex path — only the envelope differed.
    let (_d, path) = temp_db();
    let mut store = open(&path);
    let gw = catalog_gw();
    arm(FaultPoint::AuditEventWrite); // the next ActionRequested append fails
    let outcome = route_codex(&gw, &mut store, "apply_patch");
    assert!(
        matches!(
            outcome,
            InterceptOutcome::Resolved(MutationVerdict::Deny { .. })
        ),
        "an audit-write fault → Deny, never Allow/Await (§15 #5)"
    );
    assert_eq!(
        action_status(&path),
        None,
        "the txn rolled back — no un-audited action persisted (fail-closed)"
    );
}
