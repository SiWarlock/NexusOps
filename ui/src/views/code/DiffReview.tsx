import { useEffect, useMemo, useState } from "react";
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
  ActionRequest,
  DiffLine,
  DiffResult,
  Hunk,
  PerHunkGitActionType,
  PullRequestRow,
  ReviewRow,
} from "../../contracts/index";
import { reviewsByPr } from "../../projections/items";
import { PrWorkspace, prHeadSha, type PrDiffState } from "./PrWorkspace";
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
  buildMergePrActionRequest,
  buildSubmitReviewActionRequest,
  isPrMutationEnabled,
  type MergePrMethod,
  type ReviewEvent,
} from "../../intent/pr-mutation-request";
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
  // The effective per-hunk submit gate (056/L2-B): a live link AND the L2 go-live enable. While
  // `mutationsEnabled` is false (the L2-B honest disabled state), the per-hunk buttons stay disabled
  // — L2-B enables nothing in the UI (the go-live is the USER-gated L2-C single flip).
  const canSubmit = useCanSubmitIntent() && gateway.mutationsEnabled;
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
 *  branch / age ride the display side-map (projection enrichment flagged). A card's
 *  header is a SELECTING button (keyboard-reachable; carries the `data-item-id`) → it
 *  opens the PR Review Workspace (ui-064); the disabled Merge button is a SIBLING (no
 *  nested-interactive — WAI-ARIA). */
