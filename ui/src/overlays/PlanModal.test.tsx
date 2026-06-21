// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from "vitest";
import { cleanup, render, screen, fireEvent, waitFor, within } from "@testing-library/react";

afterEach(cleanup);
import { PlanModal } from "./PlanModal";
import { GatewayOverlay } from "./GatewayOverlay";
import { ReadOnlyProvider, type ConnectionStatus } from "../connection/read-only";
import { MockGatewayPort } from "../gateway-client/mock";
import { makeApprovalRow } from "../projections/fixtures/proj_approval_queue";
import type { ActionPlan, ActionPlanStep, ActionRequest, Approval } from "../contracts/index";

const CONNECTED: ConnectionStatus = { connection: "connected", version: "compatible" };
const DEGRADED: ConnectionStatus = { connection: "connecting", version: "unknown" };

// The plan-level Approval the human submits (scope "plan", `plan_id` set, `action_request_id`
// NULL — a plan-level approve-all has no single action request; the daemon owns eligibility).
const PLAN_APPROVAL: Approval = {
  approval_id: "ap_plan_0001",
  required_approver: { kind: "current_user" },
  status: "awaiting_approval",
  scope: "plan",
  risk_level: 3,
  action_request_id: null,
  plan_id: "plan_0001",
};

function req(over: Partial<ActionRequest> & { action_request_id: string }): ActionRequest {
  return {
    action_request_id: over.action_request_id,
    action_type: over.action_type ?? "git.commit",
    requester_type: "project_brain",
    requester_id: "brain_1",
    resource_refs: over.resource_refs ?? [{ type: "worktree", id: "wt_1" }],
    inputs: {},
    risk_level: over.risk_level ?? 2,
    status: "awaiting_approval",
    created_at: "2026-06-21T00:00:00Z",
  };
}

function step(over: Partial<ActionPlanStep> & { step_id: string }): ActionPlanStep {
  return {
    step_id: over.step_id,
    label: over.label ?? `Step ${over.step_id}`,
    action_request: over.action_request ?? req({ action_request_id: `ar_${over.step_id}` }),
    required: over.required ?? true,
    can_skip: over.can_skip ?? false,
    status: over.status ?? "awaiting_approval",
  };
}

function plan(over: Partial<ActionPlan> = {}): ActionPlan {
  return {
    plan_id: over.plan_id ?? "plan_0001",
    title: over.title ?? "Refactor the auth module",
    steps: over.steps ?? [
      step({ step_id: "s1", label: "Create worktree", action_request: req({ action_request_id: "ar_s1", action_type: "git.create_worktree" }) }),
      step({ step_id: "s2", label: "Apply patch", action_request: req({ action_request_id: "ar_s2", action_type: "git.commit" }) }),
      step({ step_id: "s3", label: "Open PR", action_request: req({ action_request_id: "ar_s3", action_type: "github.create_pr" }) }),
    ],
    dependencies: over.dependencies ?? [],
    overall_risk: over.overall_risk ?? 3,
    approval_mode: over.approval_mode ?? "step_by_step",
  };
}

function renderModal(
  opts: { status?: ConnectionStatus; port?: MockGatewayPort; plan?: ActionPlan; approval?: Approval } = {},
) {
  const port = opts.port ?? new MockGatewayPort();
  const utils = render(
    <ReadOnlyProvider value={opts.status ?? CONNECTED}>
      <PlanModal
        plan={opts.plan ?? plan()}
        approval={opts.approval ?? PLAN_APPROVAL}
        port={port}
        onClose={() => {}}
      />
    </ReadOnlyProvider>,
  );
  return { port, ...utils };
}

const rowFor = (stepId: string) =>
  screen.getAllByTestId("plan-step").find((r) => r.getAttribute("data-step-id") === stepId)!;

