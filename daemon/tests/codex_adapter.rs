//! Phase 3.3a — the CodexAdapter observe core (parsing + normalization). RED-first.
//! ARCHITECTURE §9.1 (the `HarnessAdapter` trait + the frozen normalized types — the CodexAdapter
//! implements the SAME trait as Claude, no new trait), §5.1 (Codex events → `NormalizedStatus`,
//! derived from the event stream NEVER the PTY — safety #9). Foundation: docs/planning/0.3-codex-schema-research.md.
//!
//! NON-cat-1: this slice is the deterministic rollout-JSONL parser + the `exec --json` `ThreadEvent`
//! normalizer + the OBSERVE methods, tested against synthesized golden fixtures. Launch/interception/
//! telemetry-emission/live-drive are 3.3b/3.3c/3.3d/HITL.

use std::path::PathBuf;

use nexusops_shared::harness::MetricQuality;
use nexusops_shared::status::Session;

use nexusopsd::harness::codex::parse::{
    classify_tool, parse_rollout, parse_token_usage, session_id_from_rollout_path,
};
use nexusopsd::harness::codex::status::{derive_status, CodexSignal, CodexToolKind};
use nexusopsd::harness::codex::stream::normalize_thread_events;
use nexusopsd::harness::codex::{CodexAdapter, CODEX_CAPABILITIES};
use nexusopsd::harness::HarnessAdapter;

const ROLLOUT_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/codex/rollout-2026-05-29T17-26-18-019e75d8-6e9a-7893-839c-da1256d36380.jsonl"
);
const THREADEVENTS_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/codex/threadevents.ndjson"
);
const FIXTURE_UUID: &str = "019e75d8-6e9a-7893-839c-da1256d36380";

fn read_rollout() -> String {
    std::fs::read_to_string(ROLLOUT_FIXTURE).expect("read rollout fixture")
}

// ---- #1 — session_meta: identity + provenance; filename UUID == payload.id (§9.1 / research §2) ----

#[test]
fn test_parse_rollout_session_meta() {
    // spec(§9.1) — line-1 `session_meta` → session identity + provenance; the UUIDv7 from the filename
    // MUST equal `session_meta.payload.id` (the research §2.1 rollout identity).
    let obs = parse_rollout(&read_rollout());
    let meta = obs.session_meta.expect("session_meta parsed");
    assert_eq!(meta.id, FIXTURE_UUID);
    assert_eq!(meta.cwd, "/work/proj");
    assert_eq!(meta.model_provider, "openai");
    assert_eq!(meta.originator, "test-fixture");

    let from_name = session_id_from_rollout_path(ROLLOUT_FIXTURE).expect("uuid from filename");
    assert_eq!(
        from_name, meta.id,
        "filename UUID agrees with session_meta.id"
    );
    // bare filename (no `/` separator) still extracts the trailing UUID.
    assert_eq!(
        session_id_from_rollout_path(
            "rollout-2026-05-29T17-26-18-019e75d8-6e9a-7893-839c-da1256d36380.jsonl"
        ),
        Some(FIXTURE_UUID.to_string())
    );
    // None-returning paths: a too-short stem · a non-UUID trailing 36 chars · no `.jsonl`.
    assert_eq!(session_id_from_rollout_path("short.jsonl"), None);
    assert_eq!(
        session_id_from_rollout_path(
            "rollout-not-a-valid-uuid-here-zzzzzzzz-zzzz-zzzz-zzzz-zzzzzzzzzzzz.jsonl"
        ),
        None,
        "non-hex groups → None"
    );
}

// ---- #2 — turn_context profile (#8 profile-binding source; research §2.2) -----------------------

