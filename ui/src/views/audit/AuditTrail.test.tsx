// @vitest-environment jsdom
import { describe, it, expect, afterEach } from "vitest";
import { cleanup, render, screen, fireEvent } from "@testing-library/react";
afterEach(cleanup);
import { AuditTrail } from "./AuditTrail";
import type { AuditEventRow } from "../../contracts/index";

// An 8-field row matching the frozen daemon shape (W2-audit 0.49.0). `actor_label` defaults to a
// REAL ActorType wire value (`user`) — NOT the brief's phantom `human/agent/brain/pack` (the daemon
// serves `wire_value(&env.actor_type)` over the snake_case ActorType enum; FINDING, daemon-confirmed).
function auditRow(over: Partial<AuditEventRow> = {}): AuditEventRow {
  return {
    event_id: "event_1",
    seq: 10,
    project_id: "project_1",
    occurred_at: "2026-06-26T17:47:05Z",
    event_type: "session.started",
    headline: "Started session on auth-service",
    actor_label: "user",
    sensitivity: "internal",
    ...over,
  };
}

describe("AuditTrail — W2-audit consumer reconcile (0.49.0)", () => {
  it("renders_headline_actor_label_and_occurred_at", () => {
    // spec(§7.2 — the consumer reconcile) — the row renders the REQUIRED `headline` (the label,
    // no `?? event_type` fallback), the humanized `actor_label`, and the real `occurred_at`
    // timestamp where it used to show `#{seq}` ("timestamps land with the daemon enrichment").
    render(
      <AuditTrail
        events={[
          auditRow({
            event_id: "e3",
            headline: "Approved github.create_pr",
            actor_label: "action_gateway",
            occurred_at: "2026-06-26T17:47:05Z",
          }),
        ]}
        projectId={null}
        projectName="auth-service"
      />,
    );
    expect(screen.getByText("Approved github.create_pr")).toBeTruthy(); // headline IS the label
    expect(screen.getByText("Gateway")).toBeTruthy(); // actor_label humanized
    expect(screen.getByText("2026-06-26 17:47")).toBeTruthy(); // occurred_at (deterministic UTC)
    expect(screen.queryByText("#10")).toBeNull(); // the seq-as-time placeholder is GONE
  });

  it("occurred_at_invalid_falls_back_to_raw", () => {
    // spec(Q2 tolerant) — an unparseable occurred_at renders the RAW string verbatim, never a
    // thrown error / "Invalid Date" (formatAuditTime's NaN-guard fallback, via the real consumer).
    render(
      <AuditTrail
        events={[auditRow({ event_id: "e5", occurred_at: "not-a-date" })]}
        projectId={null}
        projectName="p"
      />,
    );
    expect(screen.getByText("not-a-date")).toBeTruthy();
  });

  it("actor_label_null_falls_back", () => {
    // spec([[32]] nullable-render discipline) — `actor_label` is `Option<String>`; a null actor
    // renders the explicit fallback ("—"), NEVER an empty string / "undefined".
    render(
      <AuditTrail
        events={[auditRow({ event_id: "e4", actor_label: null })]}
        projectId={null}
        projectName="p"
      />,
    );
    expect(screen.getByText("—")).toBeTruthy();
  });

  it("event_type_drives_namespace_filter_and_icon", () => {
    // spec(§11.2 — the real event_type) — the now-SERVED `event_type` drives the namespace filter
    // + the icon: filtering "Git" shows only `git.*` rows; a `git.*` row gets the git icon; an
    // unknown namespace falls back to the default Dot icon (never a crash).
    const { container } = render(
      <AuditTrail
        events={[
          auditRow({ event_id: "g1", event_type: "git.committed", headline: "Committed on feature" }),
          auditRow({ event_id: "s1", event_type: "session.started", headline: "Started session" }),
          auditRow({ event_id: "u1", event_type: "telemetry.sampled", headline: "Sampled telemetry" }),
        ]}
        projectId={null}
        projectName="p"
      />,
    );
    // under "All": the git.* row renders the git icon; the unknown-namespace row → default Dot.
    expect(container.querySelector(".lucide-git-commit-horizontal")).toBeTruthy();
    expect(container.querySelector(".lucide-dot")).toBeTruthy();
    // filtering "Git" narrows to git.* rows only.
    fireEvent.click(screen.getByRole("button", { name: "Git" }));
    expect(screen.getByText("Committed on feature")).toBeTruthy();
    expect(screen.queryByText("Started session")).toBeNull();
    expect(screen.queryByText("Sampled telemetry")).toBeNull();
  });

  it("actor_label_maps_known_and_falls_back_on_unknown", () => {
    // spec(the FINDING — daemon-confirmed) — a KNOWN real ActorType wire value humanizes
    // (`action_gateway`→"Gateway"); an UNKNOWN value (outside the wire set) renders the RAW string
    // (forward-compat: a future daemon-added actor renders raw, never fail-closed / enum-rejected).
    render(
      <AuditTrail
        events={[
          auditRow({ event_id: "k1", actor_label: "action_gateway", headline: "h1" }),
          auditRow({ event_id: "k2", actor_label: "future_actor_kind", headline: "h2" }),
        ]}
        projectId={null}
        projectName="p"
      />,
    );
    expect(screen.getByText("Gateway")).toBeTruthy(); // known → humanized
    expect(screen.getByText("future_actor_kind")).toBeTruthy(); // unknown → raw
  });
});
