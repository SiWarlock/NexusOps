import { describe, it, expect } from "vitest";
import {
  deriveBrainHeader,
  BRAIN_DEGRADED_BY_STATUS,
  type BrainStatusValue,
} from "./brain-status";
import { ProjectBrain } from "../../contracts/index";

const val = (over: Partial<BrainStatusValue> = {}): BrainStatusValue => ({
  status: "ready",
  grounded_at: "2026-06-21T12:00:00Z",
  absent: false,
  ...over,
});

describe("brain-status — FakeBrain header model (§5.1/§13.1)", () => {
  it("derive_header_maps_status_to_descriptor", () => {
    // spec(§5.1/LESSON 6) — deriveBrainHeader READS the ProjectBrain descriptor's label/visualKind,
    // never a recomputed value (the descriptor table is the single source).
    const h = deriveBrainHeader(val({ status: "ready" }));
    expect(h.status).toBe("ready");
    expect(h.label).toBe("Ready"); // humanize(status), from the descriptor
    expect(h.visualKind).toBe("completed"); // the ProjectBrain descriptor's kind
    expect(h.degraded).toBe(false);
  });

  it("degraded_statuses_flag_degraded", () => {
    // spec(§13.1) — the degraded states flag degraded:true; the healthy/benign states flag false.
    // The classification map is drift-pinned to the generated ProjectBrain.options so a NEW daemon
    // value is FORCED to classify (tsc completeness + the runtime keys==options pin) — never a silent default.
    for (const s of [
      "not_configured",
      "stale",
      "error",
      "partial_index",
      "graph_degraded",
      "reindex_required",
    ] as const) {
      expect(deriveBrainHeader(val({ status: s })).degraded, s).toBe(true);
    }
    for (const s of [
      "ready",
      "indexing",
      "transcript_ingestion_active",
      "transcript_ingestion_off",
    ] as const) {
      expect(deriveBrainHeader(val({ status: s })).degraded, s).toBe(false);
    }
    expect(Object.keys(BRAIN_DEGRADED_BY_STATUS).toSorted()).toEqual(
      [...ProjectBrain.options].toSorted(),
    );
  });

  it("absent_brain_is_not_configured_not_faked", () => {
    // spec(§13.1/forbidden#4) — an absent Brain resolves to not_configured (honest-absent), NEVER
    // the carried status (no fabricated "ready" when the source is absent).
    const h = deriveBrainHeader(val({ status: "ready", absent: true }));
    expect(h.status).toBe("not_configured");
    expect(h.status).not.toBe("ready");
    expect(h.degraded).toBe(true); // an absent/not_configured Brain is degraded (unusable)
  });

  it("staleness_is_reported_not_computed", () => {
    // spec(§13.1/forbidden#4) — grounded_at is the daemon's REPORTED timestamp, surfaced verbatim;
    // the UI never invents a stale threshold from a clock. A very old grounded_at on a `ready` status
    // stays ready (NEVER client-flipped to stale); passing a `now` far past it changes nothing.
    const old = "2020-01-01T00:00:00Z";
    const h = deriveBrainHeader(val({ status: "ready", grounded_at: old }), "2026-06-21T12:00:00Z");
    expect(h.grounded_at).toBe(old); // verbatim, not reformatted/recomputed
    expect(h.status).toBe("ready"); // never client-flipped to stale
    expect(h.degraded).toBe(false);
    // the daemon-reported `stale` IS surfaced — because it comes from the status, not a client clock.
    expect(deriveBrainHeader(val({ status: "stale" })).status).toBe("stale");
  });
});
