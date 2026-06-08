import { describe, it, expect } from "vitest";
import {
  buildSessionRows,
  sortSessionRows,
  filterSessionRows,
  distinctStatuses,
} from "./model";
import type { SessionRow } from "../../contracts/index";
import { sessionPageFixture } from "../../projections/fixtures/proj_session";
import { projectActivityFixture } from "../../projections/fixtures/proj_project_activity";

const sessions = sessionPageFixture.rows;
const projects = projectActivityFixture.rows;

describe("sessions table model", () => {
  it("builds_rows_with_project_name_and_attention", () => {
    const rows = buildSessionRows(sessions, projects);
    // each session enriched: base shape (toSessionItems) + attention + project join
    const sf1 = rows.find((r) => r.id === "session_fixture_1");
    expect(sf1).toEqual({
      id: "session_fixture_1",
      machine: "Session",
      status: "active",
      label: "Refactor auth module",
      attentionRank: 1, // via describeStatus (single source)
      projectName: "auth-service", // project_id→ProjectActivity.name join (§4.2)
      resumeMode: "resumed", // O-2 survival (6.4d) — carried from the session row
    });
    // a session in a different project resolves that project's name
    expect(rows.find((r) => r.id === "session_fixture_3")?.projectName).toBe(
      "billing",
    );
  });

  it("unknown_project_id_falls_back_visibly", () => {
    const custom: SessionRow[] = [
      { session_id: "s_x", status: "idle", title: "X", project_id: "nope_404" },
      { session_id: "s_y", status: "idle", title: "Y" }, // absent project_id
    ];
    const rows = buildSessionRows(custom, projects);
    // unmatched project_id → the raw id (visible), never a crash/blank
    expect(rows[0]!.projectName).toBe("nope_404");
    // absent project_id → a visible fallback marker
    expect(rows[1]!.projectName).toBe("—");
  });

  it("default_sort_is_attention_desc", () => {
    const rows = buildSessionRows(sessions, projects);
    const sorted = sortSessionRows(rows, "attention", "desc");
    // needs-attention first (§5.2 / compareByAttention); ties (sf2/sf3 both rank 4)
    // broken stably by id
    expect(sorted.map((r) => r.id)).toEqual([
      "session_fixture_5", // rank 5
      "session_fixture_2", // rank 4 (id tiebreak < sf3)
      "session_fixture_3", // rank 4
      "session_fixture_1", // rank 1
      "session_fixture_4", // rank 0
    ]);
    // pure — the input is not mutated
    expect(rows.map((r) => r.id)).toEqual(sessions.map((s) => s.session_id));
  });

  it("sorts_by_name_and_toggles_direction", () => {
    const rows = buildSessionRows(sessions, projects);
    const asc = sortSessionRows(rows, "name", "asc").map((r) => r.label);
    const desc = sortSessionRows(rows, "name", "desc").map((r) => r.label);
    expect(asc).toEqual([
      "Add rate limiting",
      "Bump dependencies",
      "Confirm migration plan",
      "Fix flaky integration test",
      "Refactor auth module",
    ]);
    // distinct names → desc is the exact reverse of asc
    expect(desc).toEqual(asc.toReversed());
    // pure — neither call mutated the input
    expect(rows.map((r) => r.id)).toEqual(sessions.map((s) => s.session_id));
  });

  it("includes_terminal_sessions", () => {
    const rows = buildSessionRows(sessions, projects);
    // full list, not active-only — the terminal `completed` session is present
    expect(rows).toHaveLength(sessions.length);
    expect(rows.find((r) => r.id === "session_fixture_4")?.status).toBe("completed");
    // and sorts to the bottom by attention (rank 0)
    const sorted = sortSessionRows(rows, "attention", "desc");
    expect(sorted[sorted.length - 1]!.id).toBe("session_fixture_4");
  });
});

describe("sessions table filtering", () => {
  const built = buildSessionRows(sessions, projects);

  it("filter_empty_is_identity", () => {
    // no active facet → no constraint; returns the input array unchanged (identity).
    expect(filterSessionRows(built, { text: "", status: null })).toBe(built);
  });

  it("filter_by_status_exact", () => {
    // the status facet keeps only rows whose status matches exactly.
    const out = filterSessionRows(built, { text: "", status: "waiting_on_permission" });
    expect(out.map((r) => r.id)).toEqual(["session_fixture_2"]);
  });

  it("filter_by_text_matches_name_and_project_ci", () => {
    // name match: "rate" hits only sf2 "Add rate limiting" (no project name has it)
    expect(filterSessionRows(built, { text: "rate", status: null }).map((r) => r.id)).toEqual([
      "session_fixture_2",
    ]);
    // project-name match, case-insensitive: "BILLING" hits the billing project's sessions
    expect(
      filterSessionRows(built, { text: "BILLING", status: null })
        .map((r) => r.id)
        .toSorted(),
    ).toEqual(["session_fixture_3", "session_fixture_5"]);
  });

  it("filter_text_no_cross_field_boundary_match", () => {
    // the text facet ORs name and project INDEPENDENTLY — a term that only spans
    // the label-end + project-start boundary must NOT match (sf1: "Refactor auth
    // module" in "auth-service" → "module auth" straddles but is in neither field).
    expect(filterSessionRows(built, { text: "module auth", status: null })).toHaveLength(0);
  });

  it("filter_ands_status_and_text", () => {
    // both facets active → only rows matching BOTH
    expect(
      filterSessionRows(built, { text: "migration", status: "waiting_on_human_input" }).map(
        (r) => r.id,
      ),
    ).toEqual(["session_fixture_5"]);
    // text matches sf5 but status excludes it → empty
    expect(
      filterSessionRows(built, { text: "migration", status: "active" }),
    ).toHaveLength(0);
  });

  it("distinct_statuses_from_unfiltered_rows", () => {
    // the present statuses, deduped + deterministic (sorted) — stable options that
    // don't collapse as the filter narrows (computed off the full set).
    expect(distinctStatuses(built)).toEqual([
      "active",
      "changes_ready",
      "completed",
      "waiting_on_human_input",
      "waiting_on_permission",
    ]);
  });

  it("filter_then_sort_preserves_attention_order", () => {
    // compose filter → sort: §5.2 attention order holds within the filtered subset.
    const filtered = filterSessionRows(built, { text: "billing", status: null });
    const sorted = sortSessionRows(filtered, "attention", "desc");
    expect(sorted.map((r) => r.id)).toEqual([
      "session_fixture_5", // waiting_on_human_input, rank 5
      "session_fixture_3", // changes_ready, rank 4
    ]);
  });
});
