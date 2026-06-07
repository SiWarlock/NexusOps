import { useEffect, useState } from "react";
import type { GatewayPort } from "../gateway-client/types";
import { MockGatewayPort } from "../gateway-client/mock";
import type { AuditEventRow, ProjectActivityRow } from "../contracts/index";
import { deriveProjectSwitcherCounts, type ProjectSwitcherCounts } from "./derive";
import { TopBar } from "./TopBar";
import { Sidebar } from "./Sidebar";
import { DrawerStack } from "./DrawerStack";
import { ActivityDock } from "./ActivityDock";
import { StatusBar } from "./StatusBar";

interface ShellData {
  projects: ProjectActivityRow[];
  counts: Record<string, ProjectSwitcherCounts>;
  events: AuditEventRow[];
}

/**
 * The top-level app shell — a projection-driven reattaching client (§11). It
 * reads projections ONLY through the gateway-client boundary (validated
 * payloads; never a raw payload, never a direct DB/git call) and renders the
 * chrome regions from them. This is the production entry point that mounts the
 * 6.1a gateway-client seam (boundary + generated contracts + mock) on the real
 * render path. The daemon-connection indicator + read-only degraded mode +
 * version-skew handling are 6.1c (their shell slots are reserved here).
 */
export function Shell({ gateway }: { gateway?: GatewayPort }) {
  // Stable client across renders (a fresh default per render would loop the effect).
  const [client] = useState<GatewayPort>(() => gateway ?? new MockGatewayPort());
  const [data, setData] = useState<ShellData | null>(null);
  const [error, setError] = useState<unknown>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const [projects, sessions, pullRequests, approvals, audit] =
          await Promise.all([
            client.get_projection("ProjectActivity"),
            client.get_projection("Session"),
            client.get_projection("PullRequest"),
            client.get_projection("ApprovalQueue"),
            client.get_projection("AuditTrail"),
          ]);
        if (cancelled) return;
        const counts = deriveProjectSwitcherCounts({
          projects: projects.rows,
          sessions: sessions.rows,
          pullRequests: pullRequests.rows,
          approvals: approvals.rows,
        });
        setData({ projects: projects.rows, counts, events: audit.rows });
      } catch (e) {
        if (!cancelled) setError(e);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [client]);

  if (error) {
    // A boundary reject (or any read failure) surfaces as a handled state — never
    // a raw render. The full read-only degraded mode + Repair lands in 6.1c.
    return (
      <div className="shell shell--error" data-testid="shell-load-error" role="alert">
        Couldn’t load projections — the daemon payload was rejected at the
        boundary. Read-only state pending (6.1c).
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

  return (
    <div
      className="shell"
      style={{ display: "flex", flexDirection: "column", height: "100vh" }}
    >
      <TopBar projects={data.projects} counts={data.counts} />
      <div style={{ display: "flex", flex: 1, minHeight: 0 }}>
        <Sidebar />
        <main style={{ flex: 1, minWidth: 0 }} aria-label="Main surface">
          {/* Screen contents (Command Center, Graph, Sessions, Terminal, Diff)
              land in 6.3; the shell only provides the container here. */}
          <div data-testid="content-pane" />
        </main>
        <DrawerStack />
      </div>
      <ActivityDock events={data.events} />
      <StatusBar />
    </div>
  );
}
