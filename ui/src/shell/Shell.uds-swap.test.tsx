// @vitest-environment jsdom
//
// Layer C-single-shot (P6.8 L1, slice 051) — the Shell read-swap: the PRODUCTION Shell
// (main.tsx <Shell/>, no gateway prop) now defaults to the real UdsGatewayPort, so the
// initial get_projection/get_capabilities load shows REAL daemon data (via the 050
// invoke bridge), not the MockGatewayPort fixtures. The Mock stays injectable for the
// rest of the suite. A transport fault on load → the read-only degraded surface (§11.1).
import { describe, it, expect, afterEach, beforeEach, vi } from "vitest";
import {
  cleanup,
  render,
  screen,
  fireEvent,
  waitFor,
  within,
} from "@testing-library/react";

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
// calls invoke, so a non-empty invoke call proves the UDS port is the one in use). `Channel`
// is stubbed so the live subscribe path (a dedicated Tauri Channel) constructs (the L2-C
// production-Shell pins need the subscribe to stay open so the connection stays connected).
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  Channel: class {
    onmessage: ((e: unknown) => void) | undefined = undefined;
  },
}));

import { invoke } from "@tauri-apps/api/core";
import { Shell } from "./Shell";
import { CONTRACT_VERSION } from "../contracts/index";
import { projectActivityFixture } from "../projections/fixtures/proj_project_activity";
import { sessionPageFixture } from "../projections/fixtures/proj_session";
import { pullRequestFixture } from "../projections/fixtures/proj_pull_request";
import { approvalQueueFixture } from "../projections/fixtures/proj_approval_queue";
import { auditTrailFixture } from "../projections/fixtures/proj_audit_trail";
import { usageFixture } from "../projections/fixtures/proj_usage";
import { reviewFixture } from "../projections/fixtures/proj_review";

const mockInvoke = vi.mocked(invoke);

const PROJECTION_FIXTURES: Record<string, unknown> = {
  ProjectActivity: projectActivityFixture,
  Session: sessionPageFixture,
  PullRequest: pullRequestFixture,
  ApprovalQueue: approvalQueueFixture,
  AuditTrail: auditTrailFixture,
  UsageLedger: usageFixture,
  Review: reviewFixture,
};

afterEach(cleanup);
beforeEach(() => {
  mockInvoke.mockReset();
});

// A minimal valid diff served for get_diff / get_pr_diff (the Code view + PR workspace fetch on mount).
const STUB_DIFF = {
  hunks: [
    {
      header: "@@ -1,1 +1,1 @@",
      old_start: 1,
      old_lines: 1,
      new_start: 1,
      new_lines: 1,
      lines: [{ kind: "context", content: "x\n" }],
    },
  ],
};

// ui-080 — the SHARED production-Shell invoke mock (unifies the productionShellAt{CodeView,PrWorkspace}
// boilerplate). A connected + version-compatible daemon (the live CONTRACT_VERSION, not a hardcoded
// string — checkVersionCompat keys on protocol_version, so this is cleanliness) serving the projection
// fixtures, a stub diff for get_diff/get_pr_diff, a fake submit_action ack, and a NEVER-settling subscribe
// (the live stream stays OPEN → the supervisor never degrades → connection stays `connected`,
// canSubmitIntent true — required for the go-live pins).
function installProductionShellInvoke(): void {
  mockInvoke.mockImplementation((cmd: string, args?: unknown) => {
    if (cmd === "gateway_get_capabilities") {
      return Promise.resolve({ protocol_version: 1, contract_version: CONTRACT_VERSION });
    }
    if (cmd === "gateway_get_projection") {
      return Promise.resolve(PROJECTION_FIXTURES[(args as { name: string }).name]);
    }
    if (cmd === "gateway_get_diff" || cmd === "gateway_get_pr_diff") {
      return Promise.resolve(STUB_DIFF);
    }
    if (cmd === "gateway_submit_action") {
      return Promise.resolve({ action_request_id: "ar_live", status: "submitted" });
    }
    if (cmd === "gateway_subscribe") {
      return new Promise(() => {});
    }
    return Promise.reject(new Error(`unexpected command ${cmd}`));
  });
}

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
      await screen.findAllByText(projectActivityFixture.rows[0]!.name!),
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

  // Render the PRODUCTION Shell (no gateway prop → the real UdsGatewayPort, mutations-enabled as of
  // L2-C) against an invoke mock for a connected + version-compatible daemon, navigate to the Code /
  // Diff Review view, and return the per-hunk Stage control. The submit command returns a fake ack so
  // a live click resolves. This is the production go-live surface (the real daemon is the deferred
  // operator walkthrough).
  async function productionShellAtCodeView(): Promise<HTMLElement> {
    installProductionShellInvoke();
    render(<Shell />); // no gateway prop → the production UdsGatewayPort, mutations-enabled (L2-C)
    await screen.findAllByText(projectActivityFixture.rows[0]!.name!); // loaded → connected + compatible
    const sidebar = screen.getByRole("navigation", { name: "Sidebar" });
    fireEvent.click(
      within(sidebar).getByRole("button", { name: /code \/ diff review/i }),
    );
    return screen.findByRole("button", { name: /^Stage hunk/ });
  }

  it("production_shell_enables_per_hunk_submit_when_connected", async () => {
    // spec(L2-C / §6.1 / §11.4 / §11.5) — THE go-live flip: the production Shell constructs the
    // UdsGatewayPort mutations-ENABLED, so with a connected + version-compatible daemon
    // (canSubmitIntent true) the cockpit's per-hunk submit control is LIVE (enabled). The ONLY way
    // the button enables is `canSubmitIntent && gateway.mutationsEnabled` — so an enabled control
    // PROVES the production port has mutationsEnabled true (the daemon Gateway stays the INV-SEC-1
    // chokepoint; the gate is defense-in-depth). The Mock-injected path is unaffected.
    const stage = await productionShellAtCodeView();
    expect(stage).toHaveProperty("disabled", false);
  });

  it("production_shell_click_reaches_live_mutation_transport", async () => {
    // spec(L2-C / §6.1) — the go-live CORE: a real user action on the production cockpit reaches the
    // LIVE mutation transport. Clicking the enabled Stage → seam → the production port's live `invoke`
    // of the typed mutation command (`gateway_submit_action`) → the daemon Gateway. (The real daemon
    // executing + auditing it is the DEFERRED operator walkthrough; this is the deterministic proxy.)
    const stage = await productionShellAtCodeView();
    fireEvent.click(stage);
    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith(
        "gateway_submit_action",
        expect.objectContaining({ request: expect.anything() }),
      ),
    );
  });
});

