// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { Sidebar, type SidebarItem } from "./Sidebar";

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
