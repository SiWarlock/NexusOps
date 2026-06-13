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
    // Retargeted to the kind-aware signature — the SDK pool keeps the prior behavior.
    expect(creditPoolState(100, 1000, "sdk")).toBe("normal"); // 90% remaining
    expect(creditPoolState(900, 1000, "sdk")).toBe("near_exhaustion"); // 10% remaining
    expect(creditPoolState(850, 1000, "sdk")).toBe("near_exhaustion"); // 15% remaining (boundary)
    expect(creditPoolState(1000, 1000, "sdk")).toBe("hard_stop"); // 0 remaining
    expect(creditPoolState(1100, 1000, "sdk")).toBe("hard_stop"); // over the cap
  });

  it("credit_pool_sdk_exhaustion_is_hard_stop", () => {
    // spec(§9.1) — the capped monthly SDK/-p pool has NO fallback → hard_stop at
    // exhaustion (incl. the degenerate no/unknown-limit input).
    expect(creditPoolState(1000, 1000, "sdk")).toBe("hard_stop"); // 0 remaining
    expect(creditPoolState(1100, 1000, "sdk")).toBe("hard_stop"); // over the cap
    expect(creditPoolState(5, 0, "sdk")).toBe("hard_stop"); // no/unknown pool
  });

  it("credit_pool_interactive_exhaustion_is_never_hard_stop", () => {
    // spec(§9.1) — the interactive pool auto-resets (rolling window) → NEVER
    // hard_stop; exhaustion is the recoverable near_exhaustion signal.
    expect(creditPoolState(1000, 1000, "interactive")).toBe("near_exhaustion"); // 0 remaining
    expect(creditPoolState(1100, 1000, "interactive")).toBe("near_exhaustion"); // past the window
    expect(creditPoolState(5, 0, "interactive")).toBe("near_exhaustion"); // no/unknown limit
  });

  it("credit_pool_near_exhaustion_both_kinds", () => {
    // spec(§11.4) — the ≤15%-remaining warning is kind-INDEPENDENT (only hard_stop is kind-gated).
    for (const kind of ["sdk", "interactive"] as const) {
      expect(creditPoolState(900, 1000, kind)).toBe("near_exhaustion"); // 10% remaining
      expect(creditPoolState(850, 1000, kind)).toBe("near_exhaustion"); // 15% boundary
    }
  });

  it("credit_pool_normal_both_kinds", () => {
    // spec(§11.4) — >15% remaining → normal for both kinds.
    for (const kind of ["sdk", "interactive"] as const) {
      expect(creditPoolState(100, 1000, kind)).toBe("normal"); // 90% remaining
      expect(creditPoolState(840, 1000, kind)).toBe("normal"); // 16% remaining
    }
  });
});
