// @vitest-environment jsdom
//
// ui-075 commit 1 — the dev-shell visual-gate harness (NON-cat-1). main.tsx gains a
// build-time env-gated Mock-injection branch (`VITE_NEXUSOPS_MOCK`) so the daemon-free
// visual gate can drive the PR-mutation workspace with ENABLED controls (the pixel-check
// surface). The DEFAULT (env-unset) production path constructs NO Mock → Shell falls back
// to the production UdsGatewayPort; the env-set path injects a MockGatewayPort. The Mock
// PR fixture carries a real head_sha so `prHeadSha != null` ⇒ Merge + Approve enable.
//
// Importing main.tsx is side-effect-safe here: its bootstrap is guarded `if (rootEl)` and
// jsdom has no `#root`, so `resolveEntryGateway` imports without mounting (no createRoot).
import { describe, it, expect, afterEach, vi } from "vitest";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";

// main.tsx → Shell → terminal modules transitively import xterm (a canvas lib, not
// jsdom-friendly). Mirror Shell.uds-swap.test.tsx's mock so the import chain never boots it.
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

import type { DiffResult } from "./contracts/index";
import { resolveEntryGateway } from "./main";
import { MockGatewayPort } from "./gateway-client/mock";
import { ReadOnlyProvider, type ConnectionStatus } from "./connection/read-only";
import { DiffReview } from "./views/code/DiffReview";
import { prHeadSha } from "./views/code/PrWorkspace";
import { pullRequestFixture } from "./projections/fixtures/proj_pull_request";

const CONNECTED: ConnectionStatus = { connection: "connected", version: "compatible" };

// A minimal valid PR code-diff so the PR workspace's on-mount get_pr_diff resolves (the
// control enablement does NOT depend on the diff content — canMerge/canReview key off
// canSubmit + isPrMutationEnabled + head_sha — but resolving avoids an honest-error frame).
const PR_DIFF: DiffResult = {
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

afterEach(() => {
  cleanup();
  vi.unstubAllEnvs();
});

describe("ui-075 commit 1 — dev-shell visual-gate harness", () => {
  it("main_default_path_uses_production_uds_no_mock", () => {
    // spec(§15) — with VITE_NEXUSOPS_MOCK unset (the production-build default) the entry resolves NO
    // gateway → Shell falls back to the production UdsGatewayPort. This pins the SOURCE-LEVEL fail-safe
    // default (the load-bearing prod-no-Mock property); the bundle-EXCLUSION half (Mock symbol absent from
    // a `vite build` output via dead-code elimination) is a build-pipeline concern, not unit-testable here.
    expect(resolveEntryGateway()).toBeUndefined();
  });

  it("main_mock_env_injects_mock_gateway", () => {
    // spec(§15) — with the env flag set to the allowlist value "1" the entry injects a MockGatewayPort
    // (the daemon-free visual-gate seam; env-isolated so it can't leak into a production build).
    vi.stubEnv("VITE_NEXUSOPS_MOCK", "1");
    expect(resolveEntryGateway()).toBeInstanceOf(MockGatewayPort);
  });

  it("main_falsy_string_env_fails_closed_to_production", () => {
    // spec(§15 / §11.4 fail-closed) — the guard is an EXPLICIT-ALLOWLIST (`=== "1"`), NOT a truthiness
    // check: a misguided `VITE_NEXUSOPS_MOCK=0` / `=false` "disable" attempt is string-TRUTHY in JS — a
    // truthiness guard would INVERT to inject the Mock into a production build. The allowlist fails closed
    // to the production UdsGatewayPort for any value that isn't exactly "1" (security-reviewer [low] fix).
    vi.stubEnv("VITE_NEXUSOPS_MOCK", "0");
    expect(resolveEntryGateway()).toBeUndefined();
  });

  it("mock_pr_fixture_enables_pr_controls", async () => {
    // spec(§11.2) — under the Mock (connected; enabledPrMutations full by default) the fixture
    // PR's real head_sha makes prHeadSha != null ⇒ Merge + Approve ENABLE — the visual gate
    // needs the controls rendered enabled to pixel-check them against the prototype.
    expect(prHeadSha(pullRequestFixture.rows[0]!)).not.toBeNull();
    const port = new MockGatewayPort();
    vi.spyOn(port, "get_pr_diff").mockResolvedValue(PR_DIFF);
    render(
      <ReadOnlyProvider value={CONNECTED}>
        <DiffReview prs={[pullRequestFixture.rows[0]!]} reviews={[]} gateway={port} />
      </ReadOnlyProvider>,
    );
    fireEvent.click(screen.getByRole("button", { name: /Pull requests/i }));
    // The PR card's selecting button name includes more than the title (status/branch) → substring
    // regex on the fixture row-0 title, mirroring DiffReview.test.tsx's renderMerge.
    fireEvent.click(
      await screen.findByRole("button", { name: /Add OAuth device flow/i }),
    );
    expect(
      (await screen.findByRole("button", { name: /^Merge/i }) as HTMLButtonElement)
        .disabled,
    ).toBe(false);
    expect(
      (screen.getByRole("button", { name: /Approve PR/i }) as HTMLButtonElement).disabled,
    ).toBe(false);
  });
});
