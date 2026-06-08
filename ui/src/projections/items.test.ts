import { describe, it, expect } from "vitest";
import { toSessionItems, toPrItems, toApprovalItems } from "./items";
import type {
  SessionRow,
  PullRequestRow,
  ApprovalQueueRow,
} from "../contracts/index";

describe("projection item mappers", () => {
  it("to_session_items_maps_rows", () => {
    const rows: SessionRow[] = [
      { session_id: "s1", status: "active", title: "Refactor auth", project_id: "p1" },
      // no title → label falls back to the id
      { session_id: "s2", status: "idle", project_id: "p1" },
    ];
    expect(toSessionItems(rows)).toEqual([
      { id: "s1", label: "Refactor auth", machine: "Session", status: "active" },
      { id: "s2", label: "s2", machine: "Session", status: "idle" },
    ]);
  });

  it("to_pr_items_maps_rows", () => {
    const rows: PullRequestRow[] = [
      { pr_number: "101", project_id: "p1", status: "open", title: "Add OAuth" },
      // no title → label falls back to `PR #${n}`
      { pr_number: "102", project_id: "p1", status: "merged" },
    ];
    expect(toPrItems(rows)).toEqual([
      { id: "101", label: "Add OAuth", machine: "PullRequest", status: "open" },
      { id: "102", label: "PR #102", machine: "PullRequest", status: "merged" },
    ]);
  });

  it("to_approval_items_maps_rows", () => {
    const rows: ApprovalQueueRow[] = [
      { approval_id: "ap1", project_id: "p1", status: "requested", title: "Approve deploy" },
      // no title → label falls back to the id
      { approval_id: "ap2", project_id: "p1", status: "approved" },
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
