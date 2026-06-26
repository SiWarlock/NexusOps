// The cockpit "Kill" (session.kill) intent surface (WAVE-1 Slice B).
//
// session.kill is a GENERIC submit_action action — there is NO dedicated IPC method (unlike Slice A's
// session.create). The executor SessionExecutor::execute_kill is LIVE + registered
// (session_executor.rs:274; SESSION_KILL arm :317); it is risk-0 AUTO-EXECUTE in the catalog
// (shared/src/catalog.rs:95,215; policy.rs:58) → NO approval modal (like project.rescan / session.create).
// execute_kill is NaturalResourceRef-keyed: it reads req.resource_refs.first() → SessionId::parse(rref.id)
// (session_executor.rs:279-293), so the intent carries resource_refs:[{type:"session", id:<session_id>}]
// and `inputs` is empty (the resource_ref is the only operand). A gone/unknown session → a clean no-op
// (Succeeded{side_effect_applied:false}), NOT an error. The UI submits the intent; the daemon Gateway is
// the single executor + INV-SEC-1 chokepoint (§4.2 law 1).
//
// Like every generic submit_action intent, the CLIENT mints action_request_id (the daemon trusts the
// wire id verbatim as the action_requests PK — only session.create mints daemon-side; an empty id
// collided on a 2nd same-session mutation → AuditWriteFailed → precondition_stale, ui-080 LESSON §39).
//
// Held behind its OWN default-OFF go-live gate (enabledSessionKill, gateway-client) — a destructive
// live-write does not auto-ride the already-live mutationsEnabled flip; it stays held until a USER cat-1
// sign-off (the enabledSessionLaunch / enabledPrMutations mirror).

import type { ActionRequest } from "../contracts/index";
import { mintActionRequestId } from "./mint-id";

/** The session.kill action_type — the generic submit_action gated behind the default-OFF
 *  `enabledSessionKill` port flag (Slice B). */
export const SESSION_KILL_ACTION_TYPE = "session.kill";

/** The per-action agent-kill gate reader (the `isPrMutationEnabled` mirror): kill is enabled iff the
 *  port's `enabledSessionKill` flag is true. Structurally typed (no GatewayPort import → no cycle). */
export function isSessionKillEnabled(gateway: { enabledSessionKill: boolean }): boolean {
  return gateway.enabledSessionKill;
}

/**
 * Assemble the typed `ActionRequest` the UI SUBMITS to kill a session (the cockpit Kill control; the UI
 * submits an intent, NEVER executes — INV-SEC-1 / §4.2 law 1). The session `resource_ref` is the only
 * operand (execute_kill keys off it); `inputs` is empty. The CLIENT mints `action_request_id` (LESSON
 * §39 — an empty PK collides on a 2nd same-session mutation → precondition_stale). `risk_level` is a
 * NON-AUTHORITATIVE hint (catalog-authoritative [risk-0], daemon-reconciled, never displayed). `created_at`
 * is the UI submission time (a real fact, not an invented consequence). Mirrors buildRescanProjectActionRequest.
 */
export function buildKillSessionActionRequest(
  args: { session_id: string },
  createdAt: string,
): ActionRequest {
  return {
    action_request_id: mintActionRequestId(),
    action_type: SESSION_KILL_ACTION_TYPE,
    requester_type: "user",
    requester_id: "current_user",
    resource_refs: [{ type: "session", id: args.session_id }],
    inputs: {},
    risk_level: 0, // non-authoritative hint — catalog-authoritative (risk-0), daemon-reconciled
    status: "submitted",
    created_at: createdAt,
  };
}
