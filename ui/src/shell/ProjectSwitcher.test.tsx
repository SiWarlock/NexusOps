// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from "vitest";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";
import { ProjectSwitcher } from "./ProjectSwitcher";
import { ActiveProjectProvider } from "./active-project";
import type { ProjectActivityRow } from "../contracts/index";

afterEach(cleanup);

const projects = [
  { project_id: "project_fixture_1", name: "auth-service" },
  { project_id: "project_fixture_2", name: "billing" },
];

function renderSwitcher(
  activeProjectId: string | null,
  setActiveProject: (id: string) => void = () => {},
  projectList: ProjectActivityRow[] = projects,
) {
  return render(
    <ActiveProjectProvider value={{ activeProjectId, setActiveProject }}>
      <ProjectSwitcher projects={projectList} counts={{}} />
    </ActiveProjectProvider>,
  );
}

// L1 — popover shell (trigger + listbox structure + click-select-close + zero-disabled).
describe("ProjectSwitcher dropdown — popover shell (L1)", () => {
  it("trigger_shows_active_project_and_toggles", () => {
    renderSwitcher("project_fixture_2");
    const trigger = screen.getByRole("button", { name: /billing/i });
    // a button-triggered listbox popover (WAI-ARIA): closed by default
    expect(trigger.getAttribute("aria-haspopup")).toBe("listbox");
    expect(trigger.getAttribute("aria-expanded")).toBe("false");
    expect(screen.queryByRole("listbox")).toBeNull();
    // clicking opens the popover
    fireEvent.click(trigger);
    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByRole("listbox")).toBeTruthy();
  });

  it("listbox_lists_projects_with_selected", () => {
    renderSwitcher("project_fixture_2");
    fireEvent.click(screen.getByRole("button", { name: /billing/i }));
    const options = screen.getAllByRole("option");
    // one option per project; the active one is aria-selected + ✓ glyph+label
    expect(options).toHaveLength(projects.length);
    const active = options.find((o) => o.getAttribute("aria-selected") === "true");
    expect(active?.getAttribute("data-project-id")).toBe("project_fixture_2");
    // ✓ check glyph (aria-hidden svg) + the sr-only "Active" label — never color alone
    const activeMark = active?.querySelector(".project-switcher__active");
    expect(activeMark?.querySelector('[aria-hidden="true"]')).not.toBeNull();
    expect(activeMark?.querySelector(".sr-only")?.textContent).toBe("Active");
    // the inactive option is not selected (never color alone — aria-selected, not hue)
    const inactive = options.find((o) => o.getAttribute("data-project-id") === "project_fixture_1");
    expect(inactive?.getAttribute("aria-selected")).toBe("false");
  });

  it("option_click_selects_and_closes", () => {
    const setActiveProject = vi.fn();
    renderSwitcher("project_fixture_1", setActiveProject);
    fireEvent.click(screen.getByRole("button", { name: /auth-service/i }));
    fireEvent.click(screen.getByRole("option", { name: /billing/i }));
    // selection behavior preserved (setActiveProject) + the popover closes
    expect(setActiveProject).toHaveBeenCalledWith("project_fixture_2");
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("zero_projects_trigger_disabled", () => {
    renderSwitcher(null, () => {}, []);
    // wire-or-disable (§11.6): no projects → a disabled trigger, never a dead click
    const trigger = screen.getByRole("button");
    expect(trigger).toHaveProperty("disabled", true);
    expect(trigger.textContent).toContain("No project");
  });
});

// L2 — keyboard a11y (roving tabindex + Arrow/Home/End/Enter/Escape + focus mgmt).
describe("ProjectSwitcher dropdown — keyboard a11y (L2)", () => {
  it("roving_one_tabstop_and_arrows_move", () => {
    renderSwitcher("project_fixture_1"); // active = auth-service (index 0)
    fireEvent.click(screen.getByRole("button", { name: /auth-service/i }));
    const options = screen.getAllByRole("option");
    // exactly one tabstop — the active option; the rest -1 (roving invariant)
    expect(options.filter((o) => o.tabIndex === 0)).toHaveLength(1);
    expect(
      options.find((o) => o.tabIndex === 0)?.getAttribute("data-project-id"),
    ).toBe("project_fixture_1");
    const listbox = screen.getByRole("listbox");
    // ArrowDown moves focus to the next option (vertical roving via nextTabIndex)
    fireEvent.keyDown(listbox, { key: "ArrowDown" });
    expect(document.activeElement?.getAttribute("data-project-id")).toBe(
      "project_fixture_2",
    );
    // the roving tabstop moved with focus (the one-tabstop invariant holds)
    const after = screen.getAllByRole("option");
    expect(after.filter((o) => o.tabIndex === 0)).toHaveLength(1);
    expect(
      after.find((o) => o.tabIndex === 0)?.getAttribute("data-project-id"),
    ).toBe("project_fixture_2");
    // Home jumps back to the first
    fireEvent.keyDown(listbox, { key: "Home" });
    expect(document.activeElement?.getAttribute("data-project-id")).toBe(
      "project_fixture_1",
    );
  });

  it("open_focuses_active_option", () => {
    renderSwitcher("project_fixture_2"); // active = billing (index 1)
    fireEvent.click(screen.getByRole("button", { name: /billing/i }));
    // opening focuses the ACTIVE option (first if none active) — APG listbox
    const active = screen
      .getAllByRole("option")
      .find((o) => o.getAttribute("data-project-id") === "project_fixture_2");
    expect(document.activeElement).toBe(active);
  });

  it("enter_selects_escape_closes", () => {
    const setActiveProject = vi.fn();
    renderSwitcher("project_fixture_1", setActiveProject); // active auth (index 0)
    const trigger = screen.getByRole("button", { name: /auth-service/i });
    fireEvent.click(trigger); // open → focus auth option
    // ArrowDown → focus billing; Enter → select it + close + return focus to trigger
    fireEvent.keyDown(screen.getByRole("listbox"), { key: "ArrowDown" });
    fireEvent.keyDown(screen.getByRole("listbox"), { key: "Enter" });
    expect(setActiveProject).toHaveBeenCalledWith("project_fixture_2");
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(document.activeElement).toBe(trigger);
    // re-open; Escape closes WITHOUT a selection change + returns focus to trigger
    fireEvent.click(trigger);
    fireEvent.keyDown(screen.getByRole("listbox"), { key: "Escape" });
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(document.activeElement).toBe(trigger);
    expect(setActiveProject).toHaveBeenCalledTimes(1); // Escape did not select
  });

  it("click_outside_closes", () => {
    const setActiveProject = vi.fn();
    renderSwitcher("project_fixture_1", setActiveProject);
    fireEvent.click(screen.getByRole("button", { name: /auth-service/i }));
    expect(screen.getByRole("listbox")).toBeTruthy();
    // a click outside the switcher closes it with no selection change
    fireEvent.mouseDown(document.body);
    expect(screen.queryByRole("listbox")).toBeNull();
    expect(setActiveProject).not.toHaveBeenCalled();
  });
});
