import { describe, it, expect } from "vitest";
import {
  buildHunkActionRequest,
  hunkResourceRef,
  HUNK_ID_SEP,
} from "./hunk-resource-ref";
import { ActionRequest, ResourceRef, type Hunk } from "../contracts/index";

// The U+001F unit separator, via escape (a literal control char is fragile in source).
const SEP = "\u001f";

// The daemon `minted_id!` ULID shape the client-minted action_request_id must satisfy
// (uppercase Crockford, no I/L/O/U, 26-char body, first char 0-7 — the `Ulid::from_string` range).
const ACT_ULID = /^act_[0-7][0-9A-HJKMNP-TV-Z]{25}$/;

const hunk: Hunk = {
  header: "@@ -10,4 +10,6 @@",
  old_start: 10,
  old_lines: 4,
  new_start: 10,
  new_lines: 6,
  lines: [{ kind: "context", content: " x\n" }],
};

describe("hunk resource_ref encoder (§6.3 — THE security-critical conformance)", () => {
  it("resource_ref_encodes_displayed_hunk_exactly", () => {
    // spec(§6.3 / the security pin) — the submitted resource_ref MUST target the EXACT
    // displayed hunk: id = "{worktree_id}<US>{file}<US>{old_start},{old_lines},{new_start},
    // {new_lines}" (<US> = U+001F), the 4 positions VERBATIM from the displayed Hunk. A
    // mismatch stages/DISCARDS the wrong content (catastrophic for the irreversible discard).
    const ref = hunkResourceRef("wt_demo_0001", "src/gateway/review.ts", hunk);
    expect(ref.id).toBe(`wt_demo_0001${SEP}src/gateway/review.ts${SEP}10,4,10,6`);
    // round-trip: split on U+001F → [worktree_id, file, positions]; positions → the hunk
    const parts = ref.id.split(HUNK_ID_SEP);
    expect(parts).toHaveLength(3);
    expect(parts[0]).toBe("wt_demo_0001");
    expect(parts[1]).toBe("src/gateway/review.ts");
    const positions = parts[2]!.split(",").map(Number);
    expect(positions).toEqual([
      hunk.old_start,
      hunk.old_lines,
      hunk.new_start,
      hunk.new_lines,
    ]);
  });

  it("resource_ref_type_is_frozen_lowercase_file", () => {
    // spec(§6.2/§5.0) — the FROZEN ResourceRef field is `type` (not `resource_type`) and
    // the value is the lowercase ResourceType member "file" (NOT "File"); a "File" value
    // would FAIL ResourceRef.parse against the generated ResourceType enum.
    const ref = hunkResourceRef("wt_1", "a.ts", hunk);
    expect(ref.type).toBe("file");
    expect(() => ResourceRef.parse(ref)).not.toThrow();
  });

  it("hunk_id_sep_is_the_unit_separator_U001F", () => {
    // spec(§6.3) — the delimiter is exactly U+001F (the unit separator), never a comma/
    // tab/other char (the frozen daemon convention the Phase-5 git executor parses back).
    expect(HUNK_ID_SEP).toBe("\u001f");
    expect(HUNK_ID_SEP.charCodeAt(0)).toBe(0x1f);
  });

  it("distinct_hunks_produce_distinct_resource_ref_ids", () => {
    // spec(§17) — hunk-precise: two distinct hunks (even same file) → distinct
    // NaturalResourceRef keys, so no false dedup and stage/discard targets THIS hunk.
    const a = hunkResourceRef("wt_1", "a.ts", { ...hunk, old_start: 10 });
    const b = hunkResourceRef("wt_1", "a.ts", { ...hunk, old_start: 20 });
    expect(a.id).not.toBe(b.id);
  });
});

describe("buildHunkActionRequest (§6.1/§6.2 — the submitted intent assembler)", () => {
  it("assembles_a_contract_valid_request_with_client_minted_id", () => {
    // spec(§6.2/Q1/Q4) — the UI submits a typed, contract-valid ActionRequest: the
    // action_type is the per-hunk id, resource_refs targets the exact displayed hunk,
    // action_request_id is CLIENT-MINTED (the daemon trusts the wire id as the PK; only
    // session.create mints daemon-side), risk_level is a non-authoritative hint (0).
    const req = buildHunkActionRequest(
      "git.discard_hunk",
      "wt_demo_0001",
      "src/gateway/review.ts",
      hunk,
      "2026-06-13T00:00:00Z",
    );
    expect(() => ActionRequest.parse(req)).not.toThrow();
    expect(req.action_type).toBe("git.discard_hunk");
    expect(req.action_request_id).toMatch(ACT_ULID); // client-minted (was the empty-PK bug)
    expect(req.risk_level).toBe(0); // non-authoritative hint — daemon reconciles, never displayed
    expect(req.resource_refs).toHaveLength(1);
    expect(req.resource_refs[0]!.id).toBe(
      hunkResourceRef("wt_demo_0001", "src/gateway/review.ts", hunk).id,
    );
  });

  it("mints_distinct_prefixed_action_request_id_across_calls", () => {
    // spec(§6.2) — each per-hunk submit carries a distinct client-minted act_<ULID>, never "".
    // A reused/empty PK collides on the daemon action_requests TEXT PK → AuditWriteFailed →
    // overloaded precondition_stale (the 2nd-mutation bug ui-080 fixed for rescan).
    const a = buildHunkActionRequest("git.stage_hunk", "wt_1", "a.ts", hunk, "2026-06-13T00:00:00Z");
    const b = buildHunkActionRequest("git.stage_hunk", "wt_1", "a.ts", hunk, "2026-06-13T00:00:00Z");
    expect(a.action_request_id).toMatch(ACT_ULID);
    expect(b.action_request_id).toMatch(ACT_ULID);
    expect(a.action_request_id).not.toBe(b.action_request_id);
  });
});
