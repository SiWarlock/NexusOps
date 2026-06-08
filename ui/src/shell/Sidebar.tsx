import { describeStatus } from "../status/descriptors";
import { compareByAttention, needsMyAttention } from "../status/attention";
import { AttentionMarker } from "../status/AttentionMarker";
import { StatusPill } from "../status/StatusPill";
import type { ProjectionItem } from "../projections/items";

/** A sidebar entry — the shared projection-item shape (id/label/machine/status). */
export type SidebarItem = ProjectionItem;

/**
 * Left sidebar (§11.3): items are attention-ordered (compareByAttention, higher
 * rank first), each carries an AttentionMarker rail + a StatusPill, and the
 * header surfaces the needs-attention count (needsMyAttention). The grouped
 * needs/working/settled Command Center triage view is 6.3.
 */
export function Sidebar({ items = [] }: { items?: SidebarItem[] }) {
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
          {ordered.map((item) => (
            <li
              key={`${item.machine}:${item.id}`}
              className="sidebar__item"
              data-item-id={`${item.machine}:${item.id}`}
            >
              <AttentionMarker rank={item.attentionRank} variant="rail" />
              <span className="sidebar__item-label">{item.label}</span>
              <StatusPill machine={item.machine} status={item.status} size="xs" />
            </li>
          ))}
        </ul>
      </nav>
    </aside>
  );
}
