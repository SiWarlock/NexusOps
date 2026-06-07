import { Button } from "../design-system/kit";
import type { ProjectActivityRow } from "../contracts/index";
import type { ProjectSwitcherCounts } from "./derive";
import { ProjectSwitcher } from "./ProjectSwitcher";

/**
 * Top bar: back/forward history + the project switcher; Brain + Settings are
 * reached here (not the sidebar), per the kit shell reference (§11.2).
 */
export function TopBar({
  projects,
  counts,
}: {
  projects: ProjectActivityRow[];
  counts: Record<string, ProjectSwitcherCounts>;
}) {
  return (
    <header
      className="topbar"
      style={{ display: "flex", alignItems: "center", gap: 8 }}
    >
      {/* TODO(6.4 a11y): the history controls need accessible names — the kit
          Button's typed props don't accept aria-label, so the glyph-only label
          gap is resolved with 6.4's a11y MUSTs (focus ring / labels / non-drag).
          The nav landmark is labelled in the meantime. */}
      <nav aria-label="History" style={{ display: "flex", gap: 4 }}>
        <Button variant="ghost" size="sm">
          ←
        </Button>
        <Button variant="ghost" size="sm">
          →
        </Button>
      </nav>
      <ProjectSwitcher projects={projects} counts={counts} />
      <div style={{ marginLeft: "auto", display: "flex", gap: 4 }}>
        <Button variant="ghost" size="sm">
          Brain
        </Button>
        <Button variant="ghost" size="sm">
          Settings
        </Button>
      </div>
    </header>
  );
}
