import { describe, it, expect } from "vitest";
import { humanizeActorLabel } from "./actor-label";

describe("humanizeActorLabel (one actor vocabulary — the W2-audit FINDING)", () => {
  it("humanizes_known_real_actortype_wire_values", () => {
    // The REAL ActorType wire values (`wire_value(&env.actor_type)`, daemon-confirmed) — NOT the
    // brief's phantom human/agent/brain/pack. All 10 map to their humanized label (a typo in any
    // map entry is caught here — the [[5]]/[[30]] complete-over-the-vocabulary net).
    expect(humanizeActorLabel("user")).toBe("You");
    expect(humanizeActorLabel("project_brain")).toBe("Brain");
    expect(humanizeActorLabel("action_gateway")).toBe("Gateway");
    expect(humanizeActorLabel("workflow_runtime")).toBe("Workflow");
    expect(humanizeActorLabel("local_runner")).toBe("Runner");
    expect(humanizeActorLabel("session_adapter")).toBe("Adapter");
    expect(humanizeActorLabel("integration_syncer")).toBe("Integration");
    expect(humanizeActorLabel("system")).toBe("System");
    expect(humanizeActorLabel("remote_client")).toBe("Remote Client");
    expect(humanizeActorLabel("automation_policy")).toBe("Automation");
  });

  it("falls_back_to_raw_on_an_unknown_value", () => {
    // Forward-compat: a future daemon-added actor renders RAW, never fail-closed / enum-rejected.
    expect(humanizeActorLabel("a_future_actor")).toBe("a_future_actor");
  });

  it("renders_an_explicit_placeholder_for_null", () => {
    // actor_label is Option<String>; null → "—", never empty / "undefined".
    expect(humanizeActorLabel(null)).toBe("—");
  });
});
