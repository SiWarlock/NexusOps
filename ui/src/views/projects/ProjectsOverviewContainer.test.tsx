// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from "vitest";
import { cleanup, render, screen, fireEvent, waitFor } from "@testing-library/react";
afterEach(cleanup);
import { ProjectsOverviewContainer } from "./ProjectsOverviewContainer";
import { ReadOnlyProvider, type ConnectionStatus } from "../../connection/read-only";
import { MockGatewayPort } from "../../gateway-client/mock";

const CONNECTED: ConnectionStatus = { connection: "connected", version: "compatible" };
const DEGRADED: ConnectionStatus = { connection: "connecting", version: "unknown" };

function renderContainer(opts: {
  status?: ConnectionStatus;
  port?: MockGatewayPort;
  pickFolder?: () => Promise<string | null>;
}) {
  const port = opts.port ?? new MockGatewayPort();
  const utils = render(
    <ReadOnlyProvider value={opts.status ?? CONNECTED}>
      <ProjectsOverviewContainer
        gateway={port}
        projects={[]}
        counts={{}}
        activeProjectId={null}
        onSelectProject={() => {}}
        pickFolder={opts.pickFolder ?? (() => Promise.resolve(null))}
      />
    </ReadOnlyProvider>,
  );
  return { port, ...utils };
}

const addButton = () =>
  screen.getByRole("button", { name: /add project/i }) as HTMLButtonElement;

describe("ProjectsOverviewContainer — add-project (project.rescan) wiring", () => {
  it("add_project_disabled_when_daemon_unavailable", () => {
    // forbidden #6 — a mutation affordance is disabled in the READ-ONLY/degraded gate (fail-safe).
    renderContainer({ status: DEGRADED });
    expect(addButton().disabled).toBe(true);
  });

  it("add_project_picks_a_folder_and_submits_a_project_rescan_intent", async () => {
    // the happy path: click → folder picker → submit project.rescan with inputs.path (NO resource_refs).
    const pickFolder = vi.fn().mockResolvedValue("/repos/auth-service");
    const { port } = renderContainer({ status: CONNECTED, pickFolder });
    const spy = vi.spyOn(port, "submit_action");
    expect(addButton().disabled).toBe(false);

    fireEvent.click(addButton());

    await waitFor(() => expect(spy).toHaveBeenCalledTimes(1));
    expect(spy).toHaveBeenCalledWith(
      expect.objectContaining({
        action_type: "project.rescan",
        resource_refs: [],
        inputs: { path: "/repos/auth-service" },
        requester_type: "user",
      }),
    );
    // non-optimistic honest feedback (the daemon accepted + auto-executed the rescan).
    expect((await screen.findByRole("status")).textContent).toMatch(/scanning .*auth-service/i);
  });

  it("add_project_cancel_is_a_noop", async () => {
    // user dismisses the picker → no intent, no notice (never submit a blank/absent path).
    const pickFolder = vi.fn().mockResolvedValue(null);
    const { port } = renderContainer({ status: CONNECTED, pickFolder });
    const spy = vi.spyOn(port, "submit_action");

    fireEvent.click(addButton());

    await waitFor(() => expect(pickFolder).toHaveBeenCalled());
    expect(spy).not.toHaveBeenCalled();
    expect(screen.queryByRole("status")).toBeNull();
  });

  it("add_project_surfaces_a_verbatim_daemon_rejection", async () => {
    // a daemon §6.4 rejection → an honest error notice carrying the code VERBATIM (never collapsed).
    const pickFolder = vi.fn().mockResolvedValue("/repos/x");
    const port = new MockGatewayPort({ mutationError: { code: "precondition_stale" } });
    renderContainer({ status: CONNECTED, port, pickFolder });

    fireEvent.click(addButton());

    expect((await screen.findByRole("status")).textContent).toMatch(/precondition_stale/i);
  });
});