#[test]
fn test_parse_turn_context_profile() {
    // spec(§9.1) — `turn_context` records the (model, approval_policy, sandbox_policy,
    // permission_profile) the turn ran under (the #8 profile-binding source 3.3b consumes).
    let obs = parse_rollout(&read_rollout());
    let tc = obs.turn_context.expect("turn_context parsed");
    assert_eq!(tc.model, "gpt-5.5");
    assert_eq!(tc.approval_policy, "on-request");
    assert_eq!(tc.sandbox_policy, "workspace-write");
    assert_eq!(tc.permission_profile, "default");
}

// ---- #3 — derive_status: the pure fold over the event stream (§5.1 / #9 / LESSON 25) ------------

#[test]
fn test_derive_status_from_events() {
    // spec(§5.1) — a pure (prev, CodexSignal) -> Session fold (the 25 `derive_status` precedent), from
    // the event stream NEVER the PTY (#9). session-start→Starting · task-started→Active · shell-exec→
    // RunningCommand · apply_patch→EditingFiles · turn-complete→Idle. R-9 terminal-sink: a terminal
    // prev is held. NOTE (Q1): turn-end states (complete/aborted/failed) → Idle (the session is alive,
    // awaiting the next turn); terminal Failed/Completed derive from the PROCESS-exit signal (3.3b/c),
    // NOT a turn boundary — so a turn abort must NOT terminally kill a recoverable multi-turn session.
    assert_eq!(
        derive_status(Session::Creating, &CodexSignal::SessionStarted),
        Session::Starting
    );
    assert_eq!(
        derive_status(Session::Starting, &CodexSignal::TaskStarted),
        Session::Active
    );
    assert_eq!(
        derive_status(
            Session::Active,
            &CodexSignal::ToolCall(CodexToolKind::ShellExec)
        ),
        Session::RunningCommand
    );
    assert_eq!(
        derive_status(
            Session::Active,
            &CodexSignal::ToolCall(CodexToolKind::FilePatch)
        ),
        Session::EditingFiles
    );
    assert_eq!(
        derive_status(
            Session::Active,
            &CodexSignal::ToolCall(CodexToolKind::McpTool)
        ),
        Session::Active
    );
    assert_eq!(
        derive_status(Session::RunningCommand, &CodexSignal::TurnCompleted),
        Session::Idle
    );
    // Q1: a turn abort/fail is a turn boundary, NOT session death → Idle (session alive).
    assert_eq!(
        derive_status(Session::Active, &CodexSignal::TurnAborted),
        Session::Idle
    );
    // R-9 terminal-sink: a terminal prev is held, never resurrected/transitioned.
    assert_eq!(
        derive_status(Session::Completed, &CodexSignal::TaskStarted),
        Session::Completed
    );
}

// ---- #4 — status derives from the structured event stream, NEVER PTY bytes (#9, structural) -----

#[test]
fn test_status_no_pty_scrape() {
    // safety #9 (the 25/28 structural grep-pin idiom, scoped to codex/status.rs — the pure fold): the
    // status path reads NO PTY/TerminalSession output to derive status. Sibling parser/stream I/O can
    // not false-alarm it (the scan is this file only).
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/harness/codex/status.rs"
    ))
    .unwrap();
    for tok in [
        "TerminalSession",
        "read_step",
        "PtyRead",
        ".pump(",
        "pty.read",
    ] {
        assert!(
            !src.contains(tok),
            "codex status path must not read PTY output (`{tok}`) — #9 display-only"
        );
    }
}

// ---- #5 — token_count.last_token_usage → TelemetrySample (§9.1/§18) -----------------------------

