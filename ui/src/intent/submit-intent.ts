// The intent seam (§6.1/§6.2) — the UI's FIRST mutation path, built in ISOLATION
// (Scope A). INV-SEC-1 / §4.2 law 1: the UI SUBMITS intents only, NEVER executes;
// every mutation routes through the single `GatewayPort` (forbidden #3). The seam is:
//   • fail-closed — when `canSubmit()` is false it returns a typed read-only rejection
//     and NEVER calls the GatewayPort. This is DEFENSE-IN-DEPTH, not the sole guard:
//     the daemon's Action Gateway (INV-SEC-1) is the load-bearing chokepoint (Lesson §4).
//   • non-optimistic — `ok` carries the daemon's reported value (its `ActionAck.status`);
//     the seam NEVER synthesizes "succeeded" (a confirmed "done" comes only from a
//     projection / `ActionResult`, a slice-2 concern) (§4.2 law 2 / §11.7).
//   • verbatim on errors — surfaces the daemon's `IpcErrorCode` (`WireError`) as-is,
//     never collapsed/remapped (§6.4; the consumer routes each to its §11.5 card,
//     `fencing_conflict` stays never-auto-resolved #6).
//   • stateless — holds NO intent state across a disconnect (no cache, no auto-retry);
//     `canSubmit` is re-read per call (Q7-A; B/C cache-retry PARKED-for-user).
import { useMemo } from "react";
import { WireError as WireErrorSchema } from "../contracts/index";
import type {
  ActionAck,
  ActionPreview,
  ActionRequest,
  Approval,
  WireError,
} from "../contracts/index";
import type { GatewayPort } from "../gateway-client/types";
import { canSubmitIntent, useConnectionStatus } from "../connection/read-only";

/**
 * The honest, non-optimistic result of an intent submission:
 *   • `ok`       — the DAEMON's reported value (never a UI-synthesized success).
 *   • `error`    — the daemon's `IpcErrorCode`, VERBATIM (the consumer routes the card).
 *   • `readOnly` — the fail-safe gate refused; the `GatewayPort` was NEVER called.
 */
export type IntentResult<T> =
  | { ok: T }
  | { error: WireError }
  | { readOnly: true };

export interface IntentSeam {
  submitAction(request: ActionRequest): Promise<IntentResult<ActionAck>>;
  previewAction(actionRequestId: string): Promise<IntentResult<ActionPreview>>;
  // approve/deny ACCEPT the daemon's rendered `Approval` and carry its `approval_id`
  // VERBATIM to the wire (the GatewayPort takes the id; the daemon owns the record).
  // The seam never synthesizes an id or a risk/decision (Q4 — policy-from-daemon).
  approve(approval: Approval): Promise<IntentResult<ActionAck>>;
  deny(approval: Approval, reason: string): Promise<IntentResult<ActionAck>>;
}

/**
 * Build the intent seam over a `GatewayPort`. `canSubmit` is RE-EVALUATED per call (the
 * seam holds no state — Q7), so a disconnect between calls flips it closed with nothing
 * cached or replayed. The seam exposes ONLY the §6.1 intent methods — there is no
 * executor here (the daemon executes).
 */
export function createIntentSeam(
  port: GatewayPort,
  canSubmit: () => boolean,
): IntentSeam {
  async function guarded<T>(call: () => Promise<T>): Promise<IntentResult<T>> {
    // Fail-closed: when the gate is false the GatewayPort is NEVER called.
    if (!canSubmit()) return { readOnly: true };
    try {
      return { ok: await call() };
    } catch (e) {
      // A real runtime / transport failure is thrown as an `Error` INSTANCE — a daemon
      // error frame is NEVER an Error (it arrives as deserialized plain data). Re-throw
      // any Error so a real bug is never swallowed as `{error}` (a fake non-success):
      // an `instanceof` discriminant, NOT a structural parse — because `Error`'s
      // message/stack are non-enumerable, so an `Error` carrying a colliding `code`
      // would slip a keys-only parse (even a strict one).
      if (e instanceof Error) throw e;
      // Only a plain daemon error frame reaches here. Surface its §6.4 IpcErrorCode
      // VERBATIM — never collapse/remap (#6); the strict shadow rejects extra keys.
      const wire = WireErrorSchema.safeParse(e);
      if (wire.success) return { error: wire.data };
      // Anything else (a non-Error, non-WireError throw) is also a bug — never swallowed.
      throw e;
    }
  }
  return {
    submitAction: (request) => guarded(() => port.submit_action(request)),
    previewAction: (actionRequestId) =>
      guarded(() => port.preview_action(actionRequestId)),
    // Carry the daemon's approval_id VERBATIM (extracted from the rendered Approval).
    approve: (approval) => guarded(() => port.approve(approval.approval_id)),
    deny: (approval, reason) => guarded(() => port.deny(approval.approval_id, reason)),
  };
}

/**
 * React hook over `createIntentSeam`: re-derives the fail-safe gate from the CURRENT
 * connection status each render (and the seam re-reads it per call), so no stale
 * decision is ever cached.
 */
export function useSubmitIntent(port: GatewayPort): IntentSeam {
  const status = useConnectionStatus();
  return useMemo(
    () => createIntentSeam(port, () => canSubmitIntent(status)),
    [port, status],
  );
}
