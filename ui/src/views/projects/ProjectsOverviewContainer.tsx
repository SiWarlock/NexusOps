import { useEffect, useState } from "react";
import type { ProjectActivityRow } from "../../contracts/index";
import type { GatewayPort } from "../../gateway-client/types";
import type { ProjectSwitcherCounts } from "../../shell/derive";
import { useSubmitIntent } from "../../intent/submit-intent";
import { useCanSubmitIntent } from "../../connection/read-only";
import { buildRescanProjectActionRequest } from "../../intent/rescan-project-request";
import { pickFolder as defaultPickFolder } from "../../host/pick-folder";
import { ProjectsOverview, type AddProjectNotice } from "./ProjectsOverview";

/** The last path segment (the project's folder name) for the notice copy — `/` and `\` aware. */
function basename(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

/**
 * The add-project CONTAINER (the cockpit "Add project" wiring). Mirrors `PrWorkspaceContainer`: it
 * owns the gateway + the submit seam, so `ProjectsOverview` stays a presentational view.
 *
 * Flow: click "Add project" → native folder picker (`pickFolder`, injectable for tests) → submit a
 * `project.rescan` intent (the daemon AUTO-EXECUTES it — risk-0 allowlist; the registered project
 * surfaces via the ProjectActivity projection nudge). The button is gated on `canSubmitIntent`
 * (fail-safe READ-ONLY/degraded gate, forbidden #6) — the UI submits an intent, never executes
 * (INV-SEC-1; the daemon Gateway is the chokepoint). NON-optimistic: the notice reflects the daemon's
 * ack / verbatim §6.4 rejection, never a synthesized success.
 */
export function ProjectsOverviewContainer({
  gateway,
  projects,
  counts,
  activeProjectId,
  onSelectProject,
  pickFolder = defaultPickFolder,
  scanTimeoutMs = 8000,
}: {
  gateway: GatewayPort;
  projects: ProjectActivityRow[];
  counts: Record<string, ProjectSwitcherCounts>;
  activeProjectId: string | null;
  onSelectProject: (id: string) => void;
  /** Injectable for tests; defaults to the real Tauri folder picker. */
  pickFolder?: () => Promise<string | null>;
  /** Injectable scan-resolution timeout (ms) before the honest "registered — refresh" fallback. */
  scanTimeoutMs?: number;
}) {
  const seam = useSubmitIntent(gateway);
  const canSubmit = useCanSubmitIntent();
  const [adding, setAdding] = useState(false);
  const [notice, setNotice] = useState<AddProjectNotice | null>(null);
  // The in-flight scan we're waiting to see appear (its minted project_id + the folder basename).
  const [pendingScan, setPendingScan] = useState<{ projectId: string; basename: string } | null>(
    null,
  );

  // Resolve the Scanning notice when the minted project_id appears in the projections prop (daemon
  // slice 091's ProjectRescanned→ProjectActivity fold+nudge surfaces it) — confirm + auto-select it.
  useEffect(() => {
    if (!pendingScan) return;
    if (projects.some((p) => p.project_id === pendingScan.projectId)) {
      setNotice({ kind: "ok", message: `Added ${pendingScan.basename}.` });
      onSelectProject(pendingScan.projectId);
      setPendingScan(null);
    }
  }, [projects, pendingScan, onSelectProject]);

  // A missed nudge must NEVER hang the Scanning notice (§11.4/§11.7): after the (injectable)
  // timeout, replace it with an honest fallback — registration DID succeed, it's just unconfirmed.
  // The timer is cancelled on resolve (pendingScan→null re-runs this) AND on unmount (cleanup).
  useEffect(() => {
    if (!pendingScan) return;
    const base = pendingScan.basename;
    const timer = setTimeout(() => {
      setNotice({ kind: "info", message: `Registered ${base} — refresh if it doesn't appear.` });
      setPendingScan(null);
    }, scanTimeoutMs);
    return () => clearTimeout(timer);
  }, [pendingScan, scanTimeoutMs]);

  async function onAddProject() {
    // belt-and-suspenders: the button is disabled when these hold, but never form an intent without them.
    if (!canSubmit || adding) return;
    setNotice(null);
    let path: string | null;
    try {
      path = await pickFolder();
    } catch (e) {
      // a host fault opening the dialog — degrade honestly, never silently swallow (§11.7).
      console.error("folder picker failed", e);
      setNotice({ kind: "error", message: "Couldn't open the folder picker." });
      return;
    }
    if (path == null) return; // user cancelled — a no-op, no notice

    setAdding(true);
    try {
      const req = buildRescanProjectActionRequest({ path }, new Date().toISOString());
      const r = await seam.submitAction(req);
      if ("ok" in r) {
        // the daemon accepted + auto-executed the rescan; track the minted project_id so we resolve
        // the notice when it appears in the projections (or fall back on timeout — never hang).
        const base = basename(path);
        if (req.project_id) {
          setPendingScan({ projectId: req.project_id, basename: base });
          setNotice({ kind: "pending", message: `Scanning ${base}…` });
        } else {
          // no id to track (defends the optional-project_id type boundary) — never show an
          // unresolvable "Scanning"; go straight to the honest fallback.
          setNotice({ kind: "info", message: `Registered ${base} — refresh if it doesn't appear.` });
        }
      } else if ("error" in r) {
        // surface the daemon's §6.4 code VERBATIM (never collapse/remap) — honest rejection.
        setNotice({ kind: "error", message: `Couldn't add the project (${r.error.code}).` });
      } else {
        // the fail-safe gate refused (the GatewayPort was never called) — disconnected/degraded.
        setNotice({ kind: "error", message: "Can't add a project while the daemon is unavailable." });
      }
    } catch (e) {
      // the seam RE-THROWS Error instances (a real transport/host fault is never swallowed as a
      // fake success, LESSON §16) — catch it here + degrade honestly (§11.7), never an unhandled reject.
      console.error("add-project submit failed", e);
      setNotice({ kind: "error", message: "Couldn't add the project — the daemon is unavailable." });
    } finally {
      setAdding(false);
    }
  }

  return (
    <ProjectsOverview
      projects={projects}
      counts={counts}
      activeProjectId={activeProjectId}
      onSelectProject={onSelectProject}
      canAddProject={canSubmit && !adding}
      onAddProject={onAddProject}
      addProjectNotice={notice}
    />
  );
}
