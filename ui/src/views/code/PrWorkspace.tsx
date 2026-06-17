import type { CSSProperties } from "react";
import { ArrowLeft, Brain, FileDiff, GitMerge } from "lucide-react";
import type { PullRequestRow, ReviewRow } from "../../contracts/index";
import { Button, MetaChip } from "../../design-system/kit";
import { StatusPill } from "../../status/StatusPill";
import { Eyebrow } from "../cockpit";
import { ReviewsList } from "./ReviewsList";

const PLACEHOLDER: CSSProperties = {
  font: "var(--fs-label)/1.5 var(--font-sans)",
  color: "var(--text-muted)",
  padding: "12px 14px",
  border: "1px dashed var(--border-subtle)",
  borderRadius: "var(--r-2)",
};

const SECTION: CSSProperties = { display: "flex", flexDirection: "column", gap: 8, marginTop: 16 };

/** Mergeability → a glyph + LABEL (never color alone — forbidden #5). `null` is an HONEST "unknown",
 *  not a fabricated state (the daemon hasn't reported it). */
function mergeability(mergeable: boolean | null | undefined): { glyph: string; label: string; color: string } {
  if (mergeable === true) return { glyph: "✓", label: "Mergeable", color: "var(--success-ink)" };
  if (mergeable === false) return { glyph: "✗", label: "Conflicts", color: "var(--danger-ink)" };
  return { glyph: "?", label: "Mergeability unknown", color: "var(--text-faint)" };
}

/**
 * The read-only PR Review Workspace (ui-064 Layer 2, §11.2) — the PR-detail panel for a selected PR.
 * Renders the parts backed by the frozen `PullRequestRow` + `ReviewRow` (header, mergeability, checks,
 * reviews-list); the un-buildable parts (diff-stats D6, PR code-diff D7) are HONEST "unavailable — needs
 * daemon <X>" placeholders (never a fabricated stat, never `get_diff`-as-PR-diff — this component takes
 * NO gateway, so it cannot reach `get_diff` by construction). ALL mutations (Merge / Approve PR / Request
 * changes) + Brain controls render DISABLED — a future cat-1 arc + the deferred Brain sibling
 * (wire-or-disable, never a dead click). The "← Worktree diff" deselect returns to the 6.3e per-hunk view.
 */
export function PrWorkspace({
  pr,
  reviews,
  onBack,
}: {
  pr: PullRequestRow;
  reviews: ReviewRow[];
  /** Deselect → return to the 6.3e worktree per-hunk diff. Always provided (the workspace is only shown
   *  in place of the worktree diff, so a way back is mandatory) — a wired control, never a dead click. */
  onBack: () => void;
}) {
  const merge = mergeability(pr.mergeable);
  return (
    <div style={{ height: "100%", overflowY: "auto", padding: "14px 16px", background: "var(--surface-canvas)" }}>
      {/* Header: number + title + status + the wired deselect. */}
      <div style={{ display: "flex", alignItems: "center", gap: 10, flexWrap: "wrap" }}>
        <MetaChip tone="pr">#{pr.pr_number ?? "—"}</MetaChip>
        <h1 style={{ margin: 0, font: "var(--fw-semibold) var(--fs-h3)/1.2 var(--font-sans)", color: "var(--text-primary)" }}>
          {pr.title ?? (pr.pr_number != null ? `PR #${pr.pr_number}` : pr.pr_id)}
        </h1>
        <StatusPill machine="PullRequest" status={pr.status} size="xs" />
        <div style={{ marginLeft: "auto" }}>
          <Button variant="ghost" size="sm" icon={<ArrowLeft size={13} />} onClick={onBack}>
            Worktree diff
          </Button>
        </div>
      </div>

      {/* Branch line + mergeability + checks (from the frozen row; never color alone). */}
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginTop: 8, flexWrap: "wrap" }}>
        <span style={{ font: "var(--fs-meta) var(--font-mono)", color: "var(--text-secondary)" }}>
          {pr.head_branch ?? "—"} → {pr.base_branch ?? "—"}
        </span>
        <span style={{ display: "inline-flex", alignItems: "center", gap: 5, color: merge.color, font: "var(--fs-meta) var(--font-sans)" }}>
          <span aria-hidden="true">{merge.glyph}</span>
          {merge.label}
        </span>
        <span style={{ font: "var(--fs-meta) var(--font-sans)", color: "var(--text-muted)" }}>
          {pr.checks_summary ?? "Checks unknown"}
        </span>
      </div>

      {/* Reviews-list (Layer 1) for this PR. */}
      <div style={SECTION}>
        <Eyebrow>Reviews</Eyebrow>
        <ReviewsList reviews={reviews} />
      </div>

      {/* D6 diff-stats — honest daemon-gap placeholder (NO fabricated +/−/files/commits). */}
      <div style={SECTION}>
        <Eyebrow>Changes</Eyebrow>
        <div data-testid="pr-diffstats-unavailable" style={PLACEHOLDER}>
          Diff stats (additions / deletions / files / commits) are unavailable — they need the daemon’s
          PR diff-stats capture (D6). No numbers are shown rather than fabricated ones.
        </div>
        {/* D7 PR code-diff — honest placeholder naming the missing RPC; never get_diff (worktree-scoped). */}
        <div
          data-testid="pr-diff-unavailable"
          style={{ ...PLACEHOLDER, display: "flex", alignItems: "center", gap: 8 }}
        >
          <span aria-hidden="true" style={{ display: "inline-flex", color: "var(--text-faint)" }}>
            <FileDiff size={15} />
          </span>
          The PR code-diff is unavailable — it needs a daemon <code>get_pr_diff(repo_id, pr_number)</code>{" "}
          RPC (the worktree-scoped <code>get_diff</code> is not a PR diff; D7).
        </div>
      </div>

      {/* DISABLED mutation + Brain controls (future cat-1 arc + the deferred Brain sibling). */}
      <div style={{ display: "flex", gap: 8, marginTop: 16, flexWrap: "wrap" }}>
        <span title="Merge is a Gateway mutation — a future cat-1 approval arc (requires sign-off)">
          <Button variant="primary" size="sm" icon={<GitMerge size={13} />} disabled>
            Merge
          </Button>
        </span>
        <span title="Approve PR is a Gateway mutation — a future cat-1 approval arc">
          <Button variant="secondary" size="sm" disabled>
            Approve PR
          </Button>
        </span>
        <span title="Request changes is a Gateway mutation — a future cat-1 approval arc">
          <Button variant="secondary" size="sm" disabled>
            Request changes
          </Button>
        </span>
        <span title="Brain co-pilot arrives with the sidecar contract (Phase 8)">
          <Button variant="brain" size="sm" icon={<Brain size={13} />} disabled>
            Ask Brain
          </Button>
        </span>
      </div>
    </div>
  );
}
