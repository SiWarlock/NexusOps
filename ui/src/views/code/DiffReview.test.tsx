// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from "vitest";
import { cleanup, render, screen, fireEvent, waitFor } from "@testing-library/react";
afterEach(cleanup);
import { DiffReview } from "./DiffReview";
import { ReadOnlyProvider, type ConnectionStatus } from "../../connection/read-only";
import { ZodError } from "zod";
import { MockGatewayPort } from "../../gateway-client/mock";
import { BoundaryValidationError } from "../../gateway-client/boundary";
import { hunkResourceRef } from "../../intent/hunk-resource-ref";
import { makeApprovalRow } from "../../projections/fixtures/proj_approval_queue";
import { diffReviewContext } from "../display-fixtures";
import type {
  ApprovalQueuePage,
  DiffResult,
  PullRequestRow,
  WireError,
} from "../../contracts/index";

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
      <DiffReview prs={[]} reviews={[]} gateway={port} />
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
        <DiffReview prs={[]} reviews={[]} gateway={port} />
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

  it("per_hunk_buttons_disabled_when_mutations_not_enabled", async () => {
    // spec(L2-B 🔒 L2-O3) — even CONNECTED (canSubmitIntent true), the per-hunk git buttons stay
    // disabled while the port's `mutationsEnabled` gate is false (the L2-B honest disabled state —
    // wired-but-not-yet-enabled; the go-live enable is L2-C). L2-B enables nothing in the UI.
    renderReview({ status: CONNECTED, port: new MockGatewayPort({ mutationsEnabled: false }) });
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
    // spec(Q4) — on submit the GatewayModal renders the DAEMON's PolicyDecision, never a
    // UI-derived risk; the hunk bar shows no risk. With the default mock the freshly-minted
    // approval isn't in the ApprovalQueue snapshot yet (053b absent-row timing) → the honest
    // awaiting placeholder, whose status is still "require_approval".
    renderReview();
    fireEvent.click(await screen.findByRole("button", { name: /^Discard hunk/ }));
    expect(await screen.findByTestId("gateway-modal")).toBeTruthy();
    expect((await screen.findByTestId("policy-status")).textContent).toBe("require_approval");
  });

  it("per_hunk_modal_shows_real_row_policy_on_match", async () => {
    // spec(§11.5/044) — 053b: on submit, enrichHunkAction re-fetches the ApprovalQueue, matches
    // the minted action_request_id, and the modal renders the daemon ROW's REAL policy (not a
    // UI-invented per-hunk reason). The mock mints ar_mock_0001 → serve a row with that id.
    const port = new MockGatewayPort();
    // Typed as ApprovalQueuePage so the mock payload's SHAPE is compiler-checked (not silenced with
    // `as never`); the cast only bridges the generic get_projection spy's union return.
    const realRowPage: ApprovalQueuePage = {
      projection: "ApprovalQueue",
      rows: [
        makeApprovalRow({
          approval_id: "appr_live",
          action_request_id: "ar_mock_0001",
          risk_level: 4,
          policy_decision: {
            status: "deny",
            reasons: ["Daemon: discards tracked content outside the worktree."],
            required_approvals: [{ kind: "project_owner" }],
            constraints: [],
            safer_alt: null,
          },
        }),
      ],
      cursor: null,
    };
    vi.spyOn(port, "get_projection").mockResolvedValue(realRowPage);
    renderReview({ port });
    fireEvent.click(await screen.findByRole("button", { name: /^Discard hunk/ }));
    await screen.findByTestId("gateway-modal");
    // the modal's policy reasons are the daemon ROW's REAL policy, verbatim (not UI-invented).
    expect((await screen.findByTestId("policy-reasons")).textContent).toMatch(
      /discards tracked content outside the worktree/i,
    );
  });

  it("per_hunk_enrich_refetch_failure_degrades_honestly", async () => {
    // spec(§11.7/forbidden#2) — 053b: if the post-submit ApprovalQueue re-fetch fails (a malformed
    // payload → BoundaryValidationError, or a transport fault), the UI surfaces an HONEST notice (the
    // intent was recorded; the card preview couldn't load), NEVER a silent stall or a card built from
    // un-parsed data. The approval is still reachable in the global queue.
    const port = new MockGatewayPort();
    vi.spyOn(port, "get_projection").mockRejectedValue(
      new BoundaryValidationError("ApprovalQueue", new ZodError([])),
    );
    renderReview({ port });
    fireEvent.click(await screen.findByRole("button", { name: /^Discard hunk/ }));
    // an honest degraded notice surfaces…
    expect(await screen.findByTestId("enrich-unavailable")).toBeTruthy();
    // …and NO card is opened from un-enriched / un-parsed data.
    expect(screen.queryByTestId("gateway-modal")).toBeNull();
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

// ─── ui-064 Layer 2 — PR selection ↔ the worktree per-hunk diff coexistence ──────────────────────────
describe("DiffReview — PR Workspace selection (ui-064 Layer 2)", () => {
  const PR: PullRequestRow = {
    pr_id: "repo_1#101",
    project_id: "p1",
    repo_id: "repo_1",
    pr_number: 101,
    title: "Add OAuth device flow",
    status: "open",
    head_branch: "agent/auth",
    base_branch: "main",
    pr_checked_at: null,
    mergeable: true,
    checks_summary: "3/3 passing",
  };

  it("pr_card_opens_workspace_and_back_returns_to_worktree_diff", async () => {
    // spec(§11.2/LESSON §13) — selecting a PR card opens its read-only Workspace in the Review tab;
    // "← Worktree diff" deselects → the 6.3e worktree per-hunk diff returns (preserved, not deleted).
    const port = new MockGatewayPort();
    vi.spyOn(port, "get_diff").mockResolvedValue(DIFF);
    vi.spyOn(port, "get_pr_diff").mockResolvedValue(DIFF); // D7: the workspace fetches the PR code-diff
    render(
      <ReadOnlyProvider value={CONNECTED}>
        <DiffReview prs={[PR]} reviews={[]} gateway={port} />
      </ReadOnlyProvider>,
    );
    // open the Kanban + select PR #101 → the PR Workspace (its read-only PR code-diff is unique to it)
    fireEvent.click(screen.getByRole("button", { name: /Pull requests/i }));
    fireEvent.click(screen.getByRole("button", { name: /Add OAuth device flow/i }));
    expect(await screen.findByTestId("pr-diff")).toBeTruthy();
    // deselect → the worktree per-hunk diff returns (get_diff-sourced), the workspace is gone
    fireEvent.click(screen.getByRole("button", { name: /Worktree diff/i }));
    await waitFor(() => {
      expect(screen.queryByTestId("pr-diff")).toBeNull();
    });
    expect(port.get_diff).toHaveBeenCalled(); // the worktree ReviewTab is back (sources its diff)
  });
});

// ─── ui-069 — D7 PR code-diff via get_pr_diff (§11.2/§11.7) ──────────────────
// a diff whose content encodes (repo_id, pr_number) so a stale diff under the wrong PR is detectable.
const diffFor = (repo_id: string, pr_number: number): DiffResult => ({
  hunks: [
    {
      header: `@@ ${repo_id}#${pr_number} @@`,
      old_start: 1,
      old_lines: 1,
      new_start: 1,
      new_lines: 1,
      lines: [{ kind: "context", content: `diff for ${repo_id}#${pr_number}\n` }],
    },
  ],
});
const renderWithPrs = (prs: PullRequestRow[], port: MockGatewayPort) => {
  vi.spyOn(port, "get_diff").mockResolvedValue(DIFF);
  render(
    <ReadOnlyProvider value={CONNECTED}>
      <DiffReview prs={prs} reviews={[]} gateway={port} />
    </ReadOnlyProvider>,
  );
};
const openKanban = () =>
  fireEvent.click(screen.getByRole("button", { name: /Pull requests/i }));

describe("DiffReview — D7 PR code-diff (ui-069)", () => {
  const PR_A: PullRequestRow = {
    pr_id: "r1#84",
    project_id: "p1",
    repo_id: "r1",
    pr_number: 84,
    title: "PR Alpha",
    status: "open",
    head_branch: "agent/a",
    base_branch: "main",
    pr_checked_at: null,
    mergeable: true,
    checks_summary: null,
  };
  const PR_B: PullRequestRow = {
    ...PR_A,
    pr_id: "r2#85",
    repo_id: "r2",
    pr_number: 85,
    title: "PR Beta",
  };
  const PR_NO_REPO: PullRequestRow = {
    ...PR_A,
    pr_id: "r0#orphan",
    repo_id: null,
    title: "PR NoRepo",
  };

  it("diff_review_fetches_pr_diff_for_selected_pr", async () => {
    // spec(§11.2 + ui-064) — selecting a PR fetches get_pr_diff(repo_id, pr_number) once and the result
    // flows into the PR Workspace; the container owns the fetch (PrWorkspace is pure-display, no gateway).
    const port = new MockGatewayPort();
    const prDiffSpy = vi.spyOn(port, "get_pr_diff").mockResolvedValue(diffFor("r1", 84));
    renderWithPrs([PR_A], port);
    openKanban();
    fireEvent.click(screen.getByRole("button", { name: /PR Alpha/i }));
    await waitFor(() => expect(prDiffSpy).toHaveBeenCalledWith("r1", 84, null));
    expect(prDiffSpy).toHaveBeenCalledTimes(1);
    expect(await screen.findByText(/diff for r1#84/)).toBeTruthy();
  });

  it("pr_workspace_no_fetch_when_repo_or_pr_number_null", async () => {
    // spec(§11.7) — a null repo_id (or pr_number) → DON'T fetch get_pr_diff; render an honest
    // "no repo link" state (distinct from a daemon error, never a fabricated diff).
    const port = new MockGatewayPort();
    const prDiffSpy = vi.spyOn(port, "get_pr_diff").mockResolvedValue(diffFor("x", 1));
    renderWithPrs([PR_NO_REPO], port);
    openKanban();
    fireEvent.click(screen.getByRole("button", { name: /PR NoRepo/i }));
    expect(await screen.findByTestId("pr-diff-no-link")).toBeTruthy();
    expect(prDiffSpy).not.toHaveBeenCalled();
  });

  it("diff_review_reselect_pr_refetches_no_stale_diff", async () => {
    // spec(LESSON §17) — the fetch is keyed on the stable (repo_id, pr_number) primitives: reselecting a
    // different PR re-fires get_pr_diff with the NEW (repo_id, pr_number) and NEVER renders the prior
    // PR's diff under the new PR (no stale-diff-across-selection — a real correctness bug, not cosmetic).
    const port = new MockGatewayPort();
    const prDiffSpy = vi
      .spyOn(port, "get_pr_diff")
      .mockImplementation((repo_id, pr_number) => Promise.resolve(diffFor(repo_id, pr_number)));
    renderWithPrs([PR_A, PR_B], port);
    openKanban();
    fireEvent.click(screen.getByRole("button", { name: /PR Alpha/i }));
    expect(await screen.findByText(/diff for r1#84/)).toBeTruthy();
    // reselect PR B via the Kanban → re-fetch with B's primitives
    openKanban();
    fireEvent.click(screen.getByRole("button", { name: /PR Beta/i }));
    await waitFor(() => expect(prDiffSpy).toHaveBeenCalledWith("r2", 85, null));
    expect(await screen.findByText(/diff for r2#85/)).toBeTruthy();
    // PR A's diff is NEVER shown under PR B (no stale diff across selection).
    expect(screen.queryByText(/diff for r1#84/)).toBeNull();
  });
});

// ─── ui-067 item 1 — null-safe PR-number chip (§11.2/§11.7) ──────────────────
describe("DiffReview — null-safe PR-number chip (ui-067)", () => {
  // A PR with NO pr_number and NO title — the worst case for the chip + label (ui-061 nullable
  // reconcile). Identity must still come through via the always-present pr_id PK.
  const PR_NO_NUMBER: PullRequestRow = {
    pr_id: "repo_9#orphan",
    project_id: "p1",
    repo_id: "repo_9",
    pr_number: null,
    title: null,
    status: "open",
    head_branch: "agent/x",
    base_branch: "main",
    pr_checked_at: null,
    mergeable: null,
    checks_summary: null,
  };
  const PR_WITH_NUMBER: PullRequestRow = {
    ...PR_NO_NUMBER,
    pr_id: "repo_9#101",
    pr_number: 101,
    title: "Add device flow",
  };

  function renderPrs(prs: PullRequestRow[]) {
    const port = new MockGatewayPort();
    vi.spyOn(port, "get_diff").mockResolvedValue(DIFF);
    render(
      <ReadOnlyProvider value={CONNECTED}>
        <DiffReview prs={prs} reviews={[]} gateway={port} />
      </ReadOnlyProvider>,
    );
    // open the "Pull requests" Kanban so the PR cards render
    fireEvent.click(screen.getByRole("button", { name: /Pull requests/i }));
  }

  it("pr_chip_null_safe_when_pr_number_absent", () => {
    // spec(§11.2/§11.7) — a null pr_number renders NO bare-`#` chip (the tone="pr" badge is a
    // "#<number>" affordance; with no number it is omitted, never rendered as a lone "#"). The
    // card still identifies the PR via the label's pr_id fallback (:485) — honest display.
    renderPrs([PR_NO_NUMBER]);
    expect(screen.queryByText("#")).toBeNull(); // no bare-`#` chip
    expect(screen.getByText("repo_9#orphan")).toBeTruthy(); // identity via the pr_id label fallback
  });

  it("pr_chip_shows_number_when_present", () => {
    // spec(§11.2) — regression: a present pr_number still renders the "#<n>" chip (happy path).
    renderPrs([PR_WITH_NUMBER]);
    expect(screen.getByText("#101")).toBeTruthy();
  });
});

// ─── ui-070 cat-1 — github.merge_pr Merge control (guarded-disabled) ─────────────
// A PR with a captured head_sha (injected via cast — the daemon field isn't on PullRequestRow yet;
// prHeadSha reads it forward-compatibly). Production rows lack it → prHeadSha null → Merge disabled.
const PR_MERGEABLE = {
  pr_id: "repo_1#101",
  project_id: "p1",
  repo_id: "repo_1",
  pr_number: 101,
  title: "Add OAuth device flow",
  status: "open",
  head_branch: "agent/auth",
  base_branch: "main",
  pr_checked_at: null,
  mergeable: true,
  checks_summary: null,
  head_sha: "headsha123",
} as unknown as PullRequestRow;

function renderMerge(port: MockGatewayPort, status: ConnectionStatus = CONNECTED) {
  vi.spyOn(port, "get_diff").mockResolvedValue(DIFF);
  vi.spyOn(port, "get_pr_diff").mockResolvedValue(DIFF);
  render(
    <ReadOnlyProvider value={status}>
      <DiffReview prs={[PR_MERGEABLE]} reviews={[]} gateway={port} />
    </ReadOnlyProvider>,
  );
  fireEvent.click(screen.getByRole("button", { name: /Pull requests/i }));
  fireEvent.click(screen.getByRole("button", { name: /Add OAuth device flow/i }));
}

describe("DiffReview — github.merge_pr Merge control (cat-1 ui-070)", () => {
  it("merge_disabled_when_pr_mutations_not_enabled", async () => {
    // spec(cat-1) — even with a head_sha + a live link, Merge stays DISABLED while merge_pr is NOT in
    // enabledPrMutations (the guarded-disabled default; the go-live flip is a future USER-signed-off slice).
    const port = new MockGatewayPort({ enabledPrMutations: new Set() });
    renderMerge(port);
    const btn = await screen.findByRole("button", { name: /^Merge/i });
    expect((btn as HTMLButtonElement).disabled).toBe(true);
  });

  it("merge_disabled_when_connection_degraded", async () => {
    // spec(§11.6 defense-in-depth) — Merge stays disabled when the link is degraded (canSubmitIntent
    // false) even with merge_pr enabled + a head_sha.
    const port = new MockGatewayPort(); // enabledPrMutations defaults to the full set
    renderMerge(port, DEGRADED);
    const btn = await screen.findByRole("button", { name: /^Merge/i });
    expect((btn as HTMLButtonElement).disabled).toBe(true);
  });

  it("merge_click_forms_and_submits_then_opens_gateway_modal", async () => {
    // spec(§11.2 + ui-064) — with prMutationsEnabled (Mock default true) + a head_sha + a live link Merge
    // is enabled; clicking forms buildMergePrActionRequest (github.merge_pr, sha-pinned, repo resource_ref,
    // no owner/repo) + submits + opens the GatewayModal. The container owns the gateway; PrWorkspace none.
    const port = new MockGatewayPort();
    const submitSpy = vi.spyOn(port, "submit_action");
    renderMerge(port);
    fireEvent.click(await screen.findByRole("button", { name: /^Merge/i }));
    await waitFor(() => expect(submitSpy).toHaveBeenCalledTimes(1));
    const req = submitSpy.mock.calls[0]![0];
    expect(req.action_type).toBe("github.merge_pr");
    expect((req.inputs as { sha: string }).sha).toBe("headsha123"); // the displayed head pinned
    expect(req.resource_refs[0]).toEqual({ type: "repo", id: "repo_1" });
    expect(req.inputs).not.toHaveProperty("owner"); // ruling A — UI never names owner/repo
    expect(await screen.findByTestId("gateway-modal")).toBeTruthy();
  });

  it("merge_no_optimistic_done", async () => {
    // spec([[16]]/[[17]]) — submit opens the daemon's pending approval card; the UI NEVER shows
    // "merged"/"done" optimistically (merged only on the confirming PullRequestMerged projection fold).
    const port = new MockGatewayPort();
    renderMerge(port);
    fireEvent.click(await screen.findByRole("button", { name: /^Merge/i }));
    await screen.findByTestId("gateway-modal");
    const modal = screen.getByTestId("gateway-modal");
    expect(modal.textContent ?? "").not.toMatch(/\b(merged|done|succeeded|completed)\b/i);
  });

  it("merge_failure_is_honest_re_review_not_fabricated", async () => {
    // spec(§11.7/D2/forbidden#2) — a rejected merge submit surfaces the daemon's §6.4 code VERBATIM +
    // an honest re-review affordance, NEVER a fabricated success/card.
    const port = new MockGatewayPort({ mutationError: { code: "fencing_conflict" } });
    renderMerge(port);
    fireEvent.click(await screen.findByRole("button", { name: /^Merge/i }));
    const region = await screen.findByTestId("pr-mutation-result");
    expect(region.textContent).toMatch(/fencing_conflict/);
    expect(region.textContent).toMatch(/re-review/i);
    expect(screen.getByTestId("pr-mutation-rereview")).toBeTruthy(); // a real re-review button, not just text
    expect(screen.queryByText(/\bmerged\b/i)).toBeNull(); // no fabricated success
  });

  it("merge_enrich_failure_degrades_honestly", async () => {
    // spec(§11.7/LESSON §16) — the merge submit succeeds but the post-submit ApprovalQueue re-fetch fails
    // (malformed payload / transport fault) → the intent WAS recorded but the card can't load → an honest
    // notice, NEVER a silent stall and NEVER a card built from un-parsed data.
    const port = new MockGatewayPort();
    vi.spyOn(port, "get_projection").mockRejectedValue(
      new BoundaryValidationError("ApprovalQueue", new ZodError([])),
    );
    renderMerge(port);
    fireEvent.click(await screen.findByRole("button", { name: /^Merge/i }));
    expect(await screen.findByTestId("pr-mutation-enrich-unavailable")).toBeTruthy();
    expect(screen.queryByTestId("gateway-modal")).toBeNull(); // no card from un-enriched data
  });
});

// ─── ui-071 cat-1 — github.submit_review verdict controls (guarded-disabled) ─────
describe("DiffReview — github.submit_review controls (cat-1 ui-071)", () => {
  // open the PR Workspace + type a body (Request changes / Comment need a non-empty one).
  function openReview(port: MockGatewayPort, status: ConnectionStatus = CONNECTED, body = "lgtm") {
    vi.spyOn(port, "get_diff").mockResolvedValue(DIFF);
    vi.spyOn(port, "get_pr_diff").mockResolvedValue(DIFF);
    render(
      <ReadOnlyProvider value={status}>
        <DiffReview prs={[PR_MERGEABLE]} reviews={[]} gateway={port} />
      </ReadOnlyProvider>,
    );
    fireEvent.click(screen.getByRole("button", { name: /Pull requests/i }));
    fireEvent.click(screen.getByRole("button", { name: /Add OAuth device flow/i }));
    fireEvent.change(screen.getByTestId("pr-review-body"), { target: { value: body } });
  }

  it("review_controls_disabled_when_submit_review_not_enabled", async () => {
    // spec(cat-1) — the verdict controls stay DISABLED while submit_review is NOT in enabledPrMutations
    // (even merge_pr enabled doesn't enable review — per-action gate).
    const port = new MockGatewayPort({ enabledPrMutations: new Set(["github.merge_pr"]) });
    openReview(port);
    expect((await screen.findByRole("button", { name: /Approve PR/i })) as HTMLButtonElement).toHaveProperty(
      "disabled",
      true,
    );
    expect(screen.getByRole("button", { name: /Request changes/i })).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: /^Comment/i })).toHaveProperty("disabled", true);
  });

  it("submit_review_click_forms_event_value_per_control_and_opens_modal", async () => {
    // spec(§11.2 + ui-064 + cat-1 no-cross-wiring) — clicking a verdict forms buildSubmitReviewActionRequest
    // with the EXACT event of the clicked control (Approve→approve / Request-changes→request_changes /
    // Comment→comment), commit_id-pinned, repo resource_ref, no owner/repo, body trimmed; submits + opens
    // the GatewayModal. approve carries merge-gate power → a mis-wired verdict is a real safety bug.
    const cases: { name: RegExp; event: string }[] = [
      { name: /Approve PR/i, event: "approve" },
      { name: /Request changes/i, event: "request_changes" },
      { name: /^Comment/i, event: "comment" },
    ];
    for (const c of cases) {
      const port = new MockGatewayPort(); // enabledPrMutations defaults full
      const submitSpy = vi.spyOn(port, "submit_action");
      openReview(port, CONNECTED, "needs a tweak");
      fireEvent.click(await screen.findByRole("button", { name: c.name }));
      await waitFor(() => expect(submitSpy).toHaveBeenCalledTimes(1));
      const req = submitSpy.mock.calls[0]![0];
      expect(req.action_type).toBe("github.submit_review");
      expect((req.inputs as { event: string }).event).toBe(c.event); // event matches the control — no cross-wiring
      expect((req.inputs as { commit_id: string }).commit_id).toBe("headsha123"); // displayed head pinned
      expect((req.inputs as { body: string }).body).toBe("needs a tweak");
      expect(req.resource_refs[0]).toEqual({ type: "repo", id: "repo_1" });
      expect(req.inputs).not.toHaveProperty("owner");
      expect(await screen.findByTestId("gateway-modal")).toBeTruthy();
      cleanup();
    }
  });

  it("submit_review_no_optimistic_done", async () => {
    // spec([[16]]/[[17]]) — submit opens the daemon's pending approval card; the UI NEVER shows a
    // "reviewed"/"done" success optimistically (the terminal state lands via the daemon ActionResult).
    const port = new MockGatewayPort();
    openReview(port);
    fireEvent.click(await screen.findByRole("button", { name: /Approve PR/i }));
    await screen.findByTestId("gateway-modal");
    const modal = screen.getByTestId("gateway-modal");
    expect(modal.textContent ?? "").not.toMatch(/\b(reviewed|approved|done|succeeded|completed)\b/i);
  });

  it("submit_review_failure_is_honest_re_review", async () => {
    // spec(§11.7/D2/forbidden#2) — a rejected review submit surfaces the daemon's §6.4 code VERBATIM +
    // the honest re-review affordance, NEVER a fabricated success.
    const port = new MockGatewayPort({ mutationError: { code: "fencing_conflict" } });
    openReview(port);
    fireEvent.click(await screen.findByRole("button", { name: /Request changes/i }));
    const region = await screen.findByTestId("pr-mutation-result");
    expect(region.textContent).toMatch(/fencing_conflict/);
    expect(screen.getByTestId("pr-mutation-rereview")).toBeTruthy();
    // no fabricated success (symmetry with the merge-path twin; "reviewed" excluded — honest re-review prose).
    expect(region.textContent ?? "").not.toMatch(/\b(merged|done|succeeded)\b/i);
  });
});
