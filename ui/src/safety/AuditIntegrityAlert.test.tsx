// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render, screen, within } from "@testing-library/react";
import { AuditIntegrityAlert } from "./AuditIntegrityAlert";
import {
  AuditOutcomeStatus,
  AuditIntegrityKind,
  type AuditIntegrityState,
} from "../contracts/index";

afterEach(cleanup);

describe("AuditIntegrityAlert (§15/§17 fail-closed — safety #5)", () => {
  it("renders_each_audit_integrity_treatment", () => {
    // Completeness (drift-proof, mirrors the 6.2a attention-table pin): EVERY
    // audit-integrity state the model can produce MUST render — #5 fail-closed
    // means no signal may silently fail to surface. Derived from the enum
    // `.options`, so a future-added integrity kind is forced to render here, not
    // silently dropped. Covers all 5 treatments incl. corrupt_payload.
    const states: AuditIntegrityState[] = [
      ...AuditOutcomeStatus.options.map(
        (status): AuditIntegrityState => ({ source: "action_status", status }),
      ),
      ...AuditIntegrityKind.options.map(
        (kind): AuditIntegrityState => ({ source: "integrity", kind }),
      ),
    ];
    expect(states.length).toBe(5); // 2 frozen outcomes + 3 provisional integrity signals
    for (const state of states) {
      const { unmount } = render(<AuditIntegrityAlert integrity={state} />);
      const alert = screen.getByTestId("audit-integrity-alert");
      const treatment = state.source === "action_status" ? state.status : state.kind;
      // each treatment renders its glyph + label + severity (a non-color channel
      // — §11.6); data-severity is the styling/automation hook
      expect(alert.getAttribute("data-treatment")).toBe(treatment);
      expect(alert.getAttribute("data-severity")).toBeTruthy();
      const label = alert.querySelector(".audit-integrity-alert__label");
      expect(label?.textContent).toBeTruthy();
      unmount();
    }
  });

  it("audit_integrity_alert_is_non_dismissible", () => {
    // structural by construction (acknowledgeParked is literal-true) → holds for
    // EVERY treatment, not just this one; one representative case suffices.
    render(
      <AuditIntegrityAlert integrity={{ source: "integrity", kind: "audit_write_failed" }} />,
    );
    const alert = screen.getByTestId("audit-integrity-alert");
    // #5 fail-closed: NO enabled dismiss/close control — the signal must be SEEN
    expect(screen.queryByRole("button", { name: /dismiss|close/i })).toBeNull();
    // any acknowledge is rendered disabled/parked (intent → daemon-1.5)
    const ack = screen.getByTestId("audit-integrity-acknowledge");
    expect(ack).toHaveProperty("disabled", true);
    // no enabled button at all → the alert cannot be locally cleared
    const enabled = within(alert)
      .queryAllByRole("button")
      .filter((b) => !(b as HTMLButtonElement).disabled);
    expect(enabled).toHaveLength(0);
  });

  it("clean_audit_integrity_renders_nothing", () => {
    const { container } = render(<AuditIntegrityAlert integrity={null} />);
    // no integrity signal → non-intrusive, nothing renders
    expect(container.querySelector('[data-testid="audit-integrity-alert"]')).toBeNull();
  });
});
