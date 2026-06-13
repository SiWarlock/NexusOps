import { describe, it, expect, vi } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { createIntentSeam } from "./submit-intent";
import { MockGatewayPort } from "../gateway-client/mock";
import {
  ActionAck,
  ActionPreview,
  ActionRequest,
  ActorRefBody,
  Approval,
  RequiredApprover,
  ResourceRef,
} from "../contracts/index";

// A minimal, contract-valid ActionRequest the seam SUBMITS (the daemon owns lifecycle).
const REQUEST: ActionRequest = {
  action_request_id: "ar_test_0001",
  action_type: "git.commit",
  requester_type: "user",
  requester_id: "u_1",
  resource_refs: [{ type: "worktree", id: "wt_1" }],
  inputs: { message: "wip" },
  risk_level: 1,
  status: "submitted",
  created_at: "2026-06-13T00:00:00Z",
};

// A contract-valid Approval the human submits (the daemon's policy owns the risk).
const APPROVAL: Approval = {
  approval_id: "ap_test_0001",
  required_approver: { kind: "current_user" },
  status: "approved",
  scope: "single_action",
  risk_level: 1,
  action_request_id: "ar_test_0001",
};

const open = () => true;
const closed = () => false;

describe("intent seam — cat-1 safety pins", () => {
  it("intent_seam_submits_only_no_executor", () => {
    // spec(§4.2/§15) — INV-SEC-1 / §4.2 law 1: the seam exposes ONLY the §6.1 intent
    // methods (no executor), and every mutation routes through the single GatewayPort
    // (forbidden #3). A UI execution path must not exist.
    const port = new MockGatewayPort();
    const seam = createIntentSeam(port, open);
    expect(Object.keys(seam).toSorted()).toEqual(
      ["approve", "deny", "previewAction", "submitAction"].toSorted(),
    );
    // No method whose name implies local execution.
    for (const k of Object.keys(seam)) {
      expect(k).not.toMatch(/execut|perform|apply|commit|write|mutate/i);
    }
  });

  it("submit_blocked_when_canSubmitIntent_false", async () => {
    // spec(§11.1/§15) — fail-closed: when the gate is false the seam returns a typed
    // read-only rejection and NEVER calls the GatewayPort (defense-in-depth; the daemon
    // Gateway is the real chokepoint).
    const port = new MockGatewayPort();
    const spies = [
      vi.spyOn(port, "submit_action"),
      vi.spyOn(port, "preview_action"),
      vi.spyOn(port, "approve"),
      vi.spyOn(port, "deny"),
    ];
    const seam = createIntentSeam(port, closed);
    expect(await seam.submitAction(REQUEST)).toEqual({ readOnly: true });
    expect(await seam.previewAction(REQUEST.action_request_id)).toEqual({ readOnly: true });
    expect(await seam.approve(APPROVAL)).toEqual({ readOnly: true });
    expect(await seam.deny(APPROVAL, "no")).toEqual({ readOnly: true });
    for (const spy of spies) expect(spy).not.toHaveBeenCalled();
  });

  it("submit_never_reports_done_from_ack_alone", async () => {
    // spec(§4.2-law2/§11.7) — the seam surfaces the daemon's ActionAck.status VERBATIM;
    // it NEVER synthesizes "succeeded". A non-terminal daemon status stays non-terminal.
    const port = new MockGatewayPort();
    const result = await createIntentSeam(port, open).submitAction(REQUEST);
    expect(result).toEqual({ ok: { action_request_id: "ar_mock_0001", status: "submitted" } });
    if ("ok" in result) {
      expect(result.ok.status).toBe("submitted"); // the daemon-reported status, not "succeeded"
      expect(result.ok.status).not.toBe("succeeded");
    }
  });

  it("approve_deny_carry_daemon_approval_no_ui_risk", async () => {
    // spec(§6.2) — approve/deny carry the daemon's `approval_id` VERBATIM to the wire
    // (extracted from the rendered Approval; the GatewayPort takes the id, the daemon
    // owns the record). The seam computes NO risk / approval-requirement / id of its
    // own (the policy engine is authoritative).
    const port = new MockGatewayPort();
    const approveSpy = vi.spyOn(port, "approve");
    const denySpy = vi.spyOn(port, "deny");
    const seam = createIntentSeam(port, open);
    await seam.approve(APPROVAL);
    await seam.deny(APPROVAL, "not now");
    expect(approveSpy).toHaveBeenCalledWith(APPROVAL.approval_id);
    expect(denySpy).toHaveBeenCalledWith(APPROVAL.approval_id, "not now");
    // The id passed is the daemon's, verbatim — never a UI-synthesized one.
    expect(approveSpy.mock.calls[0]![0]).toBe(APPROVAL.approval_id);
  });

  it("preview_surfaces_daemon_preview_never_synthesized", async () => {
    // spec(forbidden#2) — previewAction surfaces the daemon's ActionPreview consequences
    // VERBATIM (incl. cannot_preview_reason); the seam fabricates nothing.
    const port = new MockGatewayPort();
    const result = await createIntentSeam(port, open).previewAction(REQUEST.action_request_id);
    expect("ok" in result).toBe(true);
    if ("ok" in result) {
      // Exactly the daemon's preview — the seam added no summary / consequence of its own.
      expect(result.ok).toEqual(await port.preview_action(REQUEST.action_request_id));
      // Pin a concrete daemon-sourced field so the assertion isn't a self-referential
      // round-trip: the summary is the daemon's verbatim, never UI-synthesized.
      expect(result.ok.summary).toBe("Would modify 1 file in the worktree.");
    }
  });

  it("seam_surfaces_daemon_error_code_verbatim", async () => {
    // spec(§6.4) — the daemon's IpcErrorCode (WireError) is surfaced VERBATIM, never
    // collapsed/remapped; fencing_conflict stays fencing_conflict (the consumer routes
    // it to the never-auto-resolved card #6).
    const port = new MockGatewayPort({ mutationError: { code: "fencing_conflict" } });
    const seam = createIntentSeam(port, open);
    expect(await seam.submitAction(REQUEST)).toEqual({ error: { code: "fencing_conflict" } });
    // A different code is likewise passed through unchanged.
    const port2 = new MockGatewayPort({ mutationError: { code: "internal_error" } });
    expect(await createIntentSeam(port2, open).approve(APPROVAL)).toEqual({
      error: { code: "internal_error" },
    });
  });

  it("seam_holds_no_intent_state_across_disconnect", async () => {
    // spec(§6.1-no-authoritative-state) — the seam holds NO intent state: the gate is
    // re-read per call, so a disconnect between calls flips it closed with nothing
    // cached or auto-replayed (Q7-A).
    let gateOpen = true;
    const port = new MockGatewayPort();
    const spy = vi.spyOn(port, "submit_action");
    const seam = createIntentSeam(port, () => gateOpen);

    const first = await seam.submitAction(REQUEST);
    expect("ok" in first).toBe(true);
    expect(spy).toHaveBeenCalledTimes(1);

    gateOpen = false; // a "disconnect" between calls
    const second = await seam.submitAction(REQUEST);
    expect(second).toEqual({ readOnly: true });
    expect(spy).toHaveBeenCalledTimes(1); // NOT re-called / no replay of the held intent
  });

  it("seam_rethrows_non_wireerror_never_fake_success", async () => {
    // spec(§6.4/§11.7) — a thrown value that is NOT a daemon WireError (a real bug / a
    // transport Error) is RE-THROWN, never swallowed as {error} or surfaced as a fake
    // success. WireError is `.strict()` (== the frozen additionalProperties:false), so
    // even an Error carrying a COLLIDING `code` (plus its message/stack) is rejected by
    // the parse and re-thrown — the §6.4-verbatim + fail-closed-on-real-error guarantees
    // rest on this exact-match classification.
    const port = new MockGatewayPort();
    vi.spyOn(port, "submit_action").mockRejectedValue(new Error("boom"));
    await expect(createIntentSeam(port, open).submitAction(REQUEST)).rejects.toThrow("boom");

    const colliding = Object.assign(new Error("transport down"), { code: "internal_error" });
    vi.spyOn(port, "approve").mockRejectedValue(colliding);
    await expect(createIntentSeam(port, open).approve(APPROVAL)).rejects.toBe(colliding);
  });

  it("mock_gateway_mutation_methods_return_frozen_shapes", async () => {
    // spec(§6.1) — the MockGatewayPort fixtures parse against the modeled frozen-shadow
    // validators (no invented shape leaks through the §14 test seam).
    const port = new MockGatewayPort();
    expect(ActionAck.safeParse(await port.submit_action(REQUEST)).success).toBe(true);
    expect(
      ActionPreview.safeParse(await port.preview_action(REQUEST.action_request_id)).success,
    ).toBe(true);
    expect(ActionAck.safeParse(await port.approve(APPROVAL.approval_id)).success).toBe(true);
    expect(ActionAck.safeParse(await port.deny(APPROVAL.approval_id, "x")).success).toBe(true);
  });
});

