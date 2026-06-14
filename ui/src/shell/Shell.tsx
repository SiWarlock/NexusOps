import { useEffect, useState } from "react";
import type { GatewayPort } from "../gateway-client/types";
import { UdsGatewayPort } from "../gateway-client/uds";
import type {
  ApprovalQueueRow,
  AuditEventRow,
  CreditPool,
  ProjectActivityRow,
  PullRequestRow,
  RecoveryStatus,
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
import { CommandCenter } from "../views/command/CommandCenter";
import { ProjectGraph } from "../views/graph/ProjectGraph";
import { Settings } from "../views/settings/Settings";
import { ProjectsOverview } from "../views/projects/ProjectsOverview";
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
import { enrichApproval, sessionDisplayFixture } from "./display-meta";
import { useViewHistory } from "./view-history";
import { TopBar } from "./TopBar";
import { Sidebar } from "./Sidebar";
import { EventDock } from "./EventDock";
import { CommandPalette, type PaletteAction } from "../overlays/CommandPalette";
import { HumanInputQueue } from "../overlays/HumanInputQueue";
import { TaskInbox } from "../overlays/TaskInbox";
import { GatewayModal } from "../overlays/GatewayModal";
import { BrainDrawer } from "../overlays/BrainDrawer";
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
}

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
  // PRODUCTION DEFAULT = the real UdsGatewayPort (L1 read-swap, 051) — the initial
  // get_projection/get_capabilities load now shows REAL daemon data over the 050 invoke
  // bridge. The MockGatewayPort stays the injectable test/dev seam (passed via `gateway`).
  const [client] = useState<GatewayPort>(() => gateway ?? new UdsGatewayPort());
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
      try {
        const [projects, sessions, pullRequests, approvals, audit, usage, caps] =
          await Promise.all([
            client.get_projection("ProjectActivity"),
            client.get_projection("Session"),
            client.get_projection("PullRequest"),
            client.get_projection("ApprovalQueue"),
            client.get_projection("AuditTrail"),
            client.get_projection("UsageLedger"),
            client.get_capabilities(),
          ]);
        if (cancelled) return;
        const counts = deriveProjectSwitcherCounts({
          projects: projects.rows,
          sessions: sessions.rows,
          pullRequests: pullRequests.rows,
          approvals: approvals.rows,
        });
        setVersion(checkVersionCompat(caps));
        setData({
          projects: projects.rows,
          counts,
          events: audit.rows,
          sessions: sessions.rows,
          pullRequests: pullRequests.rows,
          approvals: approvals.rows,
          usage: usage.rows,
          creditPool: usage.creditPool ?? null,
        });
      } catch (e) {
        if (!cancelled) setError(e);
      }
    })();
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
              projectName={activeProject?.name ?? "No project"}
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
            <ProjectsOverview
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
            <DiffReview prs={filterByActiveProject(data.pullRequests, activeProjectId)} gateway={client} />
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
              projectName={activeProject?.name ?? "No project"}
            />
          )}
        </main>
        <EventDock
          events={data.events}
          connection={connection}
          projectId={activeProjectId}
          projectName={activeProject?.name}
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
          <GatewayModal
            {...enrichApproval(overlay.approval)}
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
      </ActiveProjectProvider>
    </ReadOnlyProvider>
  );
}
