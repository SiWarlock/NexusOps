//! Codex rollout-JSONL parser (§9.1) — `{type,timestamp,payload}` records → the normalized §9.1
//! outputs. Tolerant of unknown record / inner `type` (forward-compat: skip, never panic), strict on
//! the fields it consumes. The on-disk schema is CONFIRMED (research §2.2); the OSS-version fixture
//! refresh (research Open-Q #5) is the HITL follow-on. Untrusted external input — no `unwrap`/panic.

use std::collections::HashMap;

use serde::Deserialize;

use nexusops_shared::harness::{MetricQuality, TelemetrySample};

use super::status::{CodexSignal, CodexToolKind};

/// `session_meta` (rollout line 1) — session identity + provenance (research §2.2). `id` is REQUIRED
/// (strict — a meta without it is malformed → skipped); the rest default-empty (tolerant). serde
/// ignores unknown fields by default (the payload carries more — timestamp, base_instructions, …).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    #[serde(default)]
    pub cwd: String,
    #[serde(default)]
    pub originator: String,
    #[serde(default)]
    pub cli_version: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub model_provider: String,
}

/// `turn_context` — the profile/policy the turn ran under (research §2.2; the #8 profile-binding
/// source 3.3b consumes). All default-tolerant; a richer `sandbox_policy` object in a future build
/// would fail this `String` field → the record is skipped (the OSS-fixture-refresh HITL re-pins it).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TurnContext {
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub approval_policy: String,
    #[serde(default)]
    pub sandbox_policy: String,
    #[serde(default)]
    pub permission_profile: String,
}

/// The normalized observation of a parsed rollout — the §9.1 outputs the adapter / 3.3b/c/d consume.
#[derive(Debug, Default, PartialEq)]
pub struct RolloutObservation {
    pub session_meta: Option<SessionMeta>,
    pub turn_context: Option<TurnContext>,
    /// the ordered structured signals (status folds over these via `derive_status`).
    pub signals: Vec<CodexSignal>,
    /// the per-`token_count` telemetry samples (parsed only — emission is 3.3d).
    pub samples: Vec<TelemetrySample>,
    /// the tool calls in order, classified by semantics.
    pub tool_calls: Vec<CodexToolKind>,
    /// whether a `patch_apply_end` (the post-mutation audit signal) was observed.
    pub had_mutation: bool,
}

/// Classify a tool name by SEMANTIC group (research §2.2 / the mirror table) — NOT a single hardcoded
/// vendor name, so OSS `codex` (`shell`/`local_shell`) and the desktop build (`exec_command`) both →
/// `ShellExec`. An MCP marker → `McpTool` (a bare MCP name like `query_docs` is reclassified via its
/// `mcp_tool_call_end` correlation in [`parse_rollout`]; the typed `item.mcp_tool_call` in the stream).
pub fn classify_tool(name: &str) -> CodexToolKind {
    match name {
        "exec_command" | "shell" | "local_shell" | "bash" => CodexToolKind::ShellExec,
        "apply_patch" => CodexToolKind::FilePatch,
        // a name-level MCP heuristic (catches `mcp_tool_call` / `mcp__*`-style names) — the WEAKER
        // tier: a BARE MCP name (`query_docs`) is NOT name-classifiable, so the AUTHORITATIVE signal is
        // the `mcp_tool_call_end` correlation in `parse_rollout` (it reclassifies Other→McpTool) / the
        // typed `item.mcp_tool_call` in the stream. This heuristic only helps the explicitly-named case.
        n if n.contains("mcp") => CodexToolKind::McpTool,
        _ => CodexToolKind::Other,
    }
}

/// Parse a `token_count` payload (`{type,info{last_token_usage,total_token_usage,model_context_window}}`)
/// → the frozen [`TelemetrySample`]. `tokens_in`=`input_tokens`, `tokens_out`=`output_tokens`
/// (`cached_input` ⊆ input, `reasoning_output` ⊆ output — the `total = in+out` convention; the superset
/// breakdowns are NOT double-counted). `context_pct` = `total_token_usage.total` / `model_context_window`
/// when both present (`metric_quality`=Exact); else None + Unavailable (never a faked number, §11.4).
/// `cost_estimate`=0.0 (Codex `token_count` carries no cost — cost derivation is the 3.3d/pricing
/// follow-on). Strict on `input_tokens`/`output_tokens` (missing → None, the record is skipped).
pub fn parse_token_usage(payload: &serde_json::Value) -> Option<TelemetrySample> {
    let info = payload.get("info")?;
    let last = info.get("last_token_usage")?;
    let tokens_in = last.get("input_tokens")?.as_u64()?;
    let tokens_out = last.get("output_tokens")?.as_u64()?;
    // the cumulative total drives the context gauge: prefer total_token_usage.total, else last.total.
    let total = info
        .get("total_token_usage")
        .and_then(|t| t.get("total_tokens"))
        .and_then(|v| v.as_u64())
        .or_else(|| last.get("total_tokens").and_then(|v| v.as_u64()));
    let window = info.get("model_context_window").and_then(|v| v.as_u64());
    let context_pct = match (total, window) {
        (Some(t), Some(w)) if w > 0 => Some((t as f64 / w as f64 * 100.0) as f32),
        _ => None,
    };
    let metric_quality = if context_pct.is_some() {
        MetricQuality::Exact
    } else {
        MetricQuality::Unavailable
    };
    Some(TelemetrySample {
        tokens_in,
        tokens_out,
        context_pct,
        cost_estimate: 0.0, // → 3.3d/pricing (Codex token_count carries no cost)
        metric_quality,
    })
}

