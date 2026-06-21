// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";

afterEach(cleanup);
import { BrainStatusHeader } from "./BrainStatusHeader";
import { BrainStatusProvider, type BrainStatusValue } from "./brain-status";
import { BrainPage } from "./BrainPage";

const val = (over: Partial<BrainStatusValue> = {}): BrainStatusValue => ({
  status: "ready",
  grounded_at: "2026-06-21T12:00:00Z",
  absent: false,
  ...over,
});

function renderHeader(value: BrainStatusValue) {
  return render(
    <BrainStatusProvider value={value}>
      <BrainStatusHeader />
    </BrainStatusProvider>,
  );
}

describe("BrainStatusHeader — status surface (§11.5/§13.1)", () => {
  it("header_renders_status_pill_glyph_and_label", () => {
    // spec(forbidden#5/LESSON 6) — the status renders via the descriptor-bound StatusPill: a label
    // (text channel) + a non-color visual-kind channel, never color alone.
    renderHeader(val({ status: "ready" }));
    const header = screen.getByTestId("brain-status-header");
    expect(header.querySelector(".status-pill")).toBeTruthy();
    expect(header.querySelector("[data-visual-kind]")).toBeTruthy(); // non-color channel
    expect(screen.getByText("Ready")).toBeTruthy(); // the descriptor label (text channel)
  });

  it("degraded_status_renders_distinct_surface", () => {
    // spec(LESSON 11) — a degraded Brain status renders a DISTINCT Brain-degraded surface (its own
    // testid + class), NOT the connection DegradedBanner (never conflated).
    renderHeader(val({ status: "error" }));
    const brainDeg = screen.getByTestId("brain-degraded");
    expect(brainDeg).toBeTruthy();
    expect(brainDeg.className).not.toContain("degraded-banner"); // distinct from the connection banner
    expect(document.querySelector(".degraded-banner")).toBeNull(); // the connection banner is NOT rendered
  });

  it("grounded_at_null_renders_honest_dash", () => {
    // spec(LESSON 32/forbidden#4) — a null grounded_at (no daemon-reported timestamp) renders an
    // honest "grounded —", never "grounded null"/empty/a fabricated time.
    renderHeader(val({ status: "ready", grounded_at: null }));
    const grounded = screen.getByTestId("brain-grounded-at").textContent ?? "";
    expect(grounded).toContain("—");
    expect(grounded).not.toMatch(/null|undefined/i);
  });

  it("healthy_status_no_degraded_surface", () => {
    // spec(§13.1) — a healthy status renders NO degraded surface (honest: degraded only when degraded).
    renderHeader(val({ status: "ready" }));
    expect(screen.queryByTestId("brain-degraded")).toBeNull();
  });

  it("content_still_renders_when_degraded_never_blocks", () => {
    // spec(§13.1) — with an error Brain the page header AND the host content BOTH render: the degraded
    // surface is ADDITIVE, never replaces/blocks/throws (the platform never hard-depends on the Brain).
    render(
      <BrainStatusProvider value={val({ status: "error", grounded_at: null })}>
        <BrainPage />
      </BrainStatusProvider>,
    );
    expect(screen.getByTestId("brain-degraded")).toBeTruthy(); // the additive degraded surface
    expect(screen.getByPlaceholderText(/Ask Project Brain/)).toBeTruthy(); // the page content still renders
  });
});
