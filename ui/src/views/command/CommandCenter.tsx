import { StatusPill } from "../../status/StatusPill";
import { AttentionMarker } from "../../status/AttentionMarker";
import {
  groupForCommandCenter,
  type CommandCenterGroups,
  type CommandItem,
  type RankedCommandItem,
} from "./group";

// The default landing view (§11.2): items triaged into a prominent Changes-ready
// cluster + needs-attention / working / settled, each attention-ordered. Renders
// exactly the items it's given (no invented state); the data arrives through the
// Shell's gateway boundary.
const GROUPS: { key: keyof CommandCenterGroups; title: string }[] = [
  { key: "changesReady", title: "Changes ready" },
  { key: "needsAttention", title: "Needs my attention" },
  { key: "working", title: "Working" },
  { key: "settled", title: "Settled" },
];

function GroupItem({ item }: { item: RankedCommandItem }) {
  return (
    <li
      className="command-center__item"
      data-item-id={`${item.machine}:${item.id}`}
    >
      <AttentionMarker rank={item.attentionRank} variant="rail" />
      <span className="command-center__item-label">{item.label}</span>
      <StatusPill machine={item.machine} status={item.status} size="xs" />
    </li>
  );
}

export function CommandCenter({ items }: { items: CommandItem[] }) {
  const groups = groupForCommandCenter(items);
  return (
    <div className="command-center" aria-label="Command Center">
      {GROUPS.map(({ key, title }) => {
        const groupItems = groups[key];
        return (
          <section
            key={key}
            className="command-center__group"
            data-group={key}
            aria-label={title}
          >
            <h2 className="command-center__group-title">
              {title} ({groupItems.length})
            </h2>
            {groupItems.length === 0 ? (
              <p className="command-center__empty" data-testid={`empty-${key}`}>
                Nothing here.
              </p>
            ) : (
              <ul>
                {groupItems.map((item) => (
                  <GroupItem key={`${item.machine}:${item.id}`} item={item} />
                ))}
              </ul>
            )}
          </section>
        );
      })}
    </div>
  );
}
