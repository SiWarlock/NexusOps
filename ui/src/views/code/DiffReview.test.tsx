// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from "vitest";
import { cleanup, render, screen, fireEvent, waitFor } from "@testing-library/react";
afterEach(cleanup);
import { DiffReview } from "./DiffReview";
import { ReadOnlyProvider, type ConnectionStatus } from "../../connection/read-only";
import { MockGatewayPort } from "../../gateway-client/mock";
import { hunkResourceRef } from "../../intent/hunk-resource-ref";
import { diffReviewContext } from "../display-fixtures";
import type { DiffResult, WireError } from "../../contracts/index";

const CONNECTED: ConnectionStatus = { connection: "connected", version: "compatible" };
const DEGRADED: ConnectionStatus = { connection: "connecting", version: "unknown" };

// A known diff the tests control (1 hunk) — so the resource_ref pin can compute the
// exact expected id from the SAME hunk the component renders.
const DIFF: DiffResult = {
  hunks: [
    {
      header: "@@ -10,4 +10,6 @@",
      old_start: 10,
      old_lines: 4,
      new_start: 10,
      new_lines: 6,
      // content EXCLUDES the +/- origin (daemon-faithful: git2 line.content()).
      lines: [
        { kind: "context", content: "export function review() {\n" },
        { kind: "removed", content: "  return execute()\n" },
        { kind: "added", content: "  const dry = dryRun()\n" },
      ],
    },
  ],
};
const HUNK = DIFF.hunks[0]!;

function renderReview(
  opts: {
    status?: ConnectionStatus;
    port?: MockGatewayPort;
    diff?: DiffResult;
    rejectDiff?: WireError;
  } = {},
) {
  const port = opts.port ?? new MockGatewayPort();
  if (opts.rejectDiff) {
    vi.spyOn(port, "get_diff").mockRejectedValue(opts.rejectDiff);
  } else {
    vi.spyOn(port, "get_diff").mockResolvedValue(opts.diff ?? DIFF);
  }
  const utils = render(
    <ReadOnlyProvider value={opts.status ?? CONNECTED}>
      <DiffReview prs={[]} gateway={port} />
    </ReadOnlyProvider>,
  );
  return { port, ...utils };
}

// ─── L1: diff sourced from get_diff (READ, non-safety) ───────────────────────
describe("DiffReview L1 — diff from get_diff (§6.1)", () => {
  it("review_tab_sources_diff_from_get_diff", async () => {
    // spec(§6.1) — the Review tab renders get_diff's DiffResult hunks (not a static fixture).
    const { port } = renderReview();
    await waitFor(() =>
      expect(port.get_diff).toHaveBeenCalledWith(
        diffReviewContext.worktreeId,
        diffReviewContext.file,
      ),
    );
    // the rendered diff is the get_diff content (a line unique to DIFF, not the old fixture)
    expect(await screen.findByText(/const dry = dryRun/)).toBeTruthy();
  });

  it("get_diff_not_found_renders_honest_empty", async () => {
    // spec(forbidden#2) — a not_found/error → an honest unavailable state, NEVER a
    // fabricated diff (a read error is not a mutation-rejection safety card either).
    renderReview({ rejectDiff: { code: "not_found" } });
    expect(await screen.findByTestId("diff-unavailable")).toBeTruthy();
    expect(screen.queryByText(/const dry = dryRun/)).toBeNull(); // no fabricated content
  });

  it("clean_file_renders_honest_no_changes", async () => {
    // spec(§11.7) — a clean file (no hunks) renders an honest "no changes", not blank/faked.
    renderReview({ diff: { hunks: [] } });
    expect(await screen.findByTestId("diff-no-changes")).toBeTruthy();
  });

  it("diff_fixture_content_excludes_origin_char", () => {
    // spec(§6.1) — the frozen DiffLine.content mirrors the daemon's git2 line.content(): it
    // EXCLUDES the +/- origin (the kind carries it; the kit re-adds the sign). A fixture that
    // bakes +/- into content misrepresents the contract → doubled signs. Guard fidelity.
    for (const hunk of DIFF.hunks)
      for (const line of hunk.lines)
        expect(/^[+-]/.test(line.content), `content baked an origin char: ${line.content}`).toBe(
          false,
        );
  });

  it("unexpected_get_diff_error_degrades_not_crashes", async () => {
    // spec(§11.7/LESSON§16) — a real (non-WireError) Error from get_diff DEGRADES to the
    // honest unavailable state (a read failure must not crash the cockpit), and is surfaced
    // via console.error (never SILENTLY swallowed), never re-thrown as an unhandled rejection.
    const errSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    const port = new MockGatewayPort();
    vi.spyOn(port, "get_diff").mockRejectedValue(new Error("transport down"));
    render(
      <ReadOnlyProvider value={CONNECTED}>
        <DiffReview prs={[]} gateway={port} />
      </ReadOnlyProvider>,
    );
    expect(await screen.findByTestId("diff-unavailable")).toBeTruthy();
    expect(errSpy).toHaveBeenCalled(); // surfaced, not swallowed
    errSpy.mockRestore();
  });
});

