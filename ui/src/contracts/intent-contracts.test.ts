import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { PerHunkGitActionType } from "./index";

// Read the Rust action-type catalog (the AUTHORITY) at test time — the per-hunk
// git action identifiers are NOT a schema enum $def; they live as string consts in
// shared/src/catalog.rs (MVP_ACTION_TYPES). This is the cross-language drift-pin:
// the ui's typing handle must stay a SUBSET of the daemon's real action types, so a
// daemon rename fails this loudly (Lesson §2/§14 — a seam type pins to its authority).
const catalogPath = fileURLToPath(
  new URL("../../../shared/src/catalog.rs", import.meta.url),
);
const catalogSrc = readFileSync(catalogPath, "utf8");

describe("intent-contracts — per-hunk git action identifiers (§6.3e)", () => {
  it("per_hunk_action_types_exist_in_catalog_authority", () => {
    // spec(§6.3) — every identifier the ui handle exposes must be a real action type
    // in the daemon's MVP_ACTION_TYPES (the catalog-authoritative SoT). The ui adopts
    // ONLY the string identifier — never the risk class / standing_grant eligibility
    // (daemon-authoritative policy, cat-1 Q4; the UI renders the daemon's policy).
    expect(PerHunkGitActionType.options).toEqual([
      "git.stage_hunk",
      "git.unstage_hunk",
      "git.discard_hunk",
    ]);
    for (const id of PerHunkGitActionType.options) {
      expect(
        catalogSrc.includes(`"${id}"`),
        `${id} must exist as a string const in shared/src/catalog.rs`,
      ).toBe(true);
    }
  });

  it("per_hunk_action_type_rejects_unknown", () => {
    // spec(§5.0/§15) — reject-unknown: a non-per-hunk action type is not a member.
    expect(PerHunkGitActionType.safeParse("git.stage_hunk").success).toBe(true);
    expect(PerHunkGitActionType.safeParse("git.commit").success).toBe(false);
    expect(PerHunkGitActionType.safeParse("workflow.command.invoke").success).toBe(false);
  });
});
