# /tdd brief — codex_adapter_parsing_and_normalization

## Feature
The deterministic **CodexAdapter observe core**: a rollout-JSONL parser + an `exec --json` `ThreadEvent`
normalizer that map Codex's persisted/streamed records onto the FROZEN §9.1 `HarnessAdapter` normalized
types, implementing the OBSERVE trait methods (`capabilities`/`stream_status`/`read_transcript`). Pure
parsing/normalization, test-first against **golden fixtures** (redacted real rollouts + NDJSON). **NON-cat-1**
— no live agent, no launch, no interception (those are 3.3b/3.3c). The first slice of the 3.3 Codex arc
(mirrors the Claude 042 observe path).

## Use case + traceability
- **Task ID:** P3.3a (the head of the 3.3 decomposition — see the 3.3 section of `IMPLEMENTATION_PLAN.md`)
- **Architecture sections it implements:** `ARCHITECTURE.md §9.1` (the `HarnessAdapter` trait + the
  normalized types — `NormalizedStatus`/`TelemetrySample`/`TranscriptRef`/`HarnessCapabilities`), **§5.1**
  (the Session status machine — Codex events → `NormalizedStatus`, derived from the event stream NEVER the
  PTY, safety #9). Session identity = the rollout UUIDv7.
- **THE FOUNDATION (read it first):** `docs/planning/0.3-codex-schema-research.md` — **the rollout record
  shapes** (CONFIRMED-LOCAL), the **`exec --json` `ThreadEvent` stream**, the **Claude-model mirror/diverge
  table**, and the **deterministic-now vs HITL** split (this slice = the "Rollout parser + ThreadEvent
  normalizer" bullets). The on-disk JSONL + the public `ThreadEvent` enum are CONFIRMED.
- **Related context:** `daemon/src/harness/claude/{mod,status}.rs` (the 042 observe precedent — `derive_status`
  as a pure fold; the CodexAdapter mirrors the structure, different inputs), `daemon/src/harness/mod.rs` (the
  frozen §9.1 `HarnessAdapter` trait — the CodexAdapter implements the SAME trait, no new trait;
  `FakeHarness` precedent), `shared/src/harness.rs` (the frozen `TelemetrySample`/`MetricQuality`/
  `TranscriptRef`/`HarnessCapabilities`), LESSON 23 (telemetry = observation; the shared-contract seam data types),
  25 (the PTY-primary adapter — status = a pure fold over a structured-signal enum, never PTY bytes #9).

## Scope ruling (restate at Step 0)
- **BUILD (deterministic observe core):** the rollout-JSONL parser + the `ThreadEvent` normalizer → the §9.1
  normalized types + the OBSERVE methods (`capabilities`/`stream_status`/`read_transcript`). Golden fixtures.
- **DEFER (the rest of the 3.3 arc):** launch + auth/profile binding + umask + ResumeMode-handle → **3.3b** ·
  the CAT-1 PreToolUse interception + `--sandbox` → **3.3c** · telemetry emission → **3.3d** · the LIVE drive
  loop → the HITL follow-on. This slice touches NO launch/interception/emission path (NON-cat-1 by construction).
- **Target the OSS `codex` surfaces** (`exec --json` `ThreadEvent` + the rollout format), NOT the desktop-build
  tool names — the local fixtures are the desktop build (cli 0.133.0, AHEAD of OSS); key on **semantics**, and
  note the OSS-version fixture refresh as the HITL item (the research build-implications / Open-Q #5).

## Acceptance criteria (what "done" means)
- [ ] A **rollout-JSONL parser** over `{type,timestamp,payload}` records (the research rollout shapes): `session_meta`
  (session identity + provenance) · `turn_context` (the per-turn profile/policy) · `response_item`
  (`function_call`/`custom_tool_call`/`message`/`reasoning`) · `event_msg` (`token_count`/`patch_apply_end`/
  `task_started`/`task_complete`/`turn_aborted`). Tolerant of unknown record/`type` (forward-compat — log+skip,
  never panic), strict on the fields it consumes.
- [ ] A **`ThreadEvent` (`exec --json`) normalizer** over the public stream (`thread.started`/`turn.*`/
  `item.*`/`error` + `Usage` + `ThreadItemDetails`), producing the SAME normalized outputs as the rollout
  parser where they overlap (status/usage/tool-call).
- [ ] **`NormalizedStatus`** derived from the event stream (`task_started`→Working/Streaming · `task_complete`/
  `turn.completed`→Idle/Completed · `turn_aborted`/`turn.failed`→the §5.1 state; fail-closed default) — a pure
  fold (the 25 `derive_status` precedent), **NEVER from PTY bytes** (#9; structurally pinned).
- [ ] **`TelemetrySample`** parsed from `token_count.info.last_token_usage` (input/cached_input/output/
  reasoning_output/total) + `model_context_window` → the frozen `TelemetrySample{tokens_in,tokens_out,
  context_pct,cost_estimate,metric_quality}` (the Codex `reasoning_output`+`cached_input` are a superset — map
  them; context_pct from `model_context_window`; metric_quality per availability). **Parsed only — emission is 3.3d.**
- [ ] **Tool-call + mutation classification BY SEMANTICS** (NOT hardcoded names): shell-exec (`function_call`
  `exec_command`/`shell`/`local_shell`) · file-patch (`custom_tool_call`/`apply_patch` → the `patch_apply_end`
  mutation signal) · mcp (`mcp_tool_call_*`). A `CodexToolKind` (or similar) keyed on semantics so OSS-vs-desktop
  names both map.
- [ ] **`TranscriptRef`** (the rollout path + hash + `is_in_place`) + **session identity** (the UUIDv7 from the
  filename AND `session_meta.payload.id` — assert they agree) + **profile-from-`turn_context`** (the
  `(model,approval_policy,sandbox_policy,permission_profile,…)` the turn ran under).
- [ ] **`capabilities()`** returns the Codex `HarnessCapabilities` (the 10 PRD-HARN-5 fields — e.g.
  supportsResume=true, supportsTranscriptRead=true, supportsContextMetadata per `model_context_window`,
  supportsCommandInjection, etc.; the per-capability UI-degradation driver).
- [ ] **Golden fixtures** committed: ≥1 **redacted** real rollout JSONL + ≥1 NDJSON `ThreadEvent` stream
  (secrets/paths scrubbed — NEVER commit real secrets; the research doc's redaction discipline). Tests parse
  the fixtures + assert the normalized outputs.
- [ ] All unit tests in `daemon/tests/codex_adapter.rs` (+ `daemon/src/harness/codex/` `#[cfg(test)]`) pass;
  `/preflight` clean. **NO CONTRACT bump** (the CodexAdapter consumes the already-frozen §9.1 types; daemon-internal).

## Wiring / entry point (Step 7.5)
**none — wiring lands in 3.3b/3.3c** (the production drive caller — the launch + the live stream + the
interception — is the later slices; this slice is the deterministic parser/normalizer + the observe methods,
exercised by the golden-fixture tests). The CodexAdapter is constructed but has no production launch caller yet
(the 042 "observe path built, live caller later" precedent). Confirm the parser/normalizer are reachable from
the observe trait methods (the seam the 3.3c live stream will feed).

## Files expected to touch
**New:**
- `daemon/src/harness/codex/mod.rs` — the `CodexAdapter` + the §9.1 trait impl (observe methods).
- `daemon/src/harness/codex/parse.rs` — the rollout-JSONL record types + parser.
- `daemon/src/harness/codex/stream.rs` — the `ThreadEvent` normalizer.
- `daemon/src/harness/codex/status.rs` — the pure `derive_status` fold (Codex events → §5.1).
- `daemon/tests/codex_adapter.rs` + `daemon/tests/fixtures/codex/` (the redacted golden fixtures).

**Modified:**
- `daemon/src/harness/mod.rs` — `pub mod codex` + re-exports.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
1. **`test_parse_rollout_session_meta`** — line-1 `session_meta` → session identity + provenance; the UUIDv7
   from filename == `session_meta.payload.id`. Why: the rollout UUIDv7 session identity / the research rollout shapes.
2. **`test_parse_turn_context_profile`** — `turn_context` → the profile/policy `(model,approval_policy,
   sandbox_policy,permission_profile)`. Why: the #8 profile-binding invariant (3.3b consumes it; the research rollout+auth shapes).
3. **`test_derive_status_from_events`** — `task_started`→working · `task_complete`/`turn.completed`→completed/idle
   · `turn_aborted`/`turn.failed`→the §5.1 state; a pure fold; fail-closed default. Why: §5.1 / 25 / #9.
4. **`test_status_no_pty_scrape`** — the status module exposes NO PTY-byte-derived status API (structural grep,
   the 25/28 idiom). Why: safety #9.
5. **`test_parse_token_count_usage`** — `token_count.info.last_token_usage` → `TelemetrySample` (input/output
   mapped; reasoning_output+cached_input superset handled; context_pct from `model_context_window`). Why: §9.1/§18.
6. **`test_classify_tool_call_by_semantics`** — `exec_command`/`shell`/`local_shell`→shell · `apply_patch`
   (`custom_tool_call`)→file-patch + `patch_apply_end`→mutation · `mcp_tool_call`→mcp; OSS + desktop names both
   classify. Why: the research rollout-shapes + mirror table (the OSS-vs-desktop divergence; semantics not names).
7. **`test_threadevent_normalizer_matches_rollout`** — the `exec --json` `ThreadEvent` stream normalizes to the
   SAME status/usage/tool-call as the rollout parser (the two vocabularies, one normalized output). Why: the research `exec --json` stream.
8. **`test_transcript_ref_and_capabilities`** — `read_transcript()` → the rollout `TranscriptRef`;
   `capabilities()` → the Codex 10-field `HarnessCapabilities`. Why: §9.1.
9. **`test_golden_fixture_parse`** — a committed redacted real rollout fixture parses end-to-end to the expected
   normalized outputs (the integration pin). Why: the research build-implications (fixtures are the contract surface).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** **none** in `shared/` — the CodexAdapter consumes the FROZEN §9.1 types; the Codex
  record types + the `CodexToolKind` are **daemon-internal** (`daemon/src/harness/codex/`). **NO CONTRACT bump.**
- **Shared-contract seam model touched?** **No** (daemon-internal parser; the §9.1 consumer types are frozen).
- **Orchestrator doc rows to write hot (Step 9):** a §9.1 AS-BUILT note (the CodexAdapter observe core LIVE,
  fixture-tested; the 3.3 arc's launch/intercept/telemetry deferred to 3.3b/c/d) + the `daemon/CLAUDE.md`
  module-org `harness/codex/` row. Orchestrator-written.

## Things to flag at Step 2.5
1. **`NormalizedStatus` mapping table — which Codex events → which §5.1 states?** The §5.1 Session machine has
   17 states; Codex emits `task_started`/`task_complete`/`turn_aborted` + `ThreadEvent` `turn.started/completed/
   failed`. My default vote: a small explicit mapping (task_started→Working/Streaming · task_complete/turn.completed→
   Idle [or Completed on session end] · turn_aborted/turn.failed→Failed; fail-closed default) — surface the exact
   table for review (it's the §5.1 binding). Confirm the Idle-vs-Completed boundary (a turn ending ≠ the session ending).
2. **Tool-call classification shape — a `CodexToolKind` enum keyed on semantics.** My default vote: an enum
   `{ShellExec, FilePatch, McpTool, Other}` classified by a semantics predicate (not a name match) — so OSS
   `shell`/`local_shell` and desktop `exec_command` both → ShellExec. Confirm; this is what 3.3c's interception keys on.
3. **Fixture provenance + redaction.** Commit ≥1 redacted real rollout + ≥1 NDJSON ThreadEvent as golden fixtures.
   My default vote: hand-authored minimal fixtures derived from the research rollout shapes (NOT the machine's real
   rollouts — those carry real paths/output) — synthesize representative records, redaction-safe by construction.
   Confirm (synthesize vs scrub-real). **NEVER commit real secrets/paths.**
4. **`context_pct` / `metric_quality` for Codex.** Codex gives `model_context_window` + `last_token_usage.total`
   → context_pct is computable (unlike the research's earlier "unknown"). My default vote: compute context_pct =
   total/model_context_window when both present (metric_quality=exact), else Unavailable. Confirm.

## Dependencies + sequencing
- **Depends on:** 3.1 (✅ the frozen §9.1 `HarnessAdapter` trait + the normalized types), the 0.3 research (✅).
  NOT a live Codex (this slice is fixtures-only).
- **Blocks:** 3.3b (launch/auth/umask — consumes the parsed profile/identity) · 3.3c (the CAT-1 interception —
  consumes the tool-call classification) · 3.3d (telemetry emission — consumes the parsed `TelemetrySample`).

## Estimated commit count
**1–2.** Likely **1** (the parser + normalizer + status fold + the observe methods + fixtures are one cohesive
`harness/codex/` add, NON-cat-1). Split to 2 (parser/rollout · stream/normalizer) only if it grows large. No
safety-critical pin (NON-cat-1 — no launch/interception; #9 is structurally pinned by test #4).

## Reviewer subagents (Step 8 policy)
- **`security-reviewer`:** the policy is `invariant`. This slice is NON-cat-1 + touches no launch/interception/
  emission path — but it DOES establish the #9 boundary (status from the event stream, never PTY bytes) and the
  parser handles untrusted external input (rollout/stream — input-validation). My call: **YES, light** — confirm
  the #9 structural boundary (test #4) + the parser's reject-unknown/no-panic on malformed input (untrusted
  external ingress). Not a cat-1 design (no interception here). _(The CAT-1 security pass is 3.3c.)_
- **`code-quality-reviewer`: YES** (every-slice).

## Lessons-logged candidates anticipated
- **Convention candidate** — "a second harness adapter (Codex) implements the SAME frozen §9.1 trait as the
  first (Claude) — no new trait; the divergence is the PARSER (Codex rollout-JSONL + `exec --json` ThreadEvent
  vs Claude hook signals) normalized to the shared types; classify tool calls BY SEMANTICS not names (OSS-vs-
  vendor-build name divergence); status is a pure fold over the structured event stream, never PTY bytes (#9);
  build the deterministic parser/normalizer test-first vs redacted golden fixtures, the live drive = HITL."
- **Architecture-doc note candidate** — §9.1 AS-BUILT (the CodexAdapter observe core LIVE; the arc's
  launch/intercept/telemetry/live-drive deferred to 3.3b/c/d/HITL).
- **Future TODO** — the OSS-version fixture refresh (HITL; local fixtures are the desktop build); the
  UUID↔`thr_` session-id interconversion (research Open-Q #4).

## How to invoke
1. **Read this brief + the 0.3 research doc (`docs/planning/0.3-codex-schema-research.md` — the rollout / stream / mirror / build-implications sections) end-to-end.**
2. **Run `/tdd codex_adapter_parsing_and_normalization`**.
3. **Step 0 (Restate)** — confirm the NON-cat-1 observe-core scope (parser + normalizer; launch/intercept/
   telemetry/live deferred to 3.3b/c/d/HITL).
4. **Step 2.5** — send the Asserts/coverage write-up + answers to the 4 design questions (Q1 the §5.1 status
   mapping table is the load-bearing one). Don't go GREEN until APPROVED.
5. **Step 8** — `security-reviewer` (light — #9 boundary + untrusted-input parsing) + `code-quality-reviewer`.
6. **Step 9 (summarize)** — surface flags + the §9.1 AS-BUILT for orchestrator hot-routing.
