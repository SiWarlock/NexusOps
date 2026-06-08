// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render, screen, within } from "@testing-library/react";
import { HardConflictCard } from "./HardConflictCard";
import { fencingConflictFixture } from "./fixtures";

afterEach(cleanup);

describe("HardConflictCard (§17 fencing — safety #6)", () => {
  it("renders_conflict_with_never_auto_resolved_and_parked_resolution", () => {
    render(<HardConflictCard conflict={fencingConflictFixture} />);
    const card = screen.getByTestId("hard-conflict-card");
    // the affected action is surfaced (from the fixture, never invented)
    expect(card.textContent).toContain(fencingConflictFixture.action_request_id);
    // the load-bearing #6 message
    expect(card.textContent).toMatch(/never auto-resolved/i);
    // a resolution control is PRESENT but disabled/parked (intent → daemon-1.5)
    const resolve = screen.getByTestId("conflict-resolve");
    expect(resolve).toHaveProperty("disabled", true);
    // #6: NO enabled control offers a resolve/auto-resolve path
    const enabled = within(card)
      .queryAllByRole("button")
      .filter((b) => !(b as HTMLButtonElement).disabled);
    expect(enabled).toHaveLength(0);
  });

  it("conflict_card_distinct_from_degraded_and_recovery_banners", () => {
    render(<HardConflictCard conflict={fencingConflictFixture} />);
    const card = screen.getByTestId("hard-conflict-card");
    // a distinct §17 safety surface — NOT the transport DegradedBanner, NOT the
    // session-survival RecoveryBanner (three distinct concerns, never conflated)
    expect(card.className).toContain("hard-conflict-card");
    expect(card.className).not.toContain("degraded-banner");
    expect(card.className).not.toContain("recovery-banner");
    expect(card.getAttribute("data-degraded")).toBeNull();
    expect(card.getAttribute("data-recovery-kind")).toBeNull();
    expect(card.getAttribute("data-conflict-reason")).toBe("fencing_conflict");
  });

  it("clean_safety_state_renders_nothing", () => {
    const { container } = render(<HardConflictCard conflict={null} />);
    // no conflict → non-intrusive, nothing renders (like 6.4d `recovered`)
    expect(container.querySelector('[data-testid="hard-conflict-card"]')).toBeNull();
  });
});
