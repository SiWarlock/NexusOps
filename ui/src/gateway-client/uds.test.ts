// Layer A — the TS UdsGatewayPort single-shot READ transport (P6.8 L1, slice 051).
//
// The real §6.1 GatewayPort read client: it `invoke`s the 050 Tauri commands and
// Zod-`.parse()`s every returned payload at the boundary.ts seam (parse-don't-trust).
// These pins fix the security-load-bearing boundary contract: a wire error surfaces
// as a PLAIN {code} value (NOT an Error instance) so the §6.4 code routes verbatim;
// a transport fault surfaces as an Error instance (an honest degrade); the mutation
// methods are NOT invokable (reads-only — L2 mutation transport is cat-1 HELD).
import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  UdsGatewayPort,
  subscriptionIterable,
  type SubscriptionEvent,
} from "./uds";
import { BoundaryValidationError, parseProjectionPage } from "./boundary";
import { WireError } from "../contracts/index";
import type { ActionRequest, ProjectionDelta } from "../contracts/index";
import { sessionPageFixture } from "../projections/fixtures/proj_session";

// the real subscribe() constructs a Channel — stub it so the smoke test can build the port; the
// streaming logic itself is tested through subscriptionIterable with a fake start (no real Channel).
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  Channel: class {
    onmessage: ((e: unknown) => void) | undefined = undefined;
  },
}));
const mockInvoke = vi.mocked(invoke);

/** A fake subscribe `start`: captures the onMessage handler so a test drives the stream. */
function fakeStart() {
  let emit: ((e: SubscriptionEvent) => void) | null = null;
  const start = (onMessage: (e: SubscriptionEvent) => void) => {
    emit = onMessage;
    return Promise.resolve();
  };
  return {
    start,
    emit: (e: SubscriptionEvent) => emit!(e),
  };
}

/** Drain an AsyncIterable to completion (collecting deltas); rethrows if the iterable throws. */
async function drain(
  iter: AsyncIterable<ProjectionDelta>,
): Promise<ProjectionDelta[]> {
  const out: ProjectionDelta[] = [];
  for await (const d of iter) out.push(d);
  return out;
}

// A contract-valid get_diff result (mirrors the mock's diffFixture / the daemon Hunk shape).
const validDiff = {
  hunks: [
    {
      header: "@@ -1,1 +1,1 @@",
      old_start: 1,
      old_lines: 1,
      new_start: 1,
      new_lines: 1,
      lines: [{ kind: "context", content: "x\n" }],
    },
  ],
};
const validCaps = { protocol_version: 1, contract_version: "0.28.0" };

beforeEach(() => {
  mockInvoke.mockReset();
});

