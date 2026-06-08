// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { Sidebar, type SidebarItem } from "./Sidebar";
import { describeResumeMode } from "../recovery/model";

afterEach(cleanup);

const items: SidebarItem[] = [
  { id: "s1", label: "idle one", machine: "Session", status: "idle" }, // rank 0
  {
    id: "s2",
    label: "waiting on you",
    machine: "Session",
    status: "waiting_on_human_input",
  }, // rank 5
  { id: "s3", label: "running", machine: "Session", status: "running_command" }, // rank 2
  {
    id: "s4",
    label: "needs perm",
    machine: "Session",
    status: "waiting_on_permission",
  }, // rank 4
];

describe("Sidebar attention wiring", () => {
  it("sidebar_orders_by_attention", () => {
    const { container } = render(<Sidebar items={items} />);
    const ids = [...container.querySelectorAll("[data-item-id]")].map((el) =>
      el.getAttribute("data-item-id"),
    );
    // ranks 5, 4, 2, 0 — namespaced `${machine}:${id}` locator (P6.3b convention)
    expect(ids).toEqual(["Session:s2", "Session:s4", "Session:s3", "Session:s1"]);
  });

  it("sidebar_needs_attention_count", () => {
    render(<Sidebar items={items} />);
    // needs-attention = ranks {4,5} → s2 + s4
    expect(screen.getByTestId("needs-attention-count").textContent).toBe("2");
  });

  it("sidebar_item_locator_is_machine_namespaced", () => {
    // One convention, no special-case emitter: every data-item-id is
    // `${machine}:${id}`, collision-safe (P6.3b — brief Q5 "for status items").
    const { container } = render(
      <Sidebar
        items={[{ id: "42", label: "a", machine: "Session", status: "idle" }]}
      />,
    );
    expect(container.querySelector('[data-item-id="Session:42"]')).not.toBeNull();
    // the bare-id locator is gone (was `data-item-id={item.id}`)
    expect(container.querySelector('[data-item-id="42"]')).toBeNull();
  });
});

const itemOf = (id: string): SidebarItem => ({
  id,
  label: `session ${id}`,
  machine: "Session",
  status: "idle",
});

describe("Sidebar resume-mode indicator", () => {
  it("sidebar_shows_resume_indicator_for_mapped_session", () => {
    const { container } = render(
      <Sidebar items={[itemOf("s2")]} resumeModes={{ s2: "replayed" }} />,
    );
    const item = container.querySelector('[data-item-id="Session:s2"]');
    const badge = item?.querySelector("[data-resume-mode]");
    const desc = describeResumeMode("replayed");
    // the indicator is built from describeResumeMode (single source — assert the
    // glyph + label come from it exactly, not a re-derived map)
    expect(badge?.getAttribute("data-resume-mode")).toBe("replayed");
    expect(badge?.querySelector(".sr-only")?.textContent).toBe(desc.label);
  });

  it("sidebar_no_indicator_when_absent", () => {
    const { container } = render(
      <Sidebar
        items={[
          itemOf("s1"), // a Session NOT in the map
          { id: "s2", label: "a PR", machine: "PullRequest", status: "checks_pending" },
        ]}
        // s2's id IS in the map, but it's a PullRequest, not a Session → guarded out
        resumeModes={{ s2: "resumed" }}
      />,
    );
    expect(
      container.querySelector('[data-item-id="Session:s1"] [data-resume-mode]'),
    ).toBeNull();
    expect(
      container.querySelector('[data-item-id="PullRequest:s2"] [data-resume-mode]'),
    ).toBeNull();
  });

  it("sidebar_resume_indicator_never_color_alone", () => {
    const { container } = render(
      <Sidebar items={[itemOf("s2")]} resumeModes={{ s2: "resumed" }} />,
    );
    const badge = container.querySelector('[data-item-id="Session:s2"] [data-resume-mode]');
    const desc = describeResumeMode("resumed");
    // never color alone: the visible glyph (aria-hidden) + the accessible label,
    // both exactly the descriptor's (single source — no re-derived glyph/label)
    expect(badge?.querySelector('[aria-hidden="true"]')?.textContent).toBe(desc.glyph);
    expect(badge?.querySelector(".sr-only")?.textContent).toBe(desc.label);
  });

  it("sidebar_does_not_widen_projection_item", () => {
    // the indicator derives from the resumeModes PROP, not an item field — the SAME
    // items without the map render no indicator (ProjectionItem stays {id,label,
    // machine,status}; Lesson §8 — the side map is the data path).
    const { container } = render(<Sidebar items={[itemOf("s2")]} />);
    expect(container.querySelector("[data-resume-mode]")).toBeNull();
  });
});
