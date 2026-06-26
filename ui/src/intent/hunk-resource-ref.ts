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
import type {
  ActionRequest,
  Hunk,
  PerHunkGitActionType,
  ResourceRef,
} from "../contracts/index";
import { mintActionRequestId } from "./mint-id";

/** The U+001F unit-separator delimiting worktree_id / file / positions (frozen §6.3). */
export const HUNK_ID_SEP = "\u001f";

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
    inputs: null,
    risk_level: 0, // non-authoritative hint — daemon reconciles to the catalog; never displayed
    status: "submitted",
    created_at: createdAt,
  };
}
