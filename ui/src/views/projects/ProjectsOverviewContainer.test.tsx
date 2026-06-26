// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from "vitest";
import { cleanup, render, screen, fireEvent, waitFor } from "@testing-library/react";
afterEach(cleanup);
import { ProjectsOverviewContainer } from "./ProjectsOverviewContainer";
import { ReadOnlyProvider, type ConnectionStatus } from "../../connection/read-only";
import { MockGatewayPort } from "../../gateway-client/mock";
import type { ProjectActivityRow } from "../../contracts/index";

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

// ui-082 — the Scanning notice resolves on the project appearing + a timeout fallback (never hangs).
describe("ProjectsOverviewContainer — Scanning resolve + timeout (ui-082)", () => {
  function renderResolve(opts: {
    onSelectProject?: (id: string) => void;
    scanTimeoutMs?: number;
    pickFolder?: () => Promise<string | null>;
  }) {
    const port = new MockGatewayPort();
    const view = (projects: ProjectActivityRow[]) => (
      <ReadOnlyProvider value={CONNECTED}>
        <ProjectsOverviewContainer
          gateway={port}
          projects={projects}
          counts={{}}
          activeProjectId={null}
          onSelectProject={opts.onSelectProject ?? (() => {})}
          pickFolder={opts.pickFolder ?? (() => Promise.resolve("/repos/auth-service"))}
          scanTimeoutMs={opts.scanTimeoutMs}
        />
      </ReadOnlyProvider>
    );
    const utils = render(view([]));
    // NB: spread utils FIRST, then our typed rerender — else utils.rerender (which takes a React
    // element) shadows ours and the test would pass a raw row array to RTL.
    return { port, ...utils, rerender: (projects: ProjectActivityRow[]) => utils.rerender(view(projects)) };
  }

  it("container_resolves_scanning_when_project_appears", async () => {
    // spec(daemon 091 pairing) — after an "ok" submit, when the minted project_id appears in `projects`
    // the Scanning notice resolves (clears the pending state) + the new project is auto-selected.
    const onSelectProject = vi.fn();
    const { port, rerender } = renderResolve({ onSelectProject });
    const spy = vi.spyOn(port, "submit_action");
    fireEvent.click(addButton());
    await waitFor(() => expect(spy).toHaveBeenCalledTimes(1));
    const projectId = (spy.mock.calls[0]![0] as { project_id: string }).project_id;
    expect((await screen.findByRole("status")).textContent).toMatch(/scanning/i);

    rerender([{ project_id: projectId, name: "auth-service" } as ProjectActivityRow]);

    await waitFor(() => expect(onSelectProject).toHaveBeenCalledWith(projectId));
    expect(screen.getByRole("status").textContent ?? "").not.toMatch(/scanning/i);
  });

  it("container_scanning_times_out_to_an_honest_fallback", async () => {
    // spec(§11.4/§11.7) — if the project never appears within the injectable timeout, the notice
    // becomes an honest "registered — refresh" fallback (never an indefinite hang, never a false error).
    renderResolve({ scanTimeoutMs: 30 });
    fireEvent.click(addButton());
    expect((await screen.findByRole("status")).textContent).toMatch(/scanning/i);
    await waitFor(() => expect(screen.getByRole("status").textContent ?? "").toMatch(/refresh/i));
    expect(screen.getByRole("status").textContent ?? "").not.toMatch(/scanning/i);
  });

  it("container_clears_the_scan_timer_on_unmount", async () => {
    // spec(no-leak) — a pending scan timer is cleared on unmount (no stale setState / no leak).
    const clearSpy = vi.spyOn(globalThis, "clearTimeout");
    const { port, unmount } = renderResolve({ scanTimeoutMs: 5000 });
    const spy = vi.spyOn(port, "submit_action");
    fireEvent.click(addButton());
    await waitFor(() => expect(spy).toHaveBeenCalledTimes(1));
    await screen.findByRole("status"); // pending scan → timer armed
    clearSpy.mockClear();
    unmount();
    expect(clearSpy).toHaveBeenCalled();
    clearSpy.mockRestore(); // don't leak the global spy onto later tests
  });
});
