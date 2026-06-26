// The Usage view-model (§11.4/§11.7/§9.1): a pure mapping of usage rows → display
// VMs that enforces three render-policy rules in ONE place (views never fabricate
// a % or drop a label):
//   (a) the accuracy label is derived from metric_quality (null → "unavailable") and is always present;
//   (b) forbidden #4 — no context metadata (context_pct_max null, §9.1 supportsContextMetadata=false)
//       renders the literal "unknown", NEVER a number/0%;
//   (c) an `unavailable` usage value renders "unknown", never 0/empty.
import type { MetricQuality, UsageRow } from "../../contracts/index";

export interface UsageRowVM {
  /** The ledger PK — the React key + data-item-id. */
  ledgerId: string;
  /** The primary display label: the model, falling back to ledger_id when model is null (Q1). */
  label: string;
  /** The raw model (or "—" when null) — the table's Model column. */
  model: string;
  /** The effective metric quality (null coalesced → "unavailable", §11.7). */
  metricQuality: MetricQuality;
  /** Always present (§11.7) — never dropped, even for `exact`. */
  accuracyLabel: string;
  /** A percentage like "64%", or the literal "unknown" (forbidden #4 / null). */
  contextDisplay: string;
  /** The token count Σ(tokens_in + tokens_out), or "unknown" when the metric is unavailable. */
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

/** metric_quality is Option<MetricQuality> — null coalesces to the honest "unavailable" degrade
 *  (§11.7 / Q3), NOT a value-hiding default (it renders "unavailable" accuracy + "unknown" values,
 *  never a fabricated number). */
function effectiveQuality(q: MetricQuality | null): MetricQuality {
  return q ?? "unavailable";
}

// forbidden #4: context is "unknown" when the daemon serves no context metadata (context_pct_max
// null, §9.1 supportsContextMetadata=false) — NEVER a number, even a stray one. A real 0 is "0%".
function isContextUnknown(row: UsageRow): boolean {
  return row.context_pct_max == null;
}

export function buildUsageRows(rows: UsageRow[]): UsageRowVM[] {
  return rows.map((row) => {
    const quality = effectiveQuality(row.metric_quality);
    const unavailable = quality === "unavailable";
    const tokens = (row.tokens_in ?? 0) + (row.tokens_out ?? 0);
    return {
      ledgerId: row.ledger_id,
      label: row.model ?? row.ledger_id,
      model: row.model ?? "—",
      metricQuality: quality,
      accuracyLabel: ACCURACY_LABEL[quality],
      contextDisplay: isContextUnknown(row) ? UNKNOWN : `${row.context_pct_max}%`,
      // "unknown" when the metric is unavailable OR the underlying numeric is null (not reported) —
      // NEVER a fabricated 0/$0.00 on an exact/estimated row with missing data (§11.7 / forbidden #2).
      tokensDisplay:
        unavailable || (row.tokens_in == null && row.tokens_out == null) ? UNKNOWN : String(tokens),
      costDisplay:
        unavailable || row.cost_estimate == null ? UNKNOWN : `$${row.cost_estimate.toFixed(2)}`,
    };
  });
}
