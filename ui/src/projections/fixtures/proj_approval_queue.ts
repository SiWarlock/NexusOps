// Fixture data for the ApprovalQueue projection. §14 test/dev infrastructure.
// approval_fixture_2 is already decided (approved) → excluded from waitingOnYou
// (only pending approvals count).
import type {
  ApprovalQueuePage,
  ApprovalQueueRow,
  ProjectionDelta,
} from "../../contracts/index";

/** Build a valid frozen 14-field ApprovalQueueRow with overrides — test convenience so call sites
 *  don't inline all 14 fields. Defaults are a pending, current-user, risk-2 approval. */
export function makeApprovalRow(
  over: Partial<ApprovalQueueRow> = {},
): ApprovalQueueRow {
  return {
    approval_id: "approval_test",
    action_request_id: null,
    plan_id: null,
    project_id: null,
    session_id: null,
    agent_team_id: null,
    risk_level: 2,
    status: "awaiting_approval",
    requester_type: "agent_session",
    requester_id: "agent_test",
    preview_summary: null,
    requested_at: "2026-06-14T00:00:00Z",
    expires_at: null,
    policy_decision: null,
    ...over,
  };
}

// The rows are the FROZEN 14-field ApprovalQueueRow (@0.30). risk_level + policy_decision are
// carried ON the row now (the daemon's authoritative values) — the approval card sources them
// directly (053 Layer C), so these mirror the prior gatewayApprovalEnrichment side-map values for
// render continuity. preview_summary replaces the old `title`; the Option<> fields serialize as null.
export const approvalQueueFixture: ApprovalQueuePage = {
  projection: "ApprovalQueue",
  rows: [
    {
      approval_id: "approval_fixture_1",
      action_request_id: "ar_fixture_1",
      plan_id: null,
      project_id: "project_fixture_1",
      session_id: null,
      agent_team_id: null,
      risk_level: 3,
      status: "awaiting_approval",
      requester_type: "agent_session",
      requester_id: "agent_fixture_1",
      preview_summary: "git.create_worktree",
      requested_at: "2026-06-14T00:00:00Z",
      expires_at: null,
      policy_decision: {
        status: "require_approval",
        reasons: ["Writes to a tracked file outside the session worktree."],
        required_approvals: [{ kind: "current_user" }],
        constraints: [],
        safer_alt: null,
      },
    },
    {
      approval_id: "approval_fixture_2",
      action_request_id: "ar_fixture_2",
      plan_id: null,
      project_id: "project_fixture_2",
      session_id: null,
      agent_team_id: null,
      risk_level: 1,
      status: "approved",
      requester_type: "agent_session",
      requester_id: "agent_fixture_2",
      preview_summary: "github.create_pr",
      requested_at: "2026-06-14T00:00:00Z",
      expires_at: null,
      policy_decision: {
        status: "require_approval",
        reasons: ["Edits documentation."],
        required_approvals: [{ kind: "project_owner" }],
        constraints: [],
        safer_alt: null,
      },
    },
  ],
  cursor: null,
};

// A daemon-shaped `row:None` NUDGE for the ApprovalQueue subscribe stream (ui-059). The daemon emits an
// id-nudge ("this projection changed") on every submit/approve/deny (gateway/pipeline.rs:79), NOT the
// row — so this carries NO `row`, distinct from `sessionDeltaFixture` (which carries a row and thus
// masked the row:None gap). The live queue consumes it via refetch-on-nudge (re-read get_projection),
// never a row-apply reducer (which would no-op on the absent row).
export const approvalQueueDeltaFixture: ProjectionDelta = {
  projection: "ApprovalQueue",
  kind: "upsert",
  id: "approval_fixture_1",
};
