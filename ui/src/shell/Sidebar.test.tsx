// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from "vitest";
import type { ComponentProps } from "react";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";
import { Sidebar } from "./Sidebar";
import { describeResumeMode } from "../recovery/model";
import type { ProjectActivityRow, SessionRow } from "../contracts/index";

afterEach(cleanup);

const projects: ProjectActivityRow[] = [
  { project_id: "p1", name: "auth-service" },
  { project_id: "p2", name: "billing" },
];

const sessionOf = (
  id: string,
  status: SessionRow["status"],
  project = "p1",
): SessionRow => ({
  session_id: id,
  status,
  title: `session ${id}`,
  project_id: project,
});

// Ranks: idle 0 · waiting_on_human_input 5 · running_command 2 · waiting_on_permission 4
const sessions: SessionRow[] = [
  sessionOf("s1", "idle"),
  sessionOf("s2", "waiting_on_human_input"),
  sessionOf("s3", "running_command"),
  sessionOf("s4", "waiting_on_permission"),
];

const renderSidebar = (
  overrides: Partial<ComponentProps<typeof Sidebar>> = {},
) =>
  render(
    <Sidebar
      projects={projects}
      sessions={sessions}
      view="command"
      onNavigate={() => {}}
      {...overrides}
    />,
  );

describe("Sidebar workspace tree", () => {
  it("sidebar_orders_sessions_by_attention_within_project", () => {
    // §11.3 sidebar weight: within a project group the session rows are
    // attention-ordered (ranks 5, 4, 2, 0) — namespaced `${machine}:${id}`
    // locator (P6.3b convention) preserved on the tree rows.
    const { container } = renderSidebar();
    const ids = [...container.querySelectorAll("[data-item-id]")].map((el) =>
      el.getAttribute("data-item-id"),
    );
    expect(ids).toEqual(["Session:s2", "Session:s4", "Session:s3", "Session:s1"]);
  });

  it("sidebar_item_locator_is_machine_namespaced", () => {
    const { container } = renderSidebar({
      sessions: [sessionOf("42", "idle")],
    });
    expect(container.querySelector('[data-item-id="Session:42"]')).not.toBeNull();
    // the bare-id locator is gone (was `data-item-id={item.id}`)
    expect(container.querySelector('[data-item-id="42"]')).toBeNull();
  });

  it("sidebar_groups_sessions_under_their_project", () => {
    // p2's group is collapsed by default (only the FIRST project auto-expands) —
    // its sessions are not rendered until the group is expanded.
    const { container } = renderSidebar({
      sessions: [sessionOf("a", "idle", "p1"), sessionOf("b", "idle", "p2")],
    });
    expect(container.querySelector('[data-item-id="Session:a"]')).not.toBeNull();
    expect(container.querySelector('[data-item-id="Session:b"]')).toBeNull();
    // expand billing → its session row appears
    fireEvent.click(screen.getByRole("button", { name: /billing/i }));
    expect(container.querySelector('[data-item-id="Session:b"]')).not.toBeNull();
  });

  it("sidebar_session_click_opens_session", () => {
    const onOpenSession = vi.fn();
    const { container } = renderSidebar({ onOpenSession });
    fireEvent.click(container.querySelector('[data-item-id="Session:s2"]')!);
    expect(onOpenSession).toHaveBeenCalledTimes(1);
    expect(onOpenSession.mock.calls[0]![0].session_id).toBe("s2");
  });

  it("sidebar_nav_drives_views", () => {
    const onNavigate = vi.fn();
    renderSidebar({ onNavigate });
    fireEvent.click(screen.getByRole("button", { name: /project graph/i }));
    expect(onNavigate).toHaveBeenCalledWith("graph");
    fireEvent.click(screen.getByRole("button", { name: /audit trail/i }));
    expect(onNavigate).toHaveBeenCalledWith("audit");
  });

  it("sidebar_waiting_badge_from_prop", () => {
    renderSidebar({ waiting: 2 });
    expect(screen.getByTestId("sidebar-waiting-badge").textContent).toBe("2");
  });
});

describe("Sidebar resume-mode indicator", () => {
  it("sidebar_shows_resume_indicator_for_mapped_session", () => {
    const { container } = renderSidebar({
      sessions: [sessionOf("s2", "idle")],
      resumeModes: { s2: "replayed" },
    });
    const item = container.querySelector('[data-item-id="Session:s2"]');
    const badge = item?.querySelector("[data-resume-mode]");
    const desc = describeResumeMode("replayed");
    // the indicator is built from describeResumeMode (single source — assert the
    // glyph + label come from it exactly, not a re-derived map)
    expect(badge?.getAttribute("data-resume-mode")).toBe("replayed");
    expect(badge?.querySelector(".sr-only")?.textContent).toBe(desc.label);
  });

  it("sidebar_no_indicator_when_absent", () => {
    const { container } = renderSidebar({
      sessions: [sessionOf("s1", "idle")], // a Session NOT in the map
      resumeModes: {},
    });
    expect(
      container.querySelector('[data-item-id="Session:s1"] [data-resume-mode]'),
    ).toBeNull();
  });

  it("sidebar_resume_indicator_never_color_alone", () => {
    const { container } = renderSidebar({
      sessions: [sessionOf("s2", "idle")],
      resumeModes: { s2: "resumed" },
    });
    const badge = container.querySelector(
      '[data-item-id="Session:s2"] [data-resume-mode]',
    );
    const desc = describeResumeMode("resumed");
    // never color alone: the visible glyph (aria-hidden) + the accessible label,
    // both exactly the descriptor's (single source — no re-derived glyph/label)
    expect(badge?.querySelector('[aria-hidden="true"]')?.textContent).toBe(desc.glyph);
    expect(badge?.querySelector(".sr-only")?.textContent).toBe(desc.label);
  });

  it("sidebar_does_not_widen_projection_row", () => {
    // the indicator derives from the resumeModes PROP, not a row field — the SAME
    // sessions without the map render no indicator (Lesson §8 — the side map is
    // the data path).
    const { container } = renderSidebar({
      sessions: [sessionOf("s2", "idle")],
    });
    expect(container.querySelector("[data-resume-mode]")).toBeNull();
  });
});