/// Extract the session UUIDv7 from a rollout filename `rollout-<ISO8601>-<uuid>.jsonl` (research
/// §2.1). The trailing 36 chars of the stem are the UUID (8-4-4-4-12); validated by group lengths so
/// the dash-bearing ISO8601 prefix can't false-match. `None` if it doesn't validate.
pub fn session_id_from_rollout_path(path: &str) -> Option<String> {
    let file = path.rsplit('/').next().unwrap_or(path);
    let stem = file.strip_suffix(".jsonl").unwrap_or(file);
    if stem.len() < 36 {
        return None;
    }
    let candidate = &stem[stem.len() - 36..];
    let groups: Vec<&str> = candidate.split('-').collect();
    let ok = groups.len() == 5
        && [8, 4, 4, 4, 12]
            .iter()
            .zip(&groups)
            .all(|(want, g)| g.len() == *want && g.chars().all(|c| c.is_ascii_hexdigit()));
    ok.then(|| candidate.to_string())
}

/// The top-level rollout record envelope — every line is `{type, timestamp?, payload}`. Tolerant:
/// `payload` is a raw `Value` (the inner shape varies by `type`); a line that isn't this shape is skipped.
#[derive(Deserialize)]
struct RolloutRecord {
    #[serde(rename = "type")]
    record_type: String,
    payload: serde_json::Value,
}

/// Parse a rollout JSONL string → the normalized [`RolloutObservation`]. Tolerant: a malformed line
/// or unknown record/inner `type` is SKIPPED (forward-compat — never a panic, the research §2.2
/// posture); strict only on the fields each consumed record needs (a `session_meta` without `id` is
/// skipped). MCP correlation: a `function_call` whose `call_id` later appears in an `mcp_tool_call_end`
/// is reclassified `Other`→`McpTool` (a bare MCP name like `query_docs` isn't name-classifiable).
pub fn parse_rollout(jsonl: &str) -> RolloutObservation {
    let mut obs = RolloutObservation::default();
    // call_id → (signals index, tool_calls index) for the MCP reclassification correlation.
    let mut tool_call_index: HashMap<String, (usize, usize)> = HashMap::new();

    for line in jsonl.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(rec) = serde_json::from_str::<RolloutRecord>(line) else {
            continue; // malformed line → skip (tolerant)
        };
        let payload = &rec.payload;
        match rec.record_type.as_str() {
            "session_meta" => {
                if let Ok(meta) = serde_json::from_value::<SessionMeta>(payload.clone()) {
                    obs.session_meta = Some(meta);
                    obs.signals.push(CodexSignal::SessionStarted);
                }
            }
            "turn_context" => {
                if let Ok(tc) = serde_json::from_value::<TurnContext>(payload.clone()) {
                    obs.turn_context = Some(tc); // last-seen wins (the current turn's profile)
                }
            }
            "response_item" => match payload.get("type").and_then(|t| t.as_str()) {
                Some("function_call") | Some("custom_tool_call") => {
                    let name = payload.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let kind = classify_tool(name);
                    obs.signals.push(CodexSignal::ToolCall(kind));
                    obs.tool_calls.push(kind);
                    if let Some(cid) = payload.get("call_id").and_then(|c| c.as_str()) {
                        tool_call_index.insert(
                            cid.to_string(),
                            (obs.signals.len() - 1, obs.tool_calls.len() - 1),
                        );
                    }
                }
                _ => {} // message / reasoning / function_call_output → not a status/tool signal
            },
            "event_msg" => match payload.get("type").and_then(|t| t.as_str()) {
                Some("task_started") => obs.signals.push(CodexSignal::TaskStarted),
                Some("task_complete") => obs.signals.push(CodexSignal::TurnCompleted),
                Some("turn_aborted") => obs.signals.push(CodexSignal::TurnAborted),
                Some("token_count") => {
                    if let Some(sample) = parse_token_usage(payload) {
                        obs.samples.push(sample);
                    }
                }
                Some("patch_apply_end") => obs.had_mutation = true,
                Some("mcp_tool_call_end") => {
                    // the authoritative MCP signal: reclassify the correlated call (a bare MCP name
                    // classified `Other` at function_call time) → McpTool, or record an unseen call.
                    let by_id = payload
                        .get("call_id")
                        .and_then(|c| c.as_str())
                        .and_then(|cid| tool_call_index.get(cid).copied());
                    match by_id {
                        Some((si, ti)) => {
                            obs.signals[si] = CodexSignal::ToolCall(CodexToolKind::McpTool);
                            obs.tool_calls[ti] = CodexToolKind::McpTool;
                        }
                        None => {
                            obs.signals
                                .push(CodexSignal::ToolCall(CodexToolKind::McpTool));
                            obs.tool_calls.push(CodexToolKind::McpTool);
                        }
                    }
                }
                _ => {} // unknown event_msg type → skip (tolerant)
            },
            _ => {} // session_index / compacted / unknown record type → skip (tolerant)
        }
    }
    obs
}
