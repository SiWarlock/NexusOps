import { StatusPill } from "../../status/StatusPill";
import { deriveBrainHeader, useBrainStatus } from "./brain-status";

/**
 * The shared Project Brain status surface (§11.5 header, §13.1 degrade-gracefully) — reused by the
 * Brain page header (and, via the hosted page, the drawer). Renders the §5.1 ProjectBrain status
 * through the descriptor-bound StatusPill (glyph + label, never color alone — forbidden #5 / LESSON 6)
 * + the daemon-reported grounded-at (verbatim; staleness is NOT client-computed — forbidden #4), and
 * — when the status is degraded (§13.1) — an ADDITIVE, DISTINCT Brain-degraded surface (its own
 * testid + class, NEVER the connection DegradedBanner — LESSON 11, never conflated). The surface is
 * additive: it never replaces or blocks the host content (the platform never hard-depends on the Brain).
 */
export function BrainStatusHeader() {
  const header = deriveBrainHeader(useBrainStatus());
  return (
    <div data-testid="brain-status-header" style={WRAP}>
      <div style={ROW}>
        <StatusPill machine="ProjectBrain" status={header.status} />
        <span data-testid="brain-grounded-at" style={GROUNDED}>
          {header.grounded_at ? `grounded ${header.grounded_at}` : "grounded —"}
        </span>
      </div>
      {header.degraded ? (
        <div data-testid="brain-degraded" role="status" className="brain-degraded" style={DEGRADED}>
          {/* glyph derived from severity (error = critical), never color alone (§11.6). */}
          <span aria-hidden="true">{header.status === "error" ? "⛔" : "⚠"}</span>
          <span>
            Project Brain is degraded ({header.label}) — the platform continues without it.
          </span>
        </div>
      ) : null}
    </div>
  );
}

const WRAP: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 6,
  minWidth: 0,
};
const ROW: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 8,
};
const GROUNDED: React.CSSProperties = {
  font: "var(--fs-meta) var(--font-mono)",
  color: "var(--text-faint)",
};
const DEGRADED: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 7,
  padding: "6px 10px",
  borderRadius: "var(--r-2)",
  background: "var(--brain-surface)",
  border: "1px solid var(--warning-line)",
  font: "var(--fs-meta)/1.4 var(--font-sans)",
  color: "var(--text-secondary)",
};
