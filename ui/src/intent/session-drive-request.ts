// The cockpit session-DRIVE intent surfaces (WAVE-1 W1-C-b) — Send message / Pause monitoring / Resume
// monitoring.
//
// All three are GENERIC submit_action actions (no dedicated methods); the executors are LIVE + registered
// (execute_send_message; execute_route for pause/resume — session_executor.rs:369/333/407-409). All key on
// the target session resource_ref (validated_target → SessionId::parse). They are APPROVAL-GATED (risk≥1,
// NOT auto-execute — only risk-0 auto-executes): the submit returns awaiting_approval → the live
// ApprovalQueue → the user approves via the existing card. The UI submits the intent ONLY; the daemon
// Gateway + the approval are the INV-SEC-1 chokepoint (§4.2 law 1).
//
// 🔴 SOFT pause (daemon LESSON 71): MVP session.pause gates the cockpit's drive/observe loop — it does NOT
// OS-suspend the agent (the agent keeps running; its tool calls are still Gateway-intercepted → INV-SEC-1
// holds). The control is honestly labeled "Pause monitoring" (NEVER "stop"/"suspend"). The real OS-suspend
// is a deferred follow-on.
//
// Catalog: session.send_message = risk-2, I::FromInputs (REQUIRED non-empty inputs.message; empty → daemon
// rejects). session.pause = risk-1, NaturalResourceRef. session.resume = risk-2, NaturalResourceRef. All
// CLIENT-mint the action_request_id (empty-PK LESSON §39; the session.kill/profile_change precedent). Held
// behind the default-OFF `enabledSessionControls` Set gate (gateway-client) until a USER cat-1 sign-off.

import type { ActionRequest } from "../contracts/index";
import { mintActionRequestId } from "./mint-id";

export const SESSION_SEND_MESSAGE_ACTION_TYPE = "session.send_message";
export const SESSION_PAUSE_ACTION_TYPE = "session.pause";
export const SESSION_RESUME_ACTION_TYPE = "session.resume";

/** The drive action types gated behind the default-OFF `enabledSessionControls` Set (the enabledPrMutations
 *  mirror — a SET, so it seeds the per-control boolean consolidation). The transport throw-never-invoke
 *  guard + the per-control enablement key off this set. */
export const SESSION_DRIVE_ACTION_TYPES: ReadonlySet<string> = new Set([
  SESSION_SEND_MESSAGE_ACTION_TYPE,
  SESSION_PAUSE_ACTION_TYPE,
  SESSION_RESUME_ACTION_TYPE,
]);

/** The per-control drive gate reader (the `isPrMutationEnabled` mirror): a drive control is enabled iff its
 *  action_type is in the port's `enabledSessionControls` set. Structurally typed (no GatewayPort import). */
export function isSessionControlEnabled(
  gateway: { enabledSessionControls: ReadonlySet<string> },
  actionType: string,
): boolean {
  return gateway.enabledSessionControls.has(actionType);
}

/**
 * Assemble the `session.send_message` intent (risk-2, FromInputs) — routes a prompt to the live session as
 * `SessionCommand::SendMessage(text)`. `inputs.message` is `.trim()`'d (defense-in-depth; the control blocks
 * an empty message first, the daemon FromInputs rejects empty too). CLIENT-mints the id (LESSON §39).
 */
export function buildSendMessageActionRequest(
  args: { session_id: string; message: string },
  createdAt: string,
): ActionRequest {
  return {
    action_request_id: mintActionRequestId(),
    action_type: SESSION_SEND_MESSAGE_ACTION_TYPE,
    requester_type: "user",
    requester_id: "current_user",
    resource_refs: [{ type: "session", id: args.session_id }],
    inputs: { message: args.message.trim() },
    risk_level: 2, // non-authoritative hint — catalog-authoritative (risk-2, approval-gated)
    status: "submitted",
    created_at: createdAt,
  };
}

/**
 * Assemble the `session.pause` intent (risk-1, NaturalResourceRef) — the SOFT "Pause monitoring" (gates the
 * cockpit drive loop, NOT an OS-suspend; daemon LESSON 71). The session resource_ref is the only operand.
 */
export function buildPauseActionRequest(
  args: { session_id: string },
  createdAt: string,
): ActionRequest {
  return {
    action_request_id: mintActionRequestId(),
    action_type: SESSION_PAUSE_ACTION_TYPE,
    requester_type: "user",
    requester_id: "current_user",
    resource_refs: [{ type: "session", id: args.session_id }],
    inputs: {},
    risk_level: 1, // non-authoritative hint — catalog-authoritative (risk-1, approval-gated)
    status: "submitted",
    created_at: createdAt,
  };
}

/**
 * Assemble the `session.resume` intent (risk-2, NaturalResourceRef) — "Resume monitoring" (re-opens the
 * cockpit drive loop). The session resource_ref is the only operand.
 */
export function buildResumeActionRequest(
  args: { session_id: string },
  createdAt: string,
): ActionRequest {
  return {
    action_request_id: mintActionRequestId(),
    action_type: SESSION_RESUME_ACTION_TYPE,
    requester_type: "user",
    requester_id: "current_user",
    resource_refs: [{ type: "session", id: args.session_id }],
    inputs: {},
    risk_level: 2, // non-authoritative hint — catalog-authoritative (risk-2, approval-gated)
    status: "submitted",
    created_at: createdAt,
  };
}