function PRsTab({
  prs,
  onSelect,
}: {
  prs: PullRequestRow[];
  onSelect: (prId: string) => void;
}) {
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
                  // pr_number is now a nullable u64 (ui-061 reconcile); the display side-map is
                  // keyed by the string form. Key/locator on the PK pr_id (the toPrItems precedent).
                  const d = prDisplayFixture[String(p.pr_number ?? "")];
                  return (
                    <div
                      key={p.pr_id}
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
                      {/* The selecting button — opens the PR Review Workspace (ui-064). Carries the
                          data-item-id locator; keyboard-reachable; the Merge button is a sibling below. */}
                      <button
                        type="button"
                        data-item-id={`PullRequest:${p.pr_id}`}
                        onClick={() => onSelect(p.pr_id)}
                        style={{
                          display: "flex",
                          flexDirection: "column",
                          gap: 8,
                          width: "100%",
                          padding: 0,
                          border: "none",
                          background: "transparent",
                          cursor: "pointer",
                          textAlign: "left",
                        }}
                      >
                        <div style={{ display: "flex", alignItems: "center", gap: 7 }}>
                          {/* null-safe: a tone="pr" chip is a "#<number>" badge — omit it entirely
                              when pr_number is null (never a bare "#"); the label below carries the
                              pr_id identity (the :485 fallback). Mirrors the label's null-safety. */}
                          {p.pr_number != null ? <MetaChip tone="pr">#{p.pr_number}</MetaChip> : null}
                          <StatusPill machine="PullRequest" status={p.status} size="xs" />
                          {d ? (
                            <span style={{ marginLeft: "auto", font: "var(--fs-micro) var(--font-mono)", color: "var(--text-faint)" }}>
                              {d.age}
                            </span>
                          ) : null}
                        </div>
                        <div style={{ font: "var(--fw-medium) var(--fs-label)/1.3 var(--font-sans)", color: "var(--text-primary)" }}>
                          {/* null-safe label: title → `PR #<n>` → the always-present pr_id PK (toPrItems parity) */}
                          {p.title ?? (p.pr_number != null ? `PR #${p.pr_number}` : p.pr_id)}
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
                        {d ? <MetaChip tone="branch">{d.branch}</MetaChip> : null}
                      </button>
                      {lane === "ready" ? (
                        <div style={{ display: "flex" }}>
                          <span
                            title="Merge is a Gateway mutation (intent seam — daemon-gated)"
                            style={{ marginLeft: "auto" }}
                          >
                            <Button variant="primary" size="sm" icon={<GitMerge size={12} />} disabled>
                              Merge
                            </Button>
                          </span>
                        </div>
                      ) : null}
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

/** ui-069 D7 + ui-070 cat-1 — owns the `get_pr_diff` fetch for the selected PR AND the github.merge_pr
 *  submit, feeding the pure-display PrWorkspace. The diff fetch is keyed on the STABLE (repo_id, pr_number)
 *  primitives (LESSON §17) — and the render site remounts it per `pr_id` — so a reselect re-fetches and
 *  NEVER shows a stale diff. PrWorkspace takes NO gateway (the ui-064 no-mutation-reach pin: a read-only
 *  display can't reach a fetch/submit by construction). The cat-1 Merge is GUARDED-DISABLED: enabled only
 *  when canSubmitIntent && prMutationsEnabled && headSha != null — prMutationsEnabled defaults false +
 *  headSha is null until the daemon field lands → no production path reaches a live merge. */
function PrWorkspaceContainer({
  gateway,
  pr,
  reviews,
  onBack,
}: {
  gateway: GatewayPort;
  pr: PullRequestRow;
  reviews: ReviewRow[];
  onBack: () => void;
}) {
  const { repo_id, pr_number } = pr;
  const [prDiff, setPrDiff] = useState<PrDiffState>({ kind: "loading" });
  const [refreshTick, setRefreshTick] = useState(0);
  const seam = useSubmitIntent(gateway);
  const canSubmit = useCanSubmitIntent();
  const [pendingApproval, setPendingApproval] = useState<GatewayApprovalEnrichment | null>(null);
  // SHARED across both cat-1 PR mutations (merge + review) — one honest outcome region per workspace.
  const [mutationResult, setMutationResult] = useState<IntentResult<ActionAck> | null>(null);
  const [mutationEnrichFailed, setMutationEnrichFailed] = useState(false);

  useEffect(() => {
    if (repo_id == null || pr_number == null) {
      // no repo link / PR number → don't fetch; an honest no-link state (distinct from a daemon error).
      setPrDiff({ kind: "no-link" });
      return;
    }
    let active = true;
    setPrDiff({ kind: "loading" });
    gateway.get_pr_diff(repo_id, pr_number, null).then(
      (diff) => {
        if (active) setPrDiff({ kind: "ready", diff });
      },
      (e: unknown) => {
        if (!active) return;
        const parsed = WireError.safeParse(e);
        if (parsed.success) {
          // a daemon-reported read error (e.g. not_found) — honest unavailable, code verbatim.
          setPrDiff({ kind: "error", code: parsed.data.code });
        } else {
          // a non-WireError (a real transport/JS Error) — degrade honestly + surface for diagnosis
          // (never silently swallow), mirroring the ReviewTab get_diff read-degrade (§11.7/LESSON §16).
          console.error("get_pr_diff failed unexpectedly", e);
          setPrDiff({ kind: "error" });
        }
      },
    );
    return () => {
      active = false;
    };
  }, [gateway, repo_id, pr_number, refreshTick]);

  // cat-1 defense-in-depth layer 1: the Merge enablement. isPrMutationEnabled(github.merge_pr) is the
  // per-action PR-mutation go-live gate (empty set in prod, SEPARATE from L2 mutationsEnabled); headSha
  // is the anti-race pin (null until the daemon field lands → disabled today); canSubmit is the live-link
  // gate. The port's throw-never-invoke guard is the provably-unreachable layer beneath this.
  const headSha = prHeadSha(pr);
  const canMerge =
    canSubmit && isPrMutationEnabled(gateway, "github.merge_pr") && headSha != null;
  // The BASE review enablement (the per-verdict body-required gate is applied in PrWorkspace).
  const canReview =
    canSubmit && isPrMutationEnabled(gateway, "github.submit_review") && headSha != null;

  // SHARED submit path for both cat-1 PR mutations — the daemon Gateway is the single executor (the UI
  // submits an intent). On ok → open the GatewayModal for the daemon's REAL policy/preview/approve-deny
  // (no optimistic "merged"/"reviewed"; the terminal state lands only via the confirming projection fold);
  // on a rejection → surface the §6.4 verdict VERBATIM (ResultNotice/describeRejection) + the honest
  // re-review affordance; on an enrich re-fetch fault → an honest degrade (never a card from un-parsed data).
  async function submitMutation(req: ActionRequest) {
    const r = await seam.submitAction(req);
    if ("ok" in r) {
      setMutationResult(null);
      setMutationEnrichFailed(false);
      try {
        setPendingApproval(await enrichHunkAction(gateway, r.ok));
      } catch (e) {
        console.error("approval enrich (ApprovalQueue re-fetch) failed", e);
        setMutationEnrichFailed(true);
      }
    } else {
      setMutationResult(r);
    }
  }

  async function onMerge(method: MergePrMethod) {
    // structural guard (the control is disabled when any is missing — belt-and-suspenders, never a
    // merge formed without a pinned head / repo identity). ui-077: the user-selected method rides into
    // inputs.merge_method (the daemon's map_merge_method is the fail-closed authority; every merge stays
    // per-action approved/audited regardless of method).
    if (repo_id == null || pr_number == null || headSha == null) return;
    await submitMutation(
      buildMergePrActionRequest(
        { repo_id, pr_number, head_sha: headSha, merge_method: method },
        new Date().toISOString(),
      ),
    );
  }

  async function onSubmitReview(event: ReviewEvent, body: string) {
    // structural guard (the verdict controls are disabled when any is missing — belt-and-suspenders).
    if (repo_id == null || pr_number == null || headSha == null) return;
    await submitMutation(
      buildSubmitReviewActionRequest(
        { repo_id, pr_number, head_sha: headSha, event, body },
        new Date().toISOString(),
      ),
    );
  }

  function onReReview() {
    setMutationResult(null);
    setMutationEnrichFailed(false); // clear BOTH honest-degrade notices on a re-review
    setRefreshTick((t) => t + 1); // re-fetch the latest PR diff so the user re-reviews the current head
  }

  return (
    <>
      <PrWorkspace
        pr={pr}
        reviews={reviews}
        onBack={onBack}
        prDiff={prDiff}
        canMerge={canMerge}
        onMerge={onMerge}
        canReview={canReview}
        onSubmitReview={onSubmitReview}
        mutationResult={mutationResult}
        mutationEnrichFailed={mutationEnrichFailed}
        onReReview={onReReview}
      />
      {pendingApproval ? (
        <GatewayModal
          approval={pendingApproval.approval}
          policyDecision={pendingApproval.policyDecision}
          port={gateway}
          onClose={() => setPendingApproval(null)}
        />
      ) : null}
    </>
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
  reviews,
  gateway,
}: {
  prs: PullRequestRow[];
  reviews: ReviewRow[];
  gateway: GatewayPort;
}) {
  const [tab, setTab] = useState<Tab>("Review");
  // The PR selected from the Kanban → the Review tab shows its PR Workspace (ui-064, §11.2). Pure UI
  // selection state (LESSON §13 family — mirrors selectedSessionId); a stale id (the PR left the set)
  // re-resolves to none (no ghost), falling back to the 6.3e worktree per-hunk diff.
  const [selectedPrId, setSelectedPrId] = useState<string | null>(null);
  const selectedPr =
    selectedPrId != null ? prs.find((p) => p.pr_id === selectedPrId) : undefined;
  // Build the pr_number→reviews join once per `reviews` change (not per render — mirrors the memoized
  // item-builder pattern), so a tab keystroke doesn't re-group the whole set.
  const reviewsByPrNumber = useMemo(() => reviewsByPr(reviews), [reviews]);
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
        {tab === "Review" ? (
          // A selected PR → its read-only PR Review Workspace (ui-064); else the 6.3e worktree per-hunk
          // diff (preserved, not deleted). Reviews join to the PR client-side on pr_number.
          selectedPr ? (
            <PrWorkspaceContainer
              key={selectedPr.pr_id}
              gateway={gateway}
              pr={selectedPr}
              reviews={
                selectedPr.pr_number != null
                  ? (reviewsByPrNumber.get(selectedPr.pr_number) ?? [])
                  : []
              }
              onBack={() => setSelectedPrId(null)}
            />
          ) : (
            <ReviewTab gateway={gateway} />
          )
        ) : tab === "Worktrees" ? (
          <WorktreesTab />
        ) : (
          <PRsTab
            prs={prs}
            onSelect={(id) => {
              setSelectedPrId(id);
              setTab("Review");
            }}
          />
        )}
      </div>
    </div>
  );
}