// The §2.5-seam drift-pin: the provisional frozen-SHADOW shapes must keep field-sets
// equal to the frozen schema (the same snapshot discipline as ServerFrame). This is what
// makes "no UI-invented shape" enforceable — a daemon field add/remove fails loudly.
const schemaPath = fileURLToPath(
  new URL("../../../shared/contracts/schema/nexusops-contract.schema.json", import.meta.url),
);

describe("intent contracts — frozen-shadow drift pin (§2.5-seam)", () => {
  it("intent_contracts_field_sets_match_frozen_schema", () => {
    // spec(§6.2) — provisional shadow field-sets == the frozen schema's object field-sets.
    const schema = JSON.parse(readFileSync(schemaPath, "utf8")) as {
      $defs: Record<string, { properties?: Record<string, unknown> }>;
    };
    const cases: [string, { shape: Record<string, unknown> }][] = [
      ["ActionAck", ActionAck],
      ["ActionPreview", ActionPreview],
      ["Approval", Approval],
      ["ActionRequest", ActionRequest],
      ["ResourceRef", ResourceRef],
      ["RequiredApprover", RequiredApprover],
      ["ActorRefBody", ActorRefBody],
    ];
    for (const [name, zod] of cases) {
      const frozen = Object.keys(schema.$defs[name]!.properties ?? {}).toSorted();
      expect(Object.keys(zod.shape).toSorted(), `field drift in ${name}`).toEqual(frozen);
    }
  });
});