describe("PlanModal — N-step ActionPlan render (§11.5)", () => {
  it("renders_n_steps_as_rows", () => {
    // spec(§11.5) — an N-step plan renders N step rows, each showing its action_type + label.
    renderModal({ plan: plan() });
    const rows = screen.getAllByTestId("plan-step");
    expect(rows).toHaveLength(3);
    expect(within(rowFor("s1")).getByText(/git\.create_worktree/)).toBeTruthy();
    expect(within(rowFor("s2")).getByText(/git\.commit/)).toBeTruthy();
    expect(within(rowFor("s3")).getByText(/github\.create_pr/)).toBeTruthy();
    expect(within(rowFor("s1")).getByText(/Create worktree/)).toBeTruthy();
  });

  it("header_renders_daemon_overall_risk_never_derived", () => {
    // spec(§11.5/LESSON 17) — the header risk is the daemon's plan-level `overall_risk`, NOT
    // recomputed from the steps: a plan whose steps are all risk 0 but `overall_risk` 4 shows 4.
    const lowSteps = [
      step({ step_id: "s1", action_request: req({ action_request_id: "ar_s1", risk_level: 0 }) }),
      step({ step_id: "s2", action_request: req({ action_request_id: "ar_s2", risk_level: 0 }) }),
    ];
    const first = renderModal({ plan: plan({ steps: lowSteps, overall_risk: 4 }) });
    expect(screen.getByTestId("plan-risk").textContent).toContain("4");
    first.unmount();
    // header title + approval_mode + plan_id are read from the daemon's plan, not invented.
    renderModal({ plan: plan({ title: "Wire the brain seam", approval_mode: "approve_all", plan_id: "plan_xyz" }) });
    expect(screen.getByTestId("plan-title").textContent).toContain("Wire the brain seam");
    expect(screen.getByTestId("plan-approval-mode").textContent).toContain("approve_all");
    expect(screen.getByTestId("plan-id").textContent).toContain("plan_xyz");
  });

  it("per_step_risk_absent_renders_honest_dash", () => {
    // spec(forbidden#2/LESSON 17) — a step whose daemon risk is absent renders "—", NEVER a
    // fabricated digit (the component derives no risk of its own).
    const noRisk = step({
      step_id: "s1",
      action_request: { ...req({ action_request_id: "ar_s1" }), risk_level: undefined as unknown as number },
    });
    renderModal({ plan: plan({ steps: [noRisk] }) });
    const risk = within(rowFor("s1")).getByTestId("step-risk");
    expect(risk.textContent).toContain("—");
    expect(risk.textContent).not.toMatch(/\d/); // no invented digit
  });

  it("per_step_policy_absent_is_honest_not_synthesized", () => {
    // spec(forbidden#2) — per-step policy_decision is daemon-NULL today (8.1 follow-on) → an
    // honest "pending"/absent surface, NEVER a synthesized consequence.
    renderModal({ plan: plan() });
    for (const id of ["s1", "s2", "s3"]) {
      const policy = within(rowFor(id)).getByTestId("step-policy");
      expect(policy.textContent).toMatch(/pending|not yet|—/i);
    }
  });
});

