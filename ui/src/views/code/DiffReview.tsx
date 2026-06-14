import { useEffect, useState } from "react";
import {
  Brain,
  ChevronRight,
  FileCode,
  FolderGit2,
  GitCommitHorizontal,
  GitMerge,
  GitPullRequest,
  Minus,
  Plus,
  Trash2,
} from "lucide-react";
import type {
  ActionAck,
  DiffLine,
  DiffResult,
  Hunk,
  PerHunkGitActionType,
  PullRequestRow,
} from "../../contracts/index";
import { WireError } from "../../contracts/index";
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
import { prDisplayFixture, worktreesFixture, diffReviewContext } from "../display-fixtures";
import type { GatewayPort } from "../../gateway-client/types";
import { useSubmitIntent, type IntentResult } from "../../intent/submit-intent";
import { useCanSubmitIntent } from "../../connection/read-only";
import { buildHunkActionRequest } from "../../intent/hunk-resource-ref";
import {
  enrichHunkAction,
  type GatewayApprovalEnrichment,
} from "../../shell/display-meta";
import { GatewayModal, ResultNotice } from "../../overlays/GatewayModal";

/** DiffLineKind → the kit DiffHunk line `type`. */
const KIT_LINE_TYPE: Record<DiffLine["kind"], "ctx" | "add" | "del"> = {
  context: "ctx",
  added: "add",
  removed: "del",
};

type DiffState =
  | { kind: "loading" }
  | { kind: "ready"; diff: DiffResult }
  | { kind: "error"; code?: string };

/** The per-hunk git-action bar (6.3e, cat-1). stage/unstage/discard are PURE SUBMITTERS
 *  over the seam (Q1); disabled when !canSubmitIntent (Q2 fail-safe); discard is the
 *  destructive (risk-3 daemon-side), explicitly labeled + danger-toned. Each button's
 *  accessible name carries the hunk header (unique per hunk; §11.6). */
function HunkGitActions({
  hunk,
  canSubmit,
  onAction,
}: {
  hunk: Hunk;
  canSubmit: boolean;
  onAction: (actionType: PerHunkGitActionType, hunk: Hunk) => void;
}) {
  return (
    <div
      role="group"
      aria-label={`Hunk actions ${hunk.header}`}
      style={{ display: "flex", gap: 6, padding: "6px 0 2px" }}
    >
      <Button
        size="sm"
        variant="secondary"
        icon={<Plus size={13} />}
        disabled={!canSubmit}
        aria-label={`Stage hunk ${hunk.header}`}
        onClick={() => onAction("git.stage_hunk", hunk)}
      >
        Stage
      </Button>
      <Button
        size="sm"
        variant="ghost"
        icon={<Minus size={13} />}
        disabled={!canSubmit}
        aria-label={`Unstage hunk ${hunk.header}`}
        onClick={() => onAction("git.unstage_hunk", hunk)}
      >
        Unstage
      </Button>
      <Button
        size="sm"
        variant="danger"
        icon={<Trash2 size={13} />}
        disabled={!canSubmit}
        aria-label={`Discard hunk ${hunk.header}`}
        onClick={() => onAction("git.discard_hunk", hunk)}
      >
        Discard
      </Button>
    </div>
  );
}

type Tab = "Review" | "Worktrees" | "Pull requests";

/** Review tab (6.3e, cat-1) — sources the diff from the `get_diff` READ RPC and wires
 *  the per-hunk stage/unstage/discard buttons to the 043/044 intent seam. The UI NEVER
 *  mutates: a click submits a typed `git.*` ActionRequest (resource_ref targets the EXACT
 *  displayed hunk) → the daemon adjudicates → the GatewayModal renders the daemon's
 *  policy/preview → the human approves/denies. No optimistic "done"; a get_diff error/
 *  not_found renders an honest unavailable state (never a fabricated diff, forbidden #2). */
