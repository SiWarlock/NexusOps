// Fixture data for the ProjectActivity projection (the projects list).
// §14 test/dev infrastructure. project_fixture_3 is deliberately activity-free
// to exercise the explicit-zero switcher-count path.
import type { ProjectActivityPage } from "../../contracts/index";

export const projectActivityFixture: ProjectActivityPage = {
  projection: "ProjectActivity",
  rows: [
    { project_id: "project_fixture_1", name: "auth-service" },
    { project_id: "project_fixture_2", name: "billing" },
    { project_id: "project_fixture_3", name: "docs-site" },
  ],
  cursor: null,
};
