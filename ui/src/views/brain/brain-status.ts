// The Project Brain status seam (§11.5 Brain header, §13.1 degrade-gracefully). A PURE UI
// provider over a FAKE-BRAIN value — the swap-point the live daemon 8.1 ProjectBrain status
// source replaces later (there is NO ProjectBrain projection in the frozen ProjectionName enum
// yet; the status is daemon-cached §13.1, served by the not-yet-built 8.1 seam). Mirrors
// active-project.ts / read-only.ts (LESSON 13: pure model + thin context/hook provider —
// createElement not JSX so the pure mappings import freely; NOT a daemon mutation, NOT a
// provisional cross-track contract, NO canSubmitIntent gate). §13.1: the platform NEVER
// hard-depends on or blocks for the Brain — the header degrades to an ADDITIVE distinct surface.
import { createContext, createElement, useContext, type ReactNode } from "react";
import { z } from "zod";
import { ProjectBrain } from "../../contracts/index";
import { describeStatus } from "../../status/descriptors";

/** The frozen §5.1 ProjectBrain status (10 states), delegated to the generated enum. */
export type ProjectBrainStatus = z.infer<typeof ProjectBrain>;

/** The FakeBrain value the provider exposes (and the live 8.1 source later feeds): the daemon's
 *  §5.1 status + the §13.1 `brain_status_reported_at` (`grounded_at`) + an `absent` flag (no seam). */
export interface BrainStatusValue {
  status: ProjectBrainStatus;
  /** The daemon's `brain_status_reported_at`, surfaced verbatim — NOT a client-computed staleness. */
  grounded_at: string | null;
  /** No Brain seam configured/reachable → honest-absent (resolves to not_configured, never faked). */
  absent: boolean;
}

/**
 * §13.1 degraded classification — a TOTAL map over the 10 frozen ProjectBrain states. tsc
 * completeness (a `Record<ProjectBrainStatus, …>`) + the runtime keys==`ProjectBrain.options`
 * drift-pin (brain-status.test) FORCE a new daemon value to classify — never a silent default
 * (LESSON 11 "force a future state to render"). NOT attention-rank-derived: `not_configured` is a
 * rank-0 idle state but IS degraded (the Brain is unusable); `transcript_ingestion_off` is a benign
 * CONFIG choice (ingestion off, not a fault) → NON-degraded.
 */
export const BRAIN_DEGRADED_BY_STATUS: Record<ProjectBrainStatus, boolean> = {
  not_configured: true,
  indexing: false,
  ready: false,
  partial_index: true,
  stale: true,
  graph_degraded: true,
  transcript_ingestion_off: false,
  transcript_ingestion_active: false,
  reindex_required: true,
  error: true,
};

/** The derived header model the surface renders — descriptor-bound label/visualKind + the §13.1
 *  degraded flag + the verbatim grounded_at. A PURE mapping (LESSON 6: read the descriptor, never
 *  recompute the visual). */
export interface BrainHeader {
  status: ProjectBrainStatus;
  label: string;
  visualKind: string;
  degraded: boolean;
  grounded_at: string | null;
}

/**
 * Map a FakeBrain value → the header model. `absent` resolves to `not_configured` (honest-absent,
 * NEVER the carried status — no fabricated "ready"; forbidden #4 / §13.1). The descriptor supplies
 * label + visualKind (read, not recomputed). `_now` is RESERVED for a future relative-time DISPLAY
 * only — it is NEVER used to derive `stale` from a clock; the daemon owns §13.1 status (forbidden #4).
 */
export function deriveBrainHeader(value: BrainStatusValue, _now?: string): BrainHeader {
  const status: ProjectBrainStatus = value.absent ? "not_configured" : value.status;
  const d = describeStatus("ProjectBrain", status);
  return {
    status,
    label: d.label,
    visualKind: d.visualKind,
    degraded: BRAIN_DEGRADED_BY_STATUS[status],
    grounded_at: value.grounded_at,
  };
}

/** The FakeBrain default — the exposed-ahead fixture the provider seeds (FLAG, not faked: replaced
 *  by the live daemon 8.1 ProjectBrain status source — the single swap-point). */
export const fakeBrainStatus: BrainStatusValue = {
  status: "ready",
  grounded_at: "2026-06-21T12:00:00Z",
  absent: false,
};

const BrainStatusContext = createContext<BrainStatusValue>(fakeBrainStatus);

export function BrainStatusProvider(props: {
  value: BrainStatusValue;
  children: ReactNode;
}) {
  return createElement(BrainStatusContext.Provider, { value: props.value }, props.children);
}

export function useBrainStatus(): BrainStatusValue {
  return useContext(BrainStatusContext);
}
