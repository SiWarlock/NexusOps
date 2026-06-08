import { describe, it, expect } from "vitest";
import { buildUsageRows, creditPoolState } from "./model";
import type { UsageRow } from "../../contracts/index";

describe("usage view-model", () => {
  it("codex_context_is_unknown_not_a_number", () => {
    // forbidden #4: Codex reports no context metadata (§9.1 supportsContextMetadata
    // =false) — context is the literal "unknown", NEVER a number/0%, even if a
    // stray number arrives on a Codex row.
    const rows: UsageRow[] = [
      { subject_id: "x", harness: "codex", tokens: 100, cost: 1, metric_quality: "exact", context_pct: null },
      { subject_id: "y", harness: "codex", tokens: 100, cost: 1, metric_quality: "exact", context_pct: 42 },
    ];
    const [nullCtx, numCtx] = buildUsageRows(rows);
    expect(nullCtx!.contextDisplay).toBe("unknown");
    expect(numCtx!.contextDisplay).toBe("unknown");
  });

  it("claude_context_renders_percentage", () => {
    // the unknown rule is Codex/null-scoped, not blanket: a Claude row with a real
    // context_pct shows the percentage; a Claude row with null context → unknown.
    const [pct] = buildUsageRows([
      { subject_id: "z", harness: "claude", tokens: 100, cost: 1, metric_quality: "exact", context_pct: 64 },
    ]);
    expect(pct!.contextDisplay).toBe("64%");
    const [nullCtx] = buildUsageRows([
      { subject_id: "z2", harness: "claude", tokens: 100, cost: 1, metric_quality: "exact", context_pct: null },
    ]);
    expect(nullCtx!.contextDisplay).toBe("unknown");
    // a legitimate Claude 0% is "0%", NOT "unknown" (0 is a real value — the rule
    // keys on null/Codex via `== null`, not falsiness).
    const [zeroCtx] = buildUsageRows([
      { subject_id: "z3", harness: "claude", tokens: 100, cost: 1, metric_quality: "exact", context_pct: 0 },
    ]);
    expect(zeroCtx!.contextDisplay).toBe("0%");
  });

  it("accuracy_label_from_metric_quality", () => {
    // §11.7: the accuracy label is derived from metric_quality (present per row);
    // an `unavailable` usage shows "unknown" for the value, not 0/empty.
    const [ex, est, un] = buildUsageRows([
      { subject_id: "a", harness: "claude", tokens: 100, cost: 1.5, metric_quality: "exact", context_pct: 50 },
      { subject_id: "b", harness: "claude", tokens: 100, cost: 1.5, metric_quality: "estimated", context_pct: 50 },
      { subject_id: "c", harness: "claude", tokens: 0, cost: 0, metric_quality: "unavailable", context_pct: null },
    ]);
    expect(ex!.accuracyLabel).toBe("exact");
    expect(est!.accuracyLabel).toBe("estimated");
    expect(un!.accuracyLabel).toBe("unavailable");
    // unavailable usage → "unknown", never 0/empty
    expect(un!.tokensDisplay).toBe("unknown");
    expect(un!.costDisplay).toBe("unknown");
  });

  it("credit_pool_state_from_thresholds", () => {
    // normal / near_exhaustion (≤15% remaining) / hard_stop (exhausted); pure.
    expect(creditPoolState(100, 1000)).toBe("normal"); // 90% remaining
    expect(creditPoolState(900, 1000)).toBe("near_exhaustion"); // 10% remaining
    expect(creditPoolState(850, 1000)).toBe("near_exhaustion"); // 15% remaining (boundary)
    expect(creditPoolState(1000, 1000)).toBe("hard_stop"); // 0 remaining
    expect(creditPoolState(1100, 1000)).toBe("hard_stop"); // over the cap
  });
});
