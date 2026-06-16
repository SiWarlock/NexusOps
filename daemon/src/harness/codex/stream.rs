//! Codex `exec --json` `ThreadEvent` normalizer (§9.1) — the public NDJSON stream
//! (`thread.started`/`turn.*`/`item.*`/`error`, with `Usage` + `ThreadItemDetails`) → the SAME
//! normalized `CodexSignal`/tool-call/sample outputs as the rollout parser (`parse.rs`). The two Codex
//! vocabularies, ONE normalized surface (the §9.1 mirror). The public enum is CONFIRMED (research §3 /
//! `exec_events.rs`); the exact JSON wire shape is keyed TOLERANTLY on the `type` tag + the fields we
//! need — the OSS-version fixture refresh (research Open-Q #5) is the HITL follow-on. Untrusted input —
//! no `unwrap`/panic.

use nexusops_shared::harness::{MetricQuality, TelemetrySample};

use super::status::{CodexSignal, CodexToolKind};

/// The normalized observation of a parsed `ThreadEvent` stream — the overlap with [`super::parse::RolloutObservation`]
/// (status signals + telemetry samples + tool-calls). `session_meta`/`turn_context` are rollout-only
/// (the stream doesn't persist them); the stream `Usage` carries no `model_context_window`, so its
/// `context_pct` is None (a known rollout-only field — the research divergence).
#[derive(Debug, Default, PartialEq)]
pub struct StreamObservation {
    pub signals: Vec<CodexSignal>,
    pub samples: Vec<TelemetrySample>,
    pub tool_calls: Vec<CodexToolKind>,
}

/// Classify a `ThreadItemDetails` item `type` (the typed stream surface — unlike the rollout's
/// name-only `function_call`, the stream item is already semantic) → a tool kind, or `None` for a
/// non-tool item (an assistant message / reasoning).
fn classify_item(item_type: &str) -> Option<CodexToolKind> {
    match item_type {
        "command_execution" => Some(CodexToolKind::ShellExec),
        "file_change" => Some(CodexToolKind::FilePatch),
        "mcp_tool_call" => Some(CodexToolKind::McpTool),
        _ => None, // assistant_message / reasoning / … → not a tool call
    }
}

/// A `turn.completed` `Usage{input_tokens,cached_input_tokens,output_tokens,reasoning_output_tokens}`
/// → the frozen [`TelemetrySample`]. tokens_in=input, tokens_out=output (cached ⊆ input, reasoning ⊆
/// output — not double-counted, the rollout convention). The stream `Usage` carries no
/// `model_context_window` → `context_pct` None + `metric_quality` Unavailable (never faked, §11.4).
fn sample_from_usage(usage: &serde_json::Value) -> Option<TelemetrySample> {
    let tokens_in = usage.get("input_tokens")?.as_u64()?;
    let tokens_out = usage.get("output_tokens")?.as_u64()?;
    Some(TelemetrySample {
        tokens_in,
        tokens_out,
        context_pct: None, // the stream Usage has no model_context_window (rollout-only)
        cost_estimate: 0.0,
        metric_quality: MetricQuality::Unavailable,
    })
}

/// Normalize a `ThreadEvent` NDJSON stream → the [`StreamObservation`]. Tolerant: a malformed line or
/// unknown `type` is SKIPPED (forward-compat — never a panic). `thread.started`→SessionStarted ·
/// `turn.started`→TaskStarted · `item.completed`/`item.started`(tool)→ToolCall(kind) ·
/// `turn.completed`→sample(usage)+TurnCompleted · `turn.failed`/`error`→TurnAborted.
pub fn normalize_thread_events(ndjson: &str) -> StreamObservation {
    let mut obs = StreamObservation::default();
    for line in ndjson.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
            continue; // malformed line → skip (tolerant)
        };
        match event.get("type").and_then(|t| t.as_str()) {
            Some("thread.started") => obs.signals.push(CodexSignal::SessionStarted),
            Some("turn.started") => obs.signals.push(CodexSignal::TaskStarted),
            // count a tool call ONCE — on `item.completed` (the terminal item event), NOT also on
            // `item.started`, so a start+complete pair for one call does not double-count (which would
            // diverge from the rollout's one-record-per-`function_call`). An in-flight item with no
            // completion is simply not counted yet (observe-only). The real item lifecycle is validated
            // by the OSS-version fixture refresh (research Open-Q #5).
            Some("item.completed") => {
                let item_type = event
                    .get("item")
                    .and_then(|i| i.get("type"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("");
                if let Some(kind) = classify_item(item_type) {
                    obs.signals.push(CodexSignal::ToolCall(kind));
                    obs.tool_calls.push(kind);
                }
            }
            Some("turn.completed") => {
                if let Some(usage) = event.get("usage") {
                    if let Some(sample) = sample_from_usage(usage) {
                        obs.samples.push(sample);
                    }
                }
                obs.signals.push(CodexSignal::TurnCompleted);
            }
            Some("turn.failed") | Some("error") => obs.signals.push(CodexSignal::TurnAborted),
            _ => {} // thread.* / unknown → skip (tolerant)
        }
    }
    obs
}
