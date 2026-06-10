import { useState } from "react";
import {
  Brain,
  Check,
  ChevronRight,
  FileCode,
  FolderGit2,
  GitCommitHorizontal,
  GitMerge,
  GitPullRequest,
  Plus,
} from "lucide-react";
import type { PullRequestRow } from "../../contracts/index";
import {
  Badge,
  Button,
  DiffHunk,
  DisplayStatusPill,
  IconButton,
  MetaChip,
  RiskBadge,
} from "../../design-system/kit";
import { StatusPill } from "../../status/StatusPill";
import { Eyebrow } from "../cockpit";
import { diffFixture, prDisplayFixture, worktreesFixture } from "../display-fixtures";

type Tab = "Review" | "Worktrees" | "Pull requests";

/** Review tab — kit DiffHunk over the diff DISPLAY FIXTURE (the worktree/diff
 *  contract is daemon-gated; accept/approve/fix are disabled, not faked). */
function ReviewTab() {
  return (
    <div style={{ display: "grid", gridTemplateColumns: "220px 1fr", height: "100%", background: "var(--surface-canvas)" }}>
      <aside
        style={{
          borderRight: "1px solid var(--border-default)",
          background: "var(--surface-panel)",
          overflowY: "auto",
          padding: "12px 8px",
        }}
      >
        <Eyebrow style={{ padding: "0 8px 10px" }}>Changed files</Eyebrow>
        {diffFixture.map((f, i) => (
          <div
            key={f.file}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 7,
              padding: "6px 8px",
              borderRadius: "var(--r-2)",
              background: i === 0 ? "var(--surface-active)" : "transparent",
              marginBottom: 2,
            }}
          >
            <FileCode size={13} aria-hidden="true" style={{ color: "var(--text-faint)", flex: "none" }} />
            <span
              style={{
                flex: 1,
                font: "var(--fs-meta) var(--font-mono)",
                color: "var(--text-secondary)",
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {f.file.split("/").pop()}
            </span>
            {f.comments > 0 ? (
              <Badge tone="review" size="xs" mono>
                {f.comments}
              </Badge>
            ) : null}
          </div>
        ))}
        <div style={{ marginTop: "auto", padding: "10px 8px 0", display: "flex", gap: 6 }}>
          <Badge tone="success" variant="dot">
            +476
          </Badge>
          <Badge tone="danger" variant="dot">
            −110
          </Badge>
        </div>
      </aside>
      <div style={{ overflowY: "auto", padding: "14px 16px" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 8 }}>
          <h1 style={{ margin: 0, font: "var(--fw-semibold) var(--fs-h3)/1 var(--font-sans)" }}>
            Review · PR #101
          </h1>
          <MetaChip tone="pr">#101</MetaChip>
          <Badge mono style={{ color: "var(--text-faint)" }}>
            display fixture — diff contract pending
          </Badge>
          <div style={{ marginLeft: "auto", display: "flex", gap: 6 }}>
            <span title="Brain co-pilot arrives with the sidecar contract (Phase 8)">
              <Button variant="secondary" size="sm" icon={<Brain size={14} />} disabled>
                Ask Brain
              </Button>
            </span>
            <span title="PR approval is a Gateway mutation (intent seam — daemon-gated)">
              <Button variant="primary" size="sm" icon={<Check size={14} />} disabled>
                Approve PR
              </Button>
            </span>
          </div>
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
          {/* actions={false}: the kit's per-hunk Accept/Reject bar has no
              disabled mode — until review intents land, rendering it would be
              dead clicks (§11.6 wire-or-disable). */}
          {diffFixture.map((f) => (
            <DiffHunk
              key={f.file}
              file={f.file}
              header={f.header}
              lines={f.lines}
              comments={f.comments}
              actions={false}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

// worktree status → kit pill kind + label (prototype WT_STATUS; "dirty" has no
// kit kind — the prototype falls to idle visuals with the "Dirty" label).
const WT_STATUS = {
  dirty: { pill: "idle", label: "Dirty" },
  active: { pill: "active", label: "Clean" },
  conflict: { pill: "conflict", label: "Conflict" },
} as const;

/** Worktrees tab — DISPLAY FIXTURE (worktree projection daemon-gated). */
function WorktreesTab() {
  return (
    <div style={{ height: "100%", overflowY: "auto", padding: "14px 16px" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 12 }}>
        <Eyebrow>Worktrees</Eyebrow>
        <Badge mono style={{ color: "var(--text-muted)" }}>
          {worktreesFixture.length}
        </Badge>
        <Badge mono style={{ color: "var(--text-faint)" }}>
          display fixture — worktree projection pending
        </Badge>
        <div style={{ marginLeft: "auto" }}>
          <span title="Worktree creation is a Gateway mutation (intent seam — daemon-gated)">
            <Button variant="secondary" size="sm" icon={<Plus size={14} />} disabled>
              New worktree
            </Button>
          </span>
        </div>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        {worktreesFixture.map((w) => {
          const st = WT_STATUS[w.status];
          return (
            <div
              key={w.id}
              style={{
                border: `1px solid ${w.status === "conflict" ? "var(--danger-line)" : "var(--border-default)"}`,
                borderRadius: "var(--r-3)",
                background: "var(--surface-card)",
                padding: "11px 13px",
                display: "flex",
                alignItems: "center",
                gap: 12,
              }}
            >
              <span
                aria-hidden="true"
                style={{
                  width: 30,
                  height: 30,
                  flex: "none",
                  borderRadius: "var(--r-2)",
                  background: "var(--surface-active)",
                  color: "var(--teal-ink)",
                  display: "inline-flex",
                  alignItems: "center",
                  justifyContent: "center",
                }}
              >
                <FolderGit2 size={15} />
              </span>
              <div style={{ minWidth: 0, flex: 1 }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
                  <code style={{ font: "var(--fw-medium) var(--fs-label) var(--font-mono)", color: "var(--text-primary)" }}>
                    {w.path}
                  </code>
                  <DisplayStatusPill status={st.pill} size="xs" label={st.label} />
                  {w.dirty > 0 ? (
                    <span style={{ font: "var(--fs-meta) var(--font-mono)", color: "var(--caution-ink)" }}>
                      {w.dirty} changed
                    </span>
                  ) : null}
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: 6, marginTop: 5, flexWrap: "wrap" }}>
                  <MetaChip tone="branch">{w.branch}</MetaChip>
                  <span style={{ font: "var(--fs-micro) var(--font-mono)", color: "var(--text-faint)" }}>
                    ← {w.base}
                  </span>
                  {w.task ? (
                    <MetaChip tone={w.task.tone} mono={false}>
                      {w.task.id}
                    </MetaChip>
                  ) : null}
                  <MetaChip icon={<GitCommitHorizontal size={12} />}>{w.commit}</MetaChip>
                  {w.pr ? <MetaChip tone="pr">{w.pr}</MetaChip> : null}
                </div>
              </div>
              <RiskBadge level={w.risk} />
              {w.status === "conflict" ? (
                <span title="Conflict resolution is a Gateway mutation (never auto-resolved, §17)">
                  <Button variant="danger" size="sm" icon={<GitMerge size={13} />} disabled>
                    Resolve
                  </Button>
                </span>
              ) : (
                <IconButton label={`Open ${w.path}`} size="sm" disabled>
                  <ChevronRight size={15} />
                </IconButton>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

// PR status → lane (REAL PullRequest projection drives the lanes).
function laneOf(status: string): "open" | "ready" | "merged" {
  if (status === "merged" || status === "closed") return "merged";
  if (status === "approved" || status === "mergeable") return "ready";
  return "open";
}
const LANES: { lane: "open" | "ready" | "merged"; label: string; tone: string }[] = [
  { lane: "open", label: "Open", tone: "var(--review-solid)" },
  { lane: "ready", label: "Ready to merge", tone: "var(--success-solid)" },
  { lane: "merged", label: "Merged", tone: "var(--review-solid)" },
];

/** Pull-requests tab — lanes over the REAL PullRequest projection; diff stats /
 *  branch / age ride the display side-map (projection enrichment flagged). */
function PRsTab({ prs }: { prs: PullRequestRow[] }) {
  return (
    <div style={{ height: "100%", overflowY: "auto", padding: "14px 16px" }}>
      <div style={{ display: "flex", gap: 12, alignItems: "flex-start", minWidth: 0 }}>
        {LANES.map(({ lane, label, tone }) => {
          const items = prs.filter((p) => laneOf(p.status) === lane);
          return (
            <div key={lane} style={{ flex: 1, minWidth: 0 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 7, padding: "0 2px 9px" }}>
                <span aria-hidden="true" style={{ width: 7, height: 7, borderRadius: 999, background: tone }} />
                <span
                  style={{
                    font: "var(--fw-semibold) var(--fs-micro) var(--font-sans)",
                    letterSpacing: "var(--tracking-caps)",
                    textTransform: "uppercase",
                    color: "var(--text-muted)",
                  }}
                >
                  {label}
                </span>
                <span style={{ font: "10px var(--font-mono)", color: "var(--text-faint)" }}>{items.length}</span>
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                {items.map((p) => {
                  const d = prDisplayFixture[p.pr_number];
                  return (
                    <div
                      key={p.pr_number}
                      data-item-id={`PullRequest:${p.pr_number}`}
                      style={{
                        border: "1px solid var(--border-default)",
                        borderRadius: "var(--r-3)",
                        background: "var(--surface-card)",
                        padding: "11px 12px",
                        display: "flex",
                        flexDirection: "column",
                        gap: 8,
                      }}
                    >
                      <div style={{ display: "flex", alignItems: "center", gap: 7 }}>
                        <MetaChip tone="pr">#{p.pr_number}</MetaChip>
                        <StatusPill machine="PullRequest" status={p.status} size="xs" />
                        {d ? (
                          <span style={{ marginLeft: "auto", font: "var(--fs-micro) var(--font-mono)", color: "var(--text-faint)" }}>
                            {d.age}
                          </span>
                        ) : null}
                      </div>
                      <div style={{ font: "var(--fw-medium) var(--fs-label)/1.3 var(--font-sans)", color: "var(--text-primary)" }}>
                        {p.title ?? `PR #${p.pr_number}`}
                      </div>
                      {d ? (
                        <div
                          style={{
                            display: "flex",
                            alignItems: "center",
                            gap: 8,
                            font: "var(--fs-micro) var(--font-mono)",
                            color: "var(--text-faint)",
                          }}
                        >
                          <span style={{ color: "var(--diff-add-ink)" }}>+{d.adds}</span>
                          <span style={{ color: "var(--diff-del-ink)" }}>−{d.dels}</span>
                          <span>· {d.files} files</span>
                          {d.comments > 0 ? <span>· 🗩 {d.comments}</span> : null}
                        </div>
                      ) : null}
                      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                        {d ? <MetaChip tone="branch">{d.branch}</MetaChip> : null}
                        {lane === "ready" ? (
                          <span
                            title="Merge is a Gateway mutation (intent seam — daemon-gated)"
                            style={{ marginLeft: "auto" }}
                          >
                            <Button variant="primary" size="sm" icon={<GitMerge size={12} />} disabled>
                              Merge
                            </Button>
                          </span>
                        ) : null}
                      </div>
                    </div>
                  );
                })}
                {items.length === 0 ? (
                  <div
                    style={{
                      font: "var(--fs-micro) var(--font-sans)",
                      color: "var(--text-faint)",
                      padding: "6px 2px",
                      border: "1px dashed var(--border-subtle)",
                      borderRadius: "var(--r-2)",
                      textAlign: "center",
                    }}
                  >
                    None
                  </div>
                ) : null}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

/**
 * Code & Delivery (ported from kit-views2.jsx DiffReview): the Review ·
 * Worktrees · Pull requests tab strip. Pull requests ride the REAL projection
 * (lanes derived from the frozen status enum); Review/Worktrees render the
 * prototype treatment over DISPLAY FIXTURES until the worktree/diff contracts
 * land (flagged). All mutations disabled, not faked (§11.6).
 */
export function DiffReview({ prs }: { prs: PullRequestRow[] }) {
  const [tab, setTab] = useState<Tab>("Review");
  const tabs: Tab[] = ["Review", "Worktrees", "Pull requests"];
  const counts: Partial<Record<Tab, number>> = {
    Worktrees: worktreesFixture.length,
    "Pull requests": prs.length,
  };
  return (
    <div
      aria-label="Code / Diff Review"
      style={{ display: "flex", flexDirection: "column", height: "100%", background: "var(--surface-canvas)", minHeight: 0 }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 2,
          padding: "10px 16px 0",
          borderBottom: "1px solid var(--border-subtle)",
        }}
      >
        <span aria-hidden="true" style={{ color: "var(--text-secondary)", marginRight: 8, display: "inline-flex" }}>
          <GitPullRequest size={16} />
        </span>
        {tabs.map((t) => (
          <button
            key={t}
            type="button"
            aria-pressed={tab === t}
            onClick={() => setTab(t)}
            style={{
              padding: "8px 12px",
              border: "none",
              background: "transparent",
              cursor: "pointer",
              font: `${tab === t ? "var(--fw-semibold)" : "var(--fw-medium)"} var(--fs-label) var(--font-sans)`,
              color: tab === t ? "var(--accent-ink)" : "var(--text-muted)",
              boxShadow: tab === t ? "inset 0 -2px 0 var(--accent-solid)" : "none",
            }}
          >
            {t}
            {counts[t] != null ? (
              <span style={{ marginLeft: 6, font: "var(--fs-micro) var(--font-mono)", color: "var(--text-faint)" }}>
                {counts[t]}
              </span>
            ) : null}
          </button>
        ))}
      </div>
      <div style={{ flex: 1, minHeight: 0 }}>
        {tab === "Review" ? <ReviewTab /> : tab === "Worktrees" ? <WorktreesTab /> : <PRsTab prs={prs} />}
      </div>
    </div>
  );
}
