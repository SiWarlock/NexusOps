import type { CSSProperties } from "react";
import type { ReviewRow } from "../../contracts/index";
import { Badge } from "../../design-system/kit";
import { describeReviewState } from "./review-model";

const EMPTY_NOTE: CSSProperties = {
  font: "var(--fs-label)/1.5 var(--font-sans)",
  color: "var(--text-muted)",
  padding: "10px 12px",
  border: "1px dashed var(--border-subtle)",
  borderRadius: "var(--r-2)",
};

/** One review → a verdict card: reviewer + ReviewState badge (glyph + LABEL, never color alone —
 *  forbidden #5) + the §15-redacted body + submitted_at (date). The badge's tone is ADDITIVE color. */
function ReviewCard({ review }: { review: ReviewRow }) {
  const verdict = describeReviewState(review.state);
  return (
    <div
      data-item-id={`Review:${review.review_id}`}
      style={{
        border: "1px solid var(--border-default)",
        borderRadius: "var(--r-3)",
        background: "var(--surface-card)",
        padding: "10px 12px",
        display: "flex",
        flexDirection: "column",
        gap: 6,
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <span style={{ font: "var(--fw-medium) var(--fs-label) var(--font-sans)", color: "var(--text-primary)" }}>
          {review.reviewer ?? "unknown reviewer"}
        </span>
        <Badge tone={verdict.tone} icon={<span aria-hidden="true">{verdict.glyph}</span>}>
          {verdict.label}
        </Badge>
        {review.submitted_at ? (
          <span style={{ marginLeft: "auto", font: "var(--fs-micro) var(--font-mono)", color: "var(--text-faint)" }}>
            {review.submitted_at.slice(0, 10)}
          </span>
        ) : null}
      </div>
      {review.body ? (
        <div style={{ font: "var(--fs-meta)/1.5 var(--font-sans)", color: "var(--text-secondary)" }}>
          {review.body}
        </div>
      ) : null}
    </div>
  );
}

/**
 * The PR reviews-list (ui-064 Layer 1, §11.2) — renders the `ReviewRow[]` for a PR as verdict cards.
 * An empty list renders an explicit empty-state (distinct from the list not being shown at all). Pure
 * read-only display over the frozen Review projection; no mutation surface.
 */
export function ReviewsList({ reviews }: { reviews: ReviewRow[] }) {
  if (reviews.length === 0) {
    return (
      <div data-testid="reviews-empty" style={EMPTY_NOTE}>
        No reviews yet.
      </div>
    );
  }
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
      {reviews.map((r) => (
        <ReviewCard key={r.review_id} review={r} />
      ))}
    </div>
  );
}