describe("UdsGatewayPort — single-shot reads (Layer A)", () => {
  it("get_projection invokes the 050 command and boundary-parses the page (§6.1/§5.0)", async () => {
    mockInvoke.mockResolvedValue(sessionPageFixture);
    const port = new UdsGatewayPort();

    const page = await port.get_projection("Session");

    expect(mockInvoke).toHaveBeenCalledWith("gateway_get_projection", {
      name: "Session",
      scope: undefined,
    });
    // returned through the same boundary validator the mock dogfoods (parse-don't-trust).
    expect(page).toEqual(parseProjectionPage("Session", sessionPageFixture));
  });

  it("get_projection throws BoundaryValidationError on a malformed payload — never returns it (§15 fail-closed)", async () => {
    mockInvoke.mockResolvedValue({ not_a: "page" });
    const port = new UdsGatewayPort();
    await expect(port.get_projection("Session")).rejects.toBeInstanceOf(
      BoundaryValidationError,
    );
  });

  it("get_diff invokes with camelCase args and boundary-parses the DiffResult (§6.1)", async () => {
    mockInvoke.mockResolvedValue(validDiff);
    const port = new UdsGatewayPort();

    const diff = await port.get_diff("wt_1", "a.ts");

    // Tauri auto-converts JS camelCase → Rust snake_case (worktreeId → worktree_id).
    expect(mockInvoke).toHaveBeenCalledWith("gateway_get_diff", {
      worktreeId: "wt_1",
      file: "a.ts",
    });
    expect(diff.hunks).toHaveLength(1);
    expect(diff.hunks[0]!.old_start).toBe(1);
  });

  it("get_diff throws BoundaryValidationError on a malformed payload (§15 fail-closed)", async () => {
    mockInvoke.mockResolvedValue({ not_a_diff: true });
    const port = new UdsGatewayPort();
    await expect(port.get_diff("wt_1", "a.ts")).rejects.toBeInstanceOf(
      BoundaryValidationError,
    );
  });

  it("get_capabilities invokes the command and boundary-parses Capabilities (§6.4)", async () => {
    mockInvoke.mockResolvedValue(validCaps);
    const port = new UdsGatewayPort();

    const caps = await port.get_capabilities();

    expect(mockInvoke).toHaveBeenCalledWith("gateway_get_capabilities");
    expect(caps).toEqual(validCaps);
  });

  it("a daemon WIRE error surfaces as a PLAIN {code} value (NOT an Error) so the §6.4 code routes verbatim", async () => {
    // The 050 bridge rejects with the serialized GatewayCommandError {kind:"wire", code}.
    mockInvoke.mockRejectedValue({ kind: "wire", code: "not_found" });
    const port = new UdsGatewayPort();

    let thrown: unknown;
    try {
      await port.get_diff("wt_1", "a.ts");
    } catch (e) {
      thrown = e;
    }
    // CRITICAL (LESSON §16): a daemon error frame is plain data, NEVER an Error — the
    // intent seam + DiffReview classify by `instanceof Error`, so a wire error MUST NOT
    // be thrown as an Error (else it would be re-thrown as a bug instead of routed).
    expect(thrown instanceof Error).toBe(false);
    expect(WireError.safeParse(thrown).success).toBe(true);
    expect((thrown as { code: string }).code).toBe("not_found");
  });

  it("a TRANSPORT (io) fault surfaces as an Error instance — an honest degrade, never a fake wire code (§11.7)", async () => {
    mockInvoke.mockRejectedValue({ kind: "io", message: "Connection refused" });
    const port = new UdsGatewayPort();

    let thrown: unknown;
    try {
      await port.get_capabilities();
    } catch (e) {
      thrown = e;
    }
    expect(thrown).toBeInstanceOf(Error);
    // a non-wire fault is NOT a WireError — a consumer's WireError.safeParse must miss it.
    expect(WireError.safeParse(thrown).success).toBe(false);
  });

  it("a wire fault WITHOUT a string code is treated as a transport fault (Error), never a fabricated code", async () => {
    // a malformed bridge response (kind:wire, no code) must NOT throw {code:undefined} as a
    // pseudo-WireError — it falls through to the honest transport-fault Error path.
    mockInvoke.mockRejectedValue({ kind: "wire" });
    const port = new UdsGatewayPort();
    let thrown: unknown;
    try {
      await port.get_diff("wt_1", "a.ts");
    } catch (e) {
      thrown = e;
    }
    expect(thrown).toBeInstanceOf(Error);
    expect(WireError.safeParse(thrown).success).toBe(false);
  });

  it("a successful read transitions the connection connecting → connected and notifies listeners (§11.4, fail-safe)", async () => {
    mockInvoke.mockResolvedValue(validCaps);
    const port = new UdsGatewayPort();
    const seen: string[] = [];
    port.onConnectionChange((s) => seen.push(s));

    // fail-safe: starts read-only (connecting) until a daemon response confirms it (LESSON §4).
    expect(port.getConnectionState()).toBe("connecting");

    await port.get_capabilities();

    expect(port.getConnectionState()).toBe("connected");
    expect(seen).toContain("connected");
  });

  it("a transport io fault transitions the connection to disconnected (drives the degraded banner)", async () => {
    const port = new UdsGatewayPort();
    // first a success → connected, then an io fault → disconnected (a legal hop).
    mockInvoke.mockResolvedValueOnce(validCaps);
    await port.get_capabilities();
    expect(port.getConnectionState()).toBe("connected");

    mockInvoke.mockRejectedValueOnce({ kind: "io", message: "broken pipe" });
    await expect(port.get_capabilities()).rejects.toBeInstanceOf(Error);
    expect(port.getConnectionState()).toBe("disconnected");
  });

  it("a non-io transport fault (protocol/serde/internal) also degrades to disconnected (defense-in-depth gate, §11.4)", async () => {
    const port = new UdsGatewayPort();
    mockInvoke.mockResolvedValueOnce(validCaps);
    await port.get_capabilities();
    expect(port.getConnectionState()).toBe("connected");

    // a protocol-violating / untrustworthy frame from the daemon → the link can't be
    // trusted as "connected" → disconnected (drops canSubmitIntent fail-safe).
    mockInvoke.mockRejectedValueOnce({ kind: "protocol", message: "bad frame" });
    await expect(port.get_capabilities()).rejects.toBeInstanceOf(Error);
    expect(port.getConnectionState()).toBe("disconnected");
  });

  it("a version_skew fault does NOT disconnect — the daemon answered; that's the version axis", async () => {
    const port = new UdsGatewayPort();
    mockInvoke.mockResolvedValueOnce(validCaps);
    await port.get_capabilities();
    expect(port.getConnectionState()).toBe("connected");

    mockInvoke.mockRejectedValueOnce({
      kind: "version_skew",
      supported_min: 2,
      supported_max: 2,
      client_protocol_version: 1,
    });
    await expect(port.get_capabilities()).rejects.toBeInstanceOf(Error);
    expect(port.getConnectionState()).toBe("connected");
  });

  it("the MUTATION methods throw a not-wired (L2) error and NEVER invoke a command (reads-only; cat-1 HELD)", async () => {
    const port = new UdsGatewayPort();

    // a minimal cast — the method throws before it ever reads the request (reads-only client).
    await expect(
      port.submit_action({ action_type: "git.stage_hunk" } as ActionRequest),
    ).rejects.toThrow(/not wired|L2/i);
    await expect(port.preview_action("ar_1")).rejects.toThrow(/not wired|L2/i);
    await expect(port.approve("appr_1")).rejects.toThrow(/not wired|L2/i);
    await expect(port.deny("appr_1", "no")).rejects.toThrow(/not wired|L2/i);
    // the §6.4 terminal demux is a P4 surface — not wired in L1 either.
    expect(() => port.subscribe_terminal("t1")).toThrow(/not wired|L2|P4/i);
    // (subscribe is now WIRED at 052 — covered by its own tests below; it is a READ, not a mutation.)

    // the read client can NEVER reach a mutation command — no Tauri mutation command exists.
    expect(mockInvoke).not.toHaveBeenCalled();
  });
});

