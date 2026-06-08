// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";
import { SessionsTable } from "./SessionsTable";
import { sessionPageFixture } from "../../projections/fixtures/proj_session";
import { projectActivityFixture } from "../../projections/fixtures/proj_project_activity";

afterEach(cleanup);

const sessions = sessionPageFixture.rows;
const projects = projectActivityFixture.rows;
const renderTable = (rows = sessions) =>
  render(<SessionsTable sessions={rows} projects={projects} />);

describe("SessionsTable view", () => {
  it("renders_a_row_per_session_attention_sorted", () => {
    renderTable();
    const rows = screen
      .getByTestId("sessions-table")
      .querySelectorAll("tbody tr[data-item-id]");
    // one row per session — rendered id set === projection set (no invented rows)
    expect(rows).toHaveLength(sessions.length);
    expect([...rows].map((r) => r.getAttribute("data-item-id")).toSorted()).toEqual(
      sessions.map((s) => `Session:${s.session_id}`).toSorted(),
    );
    // default attention-desc: needs-attention (sf5, rank 5) is first
    expect(rows[0]!.getAttribute("data-item-id")).toBe("Session:session_fixture_5");
  });

  it("default_sort_reflected_in_aria_on_first_paint", () => {
    // aria-sort is f(sortState), not f(clickEvent): the default-sorted table must
    // show aria-sort=descending on the attention column on FIRST paint (no click),
    // so an AT user lands on an accurately-described table.
    renderTable();
    const table = screen.getByTestId("sessions-table");
    expect(
      table.querySelector('th[data-sort-key="attention"]')?.getAttribute("aria-sort"),
    ).toBe("descending");
    // every other column header is "none" on first paint
    for (const key of ["name", "status", "project"]) {
      expect(
        table.querySelector(`th[data-sort-key="${key}"]`)?.getAttribute("aria-sort"),
      ).toBe("none");
    }
  });

  it("column_header_click_sorts_and_sets_aria_sort", () => {
    renderTable();
    const table = screen.getByTestId("sessions-table");
    const attn = table.querySelector('th[data-sort-key="attention"]');
    const name = table.querySelector('th[data-sort-key="name"]');
    // default: attention column descending, others none
    expect(attn?.getAttribute("aria-sort")).toBe("descending");
    expect(name?.getAttribute("aria-sort")).toBe("none");
    // click Name → re-sorts by name asc; aria-sort moves to Name, clears Attention
    fireEvent.click(screen.getByRole("button", { name: /name/i }));
    expect(name?.getAttribute("aria-sort")).toBe("ascending");
    expect(attn?.getAttribute("aria-sort")).toBe("none");
    // first row is now the alphabetically-first session ("Add rate limiting" = sf2)
    expect(
      table.querySelector("tbody tr[data-item-id]")?.getAttribute("data-item-id"),
    ).toBe("Session:session_fixture_2");
    // click Name again → toggles to descending
    fireEvent.click(screen.getByRole("button", { name: /name/i }));
    expect(name?.getAttribute("aria-sort")).toBe("descending");
  });

  it("status_rendered_via_pill_not_color_alone", () => {
    renderTable();
    const row = screen
      .getByTestId("sessions-table")
      .querySelector('[data-item-id="Session:session_fixture_2"]');
    // status via StatusPill (glyph+label, never color alone §11); attention via marker
    expect(row?.querySelector('[data-status="waiting_on_permission"]')).not.toBeNull();
    expect(row?.querySelector("[data-level]")).not.toBeNull();
  });

  it("empty_sessions_shows_explicit_empty_state", () => {
    renderTable([]);
    // an explicit empty state with a message, not an empty/absent table body
    expect(screen.getByTestId("sessions-empty").textContent).toBe("No sessions.");
  });
});
