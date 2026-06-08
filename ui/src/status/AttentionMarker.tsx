import { AttentionMarker as KitAttentionMarker } from "@ui-kit/status/AttentionMarker";
import type { AttentionRank } from "./attention";

// Binds an attention-rank (0–5) to the kit AttentionMarker level. The
// display:contents wrapper is layout-transparent (so the kit rail's
// alignSelf:stretch still works at the leading edge of a row) while carrying
// data-level — the kit's typed props don't accept data-*.
export function AttentionMarker({
  rank,
  variant = "rail",
}: {
  rank: AttentionRank;
  variant?: "rail" | "dot";
}) {
  return (
    <span style={{ display: "contents" }} data-level={rank}>
      <KitAttentionMarker level={rank} variant={variant} />
    </span>
  );
}
