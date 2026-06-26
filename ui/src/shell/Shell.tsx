import { useEffect, useState } from "react";
import type { GatewayPort } from "../gateway-client/types";
import { UdsGatewayPort } from "../gateway-client/uds";
import { PR_MUTATION_ACTION_TYPES } from "../intent/pr-mutation-request";
import { runSubscriptionSupervisor } from "../gateway-client/subscribe-recovery";
import { createNudgeCoalescer } from "../gateway-client/refetch-on-nudge";
import type {
  ApprovalQueueRow,
  AuditEventRow,
  Capabilities,
  CreditPool,
  ProjectActivityRow,
  ProjectionName,
  ProjectionPageByName,
  PullRequestRow,
  RecoveryStatus,
  ReviewRow,
  SafetyState,
  SessionRow,
  UsageRow,
} from "../contracts/index";
import {
  deriveProjectSwitcherCounts,
  pendingApprovals,
  waitingSessions,
  type ProjectSwitcherCounts,
} from "./derive";
import { headerProjectLabel, projectLabel } from "./project-label";
import { CommandCenter } from "../views/command/CommandCenter";
import { ProjectGraph } from "../views/graph/ProjectGraph";
import { Settings } from "../views/settings/Settings";
import { ProjectsOverviewContainer } from "../views/projects/ProjectsOverviewContainer";
import { AuditTrail } from "../views/audit/AuditTrail";
import { SessionTerminal } from "../views/terminal/SessionTerminal";
import { DiffReview } from "../views/code/DiffReview";
import { PlanView } from "../views/plan/PlanView";
import { EditorView } from "../views/editor/EditorView";
import { AgentTeamView } from "../views/team/AgentTeamView";
import { WorkflowPacksView } from "../views/packs/WorkflowPacksView";
import { BrainPage } from "../views/brain/BrainPage";
import { ReadOnlyProvider, type ConnectionStatus } from "../connection/read-only";
import {
  checkVersionCompat,
  deriveDegradedState,
  type VersionCompat,
} from "../connection/version";
import type { ConnectionState } from "../connection/state";
import { DegradedBanner } from "../connection/DegradedBanner";
import { RecoveryBanner } from "../recovery/RecoveryBanner";
import { resumeModesBySessionId } from "../recovery/model";
import { recoveryStatusFixture } from "../recovery/fixtures";
import { HardConflictCard } from "../safety/HardConflictCard";
import { AuditIntegrityAlert } from "../safety/AuditIntegrityAlert";
import { safetyCleanFixture } from "../safety/fixtures";
import {
  resolveActiveProject,
  filterByActiveProject,
  ActiveProjectProvider,
} from "./active-project";
import { sessionDisplayFixture } from "./display-meta";
import { useViewHistory } from "./view-history";
import { TopBar } from "./TopBar";
import { Sidebar } from "./Sidebar";
import { EventDock } from "./EventDock";
import { CommandPalette, type PaletteAction } from "../overlays/CommandPalette";
import { HumanInputQueue } from "../overlays/HumanInputQueue";
import { TaskInbox } from "../overlays/TaskInbox";
import { GatewayOverlay } from "../overlays/GatewayOverlay";
import { BrainDrawer } from "../overlays/BrainDrawer";
import { BrainStatusProvider, fakeBrainStatus } from "../views/brain/brain-status";
import { InspectorDrawer } from "../overlays/InspectorDrawer";
import type { GraphNode } from "../views/graph/model";

/** Which overlay surface is open (one at a time — prototype behavior). */
type OverlayState =
  | { kind: "palette" }
  | { kind: "hiq" }
  | { kind: "tasks" }
  | { kind: "brain" }
  | { kind: "gateway"; approval: ApprovalQueueRow }
  | { kind: "inspect"; node: GraphNode }
  | null;

interface ShellData {
  projects: ProjectActivityRow[];
  counts: Record<string, ProjectSwitcherCounts>;
  events: AuditEventRow[];
  sessions: SessionRow[];
  pullRequests: PullRequestRow[];
  approvals: ApprovalQueueRow[];
  usage: UsageRow[];
  creditPool: CreditPool | null;
  // The PR-review verticals (ui-064, §11.2) — the Review projection joined to a PR client-side on
  // pr_number (reviewsByPr) for the PR Review Workspace.
  reviews: ReviewRow[];
  // ui-079 (§11.7) — the projections that FAILED to load this cockpit-load (per-projection-resilient):
  // each degrades only its own slice (rendered []), surfaced honestly by the partial-data banner — never
  // silently shown as genuinely-empty. Empty set = a fully-successful (or no-degrade) load.
  degraded: ReadonlySet<ProjectionName>;
}

// Bounded backoff for the live-subscribe reconnect recovery (052) — don't hammer the daemon on a
// flapping connection; cap the wait so recovery stays responsive.
const SUBSCRIBE_BACKOFF_BASE_MS = 500;
const SUBSCRIBE_BACKOFF_MAX_MS = 30_000;