describe("PlanModal — plan-level + per-step approval controls (cat-1)", () => {
  it("approve_all_submits_plan_level_no_step_id", async () => {
    // spec(2.1c L3) — Approve-all submits the PLAN-LEVEL approve (`approve(approval_id)` with NO
    // step_id); the component computes NO per-step eligibility (critical-exclusion is daemon-authoritative).
    const port = new MockGatewayPort();
    const approveSpy = vi.spyOn(port, "approve");
    renderModal({ port });
    fireEvent.click(screen.getByTestId("approve-all"));
    await waitFor(() => expect(approveSpy).toHaveBeenCalled());
    expect(approveSpy.mock.calls[0]).toEqual([PLAN_APPROVAL.approval_id]); // length 1 — no step_id
  });

  it("approve_step_submits_step_id", async () => {
    // spec(§6.1) — a per-row approve submits `approve(approval_id, step_id)`.
    const port = new MockGatewayPort();
    const approveSpy = vi.spyOn(port, "approve");
    renderModal({ port });
    fireEvent.click(within(rowFor("s2")).getByTestId("approve-step"));
    await waitFor(() =>
      expect(approveSpy).toHaveBeenCalledWith(PLAN_APPROVAL.approval_id, "s2"),
    );
  });

  it("deny_trims_reason", async () => {
    // spec(LESSON 33) — a whitespace-only deny reason never becomes the recorded audit reason:
    // the explicit default rides instead (defense-in-depth; the daemon Gateway is the chokepoint).
    const port = new MockGatewayPort();
    const denySpy = vi.spyOn(port, "deny");
    renderModal({ port });
    fireEvent.change(screen.getByTestId("plan-deny-reason"), { target: { value: "   " } });
    fireEvent.click(screen.getByTestId("plan-deny"));
    await waitFor(() =>
      expect(denySpy).toHaveBeenCalledWith(PLAN_APPROVAL.approval_id, "Denied by the operator"),
    );
  });

  it("policy_grant_checkbox_disabled", () => {
    // spec(§11.5/§15) — the save-as-policy (policy_grant) standing-grant control is present but
    // DISABLED (own cat-1; mirrors the single-action always-allow).
    renderModal();
    expect(screen.getByTestId("plan-policy-grant")).toHaveProperty("disabled", true);
  });

  it("controls_disabled_when_not_submittable", () => {
    // spec(forbidden#6/§11.7) — degraded (canSubmitIntent false) OR mutationsEnabled false → ALL
    // approve/deny controls disabled (no enabled-button-that-throws; SAME gate as single-action).
    const degradedPort = new MockGatewayPort();
    const approveSpy = vi.spyOn(degradedPort, "approve");
    renderModal({ status: DEGRADED, port: degradedPort });
    expect(screen.getByTestId("approve-all")).toHaveProperty("disabled", true);
    expect(screen.getByTestId("plan-deny")).toHaveProperty("disabled", true);
    expect(within(rowFor("s1")).getByTestId("approve-step")).toHaveProperty("disabled", true);
    fireEvent.click(screen.getByTestId("approve-all")); // disabled → no-op
    expect(approveSpy).not.toHaveBeenCalled();

    cleanup();
    const heldPort = new MockGatewayPort({ mutationsEnabled: false });
    renderModal({ status: CONNECTED, port: heldPort });
    expect(screen.getByTestId("approve-all")).toHaveProperty("disabled", true);
    expect(within(rowFor("s1")).getByTestId("approve-step")).toHaveProperty("disabled", true);
  });

  it("per_step_controls_locked_after_plan_result", async () => {
    // spec(§11.5/INV-SEC-1) — once a plan-level decision lands (result set → footer switches to
    // Close), the PER-STEP approve controls become read-only too: a post-decision surface must NOT
    // offer a second mutation against the same approval. (The footer already gates; the rows must
    // match — otherwise a user could fire `approve(approval_id, step_id)` after approve-all/deny.)
    const port = new MockGatewayPort();
    const approveSpy = vi.spyOn(port, "approve");
    renderModal({ port });
    fireEvent.click(screen.getByTestId("approve-all"));
    await screen.findByTestId("result-ok"); // the plan-level decision landed
    expect(approveSpy).toHaveBeenCalledTimes(1);
    for (const id of ["s1", "s2", "s3"]) {
      expect(within(rowFor(id)).getByTestId("approve-step")).toHaveProperty("disabled", true);
    }
    fireEvent.click(within(rowFor("s2")).getByTestId("approve-step")); // disabled → no-op
    expect(approveSpy).toHaveBeenCalledTimes(1); // still 1 — no post-result resubmit
  });

  it("post_submit_shows_daemon_status_never_done", async () => {
    // spec(§4.2-law2/LESSON 17) — after approve, render the daemon-reported ActionAck.status
    // verbatim (the Mock returns "approved"), NEVER an optimistic "done"/"executed".
    renderModal();
    fireEvent.click(screen.getByTestId("approve-all"));
    const ok = await screen.findByTestId("result-ok");
    expect(screen.getByTestId("result-status").textContent).toBe("approved");
    expect(ok.textContent ?? "").not.toMatch(/\b(done|executed|succeeded|completed)\b/i);
  });

  it("reject_code_routed_verbatim", async () => {
    // spec(§6.4/#6) — a fencing_conflict reject → the hard-conflict card with NO re-approve
    // affordance (never re-approvable); a precondition_stale → re-approvable. Reuses describeRejection.
    const fencing = new MockGatewayPort();
    vi.spyOn(fencing, "approve").mockRejectedValue({ code: "fencing_conflict" });
    renderModal({ port: fencing });
    fireEvent.click(screen.getByTestId("approve-all"));
    const card = await screen.findByTestId("result-reject");
    expect(card.getAttribute("data-reject-kind")).toBe("hard_conflict");
    expect(screen.getByTestId("reject-code").textContent).toBe("fencing_conflict"); // verbatim
    expect(screen.queryByTestId("reapprove")).toBeNull(); // never re-approvable (#6)

    cleanup();
    const stale = new MockGatewayPort();
    vi.spyOn(stale, "approve").mockRejectedValue({ code: "precondition_stale" });
    renderModal({ port: stale });
    fireEvent.click(screen.getByTestId("approve-all"));
    const card2 = await screen.findByTestId("result-reject");
    expect(card2.getAttribute("data-reject-kind")).toBe("reapprovable");
  });

  it("n_equals_1_plan_renders_one_row", () => {
    // spec(§11.5) — an N=1 plan renders cleanly as one row (no special-casing); the single-action
    // GatewayModal suite (cited, not duplicated) covers the N=1 degenerate's own modal.
    renderModal({ plan: plan({ steps: [step({ step_id: "only" })] }) });
    expect(screen.getAllByTestId("plan-step")).toHaveLength(1);
    expect(rowFor("only")).toBeTruthy();
  });
});

