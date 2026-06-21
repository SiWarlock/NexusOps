import { Anchor, ArrowUp, Brain, Database, Gavel, Play, Search } from "lucide-react";
import type { ReactNode } from "react";
import { Button, EvidenceChip, RiskBadge } from "../../design-system/kit";
import { Eyebrow } from "../cockpit";
import { BrainStatusHeader } from "./BrainStatusHeader";
import {
  brainMemoryFixture,
  brainThreadFixture,
  type BrainMsgFx,
} from "../display-fixtures";

const BRAIN_MODES = ["Ask", "Plan", "Review", "Decisions", "Memory"];
const SCOPES = ["Entire project", "Current session", "Current PR", "Current plan task"];

/* The prototype's `rich` mini-renderer: **bold** + `code` spans. */
function rich(text: string): ReactNode[] {
  const parts: ReactNode[] = [];
  const re = /(\*\*[^*]+\*\*|`[^`]+`)/g;
  let last = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text))) {
    if (m.index > last) parts.push(text.slice(last, m.index));
    const tok = m[0];
    if (tok.startsWith("**")) {
      parts.push(
        <strong key={m.index} style={{ color: "var(--text-primary)", fontWeight: "var(--fw-semibold)" as never }}>
          {tok.slice(2, -2)}
        </strong>,
      );
    } else {
      parts.push(
        <code
          key={m.index}
          style={{
            font: "var(--fs-meta) var(--font-mono)",
            background: "var(--surface-active)",
            padding: "1px 5px",
            borderRadius: 3,
            color: "var(--brain-ink)",
          }}
        >
          {tok.slice(1, -1)}
        </code>,
      );
    }
    last = m.index + tok.length;
  }
  if (last < text.length) parts.push(text.slice(last));
  return parts;
}

function UserMsg({ text }: { text: string }) {
  return (
    <div style={{ display: "flex", justifyContent: "flex-end" }}>
      <div
        style={{
          maxWidth: "72%",
          background: "var(--surface-active)",
          border: "1px solid var(--border-default)",
          borderRadius: "var(--r-3)",
          padding: "9px 12px",
          font: "var(--fs-body)/1.5 var(--font-sans)",
          color: "var(--text-primary)",
        }}
      >
        {text}
      </div>
    </div>
  );
}

function BrainMsg({ m }: { m: BrainMsgFx }) {
  return (
    <div style={{ display: "flex", gap: 10 }}>
      <span
        aria-hidden="true"
        style={{
          width: 26,
          height: 26,
          flex: "none",
          borderRadius: "var(--r-2)",
          background: "var(--brain-surface)",
          border: "1px solid var(--brain-line)",
          color: "var(--brain-ink)",
          display: "inline-flex",
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        <Brain size={15} />
      </span>
      <div style={{ minWidth: 0, maxWidth: "82%", display: "flex", flexDirection: "column", gap: 9 }}>
        <div style={{ font: "var(--fs-body)/1.55 var(--font-sans)", color: "var(--text-secondary)" }}>
          {rich(m.text)}
        </div>
        {m.evidence?.length ? (
          <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
            {m.evidence.map((e) => (
              <EvidenceChip key={e.label} kind={e.kind} label={e.label} sub={e.sub} freshness={e.freshness} />
            ))}
          </div>
        ) : null}
        {m.plan ? (
          <div
            style={{
              border: "1px solid var(--brain-line)",
              background: "var(--brain-surface)",
              borderRadius: "var(--r-3)",
              padding: "10px 12px",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
              <span style={{ font: "var(--fw-semibold) var(--fs-label) var(--font-sans)", color: "var(--brain-ink)" }}>
                Proposed plan · {m.plan.title}
              </span>
              <span title="Run-via-Gateway arrives with the intent seam (Brain PROPOSES only — invariant #10)" style={{ marginLeft: "auto" }}>
                <Button variant="secondary" size="sm" icon={<Play size={12} />} disabled>
                  Run via Gateway
                </Button>
              </span>
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
              {m.plan.steps.map((s, i) => (
                <div key={i} style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <RiskBadge level={s.risk} />
                  <span style={{ font: "var(--fs-label)/1.4 var(--font-sans)", color: "var(--text-secondary)" }}>
                    {s.text}
                  </span>
                </div>
              ))}
            </div>
          </div>
        ) : null}
      </div>
    </div>
  );
}

/**
 * The Project Brain page (ported from kit-views4.jsx ProjectBrainPage): the
 * co-pilot chat column (mode tabs · scope select · evidence-grounded thread ·
 * composer) + the Memory & decisions rail. DISPLAY-ONLY over a provisional
 * fixture — the Brain sidecar contract is Phase 8 (flagged): the thread is the
 * fixture transcript, the composer/Ask are disabled (no canned fake replies),
 * and Run-via-Gateway is disabled (the Brain PROPOSES only — invariant #10).
 * Mode/scope are pure local UI state.
 */
export function BrainPage({ drawer = false }: { drawer?: boolean }) {
  return (
    <div
      aria-label="Project Brain"
      style={{
        display: "grid",
        gridTemplateColumns: drawer ? "1fr" : "1fr 280px",
        height: "100%",
        background: "var(--surface-canvas)",
        minHeight: 0,
      }}
    >
      {/* chat column */}
      <div style={{ display: "grid", gridTemplateRows: "auto 1fr auto", minHeight: 0, minWidth: 0 }}>
        <div style={{ padding: "12px 16px 0", borderBottom: "1px solid var(--border-subtle)" }}>
          <div style={{ display: "flex", alignItems: "center", gap: 9, marginBottom: 12 }}>
            <span
              aria-hidden="true"
              style={{
                width: 26,
                height: 26,
                borderRadius: "var(--r-2)",
                background: "var(--brain-surface)",
                border: "1px solid var(--brain-line)",
                color: "var(--brain-ink)",
                display: "inline-flex",
                alignItems: "center",
                justifyContent: "center",
              }}
            >
              <Brain size={15} />
            </span>
            <h1 style={{ margin: 0, font: "var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)" }}>
              Project Brain
            </h1>
            {/* The live ProjectBrain §5.1 status + grounded-at + §13.1 honest-degraded surface
                (replaces the static "display fixture" Badge). FakeBrain-fed via useBrainStatus —
                the swap-point for the live daemon 8.1 source. Shown in page mode AND, via the hosted
                page, in the drawer (BrainDrawer renders <BrainPage drawer/>). */}
            <BrainStatusHeader />
            <div style={{ marginLeft: "auto", display: "flex", gap: 6, alignItems: "center" }}>
              <Search size={14} aria-hidden="true" style={{ color: "var(--text-faint)" }} />
              <span style={{ font: "var(--fs-meta) var(--font-mono)", color: "var(--text-faint)" }}>scope:</span>
              <select
                aria-label="Brain scope"
                style={{
                  background: "var(--surface-input)",
                  color: "var(--text-secondary)",
                  border: "1px solid var(--border-default)",
                  borderRadius: "var(--r-1)",
                  font: "var(--fs-meta) var(--font-sans)",
                  padding: "2px 6px",
                }}
              >
                {SCOPES.map((s) => (
                  <option key={s}>{s}</option>
                ))}
              </select>
            </div>
          </div>
          <div style={{ display: "flex", gap: 2 }} role="group" aria-label="Brain modes">
            {BRAIN_MODES.map((m, i) => (
              <button
                key={m}
                type="button"
                aria-pressed={i === 0}
                disabled={i !== 0}
                title={i === 0 ? undefined : "Modes activate with the Brain sidecar (Phase 8)"}
                style={{
                  padding: "7px 11px",
                  border: "none",
                  background: "transparent",
                  cursor: i === 0 ? "pointer" : "default",
                  font: `${i === 0 ? "var(--fw-semibold)" : "var(--fw-medium)"} var(--fs-label) var(--font-sans)`,
                  color: i === 0 ? "var(--brain-ink)" : "var(--text-muted)",
                  boxShadow: i === 0 ? "inset 0 -2px 0 var(--brain-solid)" : "none",
                }}
              >
                {m}
              </button>
            ))}
          </div>
        </div>

        {/* thread (fixture transcript) */}
        <div style={{ overflowY: "auto", padding: 16, display: "flex", flexDirection: "column", gap: 16 }}>
          {brainThreadFixture.map((m, i) =>
            m.from === "user" ? <UserMsg key={i} text={m.text} /> : <BrainMsg key={i} m={m} />,
          )}
        </div>

        {/* composer — disabled until the sidecar lands (no canned fake replies) */}
        <div style={{ padding: "12px 16px", borderTop: "1px solid var(--border-default)", background: "var(--surface-panel)" }}>
          <div
            style={{
              display: "flex",
              alignItems: "flex-end",
              gap: 8,
              background: "var(--surface-input)",
              border: "1px solid var(--border-strong)",
              borderRadius: "var(--r-3)",
              padding: "8px 8px 8px 12px",
              opacity: 0.75,
            }}
          >
            <textarea
              rows={1}
              disabled
              placeholder="Ask Project Brain — activates with the sidecar contract (Phase 8)"
              style={{
                flex: 1,
                resize: "none",
                background: "transparent",
                border: "none",
                outline: "none",
                color: "var(--text-primary)",
                font: "var(--fs-body)/1.5 var(--font-sans)",
                maxHeight: 90,
              }}
            />
            <span title="The Brain co-pilot activates with the sidecar contract (Phase 8)">
              <Button variant="primary" size="sm" icon={<ArrowUp size={14} />} disabled>
                Ask
              </Button>
            </span>
          </div>
        </div>
      </div>

      {/* right rail: memory & decisions (hidden in drawer mode) */}
      {drawer ? null : (
      <aside
        style={{
          borderLeft: "1px solid var(--border-default)",
          background: "var(--surface-panel)",
          overflowY: "auto",
          padding: 14,
        }}
      >
        <Eyebrow style={{ marginBottom: 10 }}>Memory &amp; decisions</Eyebrow>
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {brainMemoryFixture.map((m) => (
            <div
              key={m.label}
              style={{
                border: "1px solid var(--border-subtle)",
                borderRadius: "var(--r-2)",
                padding: "9px 10px",
                background: "var(--surface-card)",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
                <span
                  aria-hidden="true"
                  style={{ color: m.kind === "decision" ? "var(--brain-ink)" : "var(--text-faint)", display: "inline-flex" }}
                >
                  {m.kind === "decision" ? <Gavel size={13} /> : m.kind === "anchor" ? <Anchor size={13} /> : <Database size={13} />}
                </span>
                <span style={{ font: "var(--fw-medium) var(--fs-label) var(--font-sans)", color: "var(--text-primary)" }}>
                  {m.label}
                </span>
                <span
                  style={{
                    marginLeft: "auto",
                    font: "var(--fs-micro) var(--font-mono)",
                    color: m.t === "fresh" ? "var(--success-ink)" : "var(--text-faint)",
                  }}
                >
                  {m.t}
                </span>
              </div>
              <div style={{ font: "var(--fs-meta) var(--font-sans)", color: "var(--text-muted)" }}>{m.sub}</div>
            </div>
          ))}
        </div>
      </aside>
      )}
    </div>
  );
}
