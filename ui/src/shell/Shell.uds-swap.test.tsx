// @vitest-environment jsdom
//
// Layer C-single-shot (P6.8 L1, slice 051) — the Shell read-swap: the PRODUCTION Shell
// (main.tsx <Shell/>, no gateway prop) now defaults to the real UdsGatewayPort, so the
// initial get_projection/get_capabilities load shows REAL daemon data (via the 050
// invoke bridge), not the MockGatewayPort fixtures. The Mock stays injectable for the
// rest of the suite. A transport fault on load → the read-only degraded surface (§11.1).
import { describe, it, expect, afterEach, beforeEach, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";

// xterm is a canvas lib (not jsdom-friendly) — mirror Shell.test.tsx's mock so the
// import chain never boots real xterm (the default "command" view doesn't, but be safe).
vi.mock("@xterm/xterm", () => ({
  Terminal: class {
    open() {}
    write() {}
    loadAddon() {}
    dispose() {}
    onData() {
      return { dispose() {} };
    }
    resize() {}
  },
}));
vi.mock("@xterm/addon-fit", () => ({
  FitAddon: class {
    fit() {}
    activate() {}
    dispose() {}
  },
}));

// The real transport reaches the daemon through the 050 Tauri invoke bridge — mock it
// so the DEFAULT Shell (UdsGatewayPort) loads contract-valid fixtures (the Mock never
// calls invoke, so a non-empty invoke call proves the UDS port is the one in use).
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { invoke } from "@tauri-apps/api/core";
import { Shell } from "./Shell";
import { projectActivityFixture } from "../projections/fixtures/proj_project_activity";
import { sessionPageFixture } from "../projections/fixtures/proj_session";
import { pullRequestFixture } from "../projections/fixtures/proj_pull_request";
import { approvalQueueFixture } from "../projections/fixtures/proj_approval_queue";
import { auditTrailFixture } from "../projections/fixtures/proj_audit_trail";
import { usageFixture } from "../projections/fixtures/proj_usage";

const mockInvoke = vi.mocked(invoke);

const PROJECTION_FIXTURES: Record<string, unknown> = {
  ProjectActivity: projectActivityFixture,
  Session: sessionPageFixture,
  PullRequest: pullRequestFixture,
  ApprovalQueue: approvalQueueFixture,
  AuditTrail: auditTrailFixture,
  UsageLedger: usageFixture,
};

afterEach(cleanup);
beforeEach(() => {
  mockInvoke.mockReset();
});

describe("Shell read-swap (Layer C-single-shot — UdsGatewayPort default)", () => {
  it("shell_defaults_to_uds_and_renders_real_daemon_data", async () => {
    mockInvoke.mockImplementation((cmd: string, args?: unknown) => {
      if (cmd === "gateway_get_capabilities") {
        return Promise.resolve({ protocol_version: 1, contract_version: "0.28.0" });
      }
      if (cmd === "gateway_get_projection") {
        const name = (args as { name: string }).name;
        return Promise.resolve(PROJECTION_FIXTURES[name]);
      }
      // gateway_get_diff is NOT part of the Shell's initial load (DiffReview fetches it
      // on demand from the Code view) — if a future load change calls it, this surfaces
      // as a clear "unexpected command" rather than a silent missing-fixture pass.
      return Promise.reject(new Error(`unexpected command ${cmd}`));
    });

    // No gateway prop → the production default (UdsGatewayPort), exactly as main.tsx mounts.
    render(<Shell />);

    // Real daemon data rendered (the project name from the loaded ProjectActivity page).
    expect(
      await screen.findAllByText(projectActivityFixture.rows[0]!.name),
    ).not.toHaveLength(0);
    // The UDS port reached the daemon via invoke — the Mock NEVER invokes, so this
    // proves the read-swap (the real client is in use, not the fixture mock).
    expect(mockInvoke).toHaveBeenCalledWith(
      "gateway_get_projection",
      expect.objectContaining({ name: "Session" }),
    );
    expect(mockInvoke).toHaveBeenCalledWith("gateway_get_capabilities");
  });

  it("shell_transport_fault_renders_read_only_degraded", async () => {
    // an io transport fault on load → the UdsGatewayPort throws an Error → the load
    // Promise.all rejects → the Shell's handled read-only/degraded surface (never a crash,
    // never stale-as-live; §11.1/§11.7).
    mockInvoke.mockRejectedValue({ kind: "io", message: "connection refused" });
    render(<Shell />);
    expect(await screen.findByTestId("shell-load-error")).toBeTruthy();
  });
});
