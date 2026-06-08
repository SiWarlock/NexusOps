// The Usage view-model (§11.4/§11.7/§9.1): a pure mapping of usage rows → display
// VMs that enforces three render-policy rules in ONE place (views never fabricate
// a % or drop a label):
//   (a) the accuracy label is derived from metric_quality and is always present;
//   (b) forbidden #4 — Codex / null context renders the literal "unknown", NEVER a
//       number or 0% (supportsContextMetadata=false);
//   (c) an `unavailable` usage value renders "unknown", never 0/empty.
import type { Harness, MetricQuality, UsageRow } from "../../contracts/index";

export type CreditPoolState = "normal" | "near_exhaustion" | "hard_stop";

export interface UsageRowVM {
  subjectId: string;
  harness: Harness;
  metricQuality: MetricQuality;
  /** Always present (§11.7) — never dropped, even for `exact`. */
  accuracyLabel: string;
  /** A percentage like "64%", or the literal "unknown" (forbidden #4 / null). */
  contextDisplay: string;
  /** The token count, or "unknown" when the metric is unavailable. */
  tokensDisplay: string;
  /** The cost, or "unknown" when the metric is unavailable. */
  costDisplay: string;
}

const UNKNOWN = "unknown";

const ACCURACY_LABEL: Record<MetricQuality, string> = {
  exact: "exact",
  estimated: "estimated",
  unavailable: "unavailable",
};

// forbidden #4: context is "unknown" for Codex (no context metadata, §9.1) OR a
// null/absent context_pct — NEVER a number, even if a stray one arrives on Codex.
function isContextUnknown(row: UsageRow): boolean {
  return row.harness === "codex" || row.context_pct == null;
}

export function buildUsageRows(rows: UsageRow[]): UsageRowVM[] {
  return rows.map((row) => {
    const unavailable = row.metric_quality === "unavailable";
    return {
      subjectId: row.subject_id,
      harness: row.harness,
      metricQuality: row.metric_quality,
      accuracyLabel: ACCURACY_LABEL[row.metric_quality],
      contextDisplay: isContextUnknown(row) ? UNKNOWN : `${row.context_pct}%`,
      tokensDisplay: unavailable ? UNKNOWN : String(row.tokens),
      costDisplay: unavailable ? UNKNOWN : `$${row.cost.toFixed(2)}`,
    };
  });
}

/** Provisional thresholds (§11.4 pins no exact numbers — reconcile at the daemon freeze). */
const NEAR_EXHAUSTION_REMAINING_PCT = 15;

export function creditPoolState(used: number, limit: number): CreditPoolState {
  // Assumes the daemon invariant 0 <= used (a negative `used` would read as
  // >100% remaining → normal, which is a meaningless input, not a real state).
  if (limit <= 0) return "hard_stop"; // no pool == exhausted
  const remainingPct = ((limit - used) / limit) * 100;
  if (remainingPct <= 0) return "hard_stop";
  if (remainingPct <= NEAR_EXHAUSTION_REMAINING_PCT) return "near_exhaustion";
  return "normal";
}
