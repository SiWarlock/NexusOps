import { useEffect, useState } from "react";
import {
  Brain,
  CodeXml,
  FileDiff,
  LayoutGrid,
  ListChecks,
  Package,
  ScrollText,
} from "lucide-react";
import type { GatewayPort } from "../gateway-client/types";
import { MockGatewayPort } from "../gateway-client/mock";
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
import { SessionsTable } from "../views/sessions/SessionsTable";
import { Settings } from "../views/settings/Settings";
import { PlaceholderView } from "../views/Placeholder";
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
import { useViewHistory } from "./view-history";
import { TopBar } from "./TopBar";
import { Sidebar } from "./Sidebar";
import { DrawerStack } from "./DrawerStack";
import { EventDock } from "./EventDock";

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
  const [client] = useState<GatewayPort>(() => gateway ?? new MockGatewayPort());
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

  useEffect(() => client.onConnectionChange(setConnection), [client]);

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
            client.get_projection("Usage"),
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
  // The "checking" (connected + version-unknown) window surfaces at the real
  // daemon-1.5 reconnect re-handshake; the MockGatewayPort resolves version
  // together with data (one Promise.all behind the !data load gate), so the
  // window is trigger-pending here (wired, not yet driven). See the ui↔daemon-1.5
  // Carry-forward spread.
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

  const openSession = (s: SessionRow) => {
    setSelectedSessionId(s.session_id);
    navigate("terminal");
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
        />
        <main className="main" aria-label="Main surface">
          {contentView === "command" ? (
            // The project cockpit: the center column scopes to the active project
            // (prototype anatomy); the rail's HIQ stays GLOBAL (Lesson §13 —
            // triage is cross-cutting).
            <CommandCenter
              sessions={filterByActiveProject(data.sessions, activeProjectId)}
              approvals={pendingApprovals(data.approvals)}
              waiting={waitingSessions(data.sessions)}
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
            />
          ) : contentView === "terminal" ? (
            // Session Terminal: the live PTY surface is daemon-gated (6.3d/e —
            // terminal channel). Until it lands, the view hosts the Sessions
            // table (real projection data + filters) as the session list.
            <section aria-label="Session Terminal" className="terminal-view">
              <SessionsTable
                sessions={filterByActiveProject(data.sessions, activeProjectId)}
                projects={data.projects}
              />
            </section>
          ) : contentView === "settings" ? (
            // Settings folds the Usage dashboard into its Usage tab (§11.2).
            // Reached via the TopBar gear (§11.2 nav model).
            <Settings usage={data.usage} creditPool={data.creditPool} />
          ) : contentView === "projects" ? (
            <PlaceholderView
              title="Projects Overview"
              icon={<LayoutGrid size={18} />}
              blockedOn="the Projects Overview slice (this rebuild, next chunk)"
            />
          ) : contentView === "plan" ? (
            <PlaceholderView
              title="Plan"
              icon={<ListChecks size={18} />}
              blockedOn="the Plan projection (workflow-pack parser integration)"
            />
          ) : contentView === "editor" ? (
            <PlaceholderView
              title="Editor"
              icon={<CodeXml size={18} />}
              blockedOn="the worktree file-access daemon contract"
            />
          ) : contentView === "code" ? (
            <PlaceholderView
              title="Code / Diff Review"
              icon={<FileDiff size={18} />}
              blockedOn="the diff/worktree daemon contract (Phase 7 PR review)"
            />
          ) : contentView === "team" ? (
            <PlaceholderView
              title="Agent Team"
              icon={<Brain size={18} />}
              blockedOn="the AgentTeam projection (daemon team contracts)"
            />
          ) : contentView === "packs" ? (
            <PlaceholderView
              title="Workflow Packs"
              icon={<Package size={18} />}
              blockedOn="the WorkflowInstance projection (pack registry contract)"
            />
          ) : contentView === "brain" ? (
            <PlaceholderView
              title="Project Brain"
              icon={<Brain size={18} />}
              blockedOn="the Brain sidecar contract (Phase 8)"
            />
          ) : (
            <PlaceholderView
              title="Audit Trail"
              icon={<ScrollText size={18} />}
              blockedOn="the Audit Trail view slice (this rebuild, next chunk)"
            />
          )}
        </main>
        <DrawerStack />
        <EventDock
          events={data.events}
          connection={connection}
          projectId={activeProjectId}
          projectName={activeProject?.name}
          onOpenAudit={() => navigate("audit")}
        />
      </div>
      </ActiveProjectProvider>
    </ReadOnlyProvider>
  );
}
