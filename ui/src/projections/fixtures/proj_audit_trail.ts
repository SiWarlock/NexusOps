// Fixture data for the AuditTrail projection (the event timeline). §14 test/dev
// infrastructure. seq is the canonical event order (§7.1); deriveActivityFeed
// sorts by it descending.
import type { AuditTrailPage } from "../../contracts/index";

export const auditTrailFixture: AuditTrailPage = {
  projection: "AuditTrail",
  rows: [
    {
      event_id: "event_fixture_1",
      seq: 10,
      project_id: "project_fixture_1",
      actor_type: "user",
      event_type: "session.started",
      summary: "Started session on auth-service",
    },
    {
      event_id: "event_fixture_2",
      seq: 20,
      project_id: "project_fixture_2",
      actor_type: "action_gateway",
      event_type: "action.approved",
      summary: "Approved github.create_pr",
    },
    {
      event_id: "event_fixture_3",
      seq: 30,
      project_id: "project_fixture_1",
      actor_type: "session_adapter",
      event_type: "project.rescanned",
      summary: "Rescanned auth-service worktree",
    },
    {
      event_id: "event_fixture_4",
      seq: 25,
      project_id: "project_fixture_1",
      actor_type: "user",
      event_type: "git.committed",
      summary: "Committed on feature branch",
    },
    {
      event_id: "event_fixture_5",
      seq: 5,
      project_id: "project_fixture_3",
      actor_type: "system",
      event_type: "project.registered",
      summary: "Registered docs-site",
    },
  ],
  cursor: null,
};
