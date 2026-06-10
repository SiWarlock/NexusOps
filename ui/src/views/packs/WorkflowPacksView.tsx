import { useState } from "react";
import {
  ArrowUpCircle,
  BadgeCheck,
  Bot,
  FileText,
  GitMerge,
  Lock,
  Package,
  Play,
  Plus,
  Rocket,
  Settings2,
  SlashSquare,
  Terminal,
  TriangleAlert,
  UsersRound,
  WandSparkles,
  Workflow,
} from "lucide-react";
import type { ReactNode } from "react";
import { Badge, Button, DisplayStatusPill, IconButton } from "../../design-system/kit";
import { Eyebrow } from "../cockpit";
import { packsFixture, type PackFx } from "../display-fixtures";

// instance status → pill + label (the pack ≠ instance guardrail vocabulary).
const INSTANCE = {
  active: { pill: "running", label: "Active" },
  ready: { pill: "completed", label: "Ready" },
  needs_personalization: { pill: "waiting-perm", label: "Needs personalization" },
  upgrade_available: { pill: "stale", label: "Upgrade available" },
} as const;
const PROVIDER = { bundled: "Bundled", user: "You", third_party: "3rd-party" } as const;

function CapRow({ icon, label, val, mono }: { icon: ReactNode; label: string; val: ReactNode; mono?: boolean }) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 8, padding: "5px 0" }}>
      <span aria-hidden="true" style={{ color: "var(--text-faint)", display: "inline-flex" }}>
        {icon}
      </span>
      <span style={{ font: "var(--fs-label) var(--font-sans)", color: "var(--text-muted)" }}>{label}</span>
      <span
        style={{
          marginLeft: "auto",
          font: `var(--fw-medium) var(--fs-label) ${mono ? "var(--font-mono)" : "var(--font-sans)"}`,
          color: "var(--text-primary)",
        }}
      >
        {val}
      </span>
    </div>
  );
}

