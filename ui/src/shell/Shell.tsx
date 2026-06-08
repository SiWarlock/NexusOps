import { useEffect, useState } from "react";
import type { GatewayPort } from "../gateway-client/types";
import { MockGatewayPort } from "../gateway-client/mock";
import type {
  ApprovalQueueRow,
  AuditEventRow,
  ProjectActivityRow,
  PullRequestRow,
  SessionRow,
} from "../contracts/index";
import { deriveProjectSwitcherCounts, type ProjectSwitcherCounts } from "./derive";
import type { SidebarItem } from "./Sidebar";
import { CommandCenter } from "../views/command/CommandCenter";
import type { CommandItem } from "../views/command/group";
import { ProjectGraph } from "../views/graph/ProjectGraph";
import {
  toSessionItems,
  toPrItems,
  toApprovalItems,
} from "../projections/items";
import { ReadOnlyProvider, type ConnectionStatus } from "../connection/read-only";
import {
  checkVersionCompat,
  deriveDegradedState,
  type VersionCompat,
} from "../connection/version";
import type { ConnectionState } from "../connection/state";
import { DegradedBanner } from "../connection/DegradedBanner";
import { TopBar } from "./TopBar";
import { Sidebar } from "./Sidebar";
import { DrawerStack } from "./DrawerStack";
import { ActivityDock } from "./ActivityDock";
import { StatusBar } from "./StatusBar";

interface ShellData {
  projects: ProjectActivityRow[];
  counts: Record<string, ProjectSwitcherCounts>;
  events: AuditEventRow[];
  sessions: SessionRow[];
  pullRequests: PullRequestRow[];
  approvals: ApprovalQueueRow[];
}

/**
 * The top-level app shell — a projection-driven reattaching client (§11). Reads
 * projections ONLY through the gateway-client boundary (validated payloads) and
 * renders the chrome from them. It also surfaces the transport degraded state:
 * a ReadOnlyProvider exposes connected+version-compatible to every control's
 * canSubmitIntent gate (fail-safe FALSE until confirmed), a ConnectionIndicator
 * sits in the StatusBar, and a DegradedBanner appears when disconnected /
 * reconnecting / version-skewed. The daemon Gateway remains the real INV-SEC-1
 * guard; this read-only gate is defense-in-depth.
 */
export function Shell({ gateway }: { gateway?: GatewayPort }) {
  // Stable client across renders (a fresh default per render would loop the effect).
  const [client] = useState<GatewayPort>(() => gateway ?? new MockGatewayPort());
  const [data, setData] = useState<ShellData | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [connection, setConnection] = useState<ConnectionState>(() =>
    client.getConnectionState(),
  );
  // Fail-safe: version stays "unknown" (→ read-only) until a handshake confirms it.
  const [version, setVersion] = useState<VersionCompat>("unknown");
  // Which content view the main surface shows (6.3b). Command Center is default.
  const [contentView, setContentView] = useState<"command" | "graph">("command");

  useEffect(() => client.onConnectionChange(setConnection), [client]);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const [projects, sessions, pullRequests, approvals, audit, caps] =
          await Promise.all([
            client.get_projection("ProjectActivity"),
            client.get_projection("Session"),
            client.get_projection("PullRequest"),
            client.get_projection("ApprovalQueue"),
            client.get_projection("AuditTrail"),
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
  const degraded = deriveDegradedState(connection, version);

  // Sessions drive the sidebar's attention-ordered items (§11.3 sidebar weight);
  // the same session items open the Command Center list — mapped once, reused.
  const sessionItems = toSessionItems(data.sessions);
  const sidebarItems: SidebarItem[] = sessionItems;

  // Command Center items: sessions + PRs + approvals (the wired projections;
  // tasks join when a Task/PlanProgress projection lands — Phase 7). Routed
  // through the shared mappers (no inline re-map — P6.3b).
  const commandItems: CommandItem[] = [
    ...sessionItems,
    ...toPrItems(data.pullRequests),
    ...toApprovalItems(data.approvals),
  ];

  // Retry = re-attempt the transport (real). Repair is a DISTINCT affordance
  // (§16: deeper repair / update-relaunch) whose dedicated backing lands with
  // daemon-1.5 + Phase-10 packaging; until then it aliases reconnect. Named
  // separately so the divergence is explicit, not a silent duplicate lambda.
  const handleRetry = () => client.reconnect();
  const handleRepair = () => client.reconnect(); // TODO(daemon-1.5/Phase 10): real repair/update-relaunch flow.

  return (
    <ReadOnlyProvider value={status}>
      <div
        className="shell"
        style={{ display: "flex", flexDirection: "column", height: "100vh" }}
      >
        <TopBar projects={data.projects} counts={data.counts} />
        <DegradedBanner
          degraded={degraded}
          onRetry={handleRetry}
          onRepair={handleRepair}
        />
        <div style={{ display: "flex", flex: 1, minHeight: 0 }}>
          <Sidebar items={sidebarItems} />
          <main style={{ flex: 1, minWidth: 0 }} aria-label="Main surface">
            {/* Content-view switch (6.3b): Command Center (default) | Project
                Graph. Sessions / Terminal / Diff are the later 6.3 sub-slices. */}
            <div
              className="content-switch"
              role="group"
              aria-label="Content view"
            >
              <button
                type="button"
                aria-pressed={contentView === "command"}
                onClick={() => setContentView("command")}
              >
                Command Center
              </button>
              <button
                type="button"
                aria-pressed={contentView === "graph"}
                onClick={() => setContentView("graph")}
              >
                Project Graph
              </button>
            </div>
            {contentView === "command" ? (
              <CommandCenter items={commandItems} />
            ) : (
              <ProjectGraph
                projectId={data.projects[0]?.project_id ?? ""}
                projects={data.projects}
                sessions={data.sessions}
                pullRequests={data.pullRequests}
              />
            )}
          </main>
          <DrawerStack />
        </div>
        <ActivityDock events={data.events} />
        <StatusBar connection={connection} />
      </div>
    </ReadOnlyProvider>
  );
}
