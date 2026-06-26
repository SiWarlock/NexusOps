// ONE source of truth for humanizing the daemon's audit `actor_label` (shared by the AuditTrail
// tile + the Command Center event line). `actor_label` is the snake_case ActorType WIRE value
// (`wire_value(&env.actor_type)`, daemon-confirmed — NOT the brief's phantom human/agent/brain/pack;
// the values are exactly the frozen 10-value ActorType enum, snake_case). Humanize UI-side, UNBOUND
// from the ActorType Zod enum + TOLERANT (unknown key → raw fallback) so a future daemon-added actor
// renders raw, never fails. Centralized because a divergent/wrong actor map is a real hazard (the
// W2-audit FINDING: a phantom relay nearly keyed every row to dead-fallback).
const ACTOR_LABEL: Record<string, string> = {
  user: "You",
  project_brain: "Brain",
  action_gateway: "Gateway",
  workflow_runtime: "Workflow",
  local_runner: "Runner",
  session_adapter: "Adapter",
  integration_syncer: "Integration",
  system: "System",
  remote_client: "Remote Client",
  automation_policy: "Automation",
};

/** Humanize a daemon `actor_label` (Option<String>): null → the explicit "—" placeholder
 *  (never empty/undefined); a known wire value → its humanized label; an unknown value → the RAW
 *  string (forward-compat — a future daemon actor renders raw, never fail-closed / enum-rejected). */
export function humanizeActorLabel(actorLabel: string | null): string {
  if (actorLabel === null) return "—";
  return ACTOR_LABEL[actorLabel] ?? actorLabel;
}
