// Fixture data for the Usage projection (W2-usage 0.50 — the frozen 11-field UsageRow).
// §14 test/dev infrastructure, served THROUGH the boundary validator. Covers the
// load-bearing cases: a row with context (a %), an estimated-quality row, a
// no-context-metadata row (context_pct_max null → "unknown", §9.1/forbidden #4), and an
// `unavailable` row (value → "unknown", not 0; model null → the ledger_id label fallback).
// NO creditPool — the daemon has no credit-balance source (honest-omit, W2-usage).
import type { ProjectionDelta, UsageProjectionPage } from "../../contracts/index";

export const usageFixture: UsageProjectionPage = {
  projection: "UsageLedger",
  rows: [
    {
      ledger_id: "ledger_fixture_1",
      project_id: "project_fixture_1",
      session_id: "session_fixture_1",
      execution_profile_id: "ep_fixture_1",
      model: "claude-sonnet-4",
      bucket_day: "2026-06-26",
      tokens_in: 96000,
      tokens_out: 32000,
      context_pct_max: 64,
      cost_estimate: 1.92,
      metric_quality: "exact",
    },
    {
      ledger_id: "ledger_fixture_2",
      project_id: "project_fixture_1",
      session_id: "session_fixture_2",
      execution_profile_id: "ep_fixture_1",
      model: "claude-sonnet-4",
      bucket_day: "2026-06-26",
      tokens_in: 33000,
      tokens_out: 12000,
      context_pct_max: 22,
      cost_estimate: 0.68,
      metric_quality: "estimated",
    },
    {
      // no context metadata (§9.1 supportsContextMetadata=false) → null → "unknown" (forbidden #4)
      ledger_id: "ledger_fixture_3",
      project_id: "project_fixture_2",
      session_id: "session_fixture_3",
      execution_profile_id: "ep_fixture_2",
      model: "gpt-5-codex",
      bucket_day: "2026-06-26",
      tokens_in: 60000,
      tokens_out: 28000,
      context_pct_max: null,
      cost_estimate: 1.1,
      metric_quality: "exact",
    },
    {
      // unavailable → value renders "unknown", never 0; model null → the ledger_id label fallback (Q1)
      ledger_id: "ledger_fixture_4",
      project_id: "project_fixture_1",
      session_id: "session_fixture_4",
      execution_profile_id: "ep_fixture_1",
      model: null,
      bucket_day: "2026-06-26",
      tokens_in: null,
      tokens_out: null,
      context_pct_max: null,
      cost_estimate: null,
      metric_quality: "unavailable",
    },
  ],
  cursor: null,
};

// A daemon-shaped `row:None` NUDGE for the UsageLedger subscribe stream (ui-063). The daemon emits an
// id-LESS nudge (deltas_for_event, on TelemetrySampled — keyed `None`), NOT the row — so this carries
// NEITHER `row` NOR `id`. The live usage dashboard consumes it via refetch-on-nudge (re-read
// get_projection), never a row-apply reducer (which would no-op on the absent row — LESSON §29).
export const usageDeltaFixture: ProjectionDelta = {
  projection: "UsageLedger",
  kind: "upsert",
};
