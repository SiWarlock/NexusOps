import { describe, it, expect, vi } from "vitest";
import { enrichApproval, enrichHunkAction } from "./display-meta";
import { makeApprovalRow } from "../projections/fixtures/proj_approval_queue";
import {
  BoundaryValidationError,
  parseProjectionPage,
} from "../gateway-client/boundary";
import type { GatewayPort } from "../gateway-client/types";
import type { ActionAck, PolicyDecision, ProjectionName } from "../contracts/index";

// 053 Layer C — the approval card sources the daemon's AUTHORITATIVE risk + policy from the frozen
// ApprovalQueueRow (LESSON 17 — never UI-derived/fixture risk; resolves the 044 [med]). The
// mutation submit stays L2-HELD (the swap is a read-shape change; the seam is unchanged).

const realPolicy: PolicyDecision = {
  status: "deny",
  reasons: ["The daemon's real policy decision."],
  required_approvals: [{ kind: "project_owner" }],
  constraints: ["no-force-push"],
  safer_alt: "open a PR instead",
};

describe("enrichApproval — real-row risk/policy swap (053 Layer C)", () => {
  it("sources real risk + policy_decision from the row (not a fixture side-map)", () => {
    const row = makeApprovalRow({
      approval_id: "appr_real_1",
      risk_level: 4,
      policy_decision: realPolicy,
    });

    const { approval, policyDecision } = enrichApproval(row);

    expect(approval.risk_level).toBe(4); // the ROW's real risk
    expect(policyDecision).toEqual(realPolicy); // the ROW's real policy, verbatim
    expect(approval.approval_id).toBe("appr_real_1");
    expect(approval.status).toBe(row.status);
  });

  it("no_fixture_risk_on_real_path — a known fixture approval_id still reads the ROW's risk", () => {
    // approval_fixture_1's OLD gatewayApprovalEnrichment side-map had risk_level 3; the swap must
    // read the ROW's risk (here 0), proving no fixture risk leaks onto the card.
    const row = makeApprovalRow({ approval_id: "approval_fixture_1", risk_level: 0 });
    expect(enrichApproval(row).approval.risk_level).toBe(0);
  });

  it("absent policy_decision → honest pending (real risk, never a fabricated decision)", () => {
    const row = makeApprovalRow({ risk_level: 2, policy_decision: null });
    const { approval, policyDecision } = enrichApproval(row);
    // the risk is the ROW's real value (never fabricated, forbidden #2/#4)…
    expect(approval.risk_level).toBe(2);
    // …and the absent policy is a transparent "awaiting" placeholder, not an invented decision.
    expect(policyDecision.reasons.join(" ")).toMatch(/awaiting|pending/i);
    // the placeholder's required_approvals echoes the fallback approver (current_user) — pinned so a
    // future fallback change is caught.
    expect(policyDecision.required_approvals).toEqual([{ kind: "current_user" }]);
  });
});

// ── 053b — the per-hunk half completes the 044 [med] ─────────────────────────
// enrichHunkAction (the DiffReview stage/unstage/discard path) has NO row in hand — the ack carries
// an action_request_id, not an approval_id. It sources the daemon's AUTHORITATIVE risk/policy by a
// get_projection("ApprovalQueue") re-fetch + EXACT action_request_id match (reusing enrichApproval on
// a hit); an absent row → an honest awaiting placeholder, never a fabricated/UI-derived risk
// (LESSON 17, forbidden #2). parse-don't-trust on the re-fetch (LESSON 22). The per-hunk submit stays
// L2-HELD — enrichHunkAction reaches ONLY get_projection (Pick<…,"get_projection"> = no mutation).

const A_REAL_POLICY: PolicyDecision = {
  status: "deny",
  reasons: ["The daemon's real per-hunk policy decision."],
  required_approvals: [{ kind: "project_owner" }],
  constraints: ["no-force-push"],
  safer_alt: "stage the file instead",
};

/** A read-only gateway slice serving `raw` THROUGH the real boundary (parse-don't-trust, symmetric
 *  with MockGatewayPort.get_projection). Exposes ONLY get_projection — passing it to enrichHunkAction
 *  proves at compile time there is no mutation reach. */
function readGateway(raw: unknown) {
  const get_projection = vi.fn(async (name: ProjectionName) =>
    parseProjectionPage(name, raw),
  );
  return { get_projection } as Pick<GatewayPort, "get_projection"> & {
    get_projection: typeof get_projection;
  };
}

const approvalQueuePage = (rows: unknown[]) => ({
  projection: "ApprovalQueue",
  rows,
  cursor: null,
});