describe("UdsGatewayPort — subscribe streaming (Layer B-TS)", () => {
  it("subscribe_iterable_parsedelta_each_frame — yields a parseDelta'd delta then ends on close", async () => {
    const h = fakeStart();
    const iter = subscriptionIterable(h.start);
    const validDelta = { projection: "Session", kind: "upsert", id: "sess_1" };
    h.emit({ kind: "delta", delta: validDelta });
    h.emit({ kind: "closed" });

    const got: ProjectionDelta[] = [];
    for await (const d of iter) got.push(d);

    expect(got).toHaveLength(1);
    expect(got[0]!.id).toBe("sess_1");
    expect(got[0]!.kind).toBe("upsert");
  });

  it("subscribe_iterable rejects a malformed delta with BoundaryValidationError (parse-don't-trust)", async () => {
    const h = fakeStart();
    const iter = subscriptionIterable(h.start);
    // a malformed delta (bad kind, no projection) → parseDelta throws; never yielded (§5.0 fail-closed).
    h.emit({ kind: "delta", delta: { kind: "not_a_real_kind" } });

    await expect(drain(iter)).rejects.toBeInstanceOf(BoundaryValidationError);
  });

  it("subscribe_iterable_ends_on_stream_close — a `closed` event ends the iterable cleanly (no throw)", async () => {
    const h = fakeStart();
    const iter = subscriptionIterable(h.start);
    h.emit({ kind: "closed" });

    const got: ProjectionDelta[] = [];
    for await (const d of iter) got.push(d);
    expect(got).toHaveLength(0);
  });

  it("subscribe_iterable_surfaces_error_on_error_signal — an `error` event throws (honest degrade, never silent §11.7)", async () => {
    const h = fakeStart();
    const iter = subscriptionIterable(h.start);
    h.emit({ kind: "error", error: { kind: "io", message: "broken pipe" } });

    await expect(drain(iter)).rejects.toBeInstanceOf(Error);
  });

  it("subscribe_iterable yields multiple deltas queued before iteration (no lost wakeup)", async () => {
    const h = fakeStart();
    const iter = subscriptionIterable(h.start);
    // queue TWO deltas + a close BEFORE iterating — the buffered async-queue must yield both, in order.
    h.emit({
      kind: "delta",
      delta: { projection: "Session", kind: "upsert", id: "s1" },
    });
    h.emit({
      kind: "delta",
      delta: { projection: "Session", kind: "remove", id: "s2" },
    });
    h.emit({ kind: "closed" });

    const got = await drain(iter);
    expect(got.map((d) => d.id)).toEqual(["s1", "s2"]);
  });

  it("subscribe() returns an AsyncIterable invoking gateway_subscribe (wired at 052 — replaces the 051 not-wired throw)", () => {
    mockInvoke.mockResolvedValue(undefined);
    const port = new UdsGatewayPort();
    const iter = port.subscribe({ projection: "Session" });
    expect(typeof iter[Symbol.asyncIterator]).toBe("function");
    // the real subscribe eager-starts the gateway_subscribe command with the projection (the wiring pin).
    expect(mockInvoke).toHaveBeenCalledWith(
      "gateway_subscribe",
      expect.objectContaining({ projection: "Session" }),
    );
  });
});

