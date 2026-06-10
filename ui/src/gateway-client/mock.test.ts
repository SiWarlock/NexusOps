import { describe, it, expect } from "vitest";
import { MockGatewayPort } from "./mock";
import { parseDelta } from "./boundary";
import { Session, type ProjectionDelta } from "../contracts/index";

describe("MockGatewayPort read surface (§14 mandate)", () => {
  it("mock_get_projection_returns_contract_valid_fixtures", async () => {
    const mock = new MockGatewayPort();
    const page = await mock.get_projection("Session");
    expect(page.rows.length).toBeGreaterThan(0);
    for (const row of page.rows) {
      // every fixture status value is a member of the frozen §5.1 Session enum
      expect(() => Session.parse(row.status)).not.toThrow();
    }
  });

  it("mock_subscribe_streams_validated_delta", async () => {
    const mock = new MockGatewayPort();
    const deltas: ProjectionDelta[] = [];
    for await (const delta of mock.subscribe({ projection: "Session" })) {
      deltas.push(delta);
      if (deltas.length >= 1) break;
    }
    expect(deltas.length).toBeGreaterThan(0);
    const delta = deltas[0]!;
    // pin the delta's real structure (not a tautological re-parse of the
    // mock's own parseDelta output)
    expect(delta.projection).toBe("Session");
    expect(delta.kind).toBe("upsert");
    expect(delta.row?.session_id).toBeTruthy();
    // and confirm it still round-trips the boundary parser end-to-end
    expect(() => parseDelta(delta)).not.toThrow();
  });

  it("mock_get_capabilities_reports_contract_version", async () => {
    const mock = new MockGatewayPort();
    const caps = await mock.get_capabilities();
    // literal "0.12.0" is an intentional version tripwire — it must fail loudly
    // when the frozen contract bumps (the drift test chains this to the schema).
    // Bumped 0.8.0 → 0.12.0 at the main→ui cross-track merge regen (daemon 1.5/1.6).
    expect(caps.contract_version).toBe("0.12.0");
    expect(caps.protocol_version).toBe(1);
  });
});
