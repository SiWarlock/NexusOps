// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { CommandCenter } from "./CommandCenter";
import type { CommandItem } from "./group";

afterEach(cleanup);

describe("CommandCenter triage view", () => {
  it("renders_groups_from_projection", () => {
    const items: CommandItem[] = [
      { id: "x1", label: "Sess A", machine: "Session", status: "waiting_on_human_input" },
      { id: "x2", label: "PR 1", machine: "PullRequest", status: "conflict" },
      { id: "x3", label: "idle one", machine: "Session", status: "idle" },
    ];
    const { container } = render(<CommandCenter items={items} />);
    const renderedIds = [...container.querySelectorAll("[data-item-id]")]
      .map((el) => el.getAttribute("data-item-id"))
      .toSorted();
    // exactly the input items rendered — none invented (forbidden #2)
    expect(renderedIds).toEqual(["x1", "x2", "x3"]);
  });

  it("empty_group_shows_empty_state", () => {
    const items: CommandItem[] = [
      { id: "only", label: "only", machine: "Session", status: "waiting_on_human_input" }, // needs only
    ];
    const { container } = render(<CommandCenter items={items} />);
    // the populated group renders its item (NOT an empty state)
    expect(screen.queryByTestId("empty-needsAttention")).toBeNull();
    expect(
      container.querySelector(
        '[data-group="needsAttention"] [data-item-id="only"]',
      ),
    ).not.toBeNull();
    // the other three groups render an explicit empty state, not absent
    expect(screen.getByTestId("empty-changesReady")).toBeTruthy();
    expect(screen.getByTestId("empty-working")).toBeTruthy();
    expect(screen.getByTestId("empty-settled")).toBeTruthy();
  });
});
