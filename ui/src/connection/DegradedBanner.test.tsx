// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { DegradedBanner } from "./DegradedBanner";
import {
  ReadOnlyProvider,
  useCanSubmitIntent,
  type ConnectionStatus,
} from "./read-only";
import { deriveDegradedState } from "./version";

afterEach(cleanup);

function SubmitReadout() {
  return <span data-testid="can-submit">{String(useCanSubmitIntent())}</span>;
}

function Harness({ status }: { status: ConnectionStatus }) {
  return (
    <ReadOnlyProvider value={status}>
      <DegradedBanner
        degraded={deriveDegradedState(status.connection, status.version)}
        onRetry={() => {}}
        onRepair={() => {}}
      />
      <SubmitReadout />
    </ReadOnlyProvider>
  );
}

describe("DegradedBanner + read-only integration", () => {
  it("degraded_banner_retry_repair_enabled_when_disconnected", () => {
    render(
      <DegradedBanner degraded="disconnected" onRetry={() => {}} onRepair={() => {}} />,
    );
    const retry = screen.getByRole("button", { name: /retry/i });
    const repair = screen.getByRole("button", { name: /repair/i });
    // Retry/Repair are transport actions — enabled even in read-only (not gated intents).
    expect((retry as HTMLButtonElement).disabled).toBe(false);
    expect((repair as HTMLButtonElement).disabled).toBe(false);
  });

  it("degraded_banner_update_required_variant", () => {
    render(
      <DegradedBanner
        degraded="update_required"
        onRetry={() => {}}
        onRepair={() => {}}
      />,
    );
    const banner = screen.getByRole("alert");
    // distinct update-required treatment (message + data hook differ from reconnecting)
    expect(banner.getAttribute("data-degraded")).toBe("update_required");
    expect(banner.textContent).toMatch(/update required/i);
    // NO plain transport-Retry — retrying a handshake against a skewed daemon
    // can't help; §16 is update/relaunch, not retry.
    expect(screen.queryByRole("button", { name: /retry/i })).toBeNull();
    // an update/repair affordance IS offered
    expect(
      screen.getByRole("button", { name: /repair|update/i }),
    ).toBeTruthy();
  });

  it("reconnect_restores_live_state", () => {
    const disconnected: ConnectionStatus = {
      connection: "disconnected",
      version: "compatible",
    };
    const connected: ConnectionStatus = {
      connection: "connected",
      version: "compatible",
    };

    const { rerender } = render(<Harness status={disconnected} />);
    expect(screen.getByRole("alert")).toBeTruthy(); // banner shown
    expect(screen.getByTestId("can-submit").textContent).toBe("false");

    // disconnected → connected removes the banner and flips canSubmitIntent true
    rerender(<Harness status={connected} />);
    expect(screen.queryByRole("alert")).toBeNull();
    expect(screen.getByTestId("can-submit").textContent).toBe("true");
  });
});