describe("PlanModal — single-action L2-C no-regression guardrail (lead-ruled, MANDATORY)", () => {
  it("single_action_l2c_approve_path_unchanged", async () => {
    // spec(lead-ruled #13) — the N-step PlanModal addition is PURELY ADDITIVE. Driven through the
    // PRODUCTION `GatewayOverlay` entry with an EXISTING single-action approval (plan_id null):
    //   (i)  the unchanged single-action `GatewayModal` renders — NOT `PlanModal`;
    //   (ii) approve fires `port.approve(approval_id)` with NO step_id — a length-1 call (never
    //        `(id, undefined)`), byte-identical to the signed-off live L2-C path (the seam's new
    //        OPTIONAL step_id is additive — it does NOT regress the single-action capability);
    //   (iii) the GatewayModal renders its unchanged signature surface (risk eyebrow + disabled
    //        always-allow + approve/deny) — the 10 cat-1 GatewayModal pins (GatewayModal.test.tsx)
    //        stay green, cited not duplicated. security-reviewer verifies no regression.
    const row = makeApprovalRow({ approval_id: "ap_single_l2c", plan_id: null, action_request_id: "ar_l2c" });
    const port = new MockGatewayPort();
    const approveSpy = vi.spyOn(port, "approve");
    render(
      <ReadOnlyProvider value={CONNECTED}>
        <GatewayOverlay approval={row} port={port} onClose={() => {}} />
      </ReadOnlyProvider>,
    );
    // (i) the single-action modal, never the plan modal
    expect(screen.getByTestId("gateway-modal")).toBeTruthy();
    expect(screen.queryByTestId("plan-modal")).toBeNull();
    // (iii) the unchanged GatewayModal signature surface
    expect(screen.getByTestId("gateway-risk")).toBeTruthy();
    expect(screen.getByTestId("always-allow")).toHaveProperty("disabled", true);
    // (ii) approve is still the length-1 plan-less call
    await screen.findByTestId("gateway-preview");
    fireEvent.click(screen.getByRole("button", { name: /approve/i }));
    await waitFor(() => expect(approveSpy).toHaveBeenCalled());
    expect(approveSpy.mock.calls[0]).toEqual([row.approval_id]); // length 1 — NO step_id appended
  });
});
