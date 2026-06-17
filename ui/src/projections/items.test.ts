import { describe, it, expect } from "vitest";
import { toSessionItems, toPrItems, toApprovalItems, reviewsByPr } from "./items";
import type {
  SessionRow,
  PullRequestRow,
  ApprovalQueueRow,
  ReviewRow,
} from "../contracts/index";
import { makeApprovalRow } from "./fixtures/proj_approval_queue";

describe("projection item mappers", () => {
  it("to_session_items_maps_rows", () => {
    const rows: SessionRow[] = [
      { session_id: "s1", status: "active", display_name: "Refactor auth", project_id: "p1" },
      // no display_name → label falls back to the id
      { session_id: "s2", status: "idle", project_id: "p1" },
    ];
    expect(toSessionItems(rows)).toEqual([
      { id: "s1", label: "Refactor auth", machine: "Session", status: "active" },
      { id: "s2", label: "s2", machine: "Session", status: "idle" },
    ]);
  });

  it("to_pr_items_maps_rows", () => {
    const rows: PullRequestRow[] = [
      // id is the PK pr_id (not pr_number); label uses the title.
      { pr_id: "repo_1#101", pr_number: 101, project_id: "p1", status: "open", title: "Add OAuth" },
      // no title → label falls back to `PR #${pr_number}`
      { pr_id: "repo_1#102", pr_number: 102, project_id: "p1", status: "merged" },
      // no title AND null pr_number → label falls back to the always-present pr_id PK
      { pr_id: "repo_1#103", pr_number: null, project_id: "p1", status: "closed" },
    ];
    expect(toPrItems(rows)).toEqual([
      { id: "repo_1#101", label: "Add OAuth", machine: "PullRequest", status: "open" },
      { id: "repo_1#102", label: "PR #102", machine: "PullRequest", status: "merged" },
      { id: "repo_1#103", label: "repo_1#103", machine: "PullRequest", status: "closed" },
    ]);
  });

  it("to_approval_items_maps_rows", () => {
    const rows: ApprovalQueueRow[] = [
      makeApprovalRow({
        approval_id: "ap1",
        project_id: "p1",
        status: "requested",
        preview_summary: "Approve deploy",
      }),
      // no preview_summary → label falls back to the id
      makeApprovalRow({ approval_id: "ap2", project_id: "p1", status: "approved" }),
    ];
    expect(toApprovalItems(rows)).toEqual([
      { id: "ap1", label: "Approve deploy", machine: "Approval", status: "requested" },
      { id: "ap2", label: "ap2", machine: "Approval", status: "approved" },
    ]);
  });

  it("mappers_map_empty_to_empty", () => {
    // The boundary case: no rows → no items (Layer 2 spreads from these results).
    expect(toSessionItems([])).toEqual([]);
    expect(toPrItems([])).toEqual([]);
    expect(toApprovalItems([])).toEqual([]);
  });
});

describe("reviewsByPr (ui-064 — the PR-Review client-side join on pr_number)", () => {
  // §7.2/§11.2: a Review projection row joins to a PR client-side on pr_number (ReviewRow has no
  // pr_id PK reference — the GitHub-native number is the join key). A null pr_number is unattributable
  // to a PR → excluded from the join. Multiple reviews per PR are retained in row order.
  it("reviews_by_pr_groups_by_pr_number", () => {
    const rows: ReviewRow[] = [
      { review_id: 1, pr_number: 101, state: "approved", reviewer: "a" },
      { review_id: 2, pr_number: 101, state: "changes_requested", reviewer: "b" },
      { review_id: 3, pr_number: 201, state: "commented", reviewer: "c" },
      // null pr_number → contributes no key (unattributable to a PR)
      { review_id: 4, pr_number: null, state: "pending", reviewer: "d" },
      // absent (undefined) pr_number → ALSO excluded (the schema is nullable().optional() → both forms
      // reach the join; pins the `== null` guard so a `=== null` refactor can't silently drop this row)
      { review_id: 5, state: "dismissed", reviewer: "e" },
    ];
    const byPr = reviewsByPr(rows);
    expect(byPr.get(101)?.map((r) => r.review_id)).toEqual([1, 2]); // multiple per PR, in order
    expect(byPr.get(201)?.map((r) => r.review_id)).toEqual([3]);
    expect(byPr.size).toBe(2); // exactly the two non-null pr_numbers (null + undefined both excluded)
  });

  it("reviews_by_pr_maps_empty_to_empty", () => {
    expect(reviewsByPr([]).size).toBe(0);
  });
});