#[test]
fn test_parse_token_count_usage() {
    // spec(§9.1) — `token_count.info.last_token_usage` → the frozen TelemetrySample. tokens_in =
    // input_tokens, tokens_out = output_tokens (cached_input ⊆ input, reasoning_output ⊆ output — the
    // total = in+out convention; the superset breakdowns are not double-counted). context_pct =
    // total_token_usage.total / model_context_window (Q4: Exact when the window is present).
    let payload = serde_json::json!({
        "type": "token_count",
        "info": {
            "last_token_usage": {
                "input_tokens": 1000, "cached_input_tokens": 200,
                "output_tokens": 300, "reasoning_output_tokens": 50, "total_tokens": 1300
            },
            "total_token_usage": {
                "input_tokens": 1000, "cached_input_tokens": 200,
                "output_tokens": 300, "reasoning_output_tokens": 50, "total_tokens": 1300
            },
            "model_context_window": 272000
        }
    });
    let s = parse_token_usage(&payload).expect("token_count → sample");
    assert_eq!(
        s.tokens_in, 1000,
        "tokens_in = input_tokens (cached_input ⊆ input)"
    );
    assert_eq!(
        s.tokens_out, 300,
        "tokens_out = output_tokens (reasoning_output ⊆ output)"
    );
    let pct = s
        .context_pct
        .expect("context_pct computed from model_context_window");
    assert!(
        (pct - (1300.0 / 272000.0 * 100.0)).abs() < 0.001,
        "context_pct = total/window"
    );
    assert_eq!(
        s.metric_quality,
        MetricQuality::Exact,
        "window present → Exact"
    );

    // no model_context_window → context unknown for this sample (tokens still parsed).
    let no_window = serde_json::json!({
        "type": "token_count",
        "info": { "last_token_usage": { "input_tokens": 5, "output_tokens": 7, "total_tokens": 12 } }
    });
    let s2 = parse_token_usage(&no_window).expect("token_count → sample (no window)");
    assert_eq!(s2.tokens_in, 5);
    assert_eq!(s2.tokens_out, 7);
    assert_eq!(
        s2.context_pct, None,
        "no window → context_pct None, never faked"
    );
    assert_eq!(s2.metric_quality, MetricQuality::Unavailable);
}

// ---- #6 — tool-call classification BY SEMANTICS, not hardcoded names (research §2.2 / mirror) ----

#[test]
fn test_classify_tool_call_by_semantics() {
    // spec(§9.1) — classify by the SEMANTIC group, so OSS (`shell`/`local_shell`) and desktop
    // (`exec_command`) names both → ShellExec; `apply_patch` → FilePatch; an MCP marker → McpTool;
    // everything else → Other. (The research mirror table: OSS-vs-desktop name divergence.)
    assert_eq!(classify_tool("exec_command"), CodexToolKind::ShellExec);
    assert_eq!(classify_tool("shell"), CodexToolKind::ShellExec);
    assert_eq!(classify_tool("local_shell"), CodexToolKind::ShellExec);
    assert_eq!(classify_tool("apply_patch"), CodexToolKind::FilePatch);
    assert_eq!(classify_tool("mcp_tool_call"), CodexToolKind::McpTool);
    assert_eq!(classify_tool("request_user_input"), CodexToolKind::Other);
}

// ---- #7 — the ThreadEvent normalizer produces the SAME outputs as the rollout parser -----------

#[test]
fn test_threadevent_normalizer_matches_rollout() {
    // spec(§9.1) — the two Codex vocabularies (rollout `event_msg`/`response_item` vs `exec --json`
    // `ThreadEvent`) normalize to the SAME status/usage/tool-call. Overlap pinned: the tool-call kinds
    // + tokens_in/out + the folded final status agree. (context_pct may differ — the stream `Usage`
    // carries no `model_context_window`; a known rollout-only field, the research divergence.)
    let rollout = parse_rollout(&read_rollout());
    let stream = normalize_thread_events(
        &std::fs::read_to_string(THREADEVENTS_FIXTURE).expect("read threadevents fixture"),
    );

    assert_eq!(
        rollout.tool_calls,
        vec![
            CodexToolKind::ShellExec,
            CodexToolKind::FilePatch,
            CodexToolKind::McpTool
        ],
        "rollout tool-calls"
    );
    assert_eq!(
        stream.tool_calls, rollout.tool_calls,
        "stream tool-calls == rollout"
    );

    let r_sample = rollout.samples.last().expect("rollout sample");
    let s_sample = stream.samples.last().expect("stream sample");
    assert_eq!(s_sample.tokens_in, r_sample.tokens_in, "tokens_in agree");
    assert_eq!(s_sample.tokens_out, r_sample.tokens_out, "tokens_out agree");

    assert_eq!(
        fold(&rollout.signals),
        fold(&stream.signals),
        "the folded final status agrees across both vocabularies"
    );
}

