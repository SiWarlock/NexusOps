// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from "vitest";
import { cleanup, render, screen, fireEvent, waitFor } from "@testing-library/react";
afterEach(cleanup);
import { LaunchAgentControl } from "./LaunchAgentControl";
import { ReadOnlyProvider, type ConnectionStatus } from "../../connection/read-only";
import { MockGatewayPort } from "../../gateway-client/mock";

const CONNECTED: ConnectionStatus = { connection: "connected", version: "compatible" };
const DEGRADED: ConnectionStatus = { connection: "connecting", version: "unknown" };

function renderLaunch(opts: {
  status?: ConnectionStatus;
  gateway?: MockGatewayPort;
  activeProjectId?: string | null;
}) {
  const gateway = opts.gateway ?? new MockGatewayPort();
  // NB: preserve an explicit null (`?? "proj_active"` would coerce null → the default).
  const activeProjectId: string | null =
    "activeProjectId" in opts ? (opts.activeProjectId ?? null) : "proj_active";
  render(
    <ReadOnlyProvider value={opts.status ?? CONNECTED}>
      <LaunchAgentControl gateway={gateway} activeProjectId={activeProjectId} />
    </ReadOnlyProvider>,
  );
  return { gateway };
}

const launchButton = () => screen.getByRole("button", { name: /launch agent/i }) as HTMLButtonElement;

describe("LaunchAgentControl (WAVE-1 Slice A — session.create)", () => {
  it("launch_control_submits_createSession_for_the_active_project", async () => {
    // spec(§6.1/W1-A) — clicking Launch submits createSession({project_id: <active>, initial_prompt?})
    // for the ACTIVE project; the daemon mints the id + drives the SessionExecutor.
    const { gateway } = renderLaunch({ activeProjectId: "proj_active" });
    const spy = vi.spyOn(gateway, "createSession");
    expect(launchButton().disabled).toBe(false);

    fireEvent.change(screen.getByPlaceholderText(/initial prompt/i), { target: { value: "drive me" } });
    fireEvent.click(launchButton());

    await waitFor(() => expect(spy).toHaveBeenCalledTimes(1));
    expect(spy).toHaveBeenCalledWith({ project_id: "proj_active", initial_prompt: "drive me" });
  });

  it("launch_control_omits_whitespace_only_initial_prompt", async () => {
    // spec(W1-A / LESSON §33) — a WHITESPACE-only prompt is trimmed to "" → omitted (the daemon uses
    // no dev-drive prompt), never sent as a blank string.
    const { gateway } = renderLaunch({ activeProjectId: "proj_active" });
    const spy = vi.spyOn(gateway, "createSession");
    fireEvent.change(screen.getByPlaceholderText(/initial prompt/i), { target: { value: "   " } });
    fireEvent.click(launchButton());
    await waitFor(() => expect(spy).toHaveBeenCalledTimes(1));
    expect(spy).toHaveBeenCalledWith({ project_id: "proj_active", initial_prompt: undefined });
  });

  it("launch_control_disabled_without_an_active_project", () => {
    // spec(W1-A) — project_id is required → no active project → disabled (never a dead click, §11.6).
    renderLaunch({ activeProjectId: null });
    expect(launchButton().disabled).toBe(true);
  });

  it("launch_control_disabled_when_cannot_submit_intent", () => {
    // spec(forbidden #6) — the fail-safe READ-ONLY/degraded gate disables the mutation affordance.
    renderLaunch({ status: DEGRADED, activeProjectId: "proj_active" });
    expect(launchButton().disabled).toBe(true);
  });

  it("launch_control_disabled_when_mutations_not_enabled", () => {
    // spec(L2 gate) — createSession is a mutation; the live L2 seam (mutationsEnabled) gates the control
    // (defense-in-depth; the daemon Gateway is the INV-SEC-1 chokepoint).
    renderLaunch({ gateway: new MockGatewayPort({ mutationsEnabled: false }), activeProjectId: "proj_active" });
    expect(launchButton().disabled).toBe(true);
  });

  it("launch_control_error_only_feedback_no_persistent_success", async () => {
    // spec(b/§11.7) — on a daemon §6.4 rejection, an honest error notice with the VERBATIM code; on
    // SUCCESS, NO persistent notice (the launched session row in the table is the success signal).
    const rejecting = new MockGatewayPort({ mutationError: { code: "precondition_stale" } });
    renderLaunch({ gateway: rejecting, activeProjectId: "proj_active" });
    fireEvent.click(launchButton());
    expect((await screen.findByRole("status")).textContent).toMatch(/precondition_stale/i);

    cleanup();
    // a successful launch shows NO persistent status notice.
    renderLaunch({ activeProjectId: "proj_active" });
    fireEvent.click(launchButton());
    await waitFor(() => expect(launchButton().disabled).toBe(false)); // settled
    expect(screen.queryByRole("status")).toBeNull();
  });
});