function PackDetail({ pack }: { pack: PackFx }) {
  const inst = INSTANCE[pack.instance];
  const notReady = pack.instance === "needs_personalization";
  const upgrade = pack.instance === "upgrade_available";
  return (
    <div style={{ overflowY: "auto", minWidth: 0 }}>
      <div style={{ padding: 16, borderBottom: "1px solid var(--border-subtle)" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 8 }}>
          <h1 style={{ margin: 0, font: "var(--fw-semibold) var(--fs-h2)/1 var(--font-mono)", letterSpacing: "var(--tracking-tight)" }}>
            {pack.name}
          </h1>
          <DisplayStatusPill status={inst.pill} label={inst.label} beacon={notReady} />
          <Badge mono style={{ color: "var(--text-muted)" }}>
            v{pack.version}
          </Badge>
          <div style={{ marginLeft: "auto", display: "flex", gap: 6 }}>
            {notReady ? (
              <span title="Personalization is a Gateway-reviewed run (intent seam — daemon-gated)">
                <Button variant="secondary" size="sm" icon={<WandSparkles size={14} />} disabled>
                  Personalize
                </Button>
              </span>
            ) : upgrade ? (
              <span title="Upgrade is a Gateway-reviewed run (intent seam — daemon-gated)">
                <Button variant="primary" size="sm" icon={<ArrowUpCircle size={14} />} disabled>
                  Upgrade
                </Button>
              </span>
            ) : (
              <span title="Pack configuration arrives with the pack-registry contract">
                <Button variant="secondary" size="sm" icon={<Settings2 size={14} />} disabled>
                  Configure
                </Button>
              </span>
            )}
          </div>
        </div>
        <p style={{ margin: 0, font: "var(--fs-body)/1.5 var(--font-sans)", color: "var(--text-secondary)", maxWidth: 620 }}>
          {pack.desc}
        </p>
      </div>

      {/* readiness banner — the pack≠instance guardrail */}
      {notReady ? (
        <div
          style={{
            margin: "14px 16px 0",
            display: "flex",
            alignItems: "flex-start",
            gap: 10,
            padding: "11px 13px",
            borderRadius: "var(--r-3)",
            background: "var(--caution-surface)",
            border: "1px solid var(--caution-line)",
          }}
        >
          <span aria-hidden="true" style={{ color: "var(--caution-ink)", marginTop: 1, display: "inline-flex" }}>
            <TriangleAlert size={16} />
          </span>
          <div>
            <div style={{ font: "var(--fw-semibold) var(--fs-label) var(--font-sans)", color: "var(--caution-ink)" }}>
              Template pack — not ready to run
            </div>
            <div style={{ font: "var(--fs-meta)/1.5 var(--font-sans)", color: "var(--text-secondary)", marginTop: 3 }}>
              This pack is installed but has no personalized instance for {pack.project}. Commands stay
              locked until personalization completes (a Gateway-reviewed run).
            </div>
          </div>
        </div>
      ) : null}
      {upgrade ? (
        <div
          style={{
            margin: "14px 16px 0",
            display: "flex",
            alignItems: "center",
            gap: 10,
            padding: "11px 13px",
            borderRadius: "var(--r-3)",
            background: "var(--warning-surface)",
            border: "1px solid var(--warning-line)",
          }}
        >
          <span aria-hidden="true" style={{ color: "var(--warning-ink)", display: "inline-flex" }}>
            <ArrowUpCircle size={16} />
          </span>
          <div style={{ font: "var(--fs-meta)/1.5 var(--font-sans)", color: "var(--text-secondary)" }}>
            <strong style={{ color: "var(--warning-ink)" }}>v{pack.version} → newer available.</strong>{" "}
            Review the changelog before upgrading; owned files will be re-generated through the Gateway.
          </div>
        </div>
      ) : null}

      <div style={{ padding: 16, display: "grid", gridTemplateColumns: "1fr 240px", gap: 16, alignItems: "start" }}>
        {/* commands */}
        <div>
          <Eyebrow style={{ marginBottom: 10 }}>Commands · {pack.commands.length}</Eyebrow>
          <div style={{ display: "flex", flexDirection: "column", gap: 7 }}>
            {pack.commands.map((c) => {
              const locked = c.needsInstance && notReady;
              return (
                <div
                  key={c.name}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 10,
                    padding: "10px 12px",
                    borderRadius: "var(--r-2)",
                    border: "1px solid var(--border-default)",
                    background: "var(--surface-card)",
                    opacity: locked ? 0.6 : 1,
                  }}
                >
                  <span aria-hidden="true" style={{ color: locked ? "var(--text-faint)" : "var(--accent-ink)", display: "inline-flex" }}>
                    {c.type === "recipe" ? <Workflow size={15} /> : <SlashSquare size={15} />}
                  </span>
                  <code style={{ font: "var(--fw-medium) var(--fs-body) var(--font-mono)", color: "var(--text-primary)" }}>
                    {c.name}
                  </code>
                  <div style={{ display: "flex", gap: 5, marginLeft: 4 }}>
                    <Badge>{c.type}</Badge>
                    {c.creates ? <Badge tone="teal">creates {c.creates}</Badge> : null}
                  </div>
                  <span style={{ marginLeft: "auto" }}>
                    {locked ? (
                      <span
                        style={{
                          display: "inline-flex",
                          alignItems: "center",
                          gap: 5,
                          font: "var(--fs-meta) var(--font-sans)",
                          color: "var(--caution-ink)",
                        }}
                      >
                        <Lock size={12} aria-hidden="true" /> needs instance
                      </span>
                    ) : (
                      <span title="Command runs are Gateway-reviewed (intent seam — daemon-gated)">
                        <Button variant="ghost" size="sm" icon={<Play size={12} />} disabled>
                          Run
                        </Button>
                      </span>
                    )}
                  </span>
                </div>
              );
            })}
          </div>

          {pack.roles.length > 0 ? (
            <div style={{ marginTop: 18 }}>
              <Eyebrow style={{ marginBottom: 10 }}>Agent roles</Eyebrow>
              <div style={{ display: "flex", gap: 7, flexWrap: "wrap" }}>
                {pack.roles.map((r, i) => (
                  <span
                    key={r}
                    style={{
                      display: "inline-flex",
                      alignItems: "center",
                      gap: 6,
                      height: 26,
                      padding: "0 10px",
                      borderRadius: "var(--r-2)",
                      border: "1px solid var(--teal-line)",
                      background: "var(--teal-surface)",
                      color: "var(--teal-ink)",
                      font: "var(--fw-medium) var(--fs-label) var(--font-sans)",
                    }}
                  >
                    {i === 0 ? <Workflow size={13} aria-hidden="true" /> : <Bot size={13} aria-hidden="true" />} {r}
                  </span>
                ))}
              </div>
            </div>
          ) : null}
        </div>

        {/* capabilities sidebar */}
        <div
          style={{
            border: "1px solid var(--border-subtle)",
            borderRadius: "var(--r-3)",
            background: "var(--surface-card)",
            padding: "13px 14px",
          }}
        >
          <Eyebrow style={{ marginBottom: 11 }}>Capabilities</Eyebrow>
          <CapRow icon={<Terminal size={14} />} label="Commands" val={pack.commands.length} />
          <CapRow icon={<UsersRound size={14} />} label="Agent roles" val={pack.roles.length || "—"} />
          <CapRow icon={<Rocket size={14} />} label="Launch recipes" val={pack.recipes} />
          <CapRow icon={<FileText size={14} />} label="Plan parser" val={pack.parser ?? "—"} mono={!!pack.parser} />
          <div style={{ height: 1, background: "var(--border-subtle)", margin: "10px 0" }} />
          <CapRow icon={<GitMerge size={14} />} label="Mutations" val="via Gateway" />
          <CapRow icon={<BadgeCheck size={14} />} label="Provider" val={PROVIDER[pack.provider]} />
        </div>
      </div>
    </div>
  );
}

