// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { UsageDashboard } from "./UsageDashboard";
import { usageFixture } from "../../projections/fixtures/proj_usage";

afterEach(cleanup);

const renderDash = (
  rows = usageFixture.rows,
  creditPool = usageFixture.creditPool ?? null,
) => render(<UsageDashboard rows={rows} creditPool={creditPool} />);

describe("UsageDashboard view", () => {
  it("renders_rows_with_accuracy_label_in_all_variants", () => {
    renderDash();
    const cells = screen
      .getByTestId("usage-table")
      .querySelectorAll("tbody tr [data-accuracy]");
    // §11.7: one accuracy label per row, never dropped (incl. exact, where the
    // kit shows no marker — the textual label is always present) — and the label
    // matches its metric_quality (round-trip: data-accuracy === the rendered text).
    expect(cells).toHaveLength(usageFixture.rows.length);
    for (const cell of cells) {
      expect(cell.textContent?.trim()).toBe(cell.getAttribute("data-accuracy"));
    }
  });

  it("codex_row_shows_unknown_context", () => {
    renderDash();
    // the Codex fixture row renders "unknown" for context (forbidden #4)
    const codexRow = screen
      .getByTestId("usage-table")
      .querySelector('[data-item-id="Usage:session_fixture_3"]');
    expect(codexRow?.querySelector(".usage__context")?.textContent).toBe("unknown");
  });

  it("credit_pool_threshold_on_non_color_channel", () => {
    renderDash();
    // forbidden #5: the credit-pool state is exposed via a glyph/label (text),
    // not color alone. Fixture pool 870/1000 → near_exhaustion.
    const state = screen.getByTestId("credit-pool-state");
    expect(state.getAttribute("data-state")).toBe("near_exhaustion");
    expect(state.textContent).toContain("Near exhaustion");
  });

  it("renders_only_projection_rows", () => {
    const { unmount } = renderDash();
    const ids = [
      ...screen
        .getByTestId("usage-table")
        .querySelectorAll("tbody tr[data-item-id]"),
    ]
      .map((r) => r.getAttribute("data-item-id"))
      .toSorted();
    // rendered set === fixture set — no invented usage (forbidden #2)
    expect(ids).toEqual(
      usageFixture.rows.map((r) => `Usage:${r.subject_id}`).toSorted(),
    );
    unmount();
    // an empty usage set → an explicit empty state; and a null credit pool
    // renders no credit-pool section (the meter is absent, not a 0-state).
    render(<UsageDashboard rows={[]} creditPool={null} />);
    expect(screen.getByTestId("usage-empty")).toBeTruthy();
    expect(screen.queryByTestId("credit-pool-state")).toBeNull();
  });
});
