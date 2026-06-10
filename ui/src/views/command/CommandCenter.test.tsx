// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from "vitest";
import type { ComponentProps } from "react";
import { cleanup, render, screen, fireEvent, within } from "@testing-library/react";
import { CommandCenter } from "./CommandCenter";
import type { SessionRow } from "../../contracts/index";

afterEach(cleanup);

const sessionOf = (
  id: string,
  status: SessionRow["status"],
  title?: string,
): SessionRow => ({
  session_id: id,
  status,
  title: title ?? `session ${id}`,
  project_id: "p1",
});

const renderCC = (overrides: Partial<ComponentProps<typeof CommandCenter>> = {}) =>
  render(
    <CommandCenter
      sessions={[]}
      approvals={[]}
      waiting={[]}
      usage={[]}
      creditPool={null}
      events={[]}
      projectName="auth-service"
      onOpenSession={() => {}}
      onOpenProjects={() => {}}
      {...overrides}
    />,
  );

describe("CommandCenter triage cockpit", () => {
  it("renders_sessions_into_triage_sections", () => {
    const { container } = renderCC({
      sessions: [
        sessionOf("x1", "waiting_on_human_input"), // rank 5 → attention card
        sessionOf("x2", "running_command"), // rank 2 → working row
        sessionOf("x3", "idle"), // rank 0 → settled row
      ],
    });
    // exactly the input sessions rendered — none invented (forbidden #2); the
    // locator is the collision-safe `${machine}:${id}` convention (P6.3b).
    expect(
      container.querySelector('[data-group="needsAttention"] [data-item-id="Session:x1"]'),
    ).not.toBeNull();
    expect(
      container.querySelector('[data-group="working"] [data-item-id="Session:x2"]'),
    ).not.toBeNull();
    expect(
      container.querySelector('[data-group="settled"] [data-item-id="Session:x3"]'),
    ).not.toBeNull();
  });

  it("changes_ready_orders_first_in_attention", () => {
    // changes_ready keeps its 6.3 prominence: it orders BEFORE other rank-4/5
    // items in the attention section (groupForCommandCenter extraction).
    const { container } = renderCC({
      sessions: [
        sessionOf("w", "waiting_on_permission"), // rank 4
        sessionOf("c", "changes_ready"), // rank 4, extracted cluster
      ],
    });
    const ids = [
      ...container.querySelectorAll('[data-group="needsAttention"] [data-item-id]'),
    ].map((el) => el.getAttribute("data-item-id"));
    expect(ids[0]).toBe("Session:c");
  });

  it("empty_sections_render_explicit_empty_states", () => {
    renderCC({ sessions: [sessionOf("only", "waiting_on_human_input")] });
    expect(screen.queryByTestId("empty-needsAttention")).toBeNull();
    expect(screen.getByTestId("empty-working")).toBeTruthy();
    expect(screen.getByTestId("empty-settled")).toBeTruthy();
  });

  it("attention_card_open_invokes_handler_and_mutations_are_disabled", () => {
    const onOpenSession = vi.fn();
    renderCC({
      sessions: [sessionOf("x1", "waiting_on_human_input")],
      onOpenSession,
    });
    // Open is live (read-path nav)
    fireEvent.click(screen.getByRole("button", { name: /^open$/i }));
    expect(onOpenSession).toHaveBeenCalledTimes(1);
    expect(onOpenSession.mock.calls[0]![0].session_id).toBe("x1");
    // approve is a Gateway mutation — present but DISABLED until the intent
    // seam lands (§11.6 wire-or-disable, never a dead/faked click)
    expect(
      screen.getByRole("button", { name: /review & approve/i }),
    ).toHaveProperty("disabled", true);
  });

  it("rail_queue_lists_global_approvals_and_waiting_sessions", () => {
    renderCC({
      approvals: [
        {
          approval_id: "a1",
          project_id: "p2",
          status: "awaiting_approval",
          title: "git.create_worktree",
        },
      ],
      waiting: [sessionOf("w1", "waiting_on_permission", "Add rate limiting")],
    });
    const rail = screen.getByTestId("hiq-rail");
    expect(within(rail).getByText("git.create_worktree")).toBeTruthy();
    expect(within(rail).getAllByRole("button")).toHaveLength(2);
  });

  it("rail_queue_clear_state_when_empty", () => {
    renderCC();
    expect(screen.getByText(/queue clear/i)).toBeTruthy();
  });

  it("capacity_renders_real_credit_pool_meter", () => {
    renderCC({ creditPool: { used: 870, limit: 1000 } });
    expect(screen.getByText("870 / 1000")).toBeTruthy();
  });

  it("ctx_unknown_never_fabricated_for_codex_rows", () => {
    // §9.1 / forbidden #4: a session whose usage row reports no context renders
    // an explicit "unknown" — never a ring with an invented number.
    renderCC({
      sessions: [sessionOf("cx", "running_command")],
      usage: [
        {
          subject_id: "cx",
          harness: "codex",
          tokens: 1000,
          cost: 0.1,
          metric_quality: "exact",
          context_pct: null,
        },
      ],
    });
    expect(screen.getByTestId("ctx-unknown")).toBeTruthy();
  });

  it("header_projects_chip_navigates", () => {
    const onOpenProjects = vi.fn();
    renderCC({ onOpenProjects });
    fireEvent.click(screen.getByRole("button", { name: /projects/i }));
    expect(onOpenProjects).toHaveBeenCalledTimes(1);
  });
});
