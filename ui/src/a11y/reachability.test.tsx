// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { readFileSync } from "node:fs";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";
import { Shell } from "../shell/Shell";
import { MockGatewayPort } from "../gateway-client/mock";
import { projectActivityFixture } from "../projections/fixtures/proj_project_activity";

afterEach(cleanup);

// Read source files relative to THIS test (cwd-independent) — used to pin CSS
// wiring that jsdom can't compute (it never applies :focus-visible / @media).
const read = (rel: string) => readFileSync(new URL(rel, import.meta.url), "utf8");

// Audit one rendered view: every actionable control is keyboard-focusable.
// `el.tabIndex` reflects the effective tab order — 0 for natively-focusable
// elements (button/a[href]/input/…) or an explicit `tabindex>=0`, and -1 if
// removed from the order OR a non-focusable element (e.g. a `div[role="button"]`
// with no tabindex). So one `>= 0` check catches both "tabindex=-1 on an
// actionable" and "role=button on a non-focusable element". Non-vacuous: a view
// must have ≥1 control.
function auditFocusable(container: HTMLElement) {
  const interactive = [
    ...container.querySelectorAll<HTMLElement>(
      'button, a[href], [role="button"], [role="link"], input, select, summary',
    ),
  ];
  expect(interactive.length).toBeGreaterThan(0);
  for (const el of interactive) {
    expect(el.tabIndex).toBeGreaterThanOrEqual(0);
  }
}

describe("a11y reachability + wiring", () => {
  it("every_interactive_control_is_keyboard_focusable", async () => {
    const { container } = render(<Shell gateway={new MockGatewayPort()} />);
    await screen.findByText(projectActivityFixture.rows[0]!.name); // loaded

    // Sweep ALL three content views — the audit is the single §11.6 gate for any
    // per-view control (view-switch, Graph|List toggle, Sessions sort headers),
    // so a non-focusable control added to ANY view later is caught here.
    auditFocusable(container); // Command Center (default)
    // the view-switch controls are real (focusable) buttons
    for (const name of [
      /command center/i,
      /project graph/i,
      /sessions/i,
      /usage/i,
    ]) {
      expect(screen.getByRole("button", { name })).toBeTruthy();
    }

    fireEvent.click(screen.getByRole("button", { name: /project graph/i }));
    expect(screen.getByTestId("graph-canvas")).toBeTruthy();
    auditFocusable(container); // Project Graph (incl. the Graph|List toggle)

    fireEvent.click(screen.getByRole("button", { name: /sessions/i }));
    expect(screen.getByTestId("sessions-table")).toBeTruthy();
    auditFocusable(container); // Sessions (incl. the sort-header buttons)

    fireEvent.click(screen.getByRole("button", { name: /usage/i }));
    expect(screen.getByTestId("usage-table")).toBeTruthy();
    auditFocusable(container); // Usage (interim 4th view — keeps the §9 net complete)
  });

  it("focus_stylesheet_is_imported", () => {
    // jsdom can't compute :focus-visible — pin the WIRING, not the pixel.
    expect(read("../main.tsx")).toMatch(/import\s+["'][^"']*a11y\/focus\.css["']/);
    const focus = read("./focus.css");
    // one global :where(...):focus-visible rule applying the kit ring tokens
    expect(focus).toContain(":where(");
    expect(focus).toContain(":focus-visible");
    expect(focus).toContain("var(--focus-ring)");
    expect(focus).toContain("var(--ring-w)");
    expect(focus).toContain("var(--ring-offset)");
  });

  it("reduced_motion_guard_present", () => {
    // the kit's global reduced-motion guard is wired via the styles.css chain.
    expect(read("../../../NexusOps-ui-kit/styles.css")).toContain("tokens/motion.css");
    expect(read("../../../NexusOps-ui-kit/tokens/motion.css")).toMatch(
      /@media\s*\(\s*prefers-reduced-motion:\s*reduce\s*\)/,
    );
  });
});
