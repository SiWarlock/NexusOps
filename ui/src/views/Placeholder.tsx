import type { ReactNode } from "react";

/**
 * An honest placeholder surface for a prototype view whose backing content is
 * gated on daemon contracts (terminal channel / integrations / Brain sidecar).
 * Matches the cockpit visual language; names what it's blocked on — a flagged
 * placeholder, never a fake (§11.6 wire-or-disable; mission "FLAG, don't fake").
 */
export function PlaceholderView({
  title,
  icon,
  blockedOn,
}: {
  title: string;
  icon?: ReactNode;
  blockedOn: string;
}) {
  return (
    <section
      aria-label={title}
      data-testid={`placeholder-${title.toLowerCase().replace(/[^a-z]+/g, "-")}`}
      style={{ height: "100%", overflowY: "auto", padding: "18px 22px" }}
    >
      <header style={{ display: "flex", alignItems: "center", gap: 9, marginBottom: 14 }}>
        {icon ? (
          <span aria-hidden="true" style={{ display: "inline-flex", color: "var(--text-muted)" }}>
            {icon}
          </span>
        ) : null}
        <h1
          style={{
            margin: 0,
            font: "var(--fw-semibold) var(--fs-h3) var(--font-sans)",
            color: "var(--text-primary)",
          }}
        >
          {title}
        </h1>
      </header>
      <div
        style={{
          maxWidth: 520,
          padding: "14px 16px",
          borderRadius: "var(--r-3)",
          border: "1px dashed var(--border-strong)",
          background: "var(--surface-panel)",
          font: "var(--fs-label)/1.5 var(--font-sans)",
          color: "var(--text-secondary)",
        }}
      >
        This surface is daemon-gated — it lands with {blockedOn}. The layout
        slot, navigation, and history wiring are live; the content is not faked.
      </div>
    </section>
  );
}
