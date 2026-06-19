// Fixture data for the ProjectActivity projection (the projects list).
// §14 test/dev infrastructure. project_fixture_3 is deliberately activity-free
// to exercise the explicit-zero switcher-count path.
import type { ProjectActivityPage, ProjectionDelta } from "../../contracts/index";

export const projectActivityFixture: ProjectActivityPage = {
  projection: "ProjectActivity",
  rows: [
    { project_id: "project_fixture_1", name: "auth-service" },
    { project_id: "project_fixture_2", name: "billing" },
    { project_id: "project_fixture_3", name: "docs-site" },
  ],
  cursor: null,
};

// A daemon-shaped `row:None` NUDGE for the ProjectActivity subscribe stream (ui-063). The daemon emits an
// id-nudge keyed by project_id (deltas_for_event, on ProjectRescanned/SessionStarted), NOT the row — so
// this carries NO `row`. The live projects list consumes it via refetch-on-nudge (re-read get_projection),
// never a row-apply reducer (which would no-op on the absent row — LESSON §29).
export const projectActivityDeltaFixture: ProjectionDelta = {
  projection: "ProjectActivity",
  kind: "upsert",
  id: "project_fixture_1",
};
