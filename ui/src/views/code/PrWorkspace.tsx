import { useState, type CSSProperties } from "react";
import { ArrowLeft, Brain, Check, GitMerge, MessageSquare, XCircle } from "lucide-react";
import type {
  ActionAck,
  DiffLine,
  DiffResult,
  PullRequestRow,
  ReviewRow,
} from "../../contracts/index";
import { Button, DiffHunk, MetaChip } from "../../design-system/kit";
import { StatusPill } from "../../status/StatusPill";
import { Eyebrow } from "../cockpit";
import { ReviewsList } from "./ReviewsList";
import { ResultNotice } from "../../overlays/GatewayModal";
import type { IntentResult } from "../../intent/submit-intent";
import type { ReviewEvent } from "../../intent/pr-mutation-request";

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

const DIFFSTATS_ROW: CSSProperties = {
  display: "flex",
  gap: 14,
  flexWrap: "wrap",
  alignItems: "center",
  font: "var(--fs-meta) var(--font-sans)",
};

const DELTA: CSSProperties = { display: "inline-flex", alignItems: "center", gap: 4 };

/** D6 diff-stats — the real PR-card stats from the frozen `PullRequestRow`. Null-safe per LESSON §32:
 *  the guard is `!= null` (a PRESENT `0` is a real stat, NEVER hidden — never `!x` / `x || …`); each
 *  absent field is omitted; ALL four null → an honest "unavailable" state (a pre-D6 / unsynced row),
 *  never fabricated numbers. The +/− deltas carry a glyph + a text LABEL (never color alone — forbidden #5). */
function DiffStats({ pr }: { pr: PullRequestRow }) {
  const { additions, deletions, changed_files, commits } = pr;
  if (additions == null && deletions == null && changed_files == null && commits == null) {
    return (
      <div data-testid="pr-diffstats-empty" style={PLACEHOLDER}>
        Diff stats are unavailable for this PR — the daemon hasn’t captured them yet (an unsynced or
        pre-capture row). No numbers are fabricated.
      </div>
    );
  }
  return (
    <div data-testid="pr-diffstats" style={DIFFSTATS_ROW}>
      {additions != null && (
        <span style={{ ...DELTA, color: "var(--success-ink)" }}>
          <span aria-hidden="true">+</span>
          {additions} additions
        </span>
      )}
      {deletions != null && (
        <span style={{ ...DELTA, color: "var(--danger-ink)" }}>
          <span aria-hidden="true">−</span>
          {deletions} deletions
        </span>
      )}
      {changed_files != null && (
        <span style={{ color: "var(--text-secondary)" }}>
          {changed_files} {changed_files === 1 ? "file" : "files"}
        </span>
      )}
      {commits != null && (
        <span style={{ color: "var(--text-secondary)" }}>
          {commits} {commits === 1 ? "commit" : "commits"}
        </span>
      )}
    </div>
  );
}

/** The read-only PR code-diff state DiffReview computes (the get_pr_diff fetch) and passes down. A
 *  discriminated union — no silent default (the §15 discriminator discipline): `no-link` (null
 *  repo_id/pr_number → don't fetch) · `loading` · `ready{diff}` · `error{code?}`. */
export type PrDiffState =
  | { kind: "no-link" }
  | { kind: "loading" }
  | { kind: "ready"; diff: DiffResult }
  | { kind: "error"; code?: string };

/** DiffLineKind → the kit DiffHunk line `type` (mirrors DiffReview's map; the same frozen DiffLine). */
const KIT_LINE_TYPE: Record<DiffLine["kind"], "ctx" | "add" | "del"> = {
  context: "ctx",
  added: "add",
  removed: "del",
};

