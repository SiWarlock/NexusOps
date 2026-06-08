// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { RecoveryBanner } from "./RecoveryBanner";

afterEach(cleanup);

describe("RecoveryBanner", () => {
  it("renders_recovery_failed_with_parked_restart", () => {
    render(
      <RecoveryBanner
        recovery={{
          state: "recovery_failed",
          affectedSessions: ["session_fixture_2", "session_fixture_5"],
        }}
      />,
    );
    // the affected sessions are surfaced
    const affected = screen.getByTestId("recovery-affected");
    expect(affected.textContent).toContain("session_fixture_2");
    expect(affected.textContent).toContain("session_fixture_5");
    // a "Restart session" control is present but PARKED (disabled — intent
    // deferred to daemon-1.5; not wired)
    const restart = screen.getByTestId("restart-session");
    expect(restart).toHaveProperty("disabled", true);
  });

  it("distinct_from_transport_degraded_banner", () => {
    render(<RecoveryBanner recovery={{ state: "recovering" }} />);
    const banner = screen.getByTestId("recovery-banner");
    // a distinct session-survival surface — NOT the transport DegradedBanner
    expect(banner.className).toContain("recovery-banner");
    expect(banner.className).not.toContain("degraded-banner");
    expect(banner.getAttribute("data-degraded")).toBeNull();
    expect(banner.getAttribute("data-recovery-kind")).toBe("recovering");
  });

  it("recovered_renders_nothing", () => {
    const { container } = render(<RecoveryBanner recovery={{ state: "recovered" }} />);
    // non-intrusive: recovered shows no banner
    expect(container.querySelector('[data-testid="recovery-banner"]')).toBeNull();
  });

  it("recovery_failed_without_affected_still_shows_restart", () => {
    // recovery_failed with no affectedSessions list → no list, but the parked
    // restart affordance still renders (the banner is still actionable-when-wired).
    render(<RecoveryBanner recovery={{ state: "recovery_failed" }} />);
    expect(screen.queryByTestId("recovery-affected")).toBeNull();
    expect(screen.getByTestId("restart-session")).toHaveProperty("disabled", true);
  });
});
