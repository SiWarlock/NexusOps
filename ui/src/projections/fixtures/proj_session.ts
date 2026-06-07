// Fixture projection data for the Session projection.
//
// §14-sanctioned test/dev infrastructure — NOT dead production code. Every status
// value here is a member of the frozen §5.1 Session enum; the mock validates
// these through the boundary before returning, and mock.test.ts re-pins each
// row against the generated enum so fixtures can never drift from the contract.
import type {
  ProjectionDelta,
  SessionProjectionPage,
} from "../../contracts/index";

export const sessionPageFixture: SessionProjectionPage = {
  projection: "Session",
  rows: [
    {
      session_id: "session_fixture_1",
      status: "active",
      title: "Refactor auth module",
      project_id: "project_fixture_1",
    },
    {
      session_id: "session_fixture_2",
      status: "waiting_on_permission",
      title: "Add rate limiting",
      project_id: "project_fixture_1",
    },
    {
      session_id: "session_fixture_3",
      status: "changes_ready",
      title: "Fix flaky integration test",
      project_id: "project_fixture_2",
    },
    {
      // terminal session — excluded from activeSessions counts (non-terminal filter).
      session_id: "session_fixture_4",
      status: "completed",
      title: "Bump dependencies",
      project_id: "project_fixture_1",
    },
    {
      // waiting_on_human_input — exercises the second WAITING_SESSION state.
      session_id: "session_fixture_5",
      status: "waiting_on_human_input",
      title: "Confirm migration plan",
      project_id: "project_fixture_2",
    },
  ],
  cursor: null,
};

export const sessionDeltaFixture: ProjectionDelta = {
  projection: "Session",
  kind: "upsert",
  row: {
    session_id: "session_fixture_2",
    status: "idle",
    title: "Add rate limiting",
    project_id: "project_fixture_1",
  },
};
