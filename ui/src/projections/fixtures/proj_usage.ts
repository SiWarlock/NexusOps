// Fixture data for the Usage projection (PROVISIONAL — 6.4b). §14 test/dev
// infrastructure, served THROUGH the boundary validator. Covers the load-bearing
// cases: a Codex row (context_pct null → "unknown"), an estimated row, and an
// `unavailable` row (value → "unknown", not 0). The credit pool is near-exhaustion
// (870/1000 → 13% remaining) so the threshold's non-color channel is exercised.
import type { UsageProjectionPage } from "../../contracts/index";

export const usageFixture: UsageProjectionPage = {
  projection: "UsageLedger",
  rows: [
    {
      subject_id: "session_fixture_1",
      harness: "claude",
      tokens: 128000,
      cost: 1.92,
      metric_quality: "exact",
      context_pct: 64,
    },
    {
      subject_id: "session_fixture_2",
      harness: "claude",
      tokens: 45000,
      cost: 0.68,
      metric_quality: "estimated",
      context_pct: 22,
    },
    {
      // Codex → no context metadata (§9.1) → null → "unknown" (forbidden #4)
      subject_id: "session_fixture_3",
      harness: "codex",
      tokens: 88000,
      cost: 1.1,
      metric_quality: "exact",
      context_pct: null,
    },
    {
      // unavailable → value renders "unknown", never 0
      subject_id: "session_fixture_4",
      harness: "claude",
      tokens: 0,
      cost: 0,
      metric_quality: "unavailable",
      context_pct: null,
    },
  ],
  creditPool: { used: 870, limit: 1000 },
  cursor: null,
};