/** D7 — the read-only PR code-diff (head-vs-base), reusing the kit DiffHunk render WITHOUT the per-hunk
 *  git-action bar (PR-per-hunk actions are a future cat-1; `HunkGitActions` is worktree-scoped). Honest
 *  states for no-link / loading / error — never a fabricated diff (forbidden #2). The flattened changeset
 *  carries no per-file attribution (`file=""` — no misleading filename); a per-file file-tree is a
 *  post-D7 follow-on. */
function PrCodeDiff({ state }: { state: PrDiffState }) {
  if (state.kind === "no-link")
    return (
      <div data-testid="pr-diff-no-link" style={PLACEHOLDER}>
        This PR has no linked repository / PR number — there is no code diff to show.
      </div>
    );
  if (state.kind === "loading")
    return (
      <div data-testid="pr-diff-loading" style={PLACEHOLDER}>
        Loading the PR diff…
      </div>
    );
  if (state.kind === "error")
    return (
      <div data-testid="pr-diff-unavailable" style={PLACEHOLDER}>
        PR diff unavailable{state.code ? ` (the daemon reported ${state.code})` : ""} — no changes
        are shown.
      </div>
    );
  if (state.diff.hunks.length === 0)
    return (
      <div data-testid="pr-diff-no-changes" style={PLACEHOLDER}>
        No changes in this PR.
      </div>
    );
  return (
    <div data-testid="pr-diff" style={{ display: "flex", flexDirection: "column", gap: 14 }}>
      {state.diff.hunks.map((hunk, i) => (
        <DiffHunk
          key={`${hunk.old_start}:${hunk.new_start}:${i}`}
          file=""
          header={hunk.header}
          lines={hunk.lines.map((l) => ({ type: KIT_LINE_TYPE[l.kind], text: l.content }))}
          actions={false}
        />
      ))}
    </div>
  );
}

/** The displayed head SHA the Merge intent pins ([[19]]/D2 — submitted==displayed; the anti-race pin
 *  the daemon 409s against). Returns `pr.head_sha` once the daemon D-head_sha freeze + a follow-up
 *  PullRequestRow shadow-reconcile add the field (A3 parallel work, routed cross-track); until then the
 *  field is absent → null → the Merge control stays disabled. Read via a tolerant cast (NO shadow change
 *  this slice) so it works forward-compatibly the moment the daemon populates it. */
export function prHeadSha(pr: PullRequestRow): string | null {
  return (pr as { head_sha?: string | null }).head_sha ?? null;
}

/**
 * The read-only PR Review Workspace (ui-064/068/069/070, §11.2) — the PR-detail panel for a selected PR.
 * Renders the parts backed by the frozen `PullRequestRow` + `ReviewRow` (header, mergeability, checks,
 * reviews-list), the real D6 diff-stats (ui-068), and the read-only D7 PR code-diff (ui-069 — passed in
 * via `prDiff`, never `get_diff`-as-PR-diff; this component takes NO gateway, so it cannot reach a fetch
 * or a mutation by construction). The cat-1 Merge (ui-070) + Review (ui-071: Approve / Request-changes /
 * Comment) controls are container-driven: `canMerge`/`canReview` (computed enablement) + `onMerge`/
 * `onSubmitReview(event, body)` (DiffReview forms + submits the github.merge_pr / github.submit_review
 * intent) + a SHARED `mutationResult` / `mutationEnrichFailed` honest-outcome region + `onReReview` —
 * PrWorkspace stays pure-display. Brain renders DISABLED. The review `body` is a PrWorkspace-local
 * <textarea> raised on submit (per-verdict required: approve optional, request_changes/comment non-empty).
 * The "← Worktree diff" deselect returns to the 6.3e per-hunk view.
 */
