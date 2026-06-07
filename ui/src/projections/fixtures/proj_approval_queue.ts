// Fixture data for the ApprovalQueue projection. §14 test/dev infrastructure.
// approval_fixture_2 is already decided (approved) → excluded from waitingOnYou
// (only pending approvals count).
import type { ApprovalQueuePage } from "../../contracts/index";

export const approvalQueueFixture: ApprovalQueuePage = {
  projection: "ApprovalQueue",
  rows: [
    {
      approval_id: "approval_fixture_1",
      project_id: "project_fixture_1",
      status: "awaiting_approval",
      title: "git.create_worktree",
    },
    {
      approval_id: "approval_fixture_2",
      project_id: "project_fixture_2",
      status: "approved",
      title: "github.create_pr",
    },
  ],
  cursor: null,
};
