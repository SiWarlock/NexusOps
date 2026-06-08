import { describeStatus } from "../status/descriptors";
import { compareByAttention, needsMyAttention } from "../status/attention";
import { AttentionMarker } from "../status/AttentionMarker";
import { StatusPill } from "../status/StatusPill";
import { describeResumeMode } from "../recovery/model";
import type { ResumeMode } from "../contracts/index";
import type { ProjectionItem } from "../projections/items";

/** A sidebar entry — the shared projection-item shape (id/label/machine/status). */
export type SidebarItem = ProjectionItem;

// O-2 survival display on the dense sidebar: a compact resumed/replayed indicator
// (glyph + sr-only label — never color alone, §11.6). The full visible label lives
// in the Sessions table; here the distinct glyph SHAPE is the visible channel.
function SidebarResumeIndicator({ mode }: { mode: ResumeMode }) {
  const desc = describeResumeMode(mode);
  return (
    <span className="sidebar__resume-mode" data-resume-mode={mode}>
      <span aria-hidden="true">{desc.glyph}</span>
      <span className="sr-only">{desc.label}</span>
    </span>
  );
}

/**
 * Left sidebar (§11.3): items are attention-ordered (compareByAttention, higher
 * rank first), each carries an AttentionMarker rail + a StatusPill, and the
 * header surfaces the needs-attention count (needsMyAttention). The grouped
 * needs/working/settled Command Center triage view is 6.3.
 */
export function Sidebar({
  items = [],
  resumeModes = {},
}: {
  items?: SidebarItem[];
  /** id-keyed side map (Lesson §8) — resume indicators without widening the item. */
  resumeModes?: Record<string, ResumeMode>;
}) {
  const ranked = items.map((item) => ({
    ...item,
    attentionRank: describeStatus(item.machine, item.status).attentionRank,
  }));
  const ordered = ranked.toSorted(compareByAttention);
  const needsCount = ranked.filter((item) =>
    needsMyAttention(item.attentionRank),
  ).length;

  return (
    <aside className="sidebar" aria-label="Sidebar">
      <div className="sidebar__header">
        <span data-testid="needs-attention-count">{needsCount}</span>{" "}
        {needsCount === 1 ? "needs" : "need"} attention
      </div>
      <nav aria-label="Primary">
        <ul>
          {ordered.map((item) => {
            // Only sessions carry a resume mode; guard the lookup to Session items
            // (the item is the generic ProjectionItem — could be a PR/Approval).
            const resumeMode =
              item.machine === "Session" ? resumeModes[item.id] : undefined;
            return (
              <li
                key={`${item.machine}:${item.id}`}
                className="sidebar__item"
                data-item-id={`${item.machine}:${item.id}`}
              >
                <AttentionMarker rank={item.attentionRank} variant="rail" />
                <span className="sidebar__item-label">{item.label}</span>
                <StatusPill machine={item.machine} status={item.status} size="xs" />
                {resumeMode ? <SidebarResumeIndicator mode={resumeMode} /> : null}
              </li>
            );
          })}
        </ul>
      </nav>
    </aside>
  );
}
