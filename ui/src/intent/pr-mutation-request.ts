// The PR-mutation (Merge / Submit-review) cat-1 intent surface (ui-070/071).
//
// Ruling A (lead→user): the daemon resolves owner/repo from `repo_id` (the D7/§59 pattern), so the UI
// sends inputs + a Repo `resource_ref` and NEVER names the GitHub owner/repo — removing a UI
// confused-deputy / owner-spoof surface. The go-live flip is per-action (ui-071 fork-1b) via the
// `enabledPrMutations` set + `isPrMutationEnabled`; both writes default DISABLED in production.

import type { ActionRequest } from "../contracts/index";
import { mintActionRequestId } from "./mint-id";

/** The PR-mutation action types gated behind the per-action `enabledPrMutations` set (cat-1) — SEPARATE
 *  from the already-live L2 `mutationsEnabled`. The `UdsGatewayPort` throw-never-invoke guard + the
 *  per-action enablement key off this set; both `github.merge_pr` (ui-070) + `github.submit_review`
 *  (ui-071) are members. */
export const PR_MUTATION_ACTION_TYPES: ReadonlySet<string> = new Set([
  "github.merge_pr",
  "github.submit_review",
]);

/** Per-action gate (ui-071 fork-1b): a PR mutation is enabled iff its action_type is in the port's
 *  `enabledPrMutations` set — so the future go-live can stage lowest-risk-first (enable one PR mutation
 *  without the other). Structurally typed (no GatewayPort import → no cycle). */
export function isPrMutationEnabled(
  gateway: { enabledPrMutations: ReadonlySet<string> },
  actionType: string,
): boolean {
  return gateway.enabledPrMutations.has(actionType);
}

/** The GitHub merge method (ui-077, USER-approved all three). The closed `{merge, squash, rebase}` union
 *  is the tsc-enforced UI closure; the daemon's `map_merge_method` (github_write.rs) is the runtime
 *  fail-closed authority (an unknown method is rejected). Rides the EXISTING live `github.merge_pr` gate —
 *  no new go-live / no new mutation type (a method parameter on an already-approved-per-action mutation). */
export type MergePrMethod = "merge" | "squash" | "rebase";

/** The merge-method selector's options + the builder's accepted values, sharing one closed source.
 *  GitHub-labeled; ordered merge → squash → rebase (GitHub's own order; `merge` is the default). The UI
 *  offers ONLY these and never re-validates beyond the closed set (the daemon is the authority). */
export const MERGE_METHODS: ReadonlyArray<{ value: MergePrMethod; label: string }> = [
  { value: "merge", label: "Merge commit" },
  { value: "squash", label: "Squash and merge" },
  { value: "rebase", label: "Rebase and merge" },
];

/** The review verdict (fork-2b: all three GitHub review actions). The daemon maps these → octocrab
 *  ReviewAction (daemon LESSON 61); `approve` carries branch-protection merge-gate power. */
export type ReviewEvent = "approve" | "request_changes" | "comment";

/**
 * Assemble the typed `ActionRequest` the UI SUBMITS to merge a PR (cat-1; the UI submits an intent,
 * NEVER executes — INV-SEC-1 / §4.2 law 1: the daemon Gateway is the single executor + DB writer).
 * Mirrors `buildHunkActionRequest`: the CLIENT mints `action_request_id` (the daemon trusts the wire id
 * as the `action_requests` PK — only `session.create` mints daemon-side; an empty id collided on a 2nd
 * same-session mutation → AuditWriteFailed → precondition_stale, ui-080 LESSON §39); `risk_level` is a
 * NON-AUTHORITATIVE hint (catalog-authoritative [risk-3] + daemon-reconciled, NEVER displayed).
 *
 * Ruling A: `inputs` carries ONLY `{pr_number, sha, merge_method}` + a Repo `resource_ref` — the UI
 * never names the GitHub owner/repo (the daemon resolves them from `repo_id`). `sha` is the DISPLAYED
 * head_sha (submitted==displayed, [[19]]/D2 — the anti-race pin the daemon 409s against). `created_at`
 * is the UI submission time (a real fact, not an invented consequence).
 */
export function buildMergePrActionRequest(
  args: {
    repo_id: string;
    pr_number: number;
    head_sha: string;
    merge_method: MergePrMethod;
  },
  createdAt: string,
): ActionRequest {
  return {
    action_request_id: mintActionRequestId(),
    action_type: "github.merge_pr",
    requester_type: "user",
    requester_id: "current_user",
    resource_refs: [{ type: "repo", id: args.repo_id }],
    inputs: {
      pr_number: args.pr_number,
      sha: args.head_sha,
      merge_method: args.merge_method,
    },
    risk_level: 0, // non-authoritative hint — catalog-authoritative (risk-3), daemon-reconciled, never displayed
    status: "submitted",
    created_at: createdAt,
  };
}

/**
 * Assemble the typed `ActionRequest` the UI SUBMITS to submit a PR review (cat-1; pure submitter — the
 * daemon Gateway adjudicates + executes). Mirrors `buildMergePrActionRequest`.
 *
 * Ruling A: `inputs` carries `{pr_number, commit_id, event, body}` + a Repo `resource_ref` — the UI
 * never names owner/repo (daemon resolves from `repo_id`). `commit_id` is the DISPLAYED head_sha (the
 * reviewed head, [[19]]/D2 — the daemon 422/409s on a moved head). `body` is `.trim()`'d ([[33]] — a
 * whitespace body can't become the audited review content): `""` for approve-with-no-body, the trimmed
 * text otherwise (the daemon enforces non-empty for request_changes/comment, GitHub's 422 rule).
 */
export function buildSubmitReviewActionRequest(
  args: {
    repo_id: string;
    pr_number: number;
    head_sha: string;
    event: ReviewEvent;
    body: string;
  },
  createdAt: string,
): ActionRequest {
  return {
    action_request_id: mintActionRequestId(),
    action_type: "github.submit_review",
    requester_type: "user",
    requester_id: "current_user",
    resource_refs: [{ type: "repo", id: args.repo_id }],
    inputs: {
      pr_number: args.pr_number,
      commit_id: args.head_sha,
      event: args.event,
      body: args.body.trim(),
    },
    risk_level: 0, // non-authoritative hint — catalog-authoritative (risk-3), daemon-reconciled, never displayed
    status: "submitted",
    created_at: createdAt,
  };
}
