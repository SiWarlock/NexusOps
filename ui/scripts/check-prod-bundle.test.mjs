// ui-078 — the prod-bundle gate's pure core. `scanForForbidden` is the substring scan the
// CI gate runs over each built JS chunk; the I/O wrapper (reading dist/ + exit codes) is thin
// and exercised by `pnpm check:bundle` in CI. Pinned here so a future refactor can't silently
// weaken the scan (first-only, word-boundary, case-insensitive) and let a Mock leak through.
import { describe, it, expect } from "vitest";
import { scanForForbidden, FORBIDDEN } from "./check-prod-bundle.mjs";

describe("scanForForbidden (ui-078 prod-bundle gate)", () => {
  it("FORBIDDEN is exactly the Mock gateway + its build-env flag", () => {
    // pin the gate's CONFIGURATION, not just its logic: dropping/renaming a needle (which would silently
    // widen what can ship to prod) fails HERE. The logic tests below drive off this same real FORBIDDEN.
    expect(FORBIDDEN).toEqual(["MockGatewayPort", "VITE_NEXUSOPS_MOCK"]);
  });

  it("returns empty for clean production text", () => {
    // a real-shaped prod chunk: only the production port, never the Mock / env flag.
    expect(
      scanForForbidden("const c=new UdsGatewayPort({mutationsEnabled:!0});", FORBIDDEN),
    ).toEqual([]);
  });

  it("flags a forbidden needle (a Mock leak)", () => {
    expect(scanForForbidden("...class MockGatewayPort{...", FORBIDDEN)).toEqual([
      "MockGatewayPort",
    ]);
  });

  it("flags every present needle, not just the first (substring, all-matches)", () => {
    // pins TWO load-bearing properties: ALL needles reported (a `.find()`-style first-only refactor
    // would miss a 2nd leak) AND substring (a minified `MockGatewayPort2` rename still trips it).
    expect(
      scanForForbidden("a MockGatewayPort2 b VITE_NEXUSOPS_MOCK c", FORBIDDEN),
    ).toEqual(["MockGatewayPort", "VITE_NEXUSOPS_MOCK"]);
  });
});
