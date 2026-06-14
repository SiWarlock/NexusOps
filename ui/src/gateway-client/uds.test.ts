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

  it("subscribe() returns an AsyncIterable (wired at 052 — replaces the 051 not-wired throw)", () => {
    mockInvoke.mockResolvedValue(undefined);
    const port = new UdsGatewayPort();
    const iter = port.subscribe({ projection: "Session" });
    expect(typeof iter[Symbol.asyncIterator]).toBe("function");
  });
});
