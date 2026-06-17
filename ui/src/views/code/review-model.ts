// The PR-review verdict view-model (ui-064, §11.2): a pure ReviewState → badge descriptor.
//
// ReviewState is a frozen VALUE enum (a fixed GitHub review verdict — approved / changes_requested /
// commented / dismissed / pending), NOT a (machine,status) status machine. So it lives HERE, deliberately
// OUT of the cross-doc status→attention-rank descriptor table (`status/descriptors.ts`, drift-pinned to
// the status-machine enums) — a verdict badge is UI render policy, a different concern. The
// `Record<ReviewState,…>` forces every value to be covered (tsc rejects a missing key — a daemon-added
// state is a build break until it renders, never an unknown→blank, forbidden #5). The badge renders
// glyph + LABEL (the non-color channels); tone is ADDITIVE color (§11.6 never-color-alone).
import type { ReviewRow } from "../../contracts/index";

// The generated `ReviewState` enum is exported as a VALUE (a Zod schema), not a type; the string-union
// type is the ReviewRow `state` field (which delegates to it) — the codebase's narrow-type-from-row idiom.
type ReviewState = ReviewRow["state"];

/** Kit Badge tones used for verdicts (a subset of the kit's tone union). */
type ReviewTone = "success" | "danger" | "neutral" | "slate" | "caution";

export interface ReviewStateDescriptor {
  state: ReviewState;
  /** Non-color channel (never color alone — §11.6): a glyph SHAPE. */
  glyph: string;
  /** Non-color channel: the human verdict label (the badge's text). */
  label: string;
  /** Additive kit Badge tone (color is ADDITIVE to glyph+label). */
  tone: ReviewTone;
}

const REVIEW_STATE: Record<ReviewState, { glyph: string; label: string; tone: ReviewTone }> = {
  approved: { glyph: "✓", label: "Approved", tone: "success" },
  changes_requested: { glyph: "✗", label: "Changes requested", tone: "danger" },
  commented: { glyph: "🗩", label: "Commented", tone: "neutral" },
  dismissed: { glyph: "⊘", label: "Dismissed", tone: "slate" },
  pending: { glyph: "◷", label: "Pending", tone: "caution" },
};

export function describeReviewState(state: ReviewState): ReviewStateDescriptor {
  return { state, ...REVIEW_STATE[state] };
}
