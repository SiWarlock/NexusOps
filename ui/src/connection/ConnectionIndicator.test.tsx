// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render } from "@testing-library/react";
import { ConnectionIndicator } from "./ConnectionIndicator";
import type { ConnectionState } from "./state";

afterEach(cleanup);

describe("ConnectionIndicator", () => {
  it("connection_indicator_never_color_alone", () => {
    const labels: Record<ConnectionState, string> = {
      connecting: "Connecting",
      connected: "Connected",
      reconnecting: "Reconnecting",
      disconnected: "Disconnected",
    };
    for (const state of [
      "connecting",
      "connected",
      "reconnecting",
      "disconnected",
    ] as ConnectionState[]) {
      const { container, getByText, unmount } = render(
        <ConnectionIndicator state={state} />,
      );
      // text-label channel present (not color-only)
      expect(getByText(labels[state])).toBeTruthy();
      // glyph channel present too (color + glyph + label = §11.1 four-channel)
      expect(container.querySelector(".conn-indicator__glyph")).not.toBeNull();
      unmount();
    }
  });
});
