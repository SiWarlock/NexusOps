import { Button } from "../design-system/kit";
import type { ProjectActivityRow } from "../contracts/index";
import type { ProjectSwitcherCounts } from "./derive";
import { ProjectSwitcher } from "./ProjectSwitcher";

// The accessible NAME of a closed-prop kit control comes from a visually-hidden
// child INSIDE it (a wrapper aria-label would not name the inner button) — Lesson
// §6 + §11.7. The visually-hidden styling is the global `.sr-only` utility (6.5a
// theme base — ui/src/theme/global.css), the single source of truth.

/**
 * Top bar: back/forward history + the project switcher; Brain + Settings are
 * reached here (not the view-switch), per the §11.2 nav model (human-confirmed
 * 2026-06-08). The Settings control opens the Settings view; the Brain trigger is
 * deferred to Phase 8 (the Brain drawer isn't built — placeholder). The icon-only
 * back/forward controls carry visually-hidden accessible names and drive the
 * content-view history (useViewHistory): each is `disabled` at its boundary so a
 * named control is never a dead click (§11.6).
 */
export function TopBar({
  projects,
  counts,
  onOpenSettings,
  onBack,
  onForward,
  canBack,
  canForward,
}: {
  projects: ProjectActivityRow[];
  counts: Record<string, ProjectSwitcherCounts>;
  onOpenSettings: () => void;
  onBack: () => void;
  onForward: () => void;
  canBack: boolean;
  canForward: boolean;
}) {
  return (
    <header className="topbar">
      <nav aria-label="History">
        <Button variant="ghost" size="sm" onClick={onBack} disabled={!canBack}>
          <span aria-hidden="true">←</span>
          <span className="sr-only">Back</span>
        </Button>
        <Button variant="ghost" size="sm" onClick={onForward} disabled={!canForward}>
          <span aria-hidden="true">→</span>
          <span className="sr-only">Forward</span>
        </Button>
      </nav>
      <ProjectSwitcher projects={projects} counts={counts} />
      <div className="topbar__right">
        <Button variant="ghost" size="sm">
          Brain
        </Button>
        <Button variant="ghost" size="sm" onClick={onOpenSettings}>
          Settings
        </Button>
      </div>
    </header>
  );
}
