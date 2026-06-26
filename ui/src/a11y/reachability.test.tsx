// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { readFileSync } from "node:fs";
import { cleanup, render, screen, fireEvent, within } from "@testing-library/react";
import { Shell } from "../shell/Shell";
import { MockGatewayPort } from "../gateway-client/mock";
import { projectActivityFixture } from "../projections/fixtures/proj_project_activity";
import { auditFocusable, auditAccessibleNames } from "./reachability";

afterEach(cleanup);

// Read source files relative to THIS test (cwd-independent) — used to pin CSS
// wiring that jsdom can't compute (it never applies :focus-visible / @media).
const read = (rel: string) => readFileSync(new URL(rel, import.meta.url), "utf8");

// The §11.6 merge-gate sweeps BOTH a11y nets at every view: keyboard-reachability
// (auditFocusable — roving-aware, tabpanel-aware) AND accessible-name coverage
// (auditAccessibleNames, 045). Both throw on a violation, so calling auditView
// directly fails the test if any control in the swept view is unreachable OR
// nameless. Their classification is unit-tested in reachability.classify.test.tsx.
const auditView = (container: HTMLElement): void => {
  auditFocusable(container);
  auditAccessibleNames(container);
};

describe("a11y reachability + wiring", () => {
  it("every_interactive_control_is_keyboard_focusable", async () => {
    const { container } = render(<Shell gateway={new MockGatewayPort()} />);
    await screen.findAllByText(projectActivityFixture.rows[0]!.name!); // loaded (fixture always names)

    // Sweep the content views — the audit is the single §11.6 gate for any
    // per-view control (sidebar nav, Graph|List toggle, Sessions sort headers),
    // so a non-focusable control added to ANY view later is caught here.
    auditView(container); // Command Center (default) — incl. TopBar + sidebar tree
    // the sidebar nav carries the prototype view set; Settings is reached via the
    // TopBar (§11.2 nav reconcile).
    const sidebar = screen.getByRole("navigation", { name: "Sidebar" });
    for (const name of [/command center/i, /project graph/i, /session terminal/i]) {
      expect(within(sidebar).getByRole("button", { name })).toBeTruthy();
    }

    fireEvent.click(within(sidebar).getByRole("button", { name: /project graph/i }));
    expect(screen.getByTestId("graph-canvas")).toBeTruthy();
    auditView(container); // Project Graph (incl. the Graph|List toggle + nodes)

    fireEvent.click(within(sidebar).getByRole("button", { name: /session terminal/i }));
    expect(screen.getByTestId("sessions-table")).toBeTruthy();
    auditView(container); // Session Terminal picker (incl. the sort headers)

    // every remaining sidebar view sweeps clean (Plan / Editor / Code / Packs / Audit)
    for (const name of [/^plan$/i, /^editor$/i, /code \/ diff review/i, /workflow packs/i, /audit trail/i]) {
      fireEvent.click(within(sidebar).getByRole("button", { name }));
      auditView(container);
    }

    // Projects overview (sidebar tree header label; name computes "Projects<count>")
    fireEvent.click(within(sidebar).getByRole("button", { name: /^projects\s*\d+$/i }));
    auditView(container);

    // Brain page via the TopBar Brain trigger
    fireEvent.click(screen.getByRole("button", { name: /^brain$/i }));
    auditView(container);

    // Agent Team via the team session in the workspace tree
    fireEvent.click(
      container.querySelector('[data-item-id="Session:session_fixture_1"]')!,
    );
    auditView(container);

    // Settings is reached via the TopBar trigger (the only "Settings" button).
    fireEvent.click(screen.getByRole("button", { name: /settings/i }));
    expect(screen.getByRole("tablist")).toBeTruthy();
    auditView(container); // Settings (tab buttons; §9 net)

    // the expanded event dock's timeline + Full audit control sweep clean too
    fireEvent.click(screen.getByRole("button", { name: /activity/i }));
    expect(screen.getByTestId("activity-timeline")).toBeTruthy();
    auditView(container);
  });

  it("shell_sweep_green_with_dropdown_open", async () => {
    const { container } = render(<Shell gateway={new MockGatewayPort()} />);
    await screen.findAllByText(projectActivityFixture.rows[0]!.name!); // loaded (fixture always names)
    auditView(container); // dropdown closed — trigger only
    // open the ProjectSwitcher dropdown → its roving listbox must sweep clean: the
    // active option is the one tabstop, the rest are -1 roving members (§9 audit
    // now covers role="option" in a one-tabstop listbox).
    fireEvent.click(screen.getByTitle("Switch project"));
    expect(screen.getByRole("listbox")).toBeTruthy();
    auditView(container); // dropdown open — options now in the swept set
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
