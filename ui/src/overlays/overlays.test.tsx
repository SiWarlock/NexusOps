// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render, screen, fireEvent, within } from "@testing-library/react";
import { Shell } from "../shell/Shell";
import { MockGatewayPort } from "../gateway-client/mock";

afterEach(cleanup);

const awaitLoaded = () => screen.findAllByText("auth-service");

describe("overlay surfaces", () => {
  it("cmd_k_opens_palette_and_command_navigates", async () => {
    const { container } = render(<Shell gateway={new MockGatewayPort()} />);
    await awaitLoaded();
    fireEvent.keyDown(window, { key: "k", metaKey: true });
    const palette = screen.getByRole("dialog", { name: /command palette/i });
    // filter narrows the command list; Enter runs the highlighted command
    fireEvent.change(within(palette).getByRole("textbox"), {
      target: { value: "audit" },
    });
    fireEvent.click(within(palette).getByRole("button", { name: /open audit trail/i }));
    expect(screen.queryByRole("dialog")).toBeNull(); // palette closed
    expect(screen.getByTestId("audit-timeline-view")).toBeTruthy(); // navigated
    expect(container.querySelector('[aria-label="Command Center"]')).toBeNull();
  });

  it("bell_opens_hiq_with_real_queue_and_disabled_mutations", async () => {
    render(<Shell gateway={new MockGatewayPort()} />);
    await awaitLoaded();
    fireEvent.click(screen.getByRole("button", { name: /human input queue/i }));
    const drawer = screen.getByTestId("hiq-drawer");
    // the REAL queue: 1 pending approval + 2 waiting sessions (fixtures)
    expect(drawer.querySelector('[data-item-id="Approval:approval_fixture_1"]')).not.toBeNull();
    expect(drawer.querySelector('[data-item-id="Session:session_fixture_2"]')).not.toBeNull();
    expect(drawer.querySelector('[data-item-id="Session:session_fixture_5"]')).not.toBeNull();
    // the decided approval is NOT in the queue (pending only)
    expect(drawer.querySelector('[data-item-id="Approval:approval_fixture_2"]')).toBeNull();
    // resolution mutations are disabled (intent seam — §11.6), Review/Open live
    expect(within(drawer).getByRole("button", { name: /deny/i })).toHaveProperty(
      "disabled",
      true,
    );
  });

  it("hiq_review_opens_gateway_modal_wired", async () => {
    render(<Shell gateway={new MockGatewayPort()} />);
    await awaitLoaded();
    fireEvent.keyDown(window, { key: "h", metaKey: true, shiftKey: true });
    const drawer = screen.getByTestId("hiq-drawer");
    fireEvent.click(within(drawer).getByRole("button", { name: /^review$/i }));
    const modal = screen.getByTestId("gateway-modal");
    // the wired card renders the frozen status pill + the daemon policy/preview sections
    expect(modal.querySelector('[data-status="awaiting_approval"]')).not.toBeNull();
    expect(within(modal as HTMLElement).getByTestId("gateway-policy")).toBeTruthy();
    expect(within(modal as HTMLElement).getByTestId("gateway-preview")).toBeTruthy();
    // the standing-grant "Always allow" stays DISABLED (deferred — its own cat-1 slice)
    expect(within(modal as HTMLElement).getByTestId("always-allow")).toHaveProperty("disabled", true);
  });

  it("hiq_open_session_navigates_to_terminal", async () => {
    const { container } = render(<Shell gateway={new MockGatewayPort()} />);
    await awaitLoaded();
    fireEvent.keyDown(window, { key: "h", metaKey: true, shiftKey: true });
    const drawer = screen.getByTestId("hiq-drawer");
    fireEvent.click(within(drawer).getAllByRole("button", { name: /open session/i })[0]!);
    // drawer closed; the Session Terminal targets the waiting session
    expect(screen.queryByTestId("hiq-drawer")).toBeNull();
    expect(container.querySelector('[aria-label="Session Terminal"]')).not.toBeNull();
    expect(screen.getByTestId("terminal-permission-prompt")).toBeTruthy();
  });

  it("task_inbox_opens_with_shortcut_and_is_display_only", async () => {
    render(<Shell gateway={new MockGatewayPort()} />);
    await awaitLoaded();
    fireEvent.keyDown(window, { key: "p", metaKey: true, shiftKey: true });
    const list = screen.getByTestId("task-inbox-list");
    expect(list.textContent).toContain("Refactor auth module");
    // tab filter is live local state
    fireEvent.click(screen.getByRole("button", { name: /^github$/i }));
    expect(list.textContent).toContain("#214");
    expect(list.textContent).not.toContain("ENG-310");
  });

  it("brain_button_opens_drawer_and_expand_goes_to_page", async () => {
    const { container } = render(<Shell gateway={new MockGatewayPort()} />);
    await awaitLoaded();
    const topbar = screen.getByRole("banner");
    fireEvent.click(within(topbar).getByRole("button", { name: /brain/i }));
    expect(screen.getByTestId("brain-drawer")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /expand to full page/i }));
    expect(screen.queryByTestId("brain-drawer")).toBeNull();
    expect(container.querySelector('[aria-label="Project Brain"]')).not.toBeNull();
  });

  it("graph_node_click_opens_inspector_and_open_navigates", async () => {
    const { container } = render(<Shell gateway={new MockGatewayPort()} />);
    await awaitLoaded();
    fireEvent.click(screen.getByRole("button", { name: /project graph/i }));
    // activate a session node on the canvas (kit GraphNode role=button)
    const nodeHost = container.querySelector(
      '[data-testid="graph-canvas"] [data-item-id="session:session_fixture_2"]',
    )!;
    fireEvent.click(nodeHost.querySelector('[role="button"]')!);
    const inspector = screen.getByTestId("inspector-drawer");
    // REAL status pill + side-map detail fields
    expect(inspector.querySelector('[data-status="waiting_on_permission"]')).not.toBeNull();
    expect(within(inspector as HTMLElement).getByText("agent/rate-limit")).toBeTruthy();
    // Open jumps to the session's surface
    fireEvent.click(within(inspector as HTMLElement).getByRole("button", { name: /open terminal/i }));
    expect(screen.queryByTestId("inspector-drawer")).toBeNull();
    expect(container.querySelector('[aria-label="Session Terminal"]')).not.toBeNull();
  });

  it("escape_closes_the_open_overlay", async () => {
    render(<Shell gateway={new MockGatewayPort()} />);
    await awaitLoaded();
    fireEvent.keyDown(window, { key: "k", metaKey: true });
    expect(screen.getByRole("dialog", { name: /command palette/i })).toBeTruthy();
    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByRole("dialog")).toBeNull();
  });
});
