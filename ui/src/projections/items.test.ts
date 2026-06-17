import { describe, it, expect } from "vitest";
import { toSessionItems, toPrItems, toApprovalItems } from "./items";
import type {
  SessionRow,
  PullRequestRow,
  ApprovalQueueRow,
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