// ─── L2: per-hunk submission over the seam (CAT-1) ───────────────────────────
describe("DiffReview L2 — per-hunk intent submission (cat-1)", () => {
  it("per_hunk_buttons_disabled_when_cannot_submit", async () => {
    // spec(§11.6/Q2) — fail-safe gate: in degraded/read-only the per-hunk git buttons are
    // disabled (not faked); defense-in-depth (the daemon Gateway is the real chokepoint).
    renderReview({ status: DEGRADED });
    const stage = await screen.findByRole("button", { name: /^Stage hunk/ });
    expect(stage).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: /^Unstage hunk/ })).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: /^Discard hunk/ })).toHaveProperty("disabled", true);
  });

  it("stage_hunk_submits_typed_intent_never_executes", async () => {
    // spec(Q1/§4.2) — clicking Stage submits a typed git.stage_hunk ActionRequest via the
    // seam → the single GatewayPort; the UI NEVER performs a git op (pure submitter).
    const { port } = renderReview();
    const submitSpy = vi.spyOn(port, "submit_action");
    fireEvent.click(await screen.findByRole("button", { name: /^Stage hunk/ }));
    await waitFor(() => expect(submitSpy).toHaveBeenCalledTimes(1));
    const req = submitSpy.mock.calls[0]![0];
    expect(req.action_type).toBe("git.stage_hunk");
    // THE security pin: the submitted resource_ref targets the EXACT displayed hunk.
    expect(req.resource_refs[0]!.id).toBe(
      hunkResourceRef(diffReviewContext.worktreeId, diffReviewContext.file, HUNK).id,
    );
    expect(req.resource_refs[0]!.type).toBe("file");
  });

  it("discard_hunk_submits_discard_action_type", async () => {
    // spec(Q1/§6.3) — Discard submits git.discard_hunk (risk-3 daemon-side) for THIS hunk.
    const { port } = renderReview();
    const submitSpy = vi.spyOn(port, "submit_action");
    fireEvent.click(await screen.findByRole("button", { name: /^Discard hunk/ }));
    await waitFor(() => expect(submitSpy).toHaveBeenCalledTimes(1));
    expect(submitSpy.mock.calls[0]![0].action_type).toBe("git.discard_hunk");
    expect(submitSpy.mock.calls[0]![0].resource_refs[0]!.id).toBe(
      hunkResourceRef(diffReviewContext.worktreeId, diffReviewContext.file, HUNK).id,
    );
  });

  it("submit_opens_gateway_modal_with_daemon_policy_not_ui_risk", async () => {
    // spec(Q4) — on submit the GatewayModal renders the DAEMON's PolicyDecision (read from
    // the daemon-shaped enrichment), never a UI-derived risk; the hunk bar shows no risk.
    renderReview();
    fireEvent.click(await screen.findByRole("button", { name: /^Discard hunk/ }));
    expect(await screen.findByTestId("gateway-modal")).toBeTruthy();
    // the card's policy status is the daemon's (require_approval), read from PolicyDecision
    expect((await screen.findByTestId("policy-status")).textContent).toBe("require_approval");
  });

  it("discard_preview_is_daemon_actionpreview_not_fabricated", async () => {
    // spec(Q5/forbidden#2) — the discard consequence comes from the daemon's ActionPreview
    // (the modal fetches preview_action via the port); DiffReview never fabricates a preview.
    const { port } = renderReview();
    const previewSpy = vi.spyOn(port, "preview_action");
    fireEvent.click(await screen.findByRole("button", { name: /^Discard hunk/ }));
    await screen.findByTestId("gateway-modal");
    await waitFor(() => expect(previewSpy).toHaveBeenCalled()); // the daemon's preview, fetched
  });

  it("no_optimistic_done_on_submit", async () => {
    // spec(Q3) — submit renders the daemon-reported pending status (awaiting_approval), NEVER
    // an optimistic "staged"/"done"; a hunk applies only on a confirming projection/result.
    renderReview();
    fireEvent.click(await screen.findByRole("button", { name: /^Stage hunk/ }));
    await screen.findByTestId("gateway-modal");
    // no optimistic applied-badge on the hunk
    expect(screen.queryByTestId("hunk-applied")).toBeNull();
    // the modal shows the daemon's pending Approval status, never "done"/"executed"
    const modal = screen.getByTestId("gateway-modal");
    expect(modal.textContent ?? "").not.toMatch(/\b(done|executed|succeeded|completed)\b/i);
  });

  it("submit_rejection_routes_through_describe_rejection", async () => {
    // spec(Q6/§6.4) — a daemon rejection routes through describeRejection → the distinct
    // §11.5 card; fencing_conflict is never re-approvable (#6), code shown verbatim.
    const port = new MockGatewayPort({ mutationError: { code: "fencing_conflict" } });
    renderReview({ port });
    fireEvent.click(await screen.findByRole("button", { name: /^Stage hunk/ }));
    const reject = await screen.findByTestId("result-reject");
    expect(reject.getAttribute("data-reject-kind")).toBe("hard_conflict");
    expect(screen.getByTestId("reject-code").textContent).toBe("fencing_conflict");
    expect(screen.queryByTestId("reapprove")).toBeNull(); // never re-approvable (#6)
  });

  it("standing_grant_affordance_stays_disabled", async () => {
    // spec(§11.5/§15) — the "always allow" standing-grant stays DISABLED (its own cat-1
    // checkpoint; git.discard_hunk is non-standing-grantable daemon-side regardless).
    renderReview();
    fireEvent.click(await screen.findByRole("button", { name: /^Discard hunk/ }));
    await screen.findByTestId("gateway-modal");
    expect(screen.getByTestId("always-allow")).toHaveProperty("disabled", true);
  });
});
