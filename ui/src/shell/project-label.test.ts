import { describe, it, expect } from "vitest";
import { projectLabel, projectIdFallback, headerProjectLabel } from "./project-label";

// The daemon serves proj_project_activity as a COUNTERS table with NO `name` column (§7.2), so the
// switcher/header must render an honest, unique, interim id-derived label when `name` is absent —
// never a blank entry (§11.6) — while still showing the real name once the daemon provides one.
describe("project-label", () => {
  it("projectLabel_returns_the_real_name_when_present", () => {
    expect(projectLabel({ project_id: "proj_x", name: "auth-service" })).toBe("auth-service");
  });

  it("projectLabel_falls_back_to_a_unique_id_label_when_name_absent", () => {
    const id = "proj_01HX2ABCDEFGHJKMNPQRSTVW";
    const label = projectLabel({ project_id: id });
    expect(label).not.toBe("");
    expect(label).not.toBe("No project");
    expect(label).toContain(id.slice(-6)); // id-derived → unique + honest + clearly interim
  });

  it("projectLabel_falls_back_when_name_is_null", () => {
    expect(projectLabel({ project_id: "proj_xyz123", name: null })).toBe(
      projectIdFallback("proj_xyz123"),
    );
  });

  it("projectLabel_falls_back_when_name_is_whitespace_only", () => {
    // a whitespace-only name is not a real name → the id-fallback (the .trim() guard).
    expect(projectLabel({ project_id: "proj_xyz123", name: "   " })).toBe(
      projectIdFallback("proj_xyz123"),
    );
  });

  it("two_distinct_ids_produce_distinct_fallback_labels", () => {
    expect(projectIdFallback("proj_aaaaaa")).not.toBe(projectIdFallback("proj_bbbbbb"));
  });

  // The header's no-project-vs-no-name distinction (the conflation bug: `name ?? "No project"`
  // showed "No project" for an active-but-nameless project). §11.4.
  it("headerProjectLabel_returns_No_project_only_when_no_active_project", () => {
    expect(headerProjectLabel(null)).toBe("No project");
    expect(headerProjectLabel(undefined)).toBe("No project");
  });

  it("headerProjectLabel_shows_id_fallback_for_a_nameless_active_project_not_No_project", () => {
    const id = "proj_01HX2ABCDEFGHJKMNPQRSTVW";
    const label = headerProjectLabel({ project_id: id });
    expect(label).not.toBe("No project");
    expect(label).toContain(id.slice(-6));
  });

  it("headerProjectLabel_shows_the_real_name_when_present", () => {
    expect(headerProjectLabel({ project_id: "proj_x", name: "billing" })).toBe("billing");
  });
});
