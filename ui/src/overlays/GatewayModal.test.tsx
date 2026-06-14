// @vitest-environment jsdom
import { describe, it, expect, vi, afterEach } from "vitest";
import { cleanup, render, screen, fireEvent, waitFor } from "@testing-library/react";

afterEach(cleanup);
import { GatewayModal, ResultNotice } from "./GatewayModal";
import { ReadOnlyProvider, type ConnectionStatus } from "../connection/read-only";
import { MockGatewayPort } from "../gateway-client/mock";
import { gatewayApprovalEnrichment } from "../shell/display-meta";
import type { PolicyDecision } from "../contracts/index";

const CONNECTED: ConnectionStatus = { connection: "connected", version: "compatible" };
const DEGRADED: ConnectionStatus = { connection: "connecting", version: "unknown" };
const { approval, policyDecision } = gatewayApprovalEnrichment.approval_fixture_1!;

function renderModal(
  opts: { status?: ConnectionStatus; port?: MockGatewayPort; policyDecision?: PolicyDecision } = {},
) {
  const port = opts.port ?? new MockGatewayPort();
  const utils = render(
    <ReadOnlyProvider value={opts.status ?? CONNECTED}>
      <GatewayModal
        approval={approval}
        policyDecision={opts.policyDecision ?? policyDecision}
        port={port}
        onClose={() => {}}
      />
    </ReadOnlyProvider>,
  );
  return { port, ...utils };
}

describe("GatewayModal — cat-1 consumer pins (full modal)", () => {
  it("modal_actions_route_through_seam_no_executor", async () => {
    // spec(§4.2/§15) — Approve/Deny route through the seam → the single GatewayPort.
    const port = new MockGatewayPort();
    const approveSpy = vi.spyOn(port, "approve");
    renderModal({ port });
    await screen.findByTestId("gateway-preview");
    fireEvent.click(screen.getByRole("button", { name: /approve/i }));
    await waitFor(() => expect(approveSpy).toHaveBeenCalledWith(approval.approval_id));

    const port2 = new MockGatewayPort();
    const denySpy = vi.spyOn(port2, "deny");
    renderModal({ port: port2 });
    fireEvent.change(screen.getByTestId("deny-reason"), { target: { value: "not now" } });
    fireEvent.click(screen.getByRole("button", { name: /deny/i }));
    await waitFor(() => expect(denySpy).toHaveBeenCalledWith(approval.approval_id, "not now"));
  });

  it("approve_deny_disabled_when_canSubmitIntent_false", async () => {
    // spec(§11.1/§15) — fail-closed: in read-only/degraded the controls are disabled, and
    // the GatewayPort is never called (defense-in-depth; the daemon Gateway is the real gate).
    const port = new MockGatewayPort();
    const approveSpy = vi.spyOn(port, "approve");
    renderModal({ status: DEGRADED, port });
    expect(screen.getByRole("button", { name: /approve/i })).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: /deny/i })).toHaveProperty("disabled", true);
    fireEvent.click(screen.getByRole("button", { name: /approve/i })); // disabled → no-op
    expect(approveSpy).not.toHaveBeenCalled();
  });

  it("approve_deny_disabled_when_mutations_not_enabled", async () => {
    // spec(L2-B 🔒 L2-O3 / §11.7) — even CONNECTED (canSubmitIntent true), the approve/deny controls
    // stay disabled while the port's `mutationsEnabled` gate is false (the L2-B honest disabled state —
    // wired-but-not-yet-enabled; the go-live enable is L2-C). NOT an enabled-button-that-throws.
    const port = new MockGatewayPort({ mutationsEnabled: false });
    const approveSpy = vi.spyOn(port, "approve");
    renderModal({ status: CONNECTED, port });
    expect(screen.getByRole("button", { name: /approve/i })).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: /deny/i })).toHaveProperty("disabled", true);
    fireEvent.click(screen.getByRole("button", { name: /approve/i })); // disabled → no-op
    expect(approveSpy).not.toHaveBeenCalled();
  });

  it("card_risk_and_requirement_read_from_policydecision_never_ui_derived", async () => {
    // spec(§6.2) — the requirement is READ from the daemon's PolicyDecision: a DIFFERENT
    // PolicyDecision changes the displayed requirement (proves read, not hardcoded).
    const a = renderModal({
      policyDecision: { ...policyDecision, required_approvals: [{ kind: "current_user" }] },
    });
    expect(screen.getByTestId("policy-requirement").textContent).toContain("current_user");
    a.unmount();
    renderModal({
      policyDecision: { ...policyDecision, required_approvals: [{ kind: "project_owner" }] },
    });
    expect(screen.getByTestId("policy-requirement").textContent).toContain("project_owner");
  });

  it("preview_section_renders_daemon_actionpreview_or_honest_pending", async () => {
    // spec(forbidden#2) — the daemon's ActionPreview consequences VERBATIM; the stub's
    // hardcoded lines are gone. A `cannot_preview_reason` renders the reason, not invented ones.
    renderModal();
    const ready = await screen.findByTestId("preview-ready");
    expect(ready.textContent).toContain("Would modify 1 file in the worktree.");

    const port = new MockGatewayPort();
    vi.spyOn(port, "preview_action").mockResolvedValue({
      action_request_id: "ar_fixture_1",
      generated_at: "2026-06-13T00:00:00Z",
      risk_level: 2,
      risk_reasons: [],
      summary: "",
      changed_resources: [],
      cannot_preview_reason: "the daemon is still indexing",
    });
    renderModal({ port });
    const cannot = await screen.findByTestId("preview-cannot");
    expect(cannot.textContent).toContain("the daemon is still indexing");
  });

  it("preview_failed_renders_honest_note_never_blank", async () => {
    // spec(forbidden#2/§11.7) — a thrown (non-WireError) preview error keeps the honest
    // pending contract: a distinct "failed" note, never blank nor an invented consequence.
    const port = new MockGatewayPort();
    vi.spyOn(port, "preview_action").mockRejectedValue(new Error("transport down"));
    renderModal({ port });
    const failed = await screen.findByTestId("preview-failed");
    expect(failed.textContent).toMatch(/unavailable/i);
  });

  it("modal_holds_no_intent_state_no_autoretry", async () => {
    // spec(§6.1) — the modal adds no cache/auto-retry over the stateless seam: one
    // approve click → exactly one submit, no replay.
    const port = new MockGatewayPort();
    const approveSpy = vi.spyOn(port, "approve");
    renderModal({ port });
    await screen.findByTestId("gateway-preview");
    fireEvent.click(screen.getByRole("button", { name: /approve/i }));
    await screen.findByTestId("result-ok");
    await new Promise((r) => setTimeout(r, 10));
    expect(approveSpy).toHaveBeenCalledTimes(1); // no auto-retry
  });

  it("always_allow_policy_grant_stays_disabled", async () => {
    // spec(§11.5/§15) — the standing-grant control stays DISABLED (a policy_grant is a
    // separate cat-1-gated slice; it must not be wireable here).
    renderModal();
    expect(screen.getByTestId("always-allow")).toHaveProperty("disabled", true);
  });
});

