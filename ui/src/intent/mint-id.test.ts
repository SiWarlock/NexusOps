import { describe, it, expect } from "vitest";
import { decodeTime } from "ulid";
import { mintActionRequestId, mintProjectId } from "./mint-id";

// The EXACT daemon `minted_id!` ULID shape the wire id must satisfy so `ActionRequestId::parse()`
// (= strip_prefix + `ulid::Ulid::from_string`, shared/src/ids.rs:151-155) accepts it:
// uppercase Crockford base32 (no I/L/O/U), 26-char body, first char 0-7 (the 48-bit-timestamp
// range `Ulid::from_string` enforces). A hyphenated `crypto.randomUUID()` would fail this.
const ACT_ULID = /^act_[0-7][0-9A-HJKMNP-TV-Z]{25}$/;
const PROJ_ULID = /^proj_[0-7][0-9A-HJKMNP-TV-Z]{25}$/;

describe("mint-id (client-side prefixed-ULID minter)", () => {
  // spec(§5.2): prefixed-ULID IDs — the minted id is the daemon's `<prefix>_<26-char-Crockford>`
  // shape (never a hyphenated UUID, never empty), or it fails-closed at the daemon parse().
  it("mintActionRequestId produces a valid act_<Crockford-ULID>", () => {
    expect(mintActionRequestId()).toMatch(ACT_ULID);
  });

  it("mintProjectId produces a valid proj_<Crockford-ULID>", () => {
    expect(mintProjectId()).toMatch(PROJ_ULID);
  });

  // belt-and-suspenders (daemon parse conformance): the 26-char body is a DECODABLE ULID whose
  // embedded timestamp is real (≤ now) — proves it round-trips through `Ulid::from_string`, not
  // merely that it matches a regex.
  it("mints ids whose ULID body decodes to a real timestamp (both minters)", () => {
    const now = Date.now();
    for (const body of [
      mintActionRequestId().slice("act_".length),
      mintProjectId().slice("proj_".length),
    ]) {
      const t = decodeTime(body);
      expect(Number.isFinite(t)).toBe(true);
      expect(t).toBeGreaterThan(0);
      expect(t).toBeLessThanOrEqual(now);
    }
  });

  // spec(§5.2): a minted id is unique per call — a reused id collides on the daemon's
  // `action_requests` TEXT PRIMARY KEY (the 2nd-Add AuditWriteFailed→precondition_stale bug).
  it("produces a distinct id on each call", () => {
    const acts = new Set([mintActionRequestId(), mintActionRequestId(), mintActionRequestId()]);
    expect(acts.size).toBe(3);
    const projs = new Set([mintProjectId(), mintProjectId(), mintProjectId()]);
    expect(projs.size).toBe(3);
  });
});
