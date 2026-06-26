import { useState } from "react";
import type { ProjectActivityRow } from "../../contracts/index";
import type { GatewayPort } from "../../gateway-client/types";
import type { ProjectSwitcherCounts } from "../../shell/derive";
import { useSubmitIntent } from "../../intent/submit-intent";
import { useCanSubmitIntent } from "../../connection/read-only";
import { buildRescanProjectActionRequest } from "../../intent/rescan-project-request";
import { pickFolder as defaultPickFolder } from "../../host/pick-folder";
import { ProjectsOverview, type AddProjectNotice } from "./ProjectsOverview";

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
}: {
  gateway: GatewayPort;
  projects: ProjectActivityRow[];
  counts: Record<string, ProjectSwitcherCounts>;
  activeProjectId: string | null;
  onSelectProject: (id: string) => void;
  /** Injectable for tests; defaults to the real Tauri folder picker. */
  pickFolder?: () => Promise<string | null>;
}) {
  const seam = useSubmitIntent(gateway);
  const canSubmit = useCanSubmitIntent();
  const [adding, setAdding] = useState(false);
  const [notice, setNotice] = useState<AddProjectNotice | null>(null);

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
      const r = await seam.submitAction(
        buildRescanProjectActionRequest({ path }, new Date().toISOString()),
      );
      if ("ok" in r) {
        // the daemon accepted + auto-executed the rescan; the project surfaces via the projection.
        setNotice({ kind: "ok", message: `Scanning ${path}…` });
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