describe("GatewayModal ResultNotice — result-branch pins", () => {
  it("modal_renders_daemon_status_never_optimistic_done", () => {
    // spec(§4.2-law2/§11.7) — an `approved` ack renders the daemon's decision status,
    // NEVER an optimistic "done"/"executed"/"succeeded".
    render(<ResultNotice result={{ ok: { action_request_id: "ar_1", status: "approved" } }} onReapprove={() => {}} />);
    expect(screen.getByTestId("result-status").textContent).toBe("approved");
    const text = screen.getByTestId("result-ok").textContent ?? "";
    expect(text).not.toMatch(/\b(done|executed|succeeded|completed)\b/i);
  });

  it("readonly_result_renders_no_mutation", () => {
    // spec(§11.1/§15) — a `{readOnly}` seam result renders an honest no-mutation note.
    render(<ResultNotice result={{ readOnly: true }} onReapprove={() => {}} />);
    expect(screen.getByTestId("result-readonly").textContent).toMatch(/not submitted/i);
    expect(screen.queryByTestId("result-status")).toBeNull();
  });

  it("reject_codes_route_to_distinct_cards", () => {
    // spec(§6.4/§11.5/§17) — each policy/execution code → its distinct §11.5 treatment.
    const kinds: Record<string, string> = {};
    for (const code of ["fencing_conflict", "internal_error", "precondition_stale", "policy_denied"] as const) {
      const { container, unmount } = render(<ResultNotice result={{ error: { code } }} onReapprove={() => {}} />);
      const card = screen.getByTestId("result-reject");
      kinds[code] = card.getAttribute("data-reject-kind")!;
      expect(screen.getByTestId("reject-code").textContent).toBe(code); // verbatim
      expect(container.textContent?.length).toBeGreaterThan(0);
      unmount();
    }
    // all four are DISTINCT treatments — never collapsed
    expect(new Set(Object.values(kinds)).size).toBe(4);
  });

  it("fencing_conflict_never_rendered_as_reapprovable", () => {
    // spec(§6.4/#6) — fencing is the never-auto-resolved card: NO retry/re-approve affordance.
    render(<ResultNotice result={{ error: { code: "fencing_conflict" } }} onReapprove={() => {}} />);
    expect(screen.getByTestId("result-reject").getAttribute("data-reject-kind")).toBe("hard_conflict");
    expect(screen.queryByTestId("reapprove")).toBeNull();
  });

  it("precondition_stale_card_is_reapprovable_not_hard_conflict", () => {
    // spec(§17/§6.2) — the net-new re-approvable card: offers regenerate-preview + fresh
    // approval; NOT the never-auto-resolved fencing treatment.
    const onReapprove = vi.fn();
    render(<ResultNotice result={{ error: { code: "precondition_stale" } }} onReapprove={onReapprove} />);
    const card = screen.getByTestId("result-reject");
    expect(card.getAttribute("data-reject-kind")).toBe("reapprovable");
    expect(card.getAttribute("data-reject-kind")).not.toBe("hard_conflict");
    fireEvent.click(screen.getByTestId("reapprove"));
    expect(onReapprove).toHaveBeenCalledTimes(1); // MANUAL re-approve, never auto
  });

  it("unmapped_reject_code_renders_honest_rejection_never_swallowed", () => {
    // spec(§6.4/§11.7) — a transport code OUTSIDE the 4 policy/execution codes renders an
    // HONEST generic rejection (code shown), never swallowed (blank) nor re-approvable.
    render(<ResultNotice result={{ error: { code: "version_skew" } }} onReapprove={() => {}} />);
    const card = screen.getByTestId("result-reject");
    expect(card.getAttribute("data-reject-kind")).toBe("generic");
    expect(screen.getByTestId("reject-code").textContent).toBe("version_skew"); // shown, not swallowed
    expect(card.textContent?.length).toBeGreaterThan(0);
    expect(screen.queryByTestId("reapprove")).toBeNull(); // never re-approvable
  });
});
