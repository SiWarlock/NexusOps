// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { UsageDashboard } from "./UsageDashboard";
import { usageFixture } from "../../projections/fixtures/proj_usage";

afterEach(cleanup);

const renderDash = (rows = usageFixture.rows) =>
  render(<UsageDashboard rows={rows} />);

describe("UsageDashboard view", () => {
  it("renders_rows_with_accuracy_label_in_all_variants", () => {
    renderDash();
    const cells = screen
      .getByTestId("usage-table")
      .querySelectorAll("tbody tr [data-accuracy]");
    // §11.7: one accuracy label per row, never dropped — and the label matches its
    // (effective) metric_quality (round-trip: data-accuracy === the rendered text).
    expect(cells).toHaveLength(usageFixture.rows.length);
    for (const cell of cells) {
      expect(cell.textContent?.trim()).toBe(cell.getAttribute("data-accuracy"));
    }
  });

  it("context_unknown_when_no_context_metadata", () => {
    renderDash();
    // the no-context fixture row (context_pct_max null) renders "unknown" (forbidden #4)
    const row = screen
      .getByTestId("usage-table")
      .querySelector('[data-item-id="Usage:ledger_fixture_3"]');
    expect(row?.querySelector(".usage__context")?.textContent).toBe("unknown");
  });

  it("renders_no_credit_pool_meter", () => {
    renderDash();
    // the honest-OMIT: NO credit-pool meter/state renders (the Mock-only fake is gone, [[11]]/[[38]]);
    // an honest "not reported" note IS present (Q4 — explain the absence, never a silent drop / a fake).
    expect(screen.queryByTestId("credit-pool-state")).toBeNull();
    expect(screen.getByTestId("credit-pool-unavailable")).toBeTruthy();
  });

  it("stat_cards_use_frozen_aggregates", () => {
    renderDash();
    // spend = Σ cost_estimate (1.92 + 0.68 + 1.1 + 0[null] = 3.70; ≈ because an estimated row is present);
    // tokens = Σ(tokens_in + tokens_out) = 128000 + 45000 + 88000 + 0 = 261000 → "261k".
    expect(screen.getByText("≈$3.70")).toBeTruthy();
    expect(screen.getByText("261k")).toBeTruthy();
  });

  it("renders_only_projection_rows", () => {
    const { unmount } = renderDash();
    const ids = [
      ...screen.getByTestId("usage-table").querySelectorAll("tbody tr[data-item-id]"),
    ]
      .map((r) => r.getAttribute("data-item-id"))
      .toSorted();
    // rendered set === fixture set — no invented usage (forbidden #2); keyed on ledger_id.
    expect(ids).toEqual(usageFixture.rows.map((r) => `Usage:${r.ledger_id}`).toSorted());
    unmount();
    // an empty usage set → an explicit empty state (and still no credit-pool meter).
    render(<UsageDashboard rows={[]} />);
    expect(screen.getByTestId("usage-empty")).toBeTruthy();
    expect(screen.queryByTestId("credit-pool-state")).toBeNull();
  });
});