function ReviewTab({ gateway }: { gateway: GatewayPort }) {
  const seam = useSubmitIntent(gateway);
  const canSubmit = useCanSubmitIntent();
  const { worktreeId, file } = diffReviewContext;
  const [state, setState] = useState<DiffState>({ kind: "loading" });
  const [pendingApproval, setPendingApproval] =
    useState<GatewayApprovalEnrichment | null>(null);
  const [submitResult, setSubmitResult] = useState<IntentResult<ActionAck> | null>(null);
  // The submit succeeded but the approval-card enrichment re-fetch (053b) failed → an honest
  // degrade (never a silent stall, §11.7), kept separate from the intent-rejection surface.
  const [enrichFailed, setEnrichFailed] = useState(false);

  // Source the diff LIVE from get_diff (no static fixture). An error/not_found → an
  // honest unavailable state (the code carried verbatim), never a fabricated diff.
  useEffect(() => {
    let active = true;
    setState({ kind: "loading" });
    gateway.get_diff(worktreeId, file).then(
      (diff) => {
        if (active) setState({ kind: "ready", diff });
      },
      (e: unknown) => {
        if (!active) return;
        const parsed = WireError.safeParse(e);
        if (parsed.success) {
          // A daemon-reported read error (e.g. not_found) — honest unavailable, code verbatim.
          setState({ kind: "error", code: parsed.data.code });
        } else {
          // A non-WireError (a real transport/JS Error) — DEGRADE honestly (a read failure
          // must not crash the cockpit, §11.7; re-throw would be an unhandled rejection) but
          // surface the unexpected failure for diagnosis: never SILENTLY swallow a bug
          // (LESSON §16's spirit, adapted to a READ — degrade + log, the GatewayModal-preview
          // read pattern, not the mutation seam's re-throw).
          console.error("get_diff failed unexpectedly", e);
          setState({ kind: "error" });
        }
      },
    );
    return () => {
      active = false;
    };
  }, [gateway, worktreeId, file]);

  // A per-hunk button → a typed ActionRequest over the seam (Q1 pure submitter). On a
  // daemon-recorded ack, open the approval card (Q3 — its daemon-reported pending status,
  // never optimistic "done"); a rejection routes through ResultNotice → describeRejection.
  async function onAction(actionType: PerHunkGitActionType, hunk: Hunk) {
    const request = buildHunkActionRequest(
      actionType,
      worktreeId,
      file,
      hunk,
      new Date().toISOString(),
    );
    const r = await seam.submitAction(request);
    if ("ok" in r) {
      setSubmitResult(null);
      setEnrichFailed(false);
      try {
        // 053b: source the daemon's REAL risk/policy by re-fetching the ApprovalQueue + matching the
        // minted action_request_id (no UI-derived fixture); absent → an honest awaiting placeholder.
        setPendingApproval(await enrichHunkAction(gateway, r.ok));
      } catch (e) {
        // The intent WAS recorded (the daemon acked); only the approval-card enrichment re-fetch
        // failed — a malformed ApprovalQueue payload (BoundaryValidationError) or a transport fault.
        // Degrade HONESTLY (the get_diff read-degrade pattern above, §11.7): log + an honest notice,
        // NEVER a silent stall and NEVER a card built from un-parsed data (forbidden #2). The approval
        // is still in the global queue; a read failure must not crash the cockpit (no re-throw).
        console.error("enrichHunkAction (ApprovalQueue re-fetch) failed", e);
        setEnrichFailed(true);
      }
    } else {
      setSubmitResult(r);
    }
  }

  return (
    <div style={{ height: "100%", overflowY: "auto", padding: "14px 16px", background: "var(--surface-canvas)" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 10 }}>
        <span aria-hidden="true" style={{ color: "var(--text-faint)", display: "inline-flex" }}>
          <FileCode size={15} />
        </span>
        <h1 style={{ margin: 0, font: "var(--fw-semibold) var(--fs-h3)/1 var(--font-mono)" }}>
          {file}
        </h1>
        <Badge mono style={{ color: "var(--text-faint)" }}>
          changed-files list pending worktree projection
        </Badge>
        <div style={{ marginLeft: "auto", display: "flex", gap: 6 }}>
          <span title="Brain co-pilot arrives with the sidecar contract (Phase 8)">
            <Button variant="secondary" size="sm" icon={<Brain size={14} />} disabled>
              Ask Brain
            </Button>
          </span>
        </div>
      </div>

      {submitResult ? (
        <div style={{ marginBottom: 12 }}>
          <ResultNotice result={submitResult} onReapprove={() => setSubmitResult(null)} />
        </div>
      ) : null}

      {enrichFailed ? (
        <div data-testid="enrich-unavailable" style={{ marginBottom: 12 }}>
          <div style={NOTE}>
            The action was submitted, but its approval preview couldn’t load (the daemon’s approval
            queue read failed). Find it in the approval queue to approve or deny.
          </div>
        </div>
      ) : null}

      {state.kind === "loading" ? (
        <div data-testid="diff-loading" style={NOTE}>
          Loading the diff…
        </div>
      ) : state.kind === "error" ? (
        <div data-testid="diff-unavailable" style={NOTE}>
          Diff unavailable{state.code ? ` (the daemon reported ${state.code})` : ""} — no
          changes are shown.
        </div>
      ) : state.diff.hunks.length === 0 ? (
        <div data-testid="diff-no-changes" style={NOTE}>
          No changes in this file.
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
          {state.diff.hunks.map((hunk, i) => (
            // key on the hunk position identity (unique per hunk in a diff — the same
            // identity the resource_ref encodes), stable across get_diff re-reads.
            <div key={`${hunk.old_start}:${hunk.new_start}:${i}`}>
              <DiffHunk
                file={file}
                header={hunk.header}
                lines={hunk.lines.map((l) => ({ type: KIT_LINE_TYPE[l.kind], text: l.content }))}
                actions={false}
              />
              <HunkGitActions hunk={hunk} canSubmit={canSubmit} onAction={onAction} />
            </div>
          ))}
        </div>
      )}

      {pendingApproval ? (
        <GatewayModal
          approval={pendingApproval.approval}
          policyDecision={pendingApproval.policyDecision}
          port={gateway}
          onClose={() => setPendingApproval(null)}
        />
      ) : null}
    </div>
  );
}

const NOTE: React.CSSProperties = {
  font: "var(--fs-label)/1.5 var(--font-sans)",
  color: "var(--text-muted)",
  padding: "12px 14px",
  border: "1px dashed var(--border-subtle)",
  borderRadius: "var(--r-2)",
};

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
export function DiffReview({
  prs,
  gateway,
}: {
  prs: PullRequestRow[];
  gateway: GatewayPort;
}) {
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
        {tab === "Review" ? <ReviewTab gateway={gateway} /> : tab === "Worktrees" ? <WorktreesTab /> : <PRsTab prs={prs} />}
      </div>
    </div>
  );
}
