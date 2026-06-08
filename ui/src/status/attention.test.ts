import { describe, it, expect } from "vitest";
import { compareByAttention, needsMyAttention, triageBucket } from "./attention";

describe("attention derivations", () => {
  it("needs_my_attention_membership", () => {
    expect(needsMyAttention(5)).toBe(true);
    expect(needsMyAttention(4)).toBe(true);
    expect(needsMyAttention(3)).toBe(false);
    expect(needsMyAttention(2)).toBe(false);
    expect(needsMyAttention(0)).toBe(false);

    // triage buckets partition all six ranks: {4,5} attention / {2,3} working / {0,1} settled
    expect(triageBucket(5)).toBe("needs-attention");
    expect(triageBucket(4)).toBe("needs-attention");
    expect(triageBucket(3)).toBe("working");
    expect(triageBucket(2)).toBe("working");
    expect(triageBucket(1)).toBe("settled");
    expect(triageBucket(0)).toBe("settled");
  });

  it("sort_by_attention_desc", () => {
    const items = [
      { id: "a", attentionRank: 1 },
      { id: "b", attentionRank: 4 },
      { id: "c", attentionRank: 4 },
      { id: "d", attentionRank: 0 },
    ];
    const sorted = items.toSorted(compareByAttention);
    // higher rank first; equal ranks keep original order (stable tiebreak)
    expect(sorted.map((i) => i.id)).toEqual(["b", "c", "a", "d"]);
  });
});
