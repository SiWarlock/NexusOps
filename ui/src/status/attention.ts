// The Attention Ladder (§11.1) — the single source for sidebar weight, queue
// membership, and sort. Ranks 0–5; higher = more urgent.
export type AttentionRank = 0 | 1 | 2 | 3 | 4 | 5;

export type TriageBucket = "needs-attention" | "working" | "settled";

/** Ranks at/above this enter "Needs my attention" (queue membership). */
export const NEEDS_ATTENTION_THRESHOLD = 4;

export function needsMyAttention(rank: AttentionRank): boolean {
  return rank >= NEEDS_ATTENTION_THRESHOLD;
}

/** Partition every rank into one of three triage buckets (§5.2). */
export function triageBucket(rank: AttentionRank): TriageBucket {
  if (rank >= 4) return "needs-attention"; // {4,5}
  if (rank >= 2) return "working"; // {2,3}
  return "settled"; // {0,1}
}

/**
 * Sort comparator: higher attention-rank first. Returns 0 on equal ranks, so a
 * stable sort (Array.prototype.sort, ES2019+) preserves the input order as the
 * tiebreak (callers pre-order by recency/name).
 */
export function compareByAttention(
  a: { attentionRank: number },
  b: { attentionRank: number },
): number {
  return b.attentionRank - a.attentionRank;
}
