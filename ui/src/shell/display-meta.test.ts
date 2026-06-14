import { describe, it, expect } from "vitest";
import { enrichApproval } from "./display-meta";
import { makeApprovalRow } from "../projections/fixtures/proj_approval_queue";
import type { PolicyDecision } from "../contracts/index";

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