// ── 054 — connection single-authority (the 052 two-writer reconcile) ──────────
// The port is the SINGLE connection-state authority: the subscribe supervisor drives it via
// `notifyConnectionState` (not a 2nd raw React setter), the port tracks a stream-degraded axis, and
// the read-path UPGRADE (markConnected) is SUPPRESSED while the stream is degraded — so an ad-hoc
// read can never mask a down stream (forbidden #6 / LESSON 4). DEGRADE always flows (fail-safe).
describe("UdsGatewayPort — connection single-authority (054)", () => {
  it("read_success_does_not_upgrade_while_stream_degraded", async () => {
    // spec(§11.4/forbidden#6) — the core fix: the supervisor degraded the stream; a later ad-hoc read
    // SUCCESS must NOT re-assert `connected` (the 052 masking — canSubmitIntent stays false).
    const port = new UdsGatewayPort();
    mockInvoke.mockResolvedValueOnce(validCaps);
    await port.get_capabilities();
    expect(port.getConnectionState()).toBe("connected");

    port.notifyConnectionState("disconnected"); // the supervisor: the live stream is down
    expect(port.getConnectionState()).toBe("disconnected");

    mockInvoke.mockResolvedValueOnce(validCaps);
    await port.get_capabilities(); // an unrelated read succeeds…
    // …but the read-path UPGRADE is suppressed while the stream is degraded → still disconnected.
    expect(port.getConnectionState()).toBe("disconnected");
  });

  it("read_success_suppressed_during_reconnecting_too", async () => {
    // spec(§11.4/§11.7) — streamDegraded covers BOTH {disconnected, reconnecting}: a read-success
    // MID-RECOVERY (the supervisor drove `reconnecting`, before its refetch→connected) is ALSO
    // suppressed → never prematurely `connected` before the snapshot refetch confirms. A regression
    // narrowing the suppression-set to {disconnected} would re-open this premature-upgrade window.
    const port = new UdsGatewayPort();
    mockInvoke.mockResolvedValueOnce(validCaps);
    await port.get_capabilities(); // connected
    port.notifyConnectionState("disconnected");
    port.notifyConnectionState("reconnecting"); // mid-recovery (refetch not yet confirmed)
    expect(port.getConnectionState()).toBe("reconnecting");

    mockInvoke.mockResolvedValueOnce(validCaps);
    await port.get_capabilities(); // a read succeeds mid-recovery…
    // …still suppressed (reconnecting ∈ streamDegraded) → never a premature `connected`.
    expect(port.getConnectionState()).toBe("reconnecting");
  });

  it("read_success_upgrades_when_stream_healthy", async () => {
    // spec(LESSON 22) — the control case: with NO stream degrade, a read success upgrades to
    // connected (the normal initial-connect path is unaffected by the suppression).
    const port = new UdsGatewayPort();
    mockInvoke.mockResolvedValue(validCaps);
    await port.get_capabilities();
    expect(port.getConnectionState()).toBe("connected");
  });

  it("both_axes_degrade_fail_safe", async () => {
    // spec(LESSON 4) — DEGRADE is never suppressed on EITHER axis: a subscribe-stream degrade
    // (notify) AND a read transport fault both drop to disconnected. UPGRADE is the only suppressed
    // direction.
    const port = new UdsGatewayPort();
    mockInvoke.mockResolvedValueOnce(validCaps);
    await port.get_capabilities();

    // axis 1 — the subscribe stream degrades:
    port.notifyConnectionState("disconnected");
    expect(port.getConnectionState()).toBe("disconnected");

    // recover via the supervisor, then axis 2 — a read transport fault still degrades:
    port.notifyConnectionState("reconnecting");
    port.notifyConnectionState("connected");
    expect(port.getConnectionState()).toBe("connected");
    mockInvoke.mockRejectedValueOnce({ kind: "io", message: "down" });
    await expect(port.get_capabilities()).rejects.toBeInstanceOf(Error);
    expect(port.getConnectionState()).toBe("disconnected");
  });

  it("stream_recovery_returns_to_connected_and_clears_suppression", async () => {
    // spec(§11.7) — after the supervisor recovers (disconnected→reconnecting→connected), the exposed
    // connection returns to connected AND the read-upgrade suppression clears (a later read success is
    // no longer suppressed → canSubmitIntent can be true again).
    const port = new UdsGatewayPort();
    mockInvoke.mockResolvedValueOnce(validCaps);
    await port.get_capabilities(); // connected

    port.notifyConnectionState("disconnected"); // stream down → upgrade suppressed
    mockInvoke.mockResolvedValueOnce(validCaps);
    await port.get_capabilities();
    expect(port.getConnectionState()).toBe("disconnected"); // suppressed

    port.notifyConnectionState("reconnecting");
    port.notifyConnectionState("connected"); // the supervisor recovered the stream
    expect(port.getConnectionState()).toBe("connected");

    // suppression cleared: a read fault then success now upgrades normally (stream healthy).
    mockInvoke.mockRejectedValueOnce({ kind: "io", message: "blip" });
    await expect(port.get_capabilities()).rejects.toBeInstanceOf(Error);
    expect(port.getConnectionState()).toBe("disconnected");
    mockInvoke.mockResolvedValueOnce(validCaps);
    await port.get_capabilities();
    expect(port.getConnectionState()).toBe("connected"); // no longer suppressed
  });

  it("notify_connection_state_applies_guarded_transition_and_notifies", async () => {
    // spec(§11.4) — notifyConnectionState (the supervisor's single drive) applies the guarded
    // canTransition spec (illegal hops skipped) + notifies onConnectionChange (the Shell's one React
    // writer receives it). From connecting, connecting→reconnecting is illegal → skipped.
    const port = new UdsGatewayPort();
    const seen: string[] = [];
    port.onConnectionChange((s) => seen.push(s));

    port.notifyConnectionState("reconnecting"); // illegal from connecting → no-op
    expect(port.getConnectionState()).toBe("connecting");

    // the REJECTED hop must NOT have set streamDegraded for a state the port never entered — proven
    // behaviorally: a subsequent read SUCCESS still upgrades (not wrongly suppressed). streamDegraded
    // derives from the COMMITTED state, not the rejected arg.
    mockInvoke.mockResolvedValueOnce(validCaps);
    await port.get_capabilities();
    expect(port.getConnectionState()).toBe("connected"); // read upgrade NOT suppressed
    expect(seen).toEqual(["connected"]); // the illegal hop never notified; only the read did
  });
});