/**
 * The top-level app shell — a projection-driven reattaching client (§11). Reads
 * projections ONLY through the gateway-client boundary (validated payloads) and
 * renders the chrome from them. It also surfaces the transport degraded state:
 * a ReadOnlyProvider exposes connected+version-compatible to every control's
 * canSubmitIntent gate (fail-safe FALSE until confirmed), a ConnectionIndicator
 * sits in the EventDock strip, and a DegradedBanner appears when disconnected /
 * reconnecting / version-skewed. The daemon Gateway remains the real INV-SEC-1
 * guard; this read-only gate is defense-in-depth.
 *
 * Chrome anatomy is the prototype's (kit-shell.jsx): TopBar, the workspace
 * Sidebar (project tree + view nav), the main surface routed by the sidebar
 * nav (view-history back/forward), and the bottom EventDock.
 */
export function Shell({
  gateway,
  // O-2 survival display (6.4d): fixture-driven (recovered = non-intrusive) until
  // the daemon survival-schema integration supplies real recovery state.
  recovery = recoveryStatusFixture,
  // §17 safety-state display (6.4d-2): fixture-driven (clean = non-intrusive) until
  // the daemon §17/failure-mode integration supplies real safety state.
  safety = safetyCleanFixture,
}: {
  gateway?: GatewayPort;
  recovery?: RecoveryStatus;
  safety?: SafetyState;
}) {
  // Stable client across renders (a fresh default per render would loop the effect).
  // PRODUCTION DEFAULT = the real UdsGatewayPort (L1 read-swap, 051). **L2-C GO-LIVE (057,
  // USER-signed-off):** constructed `mutationsEnabled: true` — the single switch that lights up the
  // mutation transport (the port methods `invoke`) AND the UI submit controls (`canSubmitIntent &&
  // mutationsEnabled`) together. **ui-075 PR-MUTATIONS GO-LIVE (cat-1, USER-signed-off + visual-gate
  // PASSED):** the constructor below is also passed `PR_MUTATION_ACTION_TYPES` as its per-action
  // PR-mutation gate — lighting up BOTH PR mutations (`github.merge_pr` + `github.submit_review`) at once.
  // (The literal option form is left to the code below ONLY — never duplicated in this comment — so the
  // `pr_mutation_flip_confined_to_the_signed_off_shell_go_live` source-grep guard matches the real flip,
  // not prose.) A real human can now merge a PR / submit a review from the cockpit; the daemon's Action
  // Gateway executes + audits it (INV-SEC-1 chokepoint; this UI gate is defense-in-depth,
  // necessary-not-sufficient — live writes also require the daemon-side per-connection `live_writes_enabled`
  // toggle ON [default OFF, 083]). The MockGatewayPort stays the injectable test/dev seam (via `gateway`).
  const [client] = useState<GatewayPort>(
    () =>
      gateway ??
      new UdsGatewayPort({
        mutationsEnabled: true,
        enabledPrMutations: PR_MUTATION_ACTION_TYPES,
      }),
  );
  const [data, setData] = useState<ShellData | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [connection, setConnection] = useState<ConnectionState>(() =>
    client.getConnectionState(),
  );
  // Fail-safe: version stays "unknown" (→ read-only) until a handshake confirms it.
  const [version, setVersion] = useState<VersionCompat>("unknown");
  // Which content view the main surface shows. Command Center is the default;
  // back/forward navigate the view history (§11.2 — pure UI state, no daemon dep,
  // Lesson §13 family). `navigate` is the single nav entry point.
  const {
    current: contentView,
    canBack,
    canForward,
    navigate,
    back,
    forward,
  } = useViewHistory();
  // Active-project selection (P7.3): UI scope state over the frozen projects
  // projection. null until the user picks; defaults to the first project (below).
  const [rawActiveProjectId, setActiveProject] = useState<string | null>(null);
  // The session the Session Terminal view targets (sidebar tree click — pure UI
  // selection state, Lesson §13 family).
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  // The open overlay surface (palette / HIQ / tasks / brain / gateway / inspector).
  const [overlay, setOverlay] = useState<OverlayState>(null);

  useEffect(() => client.onConnectionChange(setConnection), [client]);

  // Global shortcuts (prototype bindings): ⌘K palette · ⌘⇧P tasks · ⌘⇧H queue.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return;
      const key = e.key.toLowerCase();
      if (key === "k" && !e.shiftKey) {
        e.preventDefault();
        setOverlay((o) => (o?.kind === "palette" ? null : { kind: "palette" }));
      } else if (e.shiftKey && key === "p") {
        e.preventDefault();
        setOverlay((o) => (o?.kind === "tasks" ? null : { kind: "tasks" }));
      } else if (e.shiftKey && key === "h") {
        e.preventDefault();
        setOverlay((o) => (o?.kind === "hiq" ? null : { kind: "hiq" }));
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      // ui-079 (§6.11/§11.7) — PER-PROJECTION-RESILIENT load: each of the 7 projections settles
      // independently (a typed per-call wrapper → `page | null`, cleaner than allSettled's union-narrowing
      // over the heterogeneous ProjectionPageByName[K]). One projection rejecting degrades ONLY its tile
      // (rendered [] + named in the honest partial-data banner), never blanks the whole cockpit.
      const settle = async <K extends ProjectionName>(
        name: K,
      ): Promise<ProjectionPageByName[K] | null> => {
        try {
          return await client.get_projection(name);
        } catch (e) {
          console.error(`cockpit load: the ${name} projection failed to load (degraded)`, e);
          return null;
        }
      };
      const [projects, sessions, pullRequests, approvals, audit, usage, reviews] =
        await Promise.all([
          settle("ProjectActivity"),
          settle("Session"),
          settle("PullRequest"),
          settle("ApprovalQueue"),
          settle("AuditTrail"),
          settle("UsageLedger"),
          settle("Review"),
        ]);
      // caps is INDEPENDENT of the data tiles ([[4]]): a caps fault → version stays the fail-safe "unknown"
      // (→ read-only), and must NOT blank the projections; a data-tile fault must NOT force read-only.
      let caps: Capabilities | null = null;
      try {
        caps = await client.get_capabilities();
      } catch (e) {
        console.error("cockpit load: get_capabilities failed (→ fail-safe read-only)", e);
      }
      if (cancelled) return;

      // Which projections degraded this load (null page) — the honest partial-data banner names them
      // (§11.7). Named distinctly from the render-scope transport `degraded` (deriveDegradedState) below.
      const degradedProjections = new Set<ProjectionName>();
      if (projects === null) degradedProjections.add("ProjectActivity");
      if (sessions === null) degradedProjections.add("Session");
      if (pullRequests === null) degradedProjections.add("PullRequest");
      if (approvals === null) degradedProjections.add("ApprovalQueue");
      if (audit === null) degradedProjections.add("AuditTrail");
      if (usage === null) degradedProjections.add("UsageLedger");
      if (reviews === null) degradedProjections.add("Review");

      // TOTAL fault: EVERY projection failed (daemon unreachable / all boundary-rejected) → no data at all
      // → the honest blank "couldn't load" screen (read-only pending) — one clear message, not 7 empty
      // tiles (preserves the total-fault behavior). A PARTIAL failure renders the cockpit + the banner.
      if (degradedProjections.size === 7) {
        setError(new Error("cockpit load: every projection failed to load"));
        return;
      }

      // caps resolved → record the version verdict; caps null → version stays "unknown" (read-only).
      if (caps !== null) setVersion(checkVersionCompat(caps));

      const counts = deriveProjectSwitcherCounts({
        projects: projects?.rows ?? [],
        sessions: sessions?.rows ?? [],
        pullRequests: pullRequests?.rows ?? [],
        approvals: approvals?.rows ?? [],
      });
      setData({
        projects: projects?.rows ?? [],
        counts,
        events: audit?.rows ?? [],
        sessions: sessions?.rows ?? [],
        pullRequests: pullRequests?.rows ?? [],
        approvals: approvals?.rows ?? [],
        usage: usage?.rows ?? [],
        creditPool: usage?.creditPool ?? null,
        reviews: reviews?.rows ?? [],
        degraded: degradedProjections,
      });
    })();
    return () => {
      cancelled = true;
    };
  }, [client]);

  // Live subscribe stream (052) — the Session projection's deltas keep the cockpit live as the
  // daemon mutates (the local store stays a read cache, forbidden #2); on a lag-close the recovery
  // machine reconnects → re-subscribes → re-`get_projection`s a fresh snapshot (no stale-as-live,
  // §11.7). Session-only here (the L1 mechanism on the most-dynamic projection); the other
  // projections reuse the identical mechanism — a spread. Recomputes the switcher counts so a live
  // session change is consistent across the list AND the derived counts.
  useEffect(() => {
    let cancelled = false;
    const recountFrom = (prev: ShellData, sessions: SessionRow[]): ShellData => ({
      ...prev,
      sessions,
      counts: deriveProjectSwitcherCounts({
        projects: prev.projects,
        sessions,
        pullRequests: prev.pullRequests,
        approvals: prev.approvals,
      }),
    });
    // The daemon emits a Session ProjectionDelta on every SessionStarted/Failed/Recovered, but as a
    // `row:None` id-NUDGE (deltas_for_event) — so consume it via REFETCH-ON-NUDGE (a coalesced re-read
    // of get_projection("Session")), NOT a row-apply reducer (which no-ops on the absent row — the 052
    // applySessionDelta was Mock-validated only, LESSON §29). Mirrors the ApprovalQueue effect below.
    const refetchSessions = async (): Promise<void> => {
      const page = await client.get_projection("Session");
      if (cancelled) return;
      setData((prev) => (prev ? recountFrom(prev, page.rows) : prev));
    };
    const coalescer = createNudgeCoalescer(refetchSessions);
    runSubscriptionSupervisor({
      subscribe: () => client.subscribe({ projection: "Session" }),
      // a row:None nudge → coalesced refetch (ignore the delta content; the daemon's nudge carries no row).
      onDelta: () => coalescer.nudge(),
      refetch: refetchSessions, // the supervisor's recovery snapshot reset (the 052 ground-truth re-read)
      // 054: drive the port — the SINGLE connection-state authority — NOT a 2nd raw React setter.
      // The port applies the guarded transition + suppresses the read-path upgrade while the stream
      // is degraded; `onConnectionChange` (the effect above) stays the Shell's ONE React connection
      // writer, so an ad-hoc read can never mask this stream-degrade (the 052 Finding; forbidden #6).
      // ui-059: per-stream — this is the "Session" stream (the ApprovalQueue stream is added as a 2nd
      // subscribe effect in L2; the port aggregates the two via worst-of).
      setConnection: (next) => client.notifyConnectionState("Session", next),
      delay: (attempt) =>
        new Promise((resolve) =>
          setTimeout(
            resolve,
            Math.min(
              SUBSCRIBE_BACKOFF_MAX_MS,
              SUBSCRIBE_BACKOFF_BASE_MS * 2 ** (attempt - 1),
            ),
          ),
        ),
      shouldContinue: () => !cancelled,
    }).catch((e: unknown) => {
      // The supervisor degrades internally on stream faults; a throw escaping it is unexpected
      // (e.g. a synchronous fault setting up a re-subscribe). Log it — don't crash the cockpit
      // (a live-read recovery fault must never take down the whole shell, §11.7).
      console.error("subscription supervisor exited unexpectedly", e);
    });
    return () => {
      cancelled = true;
    };
  }, [client]);

  // Live ApprovalQueue subscribe stream (ui-059) — the cockpit's ACTION surface stays live as the daemon
  // mutates. The daemon emits an ApprovalQueue ProjectionDelta on every submit/approve/deny, but as a
  // `row:None` id-NUDGE (gateway/pipeline.rs:79) — so this consumes it via REFETCH-ON-NUDGE (a coalesced
  // re-read get_projection("ApprovalQueue")), NOT a row-apply reducer (which would no-op on the absent
  // row). A 2nd stream composing with Session via the port's per-stream connection aggregation
  // (notifyConnectionState("ApprovalQueue", …) → worst-of; a healthy stream can't mask the other's
  // degrade). Recomputes the switcher counts so a live approval change is consistent across the queue
  // AND the derived waiting-on-you counts.
  useEffect(() => {
    let cancelled = false;
    const recountFromApprovals = (
      prev: ShellData,
      approvals: ApprovalQueueRow[],
    ): ShellData => ({
      ...prev,
      approvals,
      counts: deriveProjectSwitcherCounts({
        projects: prev.projects,
        sessions: prev.sessions,
        pullRequests: prev.pullRequests,
        approvals,
      }),
    });
    // The re-read-the-snapshot work — shared by the coalesced nudge path AND the recovery refetch.
    // Guard the setData with `cancelled` so an in-flight re-read that resolves after unmount (the
    // coalescer can hold work past cleanup) doesn't write to a torn-down effect's state.
    const refetchApprovals = async (): Promise<void> => {
      const page = await client.get_projection("ApprovalQueue");
      if (cancelled) return;
      setData((prev) => (prev ? recountFromApprovals(prev, page.rows) : prev));
    };
    // Coalesce a burst of nudges into a bounded number of re-reads (at-most-one-in-flight + one-trailing).
    const coalescer = createNudgeCoalescer(refetchApprovals);
    runSubscriptionSupervisor({
      subscribe: () => client.subscribe({ projection: "ApprovalQueue" }),
      // a row:None nudge → coalesced refetch (ignore the delta content; the daemon's nudge carries no row).
      onDelta: () => coalescer.nudge(),
      refetch: refetchApprovals, // the supervisor's recovery snapshot reset (the 052 ground-truth re-read)
      // ui-059: drive the port as the "ApprovalQueue" stream — the per-stream aggregate keeps this from
      // masking (or being masked by) the Session stream; the port stays the SINGLE connection authority.
      setConnection: (next) => client.notifyConnectionState("ApprovalQueue", next),
      delay: (attempt) =>
        new Promise((resolve) =>
          setTimeout(
            resolve,
            Math.min(
              SUBSCRIBE_BACKOFF_MAX_MS,
              SUBSCRIBE_BACKOFF_BASE_MS * 2 ** (attempt - 1),
            ),
          ),
        ),
      shouldContinue: () => !cancelled,
    }).catch((e: unknown) => {
      console.error("ApprovalQueue subscription supervisor exited unexpectedly", e);
    });
    return () => {
      cancelled = true;
    };
  }, [client]);

  // ── ui-063 whole-cockpit-live: spread refetch-on-nudge to the REST of the live-relevant served set ──
  // The daemon now emits deltas for ProjectActivity / PullRequest / UsageLedger too (D4) — each as a
  // `row:None` id-NUDGE (deltas_for_event) → consume via REFETCH-ON-NUDGE (a coalesced re-read), NEVER a
  // row-apply reducer (which no-ops on the absent row — LESSON §29). Each is a 3rd/4th/5th stream
  // composing with Session+ApprovalQueue through the port's per-stream worst-of connection authority
  // (notifyConnectionState("<X>", …)). AuditTrail is DELIBERATELY EXCLUDED — the daemon emits a BLANKET
  // AuditTrail nudge on every event, so a subscribe would trigger a whole-page re-read on every system
  // event (a refetch storm on a paged/forensic projection); AuditTrail stays refresh-on-open until the
  // daemon's flagged seq-cursor audit-delta enrichment lands.

  // ProjectActivity stream — `projects` IS a deriveProjectSwitcherCounts input → recompute `counts` on a
  // live project change (a new/changed project must re-key the switcher counts).
  useEffect(() => {
    let cancelled = false;
    const recountFromProjects = (
      prev: ShellData,
      projects: ProjectActivityRow[],
    ): ShellData => ({
      ...prev,
      projects,
      counts: deriveProjectSwitcherCounts({
        projects,
        sessions: prev.sessions,
        pullRequests: prev.pullRequests,
        approvals: prev.approvals,
      }),
    });
    const refetchProjects = async (): Promise<void> => {
      const page = await client.get_projection("ProjectActivity");
      if (cancelled) return;
      setData((prev) => (prev ? recountFromProjects(prev, page.rows) : prev));
    };
    const coalescer = createNudgeCoalescer(refetchProjects);
    runSubscriptionSupervisor({
      subscribe: () => client.subscribe({ projection: "ProjectActivity" }),
      onDelta: () => coalescer.nudge(),
      refetch: refetchProjects,
      setConnection: (next) => client.notifyConnectionState("ProjectActivity", next),
      delay: (attempt) =>
        new Promise((resolve) =>
          setTimeout(
            resolve,
            Math.min(
              SUBSCRIBE_BACKOFF_MAX_MS,
              SUBSCRIBE_BACKOFF_BASE_MS * 2 ** (attempt - 1),
            ),
          ),
        ),
      shouldContinue: () => !cancelled,
    }).catch((e: unknown) => {
      console.error("ProjectActivity subscription supervisor exited unexpectedly", e);
    });
    return () => {
      cancelled = true;
    };
  }, [client]);

  // PullRequest stream — `pullRequests` IS a deriveProjectSwitcherCounts input → recompute `counts` on a
  // live PR change (a new/closed PR must re-key the per-project openPRs count).
  useEffect(() => {
    let cancelled = false;
    const recountFromPullRequests = (
      prev: ShellData,
      pullRequests: PullRequestRow[],
    ): ShellData => ({
      ...prev,
      pullRequests,
      counts: deriveProjectSwitcherCounts({
        projects: prev.projects,
        sessions: prev.sessions,
        pullRequests,
        approvals: prev.approvals,
      }),
    });
    const refetchPullRequests = async (): Promise<void> => {
      const page = await client.get_projection("PullRequest");
      if (cancelled) return;
      setData((prev) => (prev ? recountFromPullRequests(prev, page.rows) : prev));
    };
    const coalescer = createNudgeCoalescer(refetchPullRequests);
    runSubscriptionSupervisor({
      subscribe: () => client.subscribe({ projection: "PullRequest" }),
      onDelta: () => coalescer.nudge(),
      refetch: refetchPullRequests,
      setConnection: (next) => client.notifyConnectionState("PullRequest", next),
      delay: (attempt) =>
        new Promise((resolve) =>
          setTimeout(
            resolve,
            Math.min(
              SUBSCRIBE_BACKOFF_MAX_MS,
              SUBSCRIBE_BACKOFF_BASE_MS * 2 ** (attempt - 1),
            ),
          ),
        ),
      shouldContinue: () => !cancelled,
    }).catch((e: unknown) => {
      console.error("PullRequest subscription supervisor exited unexpectedly", e);
    });
    return () => {
      cancelled = true;
    };
  }, [client]);

  // UsageLedger stream — usage is NOT a deriveProjectSwitcherCounts input → a PLAIN REPLACE (usage +
  // creditPool), NO recount. The live producer (TelemetrySampled ingress) is daemon-P4-dormant, so this
  // stream connects + stays quiet until telemetry flows; the handler is correct + refetches the moment it
  // does (built now, future-proof — identical pattern, harmless while dormant).
  useEffect(() => {
    let cancelled = false;
    const refetchUsage = async (): Promise<void> => {
      const page = await client.get_projection("UsageLedger");
      if (cancelled) return;
      setData((prev) =>
        prev ? { ...prev, usage: page.rows, creditPool: page.creditPool ?? null } : prev,
      );
    };
    const coalescer = createNudgeCoalescer(refetchUsage);
    runSubscriptionSupervisor({
      subscribe: () => client.subscribe({ projection: "UsageLedger" }),
      onDelta: () => coalescer.nudge(),
      refetch: refetchUsage,
      setConnection: (next) => client.notifyConnectionState("UsageLedger", next),
      delay: (attempt) =>
        new Promise((resolve) =>
          setTimeout(
            resolve,
            Math.min(
              SUBSCRIBE_BACKOFF_MAX_MS,
              SUBSCRIBE_BACKOFF_BASE_MS * 2 ** (attempt - 1),
            ),
          ),
        ),
      shouldContinue: () => !cancelled,
    }).catch((e: unknown) => {
      console.error("UsageLedger subscription supervisor exited unexpectedly", e);
    });
    return () => {
      cancelled = true;
    };
  }, [client]);

  // Review stream (ui-064) — the PR-review verticals stay live as the daemon syncs GitHub reviews
  // (ReviewSynced → a `row:None` Review nudge). reviews is NOT a deriveProjectSwitcherCounts input → a
  // PLAIN REPLACE, NO recount (mirrors UsageLedger). Completes the live-relevant served set (Review was
  // the one projection deferred from the ui-063 whole-cockpit-live spread).
  useEffect(() => {
    let cancelled = false;
    const refetchReviews = async (): Promise<void> => {
      const page = await client.get_projection("Review");
      if (cancelled) return;
      setData((prev) => (prev ? { ...prev, reviews: page.rows } : prev));
    };
    const coalescer = createNudgeCoalescer(refetchReviews);
    runSubscriptionSupervisor({
      subscribe: () => client.subscribe({ projection: "Review" }),
      onDelta: () => coalescer.nudge(),
      refetch: refetchReviews,
      setConnection: (next) => client.notifyConnectionState("Review", next),
      delay: (attempt) =>
        new Promise((resolve) =>
          setTimeout(
            resolve,
            Math.min(
              SUBSCRIBE_BACKOFF_MAX_MS,
              SUBSCRIBE_BACKOFF_BASE_MS * 2 ** (attempt - 1),
            ),
          ),
        ),
      shouldContinue: () => !cancelled,
    }).catch((e: unknown) => {
      console.error("Review subscription supervisor exited unexpectedly", e);
    });
    return () => {
      cancelled = true;
    };
  }, [client]);

  if (error) {
    return (
      <div className="shell shell--error" data-testid="shell-load-error" role="alert">
        Couldn’t load projections — the daemon payload was rejected at the
        boundary. Read-only state pending.
      </div>
    );
  }

  if (!data) {
    return (
      <div className="shell shell--loading" data-testid="shell-loading">
        Loading…
      </div>
    );
  }

  const status: ConnectionStatus = { connection, version };
  // Effective active project: the user's pick when it still exists, else the
  // first project (default scope), else null at zero-projects (the graph's
  // no-projects guard). resolveActiveProject guards the stale-ID case.
  const activeProjectId = resolveActiveProject(data.projects, rawActiveProjectId);
  const activeProject = data.projects.find((p) => p.project_id === activeProjectId);
  // The "checking" (connected + version-unknown) window: the real UdsGatewayPort now
  // drives connection→"connected" during get_capabilities while `version` resolves in
  // the same load Promise.all, so a transient checking frame is possible; after load it
  // settles to connected+compatible→ok. The live reconnect re-handshake that re-enters
  // this window is the 052 subscribe-recovery path.
  const degraded = deriveDegradedState(connection, version);

  // Global waiting-on-you count (the HIQ badge in TopBar + Sidebar): summed
  // across projects — triage is cross-cutting (Lesson §13: Command Center GLOBAL).
  const waiting = Object.values(data.counts).reduce(
    (sum, c) => sum + c.waitingOnYou,
    0,
  );

  // O-2 resume indicators: an id-keyed side map (Lesson §8 — surfaces resume mode
  // on the sidebar's session rows WITHOUT widening the row).
  const resumeModes = resumeModesBySessionId(data.sessions);

  // Retry = re-attempt the transport (real). Repair is a DISTINCT affordance
  // (§16: deeper repair / update-relaunch) whose dedicated backing lands with
  // daemon-1.5 + Phase-10 packaging; until then it aliases reconnect. Named
  // separately so the divergence is explicit, not a silent duplicate lambda.
  const handleRetry = () => client.reconnect();
  const handleRepair = () => client.reconnect(); // TODO(daemon-1.5/Phase 10): real repair/update-relaunch flow.

  // A team-lead session opens the Agent Team view (prototype behavior); others
  // open the Session Terminal. Team flag rides the display side-map (fixture
  // until the AgentTeam projection lands — flagged).
  const openSession = (s: SessionRow) => {
    setSelectedSessionId(s.session_id);
    navigate(sessionDisplayFixture[s.session_id]?.team ? "team" : "terminal");
  };

  const pending = pendingApprovals(data.approvals);
  const waitingRows = waitingSessions(data.sessions);

  // Palette actions route to views (navigate) or overlay surfaces.
  const onPaletteAction = (a: PaletteAction) => {
    if (a.kind === "view") {
      navigate(a.view);
      return;
    }
    if (a.overlay === "brain") setOverlay({ kind: "brain" });
    else if (a.overlay === "tasks") setOverlay({ kind: "tasks" });
    else if (a.overlay === "hiq") setOverlay({ kind: "hiq" });
    else if (pending[0]) setOverlay({ kind: "gateway", approval: pending[0] });
  };

  // Inspector "Open" jumps to the node's surface (live navigation).
  const onInspectorOpen = (node: GraphNode) => {
    setOverlay(null);
    const rawId = node.id.split(":")[1] ?? "";
    if (node.type === "session") {
      const row = data.sessions.find((s) => s.session_id === rawId);
      if (row) openSession(row);
    } else if (node.type === "pull_request") {
      navigate("code");
    } else {
      navigate("command");
    }
  };

  return (
    <ReadOnlyProvider value={status}>
      {/* ActiveProjectProvider wraps the WHOLE shell (incl. TopBar) so the
          ProjectSwitcher inside TopBar can read/set the active project. */}
      <ActiveProjectProvider value={{ activeProjectId, setActiveProject }}>
      {/* BrainStatusProvider feeds the Brain drawer/page header the ProjectBrain §5.1 status
          (FakeBrain default — the exposed-ahead swap-point for the live daemon 8.1 source; §13.1
          honest-degraded). A read/display provider — no daemon dep, no canSubmitIntent gate. */}
      <BrainStatusProvider value={fakeBrainStatus}>
      <div className="shell">
        <TopBar
          projects={data.projects}
          counts={data.counts}
          connection={connection}
          waiting={waiting}
          onOpenSettings={() => navigate("settings")}
          onOpenBrain={() => setOverlay({ kind: "brain" })}
          onOpenPalette={() => setOverlay({ kind: "palette" })}
          onOpenTasks={() => setOverlay({ kind: "tasks" })}
          onOpenHiq={() => setOverlay({ kind: "hiq" })}
          onOpenGateway={
            pending[0]
              ? () => setOverlay({ kind: "gateway", approval: pending[0]! })
              : undefined
          }
          onBack={back}
          onForward={forward}
          canBack={canBack}
          canForward={canForward}
        />
        {/* Banner stack (grid row) — the transport DegradedBanner, the survival
            RecoveryBanner, and the §17 safety surfaces. Auto-height: collapses to
            0 when every banner renders nothing. Three distinct concerns, stacked
            full-width above the side+main row so the signals are seen. */}
        <div className="banner-stack">
          <DegradedBanner
            degraded={degraded}
            onRetry={handleRetry}
            onRepair={handleRepair}
          />
          <RecoveryBanner recovery={recovery} />
          {/* ui-079 (§11.7) — the honest PARTIAL-DATA degrade banner: a DISTINCT surface (NOT the transport
              DegradedBanner above — never conflate, [[11]]) naming the projection(s) that failed THIS load,
              so a degraded tile is never silently shown as genuinely-empty. glyph+label (never color alone,
              §11.6); role="status" (polite — the rest of the cockpit is usable). Renders only when degraded. */}
          {data.degraded.size > 0 ? (
            <div
              role="status"
              className="partial-data-banner"
              data-testid="partial-data-banner"
              style={{
                padding: "8px 14px",
                font: "var(--fs-meta) var(--font-sans)",
                color: "var(--text-primary)",
                background: "var(--surface-sunken)",
                borderBottom: "1px solid var(--border-subtle)",
              }}
            >
              <span aria-hidden="true">⚠</span> Some data couldn’t load (showing
              what’s available) — unavailable: {[...data.degraded].join(", ")}.
            </div>
          ) : null}
          {/* §17 fail-closed / audit-integrity alert (#5) — prominent + non-dismissible. */}
          <AuditIntegrityAlert integrity={safety.integrity} />
          {/* §17 safety-state host (6.4d-2) — hosts the never-auto-resolved fencing/
              hard-conflict card (#6). Non-intrusive when clean; the full 7-group
              Human Input Queue host is Phase 8 (intent seam). */}
          <div className="safety-host" data-testid="safety-host">
            <HardConflictCard conflict={safety.conflict} />
          </div>
        </div>
        <Sidebar
          projects={data.projects}
          sessions={data.sessions}
          counts={data.counts}
          view={contentView}
          onNavigate={navigate}
          selectedSessionId={selectedSessionId}
          onOpenSession={openSession}
          waiting={waiting}
          resumeModes={resumeModes}
          onHumanInput={() => setOverlay({ kind: "hiq" })}
          onTasks={() => setOverlay({ kind: "tasks" })}
        />
        <main className="main" aria-label="Main surface">
          {contentView === "command" ? (
            // The project cockpit: the center column scopes to the active project
            // (prototype anatomy); the rail's HIQ stays GLOBAL (Lesson §13 —
            // triage is cross-cutting).
            <CommandCenter
              sessions={filterByActiveProject(data.sessions, activeProjectId)}
              approvals={pending}
              waiting={waitingRows}
              usage={data.usage}
              creditPool={data.creditPool}
              events={data.events}
              projectName={headerProjectLabel(activeProject)}
              onOpenSession={openSession}
              onOpenProjects={() => navigate("projects")}
            />
          ) : contentView === "graph" ? (
            <ProjectGraph
              projectId={activeProjectId ?? ""}
              projects={data.projects}
              sessions={data.sessions}
              pullRequests={data.pullRequests}
              usage={data.usage}
              onInspect={(node) => setOverlay({ kind: "inspect", node })}
            />
          ) : contentView === "terminal" ? (
            // Session Terminal: header/status are real; the PTY well is daemon-
            // gated (6.3d/e); no selection → the Sessions table as the picker.
            <SessionTerminal
              session={
                data.sessions.find((s) => s.session_id === selectedSessionId) ?? null
              }
              sessions={filterByActiveProject(data.sessions, activeProjectId)}
              projects={data.projects}
              usage={data.usage}
              gateway={client}
            />
          ) : contentView === "settings" ? (
            // Settings folds the Usage dashboard into its Usage tab (§11.2).
            // Reached via the TopBar gear (§11.2 nav model).
            <Settings usage={data.usage} creditPool={data.creditPool} />
          ) : contentView === "projects" ? (
            <ProjectsOverviewContainer
              gateway={client}
              projects={data.projects}
              counts={data.counts}
              activeProjectId={activeProjectId}
              onSelectProject={(id) => {
                setActiveProject(id);
                navigate("command");
              }}
            />
          ) : contentView === "plan" ? (
            <PlanView />
          ) : contentView === "editor" ? (
            <EditorView />
          ) : contentView === "code" ? (
            <DiffReview
              prs={filterByActiveProject(data.pullRequests, activeProjectId)}
              reviews={data.reviews}
              gateway={client}
            />
          ) : contentView === "team" ? (
            <AgentTeamView />
          ) : contentView === "packs" ? (
            <WorkflowPacksView />
          ) : contentView === "brain" ? (
            <BrainPage />
          ) : (
            <AuditTrail
              events={data.events}
              projectId={activeProjectId}
              projectName={headerProjectLabel(activeProject)}
            />
          )}
        </main>
        <EventDock
          events={data.events}
          connection={connection}
          projectId={activeProjectId}
          projectName={activeProject ? projectLabel(activeProject) : undefined}
          onOpenAudit={() => navigate("audit")}
        />
        {/* Overlay surfaces (one at a time — prototype behavior). */}
        {overlay?.kind === "palette" ? (
          <CommandPalette onClose={() => setOverlay(null)} onAction={onPaletteAction} />
        ) : overlay?.kind === "hiq" ? (
          <HumanInputQueue
            approvals={pending}
            waiting={waitingRows}
            onClose={() => setOverlay(null)}
            onOpenApproval={(a) => setOverlay({ kind: "gateway", approval: a })}
            onOpenSession={(s) => {
              setOverlay(null);
              openSession(s);
            }}
          />
        ) : overlay?.kind === "tasks" ? (
          <TaskInbox onClose={() => setOverlay(null)} />
        ) : overlay?.kind === "gateway" ? (
          // ui-073 — the dispatcher branches on the selected approval's plan_id: a plan-bearing
          // approval → the N-step PlanModal; a single-action approval → the unchanged GatewayModal.
          <GatewayOverlay
            approval={overlay.approval}
            port={client}
            onClose={() => setOverlay(null)}
          />
        ) : overlay?.kind === "brain" ? (
          <BrainDrawer
            onClose={() => setOverlay(null)}
            onExpand={() => {
              setOverlay(null);
              navigate("brain");
            }}
          />
        ) : overlay?.kind === "inspect" ? (
          <InspectorDrawer
            node={overlay.node}
            usage={data.usage}
            onClose={() => setOverlay(null)}
            onOpen={onInspectorOpen}
          />
        ) : null}
      </div>
      </BrainStatusProvider>
      </ActiveProjectProvider>
    </ReadOnlyProvider>
  );
}