/**
 * Workflow Packs (ported from kit-views5.jsx WorkflowPacksView): the pack
 * library list + detail pane, enforcing the pack-vs-instance distinction (an
 * installed pack never implies runnable commands). DISPLAY-ONLY over a
 * provisional fixture — the WorkflowInstance/pack-registry projection is
 * daemon-gated (flagged); Install/Personalize/Upgrade/Run disabled (§11.6).
 */
export function WorkflowPacksView() {
  const [sel, setSel] = useState(packsFixture[0]!.id);
  const pack = packsFixture.find((p) => p.id === sel) ?? packsFixture[0]!;
  return (
    <div
      aria-label="Workflow Packs"
      style={{ display: "grid", gridTemplateColumns: "300px 1fr", height: "100%", background: "var(--surface-canvas)", minHeight: 0 }}
    >
      {/* pack library */}
      <aside style={{ borderRight: "1px solid var(--border-default)", background: "var(--surface-panel)", overflowY: "auto" }}>
        <div style={{ padding: "14px 14px 8px", display: "flex", alignItems: "center", gap: 8 }}>
          <span aria-hidden="true" style={{ color: "var(--teal-ink)", display: "inline-flex" }}>
            <Package size={16} />
          </span>
          <h1 style={{ margin: 0, font: "var(--fw-semibold) var(--fs-sub)/1 var(--font-sans)" }}>Workflow Packs</h1>
          <span style={{ marginLeft: "auto" }}>
            <IconButton label="Install pack" size="sm" disabled>
              <Plus size={15} />
            </IconButton>
          </span>
        </div>
        <div style={{ padding: "0 14px 6px" }}>
          <Badge mono style={{ color: "var(--text-faint)" }}>
            display fixture — pack registry pending
          </Badge>
        </div>
        <div style={{ padding: "4px 8px 14px" }}>
          {packsFixture.map((p) => {
            const inst = INSTANCE[p.instance];
            const active = sel === p.id;
            return (
              <button
                key={p.id}
                type="button"
                onClick={() => setSel(p.id)}
                style={{
                  display: "block",
                  width: "100%",
                  textAlign: "left",
                  cursor: "pointer",
                  border: "none",
                  borderRadius: "var(--r-2)",
                  padding: "9px 10px",
                  marginBottom: 3,
                  background: active ? "var(--surface-active)" : "transparent",
                  boxShadow: active ? "inset 0 0 0 1px var(--accent-line)" : "none",
                }}
              >
                <span style={{ display: "flex", alignItems: "center", gap: 7 }}>
                  <span style={{ font: "var(--fw-medium) var(--fs-body) var(--font-mono)", color: "var(--text-primary)" }}>
                    {p.name}
                  </span>
                  <span style={{ marginLeft: "auto" }}>
                    <DisplayStatusPill
                      status={inst.pill}
                      size="xs"
                      label={inst.label}
                      beacon={p.instance === "needs_personalization"}
                    />
                  </span>
                </span>
                <span
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 6,
                    marginTop: 6,
                    font: "var(--fs-micro) var(--font-mono)",
                    color: "var(--text-faint)",
                  }}
                >
                  <span>v{p.version}</span>
                  <span>·</span>
                  <span>{PROVIDER[p.provider]}</span>
                  <span>·</span>
                  <span>{p.project}</span>
                </span>
              </button>
            );
          })}
        </div>
      </aside>

      <PackDetail pack={pack} />
    </div>
  );
}
