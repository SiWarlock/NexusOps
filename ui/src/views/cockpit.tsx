import type { CSSProperties, ReactNode } from "react";

/* Shared cockpit primitives recurring across the prototype's views
   (kit-shell.jsx Eyebrow + the card/section idioms). Presentation only. */

export function Eyebrow({ children, style }: { children: ReactNode; style?: CSSProperties }) {
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

export const cardStyle: CSSProperties = {
  border: "1px solid var(--border-default)",
  borderRadius: "var(--r-3)",
  background: "var(--surface-card)",
  padding: "13px 14px",
};

export const sectionPad: CSSProperties = { padding: "14px 16px" };

/* Terminal line treatment (kit-views2 TermLine) — used by the Session Terminal
   + Agent Team panes for display-only transcript lines. */
export type TermLineKind = "cmd" | "out" | "dim" | "live" | "warn";

const TERM_COLORS: Record<TermLineKind, string> = {
  cmd: "var(--text-primary)",
  out: "var(--text-secondary)",
  dim: "var(--text-faint)",
  live: "var(--live-ink)",
  warn: "var(--attention-ink)",
};

export function TermLine({ k, t }: { k: TermLineKind; t: string }) {
  const prefix = k === "cmd" ? "$ " : k === "live" ? "▶ " : k === "warn" ? "◆ " : "";
  return (
    <div
      style={{
        color: TERM_COLORS[k],
        lineHeight: "22px",
        minHeight: 22,
        whiteSpace: "pre-wrap",
        wordBreak: "break-word",
      }}
    >
      {prefix ? (
        <span
          style={{
            color: k === "cmd" ? "var(--accent-ink)" : "inherit",
            animation: k === "live" ? "cp-live-pulse 1.6s var(--ease-inout) infinite" : undefined,
          }}
        >
          {prefix}
        </span>
      ) : null}
      {t}
    </div>
  );
}
