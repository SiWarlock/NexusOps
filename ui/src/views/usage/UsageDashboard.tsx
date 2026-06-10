import type { CSSProperties, ReactNode } from "react";
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

/**
 * The Usage dashboard (ported to the kit-views4 UsageSection layout, driven by
 * the REAL Usage projection): stat cards (spend/tokens aggregates — sums of the
 * projection rows; estimated-quality marked with ≈), the Agent-SDK credit-pool
 * meter with its non-color threshold state (forbidden #5), top context
 * consumers (real context_pct; Codex/null renders NO bar — "unknown", forbidden
 * #4), and the per-subject table (accuracy label always present, §11.7).
 * The prototype's 14-day spend history has NO backing projection — its card
 * renders an honest pending note (flagged), never invented bars (forbidden #2).
 */
export function UsageDashboard({ rows, creditPool }: UsageDashboardProps) {
  const vms = buildUsageRows(rows);
  const poolState: CreditPoolState | null = creditPool
    ? creditPoolState(creditPool.used, creditPool.limit)
    : null;

  const spend = rows.reduce((s, r) => s + r.cost, 0);
  const tokens = rows.reduce((s, r) => s + r.tokens, 0);
  const anyEstimated = rows.some((r) => r.metric_quality !== "exact");
  // forbidden #4 (mirrors the model's isContextUnknown): Codex reports no context
  // metadata — a stray number on a codex row must still not render as a bar.
  const contextRows = rows.filter((r) => r.harness !== "codex" && r.context_pct != null);

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
          sub={`across ${rows.length} subject${rows.length === 1 ? "" : "s"}`}
        />
        <StatCard
          label="Tokens today"
          value={`${Math.round(tokens / 1000)}k`}
          sub="in + out (split lands with usage enrichment)"
        />
        <StatCard
          label="Subjects metered"
          value={String(rows.length)}
          sub={`${contextRows.length} reporting context`}
        />
      </div>

      {creditPool && poolState ? (
        <section className="usage__credit-pool" aria-label="Agent-SDK credit pool" style={card}>
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
            style={{
              display: "inline-block",
              marginTop: 8,
              font: "var(--fs-meta) var(--font-sans)",
              color:
                poolState === "normal"
                  ? "var(--success-ink)"
                  : poolState === "near_exhaustion"
                    ? "var(--warning-ink)"
                    : "var(--danger-ink)",
            }}
          >
            {CREDIT_POOL_GLYPH[poolState]} {CREDIT_POOL_LABEL[poolState]}
          </span>
        </section>
      ) : null}

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
                .toSorted((a, b) => (b.context_pct ?? 0) - (a.context_pct ?? 0))
                .slice(0, 4)
                .map((r) => (
                  <div key={r.subject_id} style={{ display: "flex", alignItems: "center", gap: 10 }}>
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
                      {r.subject_id}
                    </span>
                    <KitUsageMeter
                      value={r.context_pct ?? 0}
                      max={100}
                      valueText={`${r.context_pct}%`}
                      accuracy={r.metric_quality}
                      style={{ width: 150, flex: "none" }}
                    />
                  </div>
                ))
            )}
          </div>
        </div>
      </div>

      <div style={card}>
        <Eyebrow style={{ marginBottom: 11 }}>Per-subject usage</Eyebrow>
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
    </div>
  );
}