describe("enrichHunkAction — per-hunk real-row risk/policy swap (053b)", () => {
  it("enrich_hunk_action_sources_real_risk_by_action_request_id", async () => {
    // spec(§11.5/LESSON 17) — matches the queue row by the ack's action_request_id and reads the
    // ROW's real risk + policy (reusing enrichApproval), NOT the old UI-derived fixture risk.
    const ack: ActionAck = { action_request_id: "ar_live_7", status: "submitted" };
    const row = makeApprovalRow({
      approval_id: "appr_live_7",
      action_request_id: "ar_live_7",
      risk_level: 4,
      policy_decision: A_REAL_POLICY,
    });
    const gw = readGateway(approvalQueuePage([row]));

    const { approval, policyDecision } = await enrichHunkAction(gw, ack);

    expect(gw.get_projection).toHaveBeenCalledWith("ApprovalQueue");
    expect(approval.risk_level).toBe(4); // the ROW's real risk (was the fixture 2/3)
    expect(policyDecision).toEqual(A_REAL_POLICY); // the ROW's real policy, verbatim
    expect(approval.approval_id).toBe("appr_live_7"); // the real row id (not appr_for_…)
    expect(approval.action_request_id).toBe("ar_live_7");
  });

  it("enrich_hunk_action_absent_row_is_honest_pending", async () => {
    // spec(§11.7/forbidden#2) — no matching row (timing: the daemon mints the row async, the
    // ApprovalQueue isn't subscribed) → an honest awaiting placeholder, never a fabricated/
    // UI-derived risk or decision. The card's SHOWN risk comes from the live preview_action; this
    // risk_level is a fail-safe NON-displayed structural placeholder (the max, never under-warns).
    const ack: ActionAck = { action_request_id: "ar_not_in_queue", status: "submitted" };
    const gw = readGateway(approvalQueuePage([]));

    const { approval, policyDecision } = await enrichHunkAction(gw, ack);

    expect(approval.status).toBe("awaiting_approval");
    expect(approval.action_request_id).toBe("ar_not_in_queue"); // the ack's id, preserved
    expect(approval.scope).toBe("single_action"); // per-hunk is NEVER standing-grantable (LESSON §32)
    expect(approval.risk_level).toBe(4); // fail-safe non-displayed placeholder (Step-2.5 Q2)
    // an honest "awaiting" placeholder — never an invented per-action decision…
    expect(policyDecision.reasons.join(" ")).toMatch(/awaiting|pending/i);
    // …and NONE of the old UI-derived per-hunk reasons leak ("discards a hunk"/"git index").
    expect(policyDecision.reasons.join(" ")).not.toMatch(/discard|hunk|git index/i);
  });

  it("enrich_hunk_action_match_is_exact_not_first_row", async () => {
    // spec(security) — a DIFFERENT row is present but the target is absent: the EXACT
    // action_request_id match must NOT surface the wrong approval's risk/policy (no fuzzy/
    // first-row fallback — a wrong human-approval risk must never reach the card).
    const ack: ActionAck = { action_request_id: "ar_target", status: "submitted" };
    const wrongRow = makeApprovalRow({
      approval_id: "appr_wrong",
      action_request_id: "ar_other",
      risk_level: 4,
      policy_decision: A_REAL_POLICY,
    });
    const gw = readGateway(approvalQueuePage([wrongRow]));

    const { approval, policyDecision } = await enrichHunkAction(gw, ack);

    expect(approval.approval_id).not.toBe("appr_wrong"); // never the wrong row's identity
    expect(policyDecision).not.toEqual(A_REAL_POLICY); // never the wrong row's policy
    expect(policyDecision.reasons.join(" ")).toMatch(/awaiting|pending/i); // falls to honest absent
  });

  it("enrich_hunk_action_null_row_action_request_id_never_matches", async () => {
    // spec(security) — ApprovalQueueRow.action_request_id is Option<String> (nullable in the frozen
    // shadow). A row with a NULL action_request_id must NEVER match the per-hunk ack: the match
    // requires non-null exact string equality — never a null==null / undefined-coercion collapse
    // (which would surface a wrong, unrelated approval's risk/policy on the per-hunk card).
    const ack: ActionAck = { action_request_id: "ar_target", status: "submitted" };
    const nullRow = makeApprovalRow({
      approval_id: "appr_nullar",
      action_request_id: null,
      risk_level: 4,
      policy_decision: A_REAL_POLICY,
    });
    const gw = readGateway(approvalQueuePage([nullRow]));

    const { approval, policyDecision } = await enrichHunkAction(gw, ack);

    expect(approval.approval_id).not.toBe("appr_nullar"); // the null-id row never surfaces
    expect(policyDecision).not.toEqual(A_REAL_POLICY); // never the null-id row's policy
    expect(policyDecision.reasons.join(" ")).toMatch(/awaiting|pending/i); // falls to honest absent
  });

  it("enrich_hunk_re_fetch_parses_at_boundary", async () => {
    // spec(LESSON 22) — a malformed ApprovalQueue payload fails CLOSED at the boundary
    // (BoundaryValidationError); enrichHunkAction propagates it, never fabricating a row.
    const ack: ActionAck = { action_request_id: "ar_x", status: "submitted" };
    const gw = readGateway(
      approvalQueuePage([{ approval_id: "x" /* missing the 13 required fields */ }]),
    );

    await expect(enrichHunkAction(gw, ack)).rejects.toBeInstanceOf(
      BoundaryValidationError,
    );
  });

  it("enrich_hunk_action_no_mutation_reach_l2_held", async () => {
    // spec(INV-SEC-1) — enrichHunkAction reaches ONLY get_projection (a wired L1 read); it never
    // touches a mutation method. A read-only Pick<…,"get_projection"> handle (no submit/approve/
    // deny) suffices → the per-hunk submit stays L2-HELD; the swap adds zero mutation reach.
    const ack: ActionAck = { action_request_id: "ar_live_7", status: "submitted" };
    const row = makeApprovalRow({ action_request_id: "ar_live_7", risk_level: 1 });
    const gw = readGateway(approvalQueuePage([row]));

    const result = await enrichHunkAction(gw, ack);

    expect(result.approval.risk_level).toBe(1); // completed using get_projection alone
    expect(gw.get_projection).toHaveBeenCalledTimes(1);
  });
});