export function PrWorkspace({
  pr,
  reviews,
  onBack,
  prDiff,
  canMerge,
  onMerge,
  canReview,
  onSubmitReview,
  mutationResult,
  mutationEnrichFailed,
  onReReview,
}: {
  pr: PullRequestRow;
  reviews: ReviewRow[];
  /** Deselect → return to the 6.3e worktree per-hunk diff. Always provided (the workspace is only shown
   *  in place of the worktree diff, so a way back is mandatory) — a wired control, never a dead click. */
  onBack: () => void;
  /** The D7 PR code-diff state, computed + owned by DiffReview (the container fetches get_pr_diff). */
  prDiff: PrDiffState;
  /** cat-1 (ui-070) — the computed Merge enablement (`canSubmitIntent && isPrMutationEnabled(merge_pr) &&
   *  headSha != null`); the container computes it (the gateway + the hook live there). */
  canMerge: boolean;
  /** Raised on a Merge click (only fires when `canMerge`) — the container forms + submits the
   *  github.merge_pr intent + opens the GatewayModal (PrWorkspace never mutates). */
  onMerge: () => void;
  /** cat-1 (ui-071) — the BASE review enablement (`canSubmitIntent && isPrMutationEnabled(submit_review)
   *  && headSha != null`); the per-verdict body-required gate is applied locally (approve vs the rest). */
  canReview: boolean;
  /** Raised on a verdict click (only fires when that verdict is enabled) with the chosen event + the
   *  trimmed-at-build body — the container forms + submits the github.submit_review intent + opens the modal. */
  onSubmitReview: (event: ReviewEvent, body: string) => void;
  /** A rejected merge/review submit (the §6.4 verdict) to surface honestly via ResultNotice; null when none. */
  mutationResult: IntentResult<ActionAck> | null;
  /** The submit landed but the approval-card enrichment re-fetch failed → an honest notice. */
  mutationEnrichFailed: boolean;
  /** Re-review affordance — re-fetch the latest PR diff (the PR may have moved since review). */
  onReReview: () => void;
}) {
  // The shared review body (ui-071, design-Q3) — a PrWorkspace-local <textarea> raised on submit. The
  // per-verdict required-body gate reads its trimmed value: approve enables with an empty body;
  // request_changes/comment require a non-empty trimmed body (GitHub's 422 rule, daemon LESSON 61).
  // The body INTENTIONALLY persists across a submit (not cleared on click): a rejected/re-review retry
  // keeps the user's text (clearing would lose it on a rejection); a duplicate identical-body resubmit is
  // dedup'd daemon-side (idempotency, daemon LESSON 20) — never a silent double-execute.
  const [reviewBody, setReviewBody] = useState("");
  const bodyNonEmpty = reviewBody.trim().length > 0;
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

      {/* D6 diff-stats summary (ui-068) — real null-safe stats from the frozen row (a present 0 is real;
          all-null → an honest unavailable state; never a fabricated number — LESSON §32). */}
      <div style={SECTION}>
        <Eyebrow>Changes</Eyebrow>
        <DiffStats pr={pr} />
      </div>

      {/* D7 PR code-diff (ui-069) — the real read-only head-vs-base hunks from get_pr_diff, computed by
          DiffReview + passed down (read-only; no per-hunk action bar — PR-per-hunk is a future cat-1). */}
      <div style={SECTION}>
        <Eyebrow>Code diff</Eyebrow>
        <PrCodeDiff state={prDiff} />
      </div>

      {/* The shared review body (ui-071) — the deny-reason-precedent control ([[33]]). Required non-empty
          for Request changes + Comment (GitHub 422s otherwise); optional for Approve. */}
      <div style={SECTION}>
        <Eyebrow>Review</Eyebrow>
        <textarea
          data-testid="pr-review-body"
          value={reviewBody}
          onChange={(e) => setReviewBody(e.target.value)}
          placeholder="Review comment (required for Request changes / Comment)…"
          rows={3}
          style={{
            width: "100%",
            resize: "vertical",
            padding: "8px 10px",
            font: "var(--fs-meta) var(--font-sans)",
            color: "var(--text-primary)",
            background: "var(--surface-sunken)",
            border: "1px solid var(--border-subtle)",
            borderRadius: "var(--r-2)",
          }}
        />
      </div>

      {/* The cat-1 Merge (ui-070) + Review-verdict (ui-071) controls, all GUARDED-DISABLED. Each is enabled
          only when its gate holds (Merge: canMerge; verdicts: canReview AND [approve OR a non-empty body]);
          a click raises onMerge / onSubmitReview(event, body) → the container submits + opens the modal. */}
      <div style={{ display: "flex", gap: 8, marginTop: 16, flexWrap: "wrap" }}>
        <span
          title={
            canMerge
              ? "Merge this PR — pins the displayed head SHA + opens the approval card"
              : "Merge is gated off (needs the PR-mutation go-live enable + a captured head SHA + a live connection)"
          }
        >
          <Button
            variant="primary"
            size="sm"
            icon={<GitMerge size={13} />}
            disabled={!canMerge}
            onClick={onMerge}
          >
            Merge
          </Button>
        </span>
        <span title="Approve this PR — submits an 'approve' review (body optional)">
          <Button
            variant="secondary"
            size="sm"
            icon={<Check size={13} />}
            disabled={!canReview}
            onClick={() => onSubmitReview("approve", reviewBody)}
          >
            Approve PR
          </Button>
        </span>
        <span title="Request changes — submits a 'request_changes' review (a non-empty comment is required)">
          <Button
            variant="secondary"
            size="sm"
            icon={<XCircle size={13} />}
            disabled={!canReview || !bodyNonEmpty}
            onClick={() => onSubmitReview("request_changes", reviewBody)}
          >
            Request changes
          </Button>
        </span>
        <span title="Comment — submits a 'comment' review (a non-empty comment is required)">
          <Button
            variant="secondary"
            size="sm"
            icon={<MessageSquare size={13} />}
            disabled={!canReview || !bodyNonEmpty}
            onClick={() => onSubmitReview("comment", reviewBody)}
          >
            Comment
          </Button>
        </span>
        <span title="Brain co-pilot arrives with the sidecar contract (Phase 8)">
          <Button variant="brain" size="sm" icon={<Brain size={13} />} disabled>
            Ask Brain
          </Button>
        </span>
      </div>

      {/* cat-1 (ui-070/071) — a SHARED honest merge/review-submit outcome. A rejection surfaces the
          daemon's §6.4 code VERBATIM (ResultNotice/describeRejection) + a "PR may have moved — re-review"
          affordance (no fabricated success, no optimistic "merged"/"reviewed" — that lands only via the
          confirming projection fold). The success path opens the GatewayModal (rendered by the container). */}
      {mutationResult ? (
        <div data-testid="pr-mutation-result" style={{ marginTop: 14 }}>
          <ResultNotice result={mutationResult} onReapprove={onReReview} />
          <div style={{ ...PLACEHOLDER, marginTop: 8, display: "flex", alignItems: "center", gap: 10, flexWrap: "wrap" }}>
            <span>
              If the PR moved since you reviewed it, the action is rejected to protect the approved head —
              re-review the latest diff before retrying.
            </span>
            {/* A dedicated re-review affordance — ALWAYS present on a rejection (ResultNotice's reapprove
                button only renders for re-approvable codes; a fencing_conflict has none). */}
            <Button variant="secondary" size="sm" data-testid="pr-mutation-rereview" onClick={onReReview}>
              Re-review
            </Button>
          </div>
        </div>
      ) : null}

      {/* The intent landed but its approval-card preview couldn't load (a malformed/unavailable approval
          queue read) — an honest degrade, never a silent stall: the approval is in the queue. */}
      {mutationEnrichFailed ? (
        <div data-testid="pr-mutation-enrich-unavailable" style={{ ...PLACEHOLDER, marginTop: 14 }}>
          The action was submitted, but its approval preview couldn’t load (the daemon’s approval queue
          read failed). Find it in the approval queue to approve or deny.
        </div>
      ) : null}
    </div>
  );
}
