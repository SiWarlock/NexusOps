// The cockpit "Change profile" (session.profile_change) intent surface (WAVE-1 W1-C-a).
//
// session.profile_change is a GENERIC submit_action action (no dedicated method; the executor
// SessionExecutor::execute_profile_change is LIVE, session_executor.rs:199; arm :401; catalog
// shared/src/catalog.rs:96,251). UNLIKE Kill/Launch (risk-0 auto-execute), this is **risk-2 →
// APPROVAL-GATED** (the §15 #8 no-silent-account-hop human checkpoint): the submit returns
// `awaiting_approval` (NOT auto-execute), surfaces in the live ApprovalQueue, and applies only after a
// human approves it. UI/IPC-requester-only (PROFILE_MUTATION_TYPES, policy.rs:33 — deny-before-risk for
// Agent/Brain/Pack/System). The UI submits the intent ONLY; the daemon Gateway + the risk-2 approval are
// the INV-SEC-1 chokepoint (§4.2 law 1) — the UI never silently sets/changes a profile.
//
// execute_profile_change is NaturalResourceRef-keyed (resource_refs.first() → SessionId::parse, :207) +
// requires inputs.execution_profile_id (no default; parsed → ExecutionProfileId, fail-closed, :236). Like
// every generic submit_action intent, the CLIENT mints action_request_id (empty-PK LESSON §39, the
// session.kill precedent). Held behind its OWN default-OFF go-live gate (enabledProfileChange) — a session
// live-write does not auto-ride mutationsEnabled; it stays held until a USER cat-1 sign-off (defense-in-
// depth to the daemon's risk-2 approval; the enabledSessionKill / enabledPrMutations mirror).

import type { ActionRequest } from "../contracts/index";
import { mintActionRequestId } from "./mint-id";

/** The session.profile_change action_type — the generic submit_action gated behind the default-OFF
 *  `enabledProfileChange` port flag (W1-C-a). */
export const SESSION_PROFILE_CHANGE_ACTION_TYPE = "session.profile_change";

/** The per-action profile-change gate reader (the `isSessionKillEnabled` mirror): profile-change is
 *  enabled iff the port's `enabledProfileChange` flag is true. Structurally typed (no GatewayPort import). */
export function isProfileChangeEnabled(gateway: { enabledProfileChange: boolean }): boolean {
  return gateway.enabledProfileChange;
}

/**
 * Assemble the typed `ActionRequest` the UI SUBMITS to change a session's execution profile (the UI
 * submits an intent, NEVER executes — INV-SEC-1 / §4.2 law 1; the daemon adjudicates the risk-2 approval).
 * The session `resource_ref` keys the executor; `inputs.execution_profile_id` is REQUIRED (no default).
 * The CLIENT mints `action_request_id` (LESSON §39). `risk_level:2` is a NON-AUTHORITATIVE hint
 * (catalog-authoritative → approval-gated, the §15 #8 checkpoint; never displayed). `created_at` is the UI
 * submission time. Mirrors buildKillSessionActionRequest (+ the required execution_profile_id input).
 */
export function buildProfileChangeActionRequest(
  args: { session_id: string; execution_profile_id: string },
  createdAt: string,
): ActionRequest {
  return {
    action_request_id: mintActionRequestId(),
    action_type: SESSION_PROFILE_CHANGE_ACTION_TYPE,
    requester_type: "user",
    requester_id: "current_user",
    resource_refs: [{ type: "session", id: args.session_id }],
    inputs: { execution_profile_id: args.execution_profile_id },
    risk_level: 2, // non-authoritative hint — catalog-authoritative (risk-2, approval-gated), daemon-reconciled
    status: "submitted",
    created_at: createdAt,
  };
}
