// The github.merge_pr (Merge) cat-1 mutation intent surface (ui-070).
//
// Ruling A (lead→user): the daemon resolves owner/repo from `repo_id` (the D7/§59 pattern), so the UI
// sends `inputs:{pr_number, sha, merge_method}` + a Repo `resource_ref` and NEVER names the GitHub
// owner/repo — removing a UI confused-deputy / owner-spoof surface.

import type { ActionRequest } from "../contracts/index";

/** The PR-mutation action types gated behind the SEPARATE `prMutationsEnabled` flag (cat-1) — distinct
 *  from the already-live L2 `mutationsEnabled`. A Set so the `UdsGatewayPort` throw-never-invoke guard +
 *  the future `github.submit_review` cat-1 slice reuse it. */
export const PR_MUTATION_ACTION_TYPES: ReadonlySet<string> = new Set(["github.merge_pr"]);

/** The merge method (user-ruled: fixed merge-commit; the squash/rebase selector is DEFERRED). A
 *  one-member union now, extensible when the selector lands — the type enforces the ruling. */
export type MergePrMethod = "merge";

/**
 * Assemble the typed `ActionRequest` the UI SUBMITS to merge a PR (cat-1; the UI submits an intent,
 * NEVER executes — INV-SEC-1 / §4.2 law 1: the daemon Gateway is the single executor + DB writer).
 * Mirrors `buildHunkActionRequest`: the daemon mints `action_request_id` (empty here); `risk_level` is a
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
    action_request_id: "", // daemon mints
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
