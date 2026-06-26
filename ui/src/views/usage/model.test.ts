import { describe, it, expect } from "vitest";
import { buildUsageRows } from "./model";
import * as model from "./model";
import type { UsageRow } from "../../contracts/index";

// An 11-field frozen UsageRow (W2-usage 0.50); override per case.
function row(over: Partial<UsageRow> = {}): UsageRow {
  return {
    ledger_id: "ledger_1",
    project_id: "project_1",
    session_id: "session_1",
    execution_profile_id: "ep_1",
    model: "claude-sonnet-4",
    bucket_day: "2026-06-26",
    tokens_in: 100,
    tokens_out: 40,
    context_pct_max: 64,
    cost_estimate: 1.5,
    metric_quality: "exact",
    ...over,
  };
}

describe("usage view-model", () => {
  it("build_usage_rows_maps_frozen_fields", () => {
    // spec(the consumer reconcile) — maps the frozen fields: label = model (Q1), ledgerId = the PK
    // key, tokens = Σ(tokens_in+tokens_out), cost = cost_estimate, context = context_pct_max%.
    const [vm] = buildUsageRows([row()]);
    expect(vm!.ledgerId).toBe("ledger_1");
    expect(vm!.label).toBe("claude-sonnet-4"); // model is the primary label
    expect(vm!.tokensDisplay).toBe("140"); // 100 + 40
    expect(vm!.costDisplay).toBe("$1.50");
    expect(vm!.contextDisplay).toBe("64%");
    // Q1: model null → the ledger_id label fallback (never empty).
    const [noModel] = buildUsageRows([row({ model: null, ledger_id: "ledger_9" })]);
    expect(noModel!.label).toBe("ledger_9");
  });

  it("context_unknown_when_context_pct_max_null", () => {
    // spec(§9.1 / forbidden #4) — no context metadata (the daemon serves context_pct_max null) →
    // the literal "unknown", NEVER a number/0%; a present value renders the percentage; 0 is a real 0%.
    expect(buildUsageRows([row({ context_pct_max: null })])[0]!.contextDisplay).toBe("unknown");
    expect(buildUsageRows([row({ context_pct_max: 64 })])[0]!.contextDisplay).toBe("64%");
    expect(buildUsageRows([row({ context_pct_max: 0 })])[0]!.contextDisplay).toBe("0%");
  });

  it("accuracy_and_unavailable_values", () => {
    // spec(§11.7) — the accuracy label derives from metric_quality (always present); an `unavailable`
    // usage renders "unknown" for the value, never 0/empty.
    const [ex, est, un] = buildUsageRows([
      row({ metric_quality: "exact" }),
      row({ metric_quality: "estimated" }),
      row({ metric_quality: "unavailable", tokens_in: null, tokens_out: null, cost_estimate: null }),
    ]);
    expect(ex!.accuracyLabel).toBe("exact");
    expect(est!.accuracyLabel).toBe("estimated");
    expect(un!.accuracyLabel).toBe("unavailable");
    expect(un!.tokensDisplay).toBe("unknown");
    expect(un!.costDisplay).toBe("unknown");
  });

  it("null_metric_quality_treated_as_unavailable", () => {
    // spec(Q3 / §11.7) — metric_quality is nullable (Option<MetricQuality>); null → the honest
    // "unavailable" degrade. Use PRESENT numeric data so this isolates the effectiveQuality(null)
    // coalesce: the accuracy is "unavailable" + the values render "unknown" (unavailable hides them).
    const [vm] = buildUsageRows([row({ metric_quality: null })]);
    expect(vm!.accuracyLabel).toBe("unavailable");
    expect(vm!.metricQuality).toBe("unavailable");
    expect(vm!.tokensDisplay).toBe("unknown");
    expect(vm!.costDisplay).toBe("unknown");
  });

  it("null_numeric_data_renders_unknown_even_for_non_unavailable_quality", () => {
    // spec(§11.7 / forbidden #2) — a null numeric field (the daemon didn't report it) renders "unknown",
    // NEVER a fabricated 0/$0.00 — even when metric_quality is exact (the unavailable guard alone misses
    // the null-data-with-non-unavailable-quality diagonal).
    const [vm] = buildUsageRows([
      row({ metric_quality: "exact", tokens_in: null, tokens_out: null, cost_estimate: null }),
    ]);
    expect(vm!.accuracyLabel).toBe("exact");
    expect(vm!.tokensDisplay).toBe("unknown");
    expect(vm!.costDisplay).toBe("unknown");
  });

  it("credit_pool_state_removed", () => {
    // spec(the honest-OMIT / §11.4) — the Mock-only, daemon-unobservable, potentially-FALSE
    // hard_stop display is removed: `creditPoolState`/`CreditPoolKind` no longer exported
    // (compile-time enforced by tsc; runtime-pinned here so the removal can't silently regress —
    // `creditPoolState` was a function-valued export, so its absence IS observable at runtime, unlike
    // the type-only `CreditPoolState`/`CreditPoolKind` which tsc alone enforces).
    expect("creditPoolState" in model, "creditPoolState must no longer be exported").toBe(false);
  });
});
