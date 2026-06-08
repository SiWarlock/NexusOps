import { UsageMeter as KitUsageMeter } from "@ui-kit/status/UsageMeter";
import type { CreditPool, UsageRow } from "../../contracts/index";
import {
  buildUsageRows,
  creditPoolState,
  type CreditPoolState,
} from "./model";

interface UsageDashboardProps {
  rows: UsageRow[];
  creditPool: CreditPool | null;
}

const CREDIT_POOL_LABEL: Record<CreditPoolState, string> = {
  normal: "Normal",
  near_exhaustion: "Near exhaustion",
  hard_stop: "Hard stop",
};

// A non-color channel for the credit-pool threshold (forbidden #5): the kit
// UsageMeter escalates fill COLOR by threshold, so the state must ALSO be carried
// by a glyph + label — never color alone.
const CREDIT_POOL_GLYPH: Record<CreditPoolState, string> = {
  normal: "●",
  near_exhaustion: "⚠",
  hard_stop: "⛔",
};

/**
 * The Usage dashboard (§11.4/§11.7/§9.1): per-subject token/cost usage with the
 * accuracy label on every row (never dropped — §11.7), Codex/null context as the
 * literal "unknown" (forbidden #4), "unknown" (not 0) for unavailable values, and
 * the Agent-SDK credit-pool meter whose threshold shows on a non-color channel
 * (forbidden #5). Renders ONLY the projection's rows (forbidden #2); reads its
 * data through props (the Shell's gateway boundary). Interim 4th content-view —
 * relocates to a Settings tab at 6.4c.
 */
export function UsageDashboard({ rows, creditPool }: UsageDashboardProps) {
  const vms = buildUsageRows(rows);
  const poolState: CreditPoolState | null = creditPool
    ? creditPoolState(creditPool.used, creditPool.limit)
    : null;

  return (
    <div className="usage" aria-label="Usage">
      {creditPool && poolState ? (
        <section className="usage__credit-pool" aria-label="Agent-SDK credit pool">
          <KitUsageMeter
            value={creditPool.used}
            max={creditPool.limit}
            variant="bar"
            label="Agent-SDK credits"
            valueText={`${creditPool.used} / ${creditPool.limit}`}
            accuracy="exact"
          />
          <span
            className="usage__credit-pool-state"
            data-testid="credit-pool-state"
            data-state={poolState}
          >
            {CREDIT_POOL_GLYPH[poolState]} {CREDIT_POOL_LABEL[poolState]}
          </span>
        </section>
      ) : null}

      <table className="usage__table" data-testid="usage-table">
        <thead>
          <tr>
            <th scope="col">Subject</th>
            <th scope="col">Harness</th>
            <th scope="col">Tokens</th>
            <th scope="col">Cost</th>
            <th scope="col">Context</th>
            <th scope="col">Accuracy</th>
          </tr>
        </thead>
        <tbody>
          {vms.length === 0 ? (
            <tr>
              <td colSpan={6} data-testid="usage-empty">
                No usage data.
              </td>
            </tr>
          ) : (
            vms.map((vm) => (
              <tr key={vm.subjectId} data-item-id={`Usage:${vm.subjectId}`}>
                <td>{vm.subjectId}</td>
                <td>{vm.harness}</td>
                <td>{vm.tokensDisplay}</td>
                <td>{vm.costDisplay}</td>
                <td className="usage__context">{vm.contextDisplay}</td>
                <td className="usage__accuracy" data-accuracy={vm.metricQuality}>
                  {vm.accuracyLabel}
                </td>
              </tr>
            ))
          )}
        </tbody>
      </table>
    </div>
  );
}