fn fold(signals: &[CodexSignal]) -> Session {
    signals.iter().fold(Session::Creating, derive_status)
}

// ---- #8 — read_transcript + capabilities (§9.1) ------------------------------------------------

#[test]
fn test_transcript_ref_and_capabilities() {
    // spec(§9.1) — read_transcript() → the rollout TranscriptRef (path + is_in_place); capabilities()
    // → the Codex 10-field matrix (supports_resume/transcript_read/context_metadata=true [Codex HAS
    // model_context_window]; supports_subagents=false [no subagent concept — the coverage matrix n/a]).
    let adapter = CodexAdapter::new(PathBuf::from(ROLLOUT_FIXTURE), FIXTURE_UUID.to_string());
    let t = adapter.read_transcript().expect("transcript ref");
    assert_eq!(t.path, ROLLOUT_FIXTURE);
    assert!(
        t.is_in_place,
        "Codex writes its own rollout we read in place"
    );

    let caps = adapter.capabilities();
    assert!(caps.supports_resume);
    assert!(caps.supports_transcript_read);
    assert!(
        caps.supports_context_metadata,
        "Codex reports model_context_window"
    );
    assert!(!caps.supports_subagents, "Codex has no subagent concept");
    assert_eq!(caps, CODEX_CAPABILITIES);
}

// ---- #9 — the committed golden fixture parses end-to-end to the expected normalized outputs -----

#[test]
fn test_golden_fixture_parse() {
    // spec(§9.1) — the integration pin: the committed redacted rollout fixture parses end-to-end. The
    // unknown trailing record is tolerated + skipped (forward-compat, never a panic). The mutation
    // signal (patch_apply_end) is observed; the folded status ends Idle (the turn completed).
    let obs = parse_rollout(&read_rollout());
    assert!(obs.session_meta.is_some());
    assert!(obs.turn_context.is_some());
    assert_eq!(obs.samples.len(), 1, "one token_count → one sample");
    assert_eq!(
        obs.tool_calls,
        vec![
            CodexToolKind::ShellExec,
            CodexToolKind::FilePatch,
            CodexToolKind::McpTool
        ]
    );
    assert!(
        obs.had_mutation,
        "patch_apply_end is the post-mutation audit signal"
    );
    assert_eq!(
        fold(&obs.signals),
        Session::Idle,
        "the turn completed → Idle"
    );
}

// ---- #10 — the push_signal → stream_status ingestion seam (the 3.3c live-stream feed point) ------

#[test]
fn test_push_signal_updates_stream_status() {
    // spec(§9.1/§5.1) — the adapter's status seam: push_signal advances stream_status via derive_status
    // (the round-trip the 3.3c live stream feeds). launch() is the Creating→Starting marker; a pushed
    // shell-exec → RunningCommand; a turn-complete → Idle. NEVER from PTY bytes (#9).
    let mut adapter = CodexAdapter::new(PathBuf::from(ROLLOUT_FIXTURE), FIXTURE_UUID.to_string());
    assert_eq!(adapter.launch(), Session::Starting);
    assert_eq!(adapter.stream_status(), Session::Starting);
    adapter.push_signal(CodexSignal::ToolCall(CodexToolKind::ShellExec));
    assert_eq!(
        adapter.stream_status(),
        Session::RunningCommand,
        "the pushed signal advanced status"
    );
    adapter.push_signal(CodexSignal::TurnCompleted);
    assert_eq!(adapter.stream_status(), Session::Idle);
}
