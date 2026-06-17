// @vitest-environment jsdom
//
// ui-064 Layer 2 — the read-only PR Review Workspace panel. For a selected PullRequestRow it renders the
// header (number/title/branches/status) + mergeability/checks (from the frozen row) + the reviews-list +
// honest D6/D7 daemon-gap placeholders, with ALL mutations + Brain controls rendered DISABLED (a future
// cat-1 arc + the deferred Brain sibling; wire-or-disable, never a dead click).
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { PrWorkspace } from "./PrWorkspace";
import type { PullRequestRow, ReviewRow } from "../../contracts/index";

afterEach(cleanup);

const pr = (over: Partial<PullRequestRow> = {}): PullRequestRow => ({
  pr_id: "repo_fixture_1#101",
  project_id: "project_fixture_1",
  repo_id: "repo_fixture_1",
  pr_number: 101,
  title: "Add OAuth device flow",
  status: "open",
  head_branch: "agent/auth-refactor",
  base_branch: "main",
  pr_checked_at: "2026-06-16T09:00:00Z",
  mergeable: true,
  checks_summary: "3/3 checks passing",
  ...over,
});

const isDisabled = (name: RegExp) =>
  (screen.getByRole("button", { name }) as HTMLButtonElement).disabled;

const noop = () => {};

describe("PrWorkspace (ui-064 Layer 2)", () => {
  it("pr_workspace_renders_header_and_mergeability", () => {
    // [§11.2/§7.2] the selected PullRequestRow → header (number/title/branch/status) + mergeable/checks
    // from the frozen row (never color alone — a label, not just a hue).
    render(<PrWorkspace pr={pr()} reviews={[]} onBack={noop} />);
    expect(screen.getByText(/Add OAuth device flow/)).toBeTruthy();
    expect(screen.getByText(/#101/)).toBeTruthy();
    expect(screen.getByText(/agent\/auth-refactor/)).toBeTruthy();
    expect(screen.getByText(/Mergeable/i)).toBeTruthy(); // mergeable=true → a label
    expect(screen.getByText("3/3 checks passing")).toBeTruthy(); // checks_summary
  });

  it("pr_workspace_shows_reviews_for_selected_pr", () => {
    // [§11.2] the reviews-list is wired — the PR's reviews (reviewsByPr.get(pr_number)) render.
    const reviews: ReviewRow[] = [
      { review_id: 9001, pr_number: 101, state: "approved", reviewer: "alice", body: "LGTM" },
      { review_id: 9002, pr_number: 101, state: "changes_requested", reviewer: "bob", body: "nit" },
    ];
    render(<PrWorkspace pr={pr()} reviews={reviews} onBack={noop} />);
    expect(screen.getByText("alice")).toBeTruthy();
    expect(screen.getByText("bob")).toBeTruthy();
  });

  it("pr_workspace_diff_and_stats_are_honest_placeholders", () => {
    // [honest-degrade/§7.2] D6 diff-stats + D7 code-diff render an honest "unavailable — needs daemon
    // <X>" affordance, NEVER a fabricated number, NEVER get_diff-as-PR-diff (PrWorkspace has no gateway).
    render(<PrWorkspace pr={pr()} reviews={[]} onBack={noop} />);
    expect(screen.getByTestId("pr-diffstats-unavailable")).toBeTruthy();
    expect(screen.getByTestId("pr-diff-unavailable").textContent).toMatch(/get_pr_diff/);
  });

  it("pr_workspace_mutations_and_brain_disabled", () => {
    // [a11y wire-or-disable / forbidden #6] the future cat-1 mutation arc + the deferred Brain sibling
    // render DISABLED with accessible names — present but non-interactive, never a dead click.
    render(<PrWorkspace pr={pr()} reviews={[]} onBack={() => {}} />);
    expect(isDisabled(/Merge/i)).toBe(true);
    expect(isDisabled(/Approve PR/i)).toBe(true);
    expect(isDisabled(/Request changes/i)).toBe(true);
    expect(isDisabled(/Ask Brain/i)).toBe(true);
    // the "← Worktree diff" deselect IS wired (onBack provided) — not a dead click.
    expect(isDisabled(/Worktree diff/i)).toBe(false);
  });
});
