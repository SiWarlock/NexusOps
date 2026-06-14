import { describe, it, expect } from "vitest";
import { MockGatewayPort } from "./mock";
import { parseDelta } from "./boundary";
import {
  DiffResult,
  Session,
  TerminalOutputFrame,
  type ProjectionDelta,
} from "../contracts/index";

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

  it("mock_subscribe_terminal_yields_contract_valid_stream", async () => {
    // spec(§6.4 / §14): subscribe_terminal yields a deterministic fixture terminal
    // stream of frozen `terminal_output` frames (output ONLY — maps 1:1 to the
    // §6.4 terminal channel; the §17 exit is a daemon event→projection, NOT pushed
    // here). Every frame round-trips its frozen shadow (parse-don't-trust dogfood),
    // and the stream terminates.
    const mock = new MockGatewayPort();
    const frames: unknown[] = [];
    for await (const frame of mock.subscribe_terminal("t1")) frames.push(frame);

    expect(frames.length).toBeGreaterThan(0);
    for (const frame of frames) {
      expect(() => TerminalOutputFrame.parse(frame)).not.toThrow();
    }
  });

  it("mock_get_diff_returns_valid_diffresult", async () => {
    // spec(§6.1) — the read-only get_diff fixture is CONTRACT-shaped (a DiffResult the
    // 6.3e Code/Diff slice can consume), served so DiffResult.parse() accepts it
    // (parse-don't-trust dogfood, symmetric with get_projection/subscribe_terminal).
    const mock = new MockGatewayPort();
    const result = await mock.get_diff("wt_demo_0001", "src/main.rs");
    expect(() => DiffResult.parse(result)).not.toThrow();
    expect(result.hunks.length).toBeGreaterThan(0);
  });

  it("mock_notify_connection_state_is_guarded_setconnectionstate_stays_raw", () => {
    // spec(§11 / 054) — the mock implements the new notifyConnectionState (guarded canTransition +
    // notify, mirroring the real port so the supervisor binding works in Shell tests), while
    // setConnectionState stays the RAW, unguarded test-staging setter (the §11 DegradedBanner
    // contract — Shell.test.tsx stages `disconnected` through it). Two methods, two purposes.
    // start at the pre-handshake `connecting` (the mock defaults to `connected`) so the illegal hop
    // connecting→reconnecting actually exercises the guard.
    const mock = new MockGatewayPort({ connection: "connecting" });
    const seen: string[] = [];
    mock.onConnectionChange((s) => seen.push(s));

    // notifyConnectionState is GUARDED: connecting→reconnecting is illegal → no-op (no notify).
    mock.notifyConnectionState("reconnecting");
    expect(mock.getConnectionState()).toBe("connecting");
    expect(seen).toEqual([]);

    // setConnectionState is RAW: it stages ANY state directly + notifies (drives the degraded banner).
    mock.setConnectionState("reconnecting");
    expect(mock.getConnectionState()).toBe("reconnecting");
    expect(seen).toEqual(["reconnecting"]);
  });

  it("mock_get_capabilities_reports_contract_version", async () => {
    const mock = new MockGatewayPort();
    const caps = await mock.get_capabilities();
    // literal "0.28.0" is an intentional version tripwire — it must fail loudly
    // when the frozen contract bumps (the drift test chains this to the schema).
    // Bumped 0.8.0 → 0.12.0 (main→ui merge regen) → 0.19.0 (Phase-2 Gateway freeze)
    // → 0.23.0 (Phase-3 boundary merge: §9.1 harness / §6.4 Terminal Channel)
    // → 0.28.0 (Phase-4 boundary merge: §6.3e per-hunk git actions + get_diff)
    // → 0.31.0 (L2-prep boundary merge: survival 0.29 / ApprovalQueueRow 0.30 / SessionRecovered 0.31).
    expect(caps.contract_version).toBe("0.31.0");
    expect(caps.protocol_version).toBe(1);
  });
});
