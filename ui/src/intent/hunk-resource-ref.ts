// The per-hunk git-action resource_ref encoder (§6.3) — THE security-critical unit.
//
// A per-hunk git mutation (stage/unstage/discard) MUST target the EXACT displayed hunk:
// the daemon's Phase-5 git executor parses this resource_ref id back to {worktree_id,
// file, positions} and applies the op to THAT hunk. A mismatch stages — or DISCARDS
// (irreversible) — the wrong content. So the id is formed VERBATIM from the displayed
// `Hunk` (the `get_diff` result), conformance-pinned (hunk-resource-ref.test.ts).
//
// FROZEN convention (daemon/CLAUDE.md §6.3, LESSON §32): the ResourceRef field is `type`
// (a `ResourceType`, lowercase `"file"`) and the id is the U+001F-delimited triple. The
// brief/daemon prose's informal `resource_type:"File"` is Rust-variant shorthand —
// `ResourceType::File` serializes to the wire value `"file"`.
import { sha256 } from "@noble/hashes/sha2.js";
import { bytesToHex, utf8ToBytes } from "@noble/hashes/utils.js";
import type {
  ActionRequest,
  Hunk,
  PerHunkGitActionType,
  ResourceRef,
} from "../contracts/index";
import { mintActionRequestId } from "./mint-id";

/** The U+001F unit-separator delimiting worktree_id / file / positions (frozen §6.3). */
export const HUNK_ID_SEP = "\u001f";

/** The kind of one displayed `DiffLine`, derived from the generated `DiffLineKind` enum
 *  (`Hunk.lines[].kind` delegates to `bundle.shape.DiffLineKind`). Deriving from the consumed
 *  row keeps {@link PREFIX} tsc-COMPLETE over the generated enum: a daemon-added 4th kind
 *  grows this union → the `Record` below stops type-checking until covered (LESSON §5/§30). */
type DiffLineKind = Hunk["lines"][number]["kind"];

/**
 * The LOCKED origin-byte prefix per diff-line kind (the unified-diff convention the daemon
 * mirrors at 096): a `context` line → `" "`, an `added` line → `"+"`, a `removed` line → `"-"`.
 * `Record<DiffLineKind, …>` so tsc forces every generated kind to carry a prefix — an uncovered
 * kind would hash to a divergent canonical string vs the daemon (a destroy-gate divergence).
 * Pinned complete by `diff_line_prefix_map_is_complete_over_generated_kinds`.
 */
export const PREFIX: Record<DiffLineKind, string> = {
  context: " ",
  added: "+",
  removed: "-",
};

/**
 * Canonicalize the displayed `Hunk` BODY to the LOCKED daemon string (096 Step-2.5): for each
 * `DiffLine` IN ORDER, one {@link PREFIX} origin byte immediately followed by `DiffLine.content`
 * VERBATIM (the raw `get_diff` content — the daemon strips the origin marker INTO `content` and
 * retains the trailing `\n`), all joined with NO separator. The `@@` header is EXCLUDED. Do NOT
 * re-render / strip / re-add newlines or collapse leading whitespace — `content` carries its own
 * `\n`, so `.join("")` is correct. The intermediate string is the cross-language contract lock.
 */
export function displayedHunkCanonical(hunk: Hunk): string {
  return hunk.lines.map((line) => PREFIX[line.kind] + line.content).join("");
}

/**
 * The §17 verify-before-destroy content hash: SHA-256 of {@link displayedHunkCanonical}, as
 * lowercase hex (64 chars). The daemon re-derives the live hunk, re-computes this hash, and
 * compares before discarding — absent / empty / mismatch → `Failed` ("hunk changed,
 * re-examine"), so a stale hash never destroys the wrong content (extending LESSON §19 from
 * position-identity to content-identity). Frozen-vector-pinned both sides (the daemon pins the
 * identical {canonical, digest}).
 */
export function displayedHunkSha256(hunk: Hunk): string {
  return bytesToHex(sha256(utf8ToBytes(displayedHunkCanonical(hunk))));
}

/**
 * The displayed `Hunk` → its `File` resource_ref. The id is
 * `{worktree_id}<US>{file}<US>{old_start},{old_lines},{new_start},{new_lines}` — the 4
 * positions VERBATIM from the displayed hunk (read↔mutate consistency, §17). `type` is the
 * frozen lowercase `ResourceType` `"file"`.
 */
export function hunkResourceRef(
  worktreeId: string,
  file: string,
  hunk: Hunk,
): ResourceRef {
  const positions = `${hunk.old_start},${hunk.old_lines},${hunk.new_start},${hunk.new_lines}`;
  return {
    type: "file",
    id: `${worktreeId}${HUNK_ID_SEP}${file}${HUNK_ID_SEP}${positions}`,
  };
}

/**
 * Assemble the typed `ActionRequest` the UI SUBMITS for a per-hunk git action (Q1 — the
 * UI submits an intent, never executes). The CLIENT mints `action_request_id` (the daemon
 * trusts the wire id verbatim as the `action_requests` PK — only `session.create` mints
 * daemon-side; an empty id collided on the 2nd same-session mutation → AuditWriteFailed →
 * the overloaded precondition_stale, ui-080 LESSON §39). `risk_level` is a NON-AUTHORITATIVE
 * hint — catalog-authoritative + daemon-reconciled, NEVER displayed (Q4; the card's risk is
 * the daemon's PolicyDecision/ActionPreview). `created_at` is the UI submission time (a real
 * fact, not an invented consequence). The resource_ref targets the exact displayed hunk.
 */
export function buildHunkActionRequest(
  actionType: PerHunkGitActionType,
  worktreeId: string,
  file: string,
  hunk: Hunk,
  createdAt: string,
): ActionRequest {
  return {
    action_request_id: mintActionRequestId(),
    action_type: actionType,
    requester_type: "user",
    requester_id: "current_user",
    resource_refs: [hunkResourceRef(worktreeId, file, hunk)],
    // The §17 verify-before-destroy content hash is DISCARD-ONLY (the daemon relay scoped it to
    // git.discard_hunk; stage/unstage keep the position-only posture, daemon LESSON §72). `inputs` is
    // the §6.2 z.unknown() opaque passthrough — adding this key is NOT a contract/Zod change.
    inputs:
      actionType === "git.discard_hunk"
        ? { displayed_hunk_sha256: displayedHunkSha256(hunk) }
        : null,
    risk_level: 0, // non-authoritative hint — daemon reconciles to the catalog; never displayed
    status: "submitted",
    created_at: createdAt,
  };
}
