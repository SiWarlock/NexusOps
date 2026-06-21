import type { ApprovalQueueRow } from "../contracts/index";
import type { GatewayPort } from "../gateway-client/types";
import { enrichApproval, enrichPlan } from "../shell/display-meta";
import { GatewayModal } from "./GatewayModal";
import { PlanModal } from "./PlanModal";

/**
 * The Shell gateway-overlay dispatcher (ui-073). A selected approval branches on `plan_id`:
 *   • a plan-bearing approval (`plan_id` set) → the N-step `PlanModal` (the §11.5 multi-step Gateway
 *     Review Modal), rendered from the assembled `ActionPlan`;
 *   • a single-action approval (`plan_id` null) → the unchanged single-action `GatewayModal` (the N=1
 *     degenerate — byte-identical to the signed-off live L2-C path).
 *
 * EXPOSED-AHEAD: no live plan-submitter exists in production yet (the Brain "Run via Gateway" submit
 * path is 8.1-gated + guarded-disabled), so no `ActionPlan` reaches the queue today — the plan branch
 * renders `enrichPlan`'s daemon-shaped sample; the live plan-data feed is a deferred follow-on. The
 * branch + the modal are wired now (reachable-by-construction the moment a plan-bearing approval is
 * selected from the Human Input Queue).
 */
export function GatewayOverlay({
  approval,
  port,
  onClose,
}: {
  approval: ApprovalQueueRow;
  port: GatewayPort;
  onClose: () => void;
}) {
  if (approval.plan_id) {
    const enriched = enrichApproval(approval);
    return (
      <PlanModal
        plan={enrichPlan(approval.plan_id)}
        approval={enriched.approval}
        port={port}
        onClose={onClose}
      />
    );
  }
  return <GatewayModal {...enrichApproval(approval)} port={port} onClose={onClose} />;
}
