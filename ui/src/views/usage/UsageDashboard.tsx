import type { CSSProperties, ReactNode } from "react";
import { UsageMeter as KitUsageMeter } from "@ui-kit/status/UsageMeter";
import type { UsageRow } from "../../contracts/index";
import { buildUsageRows } from "./model";

interface UsageDashboardProps {
  rows: UsageRow[];
}

const card: CSSProperties = {
  border: "1px solid var(--border-default)",
  borderRadius: "var(--r-3)",
  background: "var(--surface-card)",
  padding: "13px 14px",
};

function Eyebrow({ children, style }: { children: ReactNode; style?: CSSProperties }) {
  return (
    <div
      style={{
        font: "var(--fw-semibold) var(--fs-micro)/1 var(--font-sans)",
        letterSpacing: "var(--tracking-caps)",
        textTransform: "uppercase",
        color: "var(--text-faint)",
        ...style,
      }}
    >
      {children}
    </div>
  );
}

/** Prototype StatCard: label / big tabular value / mono sub-line. */
function StatCard({ label, value, sub }: { label: string; value: string; sub: string }) {
  return (
    <div style={{ ...card, padding: "12px 13px" }}>
      <div style={{ font: "var(--fs-meta) var(--font-sans)", color: "var(--text-muted)" }}>{label}</div>
      <div
        style={{
          font: "var(--fw-semibold) var(--fs-h2)/1 var(--font-sans)",
          color: "var(--text-primary)",
          margin: "6px 0 4px",
          fontVariantNumeric: "tabular-nums",
        }}
      >
        {value}
      </div>
      <div style={{ font: "var(--fs-micro) var(--font-mono)", color: "var(--text-faint)" }}>{sub}</div>
    </div>
  );
}

const fmtK = (n: number) => `${Math.round(n / 1000)}k`;

/**
 * The Usage dashboard (driven by the REAL Usage projection, the frozen 11-field UsageRow):
 * stat cards (spend = Σ cost_estimate, tokens = Σ(tokens_in+tokens_out) with the real in/out split —
 * estimated-quality marked with ≈), top context consumers (real context_pct_max; no metadata renders
 * NO bar — "unknown", forbidden #4), and the per-ledger table (accuracy label always present, §11.7).
 * There is NO credit-pool meter — the daemon has no credit-balance source, so the section is honestly
 * OMITTED with a "not reported" note (§11.7), never faked. The prototype's 14-day spend history has no
 * backing projection — its card renders an honest pending note, never invented bars (forbidden #2).
 */
export function UsageDashboard({ rows }: UsageDashboardProps) {
  const vms = buildUsageRows(rows);

  const spend = rows.reduce((s, r) => s + (r.cost_estimate ?? 0), 0);
  const tokensIn = rows.reduce((s, r) => s + (r.tokens_in ?? 0), 0);
  const tokensOut = rows.reduce((s, r) => s + (r.tokens_out ?? 0), 0);
  const anyEstimated = rows.some((r) => r.metric_quality !== "exact");
  // forbidden #4 (mirrors the model's isContextUnknown): no context metadata (context_pct_max null)
  // must not render as a bar.
  const contextRows = rows.filter((r) => r.context_pct_max != null);

  return (
    <div
      className="usage"
      aria-label="Usage"
      style={{ padding: 16, maxWidth: 820, display: "flex", flexDirection: "column", gap: 14 }}
    >
      <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 10 }}>
        <StatCard
          label="Spend today"
          value={`${anyEstimated ? "≈" : ""}$${spend.toFixed(2)}`}
          sub={`across ${rows.length} ledger row${rows.length === 1 ? "" : "s"}`}
        />
        <StatCard
          label="Tokens today"
          value={fmtK(tokensIn + tokensOut)}
          sub={`${fmtK(tokensIn)} in · ${fmtK(tokensOut)} out`}
        />
        <StatCard
          label="Ledger rows"
          value={String(rows.length)}
          sub={`${contextRows.length} reporting context`}
        />
      </div>

      {/* Credit-pool meter HONESTLY OMITTED (W2-usage) — the daemon has no remaining-balance source
          (the SDK monthly pool is not telemetry-observable). Explain the absence (§11.7), never fake it. */}
      <div
        data-testid="credit-pool-unavailable"
        style={{
          ...card,
          font: "var(--fs-meta)/1.5 var(--font-sans)",
          color: "var(--text-faint)",
        }}
      >
        Credit balance not available — not reported by the daemon.
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 14 }}>
        <div style={card}>
          <Eyebrow style={{ marginBottom: 11 }}>Spend · last 14 days</Eyebrow>
          <div
            data-testid="spend-history-pending"
            style={{
              display: "flex",
              alignItems: "center",
              height: 96,
              justifyContent: "center",
              font: "var(--fs-meta)/1.5 var(--font-sans)",
              color: "var(--text-faint)",
              border: "1px dashed var(--border-subtle)",
              borderRadius: "var(--r-2)",
              padding: "0 14px",
              textAlign: "center",
            }}
          >
            Spend history arrives with the usage-history projection (daemon usage
            schema) — not fabricated.
          </div>
        </div>
        <div style={card}>
          <Eyebrow style={{ marginBottom: 11 }}>Top context consumers</Eyebrow>
          <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            {contextRows.length === 0 ? (
              <span style={{ font: "var(--fs-meta) var(--font-sans)", color: "var(--text-faint)" }}>
                No session reports context right now.
              </span>
            ) : (
              [...contextRows]
                .toSorted((a, b) => (b.context_pct_max ?? 0) - (a.context_pct_max ?? 0))
                .slice(0, 4)
                .map((r) => (
                  <div key={r.ledger_id} style={{ display: "flex", alignItems: "center", gap: 10 }}>
                    <span
                      style={{
                        flex: 1,
                        font: "var(--fs-label) var(--font-sans)",
                        color: "var(--text-secondary)",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                      }}
                    >
                      {r.model ?? r.ledger_id}
                    </span>
                    <KitUsageMeter
                      value={r.context_pct_max ?? 0}
                      max={100}
                      valueText={`${r.context_pct_max}%`}
                      accuracy={r.metric_quality ?? "unavailable"}
                      style={{ width: 150, flex: "none" }}
                    />
                  </div>
                ))
            )}
          </div>
        </div>
      </div>

      <div style={card}>
        <Eyebrow style={{ marginBottom: 11 }}>Per-ledger usage</Eyebrow>
        <table className="usage__table" data-testid="usage-table">
          <thead>
            <tr>
              <th scope="col">Subject</th>
              <th scope="col">Model</th>
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
                <tr key={vm.ledgerId} data-item-id={`Usage:${vm.ledgerId}`}>
                  <td>{vm.label}</td>
                  <td>{vm.model}</td>
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
    </div>
  );
}