// ─── ui-075 commit 2 (cat-1) — the PR-mutations go-live flip production-shell pins ──────────────────
// The cat-1 go-live: an ENABLED PR-mutation control here PROVES the production UdsGatewayPort carries the
// action_type in `enabledPrMutations` (the only enablement path is `canSubmit && isPrMutationEnabled(
// gateway, type) && head_sha != null`; the daemon Gateway stays the INV-SEC-1 chokepoint, this UI gate is
// defense-in-depth). Un-skipped + the Shell flip applied together at commit 2 (post HITL visual-gate PASS).
describe("Shell PR-mutations go-live (cat-1 ui-075)", () => {
  // Render the PRODUCTION Shell (no gateway prop → the real UdsGatewayPort) against a connected +
  // version-compatible daemon, navigate Code/Diff → "Pull requests" → the head_sha'd fixture PR
  // (project_fixture_1 "Add OAuth device flow"), landing in the PR Review Workspace. gateway_submit_action
  // returns a fake ack so a live click resolves; gateway_subscribe never settles so the stream stays
  // connected (canSubmitIntent true) for the pins.
  async function productionShellAtPrWorkspace(): Promise<void> {
    installProductionShellInvoke();
    render(<Shell />); // no gateway prop → the production UdsGatewayPort (PR-mutations enabled at commit 2)
    await screen.findAllByText(projectActivityFixture.rows[0]!.name!); // loaded → connected + compatible
    const sidebar = screen.getByRole("navigation", { name: "Sidebar" });
    fireEvent.click(
      within(sidebar).getByRole("button", { name: /code \/ diff review/i }),
    );
    fireEvent.click(await screen.findByRole("button", { name: /Pull requests/i }));
    fireEvent.click(await screen.findByRole("button", { name: /Add OAuth device flow/i }));
  }

  it("production_shell_enables_pr_merge_when_connected", async () => {
    // spec(P7.4 / §7.2 / §11.2 / [[28]]) — the go-live flip: the production Shell constructs the
    // UdsGatewayPort with `enabledPrMutations: PR_MUTATION_ACTION_TYPES`, so with a connected daemon
    // serving a head_sha'd PR the Merge control is LIVE (enabled). An enabled control proves the
    // production port carries `github.merge_pr`.
    await productionShellAtPrWorkspace();
    const merge = await screen.findByRole("button", { name: /^Merge/i });
    expect(merge).toHaveProperty("disabled", false);
  });

  it("production_shell_enables_pr_review_when_connected", async () => {
    // spec(P7.4 / §7.2 / §11.2 / [[28]]) — both-at-once: the Approve PR verdict control is also LIVE
    // (enabled), proving the production port carries `github.submit_review` too (not a staged single flip).
    await productionShellAtPrWorkspace();
    const approve = await screen.findByRole("button", { name: /Approve PR/i });
    expect(approve).toHaveProperty("disabled", false);
  });

  it("production_shell_pr_merge_click_reaches_live_transport", async () => {
    // spec(§6.1 / [[28]]) — a real Merge click on the production cockpit reaches the LIVE mutation
    // transport (`invoke("gateway_submit_action", …)` → the daemon Gateway). The real daemon executing +
    // auditing it is the DEFERRED operator walkthrough; this is the deterministic proxy.
    await productionShellAtPrWorkspace();
    fireEvent.click(await screen.findByRole("button", { name: /^Merge/i }));
    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith(
        "gateway_submit_action",
        expect.objectContaining({ request: expect.anything() }),
      ),
    );
  });

  it("production_shell_pr_review_click_reaches_live_transport", async () => {
    // spec(§6.1 / [[28]]) — the second mutation: a real Approve click reaches the live transport too.
    await productionShellAtPrWorkspace();
    fireEvent.click(await screen.findByRole("button", { name: /Approve PR/i }));
    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith(
        "gateway_submit_action",
        expect.objectContaining({ request: expect.anything() }),
      ),
    );
  });
});
