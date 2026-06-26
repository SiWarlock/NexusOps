// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";

afterEach(cleanup);
import { GatewayOverlay } from "./GatewayOverlay";
import { ReadOnlyProvider, type ConnectionStatus } from "../connection/read-only";
import { MockGatewayPort } from "../gateway-client/mock";
import { makeApprovalRow } from "../projections/fixtures/proj_approval_queue";

const CONNECTED: ConnectionStatus = { connection: "connected", version: "compatible" };

function renderOverlay(approval: Parameters<typeof GatewayOverlay>[0]["approval"]) {
  return render(
    <ReadOnlyProvider value={CONNECTED}>
      <GatewayOverlay approval={approval} port={new MockGatewayPort()} onClose={() => {}} />
    </ReadOnlyProvider>,
  );
}

describe("GatewayOverlay — the Shell gateway-overlay dispatcher (Step 7.5 wiring)", () => {
  it("plan_bearing_approval_routes_to_plan_modal", () => {
    // spec(§11.5) — a selected approval carrying a `plan_id` renders the N-step PlanModal
    // (reachable-by-construction from the Shell gateway overlay the moment a plan is selected).
    renderOverlay(makeApprovalRow({ approval_id: "ap_plan", plan_id: "plan_0001", action_request_id: null }));
    expect(screen.getByTestId("plan-modal")).toBeTruthy();
    expect(screen.queryByTestId("gateway-modal")).toBeNull();
  });

  it("single_action_approval_routes_to_gateway_modal", () => {
    // spec(§11.5) — a single-action approval (`plan_id` null) renders the unchanged single-action
    // GatewayModal (the N=1 path stays byte-identical — no plan branch taken).
    renderOverlay(makeApprovalRow({ approval_id: "ap_single", plan_id: null, action_request_id: "ar_1" }));
    expect(screen.getByTestId("gateway-modal")).toBeTruthy();
    expect(screen.queryByTestId("plan-modal")).toBeNull();
  });
});
