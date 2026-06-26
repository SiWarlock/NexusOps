// The cockpit "Add project" (project.rescan) intent surface (ui-track add-project wiring).
//
// project.rescan is a risk-0 AUTO-EXECUTE action (daemon gateway/policy.rs RISK0_AUTO_EXECUTE_ALLOWLIST)
// — so it executes without a human approval card, and the registered project surfaces via the
// ProjectActivity projection nudge. It is `requires_resource_refs=false` (daemon project/executor.rs):
// the scan PATH is the target, carried in inputs["path"], NOT a resource_ref. The UI submits the intent;
// the daemon Gateway is the single executor (INV-SEC-1 / §4.2 law 1). It rides the already-live L2
// mutation seam (`mutationsEnabled`); it is NOT a PR mutation, so NOT gated by `enabledPrMutations`.

import type { ActionRequest } from "../contracts/index";
import { mintActionRequestId, mintProjectId } from "./mint-id";

/**
 * Assemble the typed `ActionRequest` the UI SUBMITS to register/rescan a project (the "Add project"
 * flow). `inputs.path` is the selected repo path — `.trim()`'d so a whitespace-only pick can't reach
 * the daemon as the scan target (the daemon also fails CLOSED on a blank path; defense-in-depth,
 * mirrors LESSON §33). NO `resource_refs` (project.rescan is requires_resource_refs=false).
 *
 * The CLIENT mints the ids (the daemon trusts the wire id verbatim — only `session.create` mints
 * daemon-side): `action_request_id` is the `action_requests` PRIMARY KEY — sending `""` was the
 * `precondition_stale` bug (empty PK → collide on the 2nd Add → `AuditWriteFailed`, overloaded onto
 * the `precondition_stale` wire code). `project_id` rides the ENVELOPE — the daemon stamps it onto
 * `ProjectRescanned`; the projector skips a `None` project_id, so without it the project never
 * registers. `risk_level` is a NON-AUTHORITATIVE hint (catalog-authoritative [risk-0],
 * daemon-reconciled, never displayed) — mirrors `buildMergePrActionRequest`.
 *
 * MVP limit: a fresh `project_id` is minted per Add → re-adding the SAME path registers a NEW project
 * row (register-by-path dedup is the deferred §2.8 daemon-side work).
 */
export function buildRescanProjectActionRequest(
  args: { path: string },
  createdAt: string,
): ActionRequest {
  return {
    action_request_id: mintActionRequestId(),
    action_type: "project.rescan",
    requester_type: "user",
    requester_id: "current_user",
    resource_refs: [], // requires_resource_refs=false — the path is the target, not a resource_ref
    inputs: { path: args.path.trim() },
    risk_level: 0, // non-authoritative hint — catalog-authoritative (risk-0), daemon-reconciled
    status: "submitted",
    created_at: createdAt,
    project_id: mintProjectId(),
  };
}
