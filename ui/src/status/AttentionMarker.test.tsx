// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render } from "@testing-library/react";
import { AttentionMarker } from "./AttentionMarker";
import { describeStatus } from "./descriptors";

afterEach(cleanup);

describe("AttentionMarker wrapper", () => {
  it("attention_marker_level_matches_rank", () => {
    const high = describeStatus("Session", "waiting_on_human_input").attentionRank; // 5
    const { container } = render(<AttentionMarker rank={high} />);
    const marker = container.querySelector("[data-level]");
    expect(marker).not.toBeNull();
    expect(marker!.getAttribute("data-level")).toBe(String(high));

    const low = describeStatus("Session", "idle").attentionRank; // 0
    const { container: c2 } = render(<AttentionMarker rank={low} />);
    expect(c2.querySelector("[data-level]")!.getAttribute("data-level")).toBe(
      String(low),
    );
  });
});
