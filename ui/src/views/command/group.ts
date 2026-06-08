// Command Center triage grouping (§11.2) — derived purely from the 6.2a model
// (triageBucket / compareByAttention). Screens never re-derive attention.
import {
  compareByAttention,
  triageBucket,
  type AttentionRank,
} from "../../status/attention";
import { describeStatus } from "../../status/descriptors";
import type { ProjectionItem } from "../../projections/items";

/** A Command Center entry — the shared projection-item shape (id/label/machine/status). */
export type CommandItem = ProjectionItem;

export interface RankedCommandItem extends CommandItem {
  attentionRank: AttentionRank;
}

export interface CommandCenterGroups {
  /** Highlighted cluster — items whose status is `changes_ready` (all rank 4). */
  changesReady: RankedCommandItem[];
  /** Needs-my-attention {4,5}, EXCLUDING the extracted changes_ready items. */
  needsAttention: RankedCommandItem[];
  working: RankedCommandItem[];
  settled: RankedCommandItem[];
}

const CHANGES_READY = "changes_ready";

const sort = (arr: RankedCommandItem[]) => arr.toSorted(compareByAttention);
const bucket = (item: RankedCommandItem) => triageBucket(item.attentionRank);

// changes_ready is extracted into its own prominent group ONLY when it's actually
// a needs-attention item (today it's rank 4 on every machine that has it). Gating
// on the rank too means a future machine that uses the string at a non-needs rank
// would fall into its proper triage bucket, not silently into changesReady.
const isExtractedChangesReady = (item: RankedCommandItem) =>
  item.status === CHANGES_READY && bucket(item) === "needs-attention";

export function groupForCommandCenter(
  items: CommandItem[],
): CommandCenterGroups {
  const ranked: RankedCommandItem[] = items.map((item) => ({
    ...item,
    attentionRank: describeStatus(item.machine, item.status).attentionRank,
  }));

  return {
    changesReady: sort(ranked.filter(isExtractedChangesReady)),
    needsAttention: sort(
      ranked.filter(
        (i) => bucket(i) === "needs-attention" && !isExtractedChangesReady(i),
      ),
    ),
    working: sort(ranked.filter((i) => bucket(i) === "working")),
    settled: sort(ranked.filter((i) => bucket(i) === "settled")),
  };
}
