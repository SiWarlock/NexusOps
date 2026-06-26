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

// L2-B mutation fixtures — contract-valid ActionAck / ActionPreview the boundary parsers accept.
const validAck = { action_request_id: "act_1", status: "submitted" };
const validPreview = {
  action_request_id: "act_1",
  generated_at: "2026-06-14T00:00:00Z",
  risk_level: 2,
  risk_reasons: ["touches a file"],
  summary: "Would stage 1 hunk.",
  changed_resources: [],
  cannot_preview_reason: null,
};
// the request rides to the daemon opaquely (the daemon validates) — a minimal cast for the invoke-arg pin.
const sampleRequest = { action_type: "git.stage_hunk" } as ActionRequest;

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

  it("uds_get_pr_diff_invokes_and_parses (§6.1 — mirror get_diff; D7)", async () => {
    // invoke gateway_get_pr_diff with camelCase args + boundary-parse the DiffResult (parse-don't-
    // trust). The wire-vs-transport routing is the shared invokeRead path; this pins get_pr_diff's
    // invoke shape, the fail-closed boundary parse, and the wire→plain {code} (LESSON §16) for it.
    mockInvoke.mockResolvedValue(validDiff);
    const port = new UdsGatewayPort();

    const diff = await port.get_pr_diff("repo_1", 101, null);
    // Tauri auto-converts JS camelCase → Rust snake_case (repoId → repo_id, prNumber → pr_number).
    expect(mockInvoke).toHaveBeenCalledWith("gateway_get_pr_diff", {
      repoId: "repo_1",
      prNumber: 101,
      file: null,
    });
    expect(diff.hunks).toHaveLength(1);

    // a malformed payload → BoundaryValidationError (fail-closed), never a bad partial value.
    mockInvoke.mockResolvedValue({ not_a_diff: true });
    await expect(port.get_pr_diff("repo_1", 101, null)).rejects.toBeInstanceOf(
      BoundaryValidationError,
    );

    // a daemon WIRE error → PLAIN {code} (NOT an Error) so the §6.4 code routes verbatim (the
    // consumer's WireError.safeParse must match it; the not_found honest-unavailable path).
    mockInvoke.mockRejectedValue({ kind: "wire", code: "not_found" });
    let thrown: unknown;
    try {
      await port.get_pr_diff("repo_1", 101, null);
    } catch (e) {
      thrown = e;
    }
    expect(thrown instanceof Error).toBe(false);
    expect(WireError.safeParse(thrown).success).toBe(true);
    expect((thrown as { code: string }).code).toBe("not_found");
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

  it("mutation_methods_throw_not_enabled_by_default_and_never_invoke", async () => {
    // spec(L2 cat-1 crux) — the default port (mutationsEnabled=false) THROWS "not enabled" on EVERY
    // mutation method AND never `invoke`s → NO production path reaches a live mutation (the enable is
    // L2-C, USER-gated). This is the no-production-reach guard the whole L2-B wire is built behind.
    const port = new UdsGatewayPort();

    await expect(
      port.submit_action(sampleRequest),
    ).rejects.toThrow(/not enabled/i);
    await expect(port.preview_action("act_1")).rejects.toThrow(/not enabled/i);
    await expect(port.approve("appr_1")).rejects.toThrow(/not enabled/i);
    await expect(port.deny("appr_1", "no")).rejects.toThrow(/not enabled/i);
    // createSession (W1-A) is a mutation (launches an agent) → same gate, throws-never-invokes.
    await expect(port.createSession({ project_id: "proj_1" })).rejects.toThrow(/not enabled/i);

    // THE cat-1 crux: NOT ONE invoke happened — the guard short-circuits BEFORE any transport reach.
    expect(mockInvoke).not.toHaveBeenCalled();
    // subscribe_terminal stays a P4 not-wired surface (unchanged; a READ-channel, not a mutation).
    expect(() => port.subscribe_terminal("t1")).toThrow(/not wired|P4/i);
  });

  it("mutation_methods_invoke_and_parse_when_enabled", async () => {
    // spec(§6.1) — with the flag forced ON (test-only; production stays false until L2-C), each
    // mutation method invokes its typed Tauri command + boundary-parses the typed result.
    const port = new UdsGatewayPort({ mutationsEnabled: true });

    mockInvoke.mockResolvedValueOnce(validAck);
    const ack = await port.submit_action(sampleRequest);
    expect(ack.action_request_id).toBe("act_1");
    expect(mockInvoke).toHaveBeenCalledWith("gateway_submit_action", { request: sampleRequest });

    mockInvoke.mockResolvedValueOnce(validPreview);
    const preview = await port.preview_action("act_1");
    expect(preview.summary).toBe("Would stage 1 hunk.");
    expect(mockInvoke).toHaveBeenCalledWith("gateway_preview_action", { actionRequestId: "act_1" });

    mockInvoke.mockResolvedValueOnce(validAck);
    await port.approve("appr_1", "step_2");
    expect(mockInvoke).toHaveBeenCalledWith("gateway_approve", {
      approvalId: "appr_1",
      stepId: "step_2",
    });

    // step_id omitted → stepId:null on the wire (Tauri deserializes null → the Rust Option<String> None,
    // matching the daemon's "absent" shape) — pin the None path so a future null↔None drift is caught.
    mockInvoke.mockResolvedValueOnce(validAck);
    await port.approve("appr_1");
    expect(mockInvoke).toHaveBeenCalledWith("gateway_approve", {
      approvalId: "appr_1",
      stepId: null,
    });

    mockInvoke.mockResolvedValueOnce(validAck);
    await port.deny("appr_1", "no");
    expect(mockInvoke).toHaveBeenCalledWith("gateway_deny", { approvalId: "appr_1", reason: "no" });
  });

  it("mutation_wire_rejection_is_plain_data_not_error", async () => {
    // spec(L2-D6/LESSON 22) — on an ENABLED submit, a daemon WireError → the verbatim code as PLAIN
    // {code} (NOT an Error instance) so describeRejection routes the §6.4 code; a transport fault →
    // an Error (honest degrade). Same classification as the reads (the shared handleError path).
    const port = new UdsGatewayPort({ mutationsEnabled: true });

    mockInvoke.mockRejectedValueOnce({ kind: "wire", code: "fencing_conflict" });
    const wireErr = await port.submit_action(sampleRequest).then(
      () => null,
      (e: unknown) => e,
    );
    expect(wireErr).not.toBeInstanceOf(Error); // a daemon rejection is DATA, not a runtime fault
    expect(WireError.safeParse(wireErr).success).toBe(true);
    expect((wireErr as { code: string }).code).toBe("fencing_conflict"); // verbatim §6.4 code

    // a transport fault → an Error instance (honest degrade, never faked as a wire code).
    mockInvoke.mockRejectedValueOnce({ kind: "io", message: "down" });
    await expect(port.approve("appr_1", "step_2")).rejects.toBeInstanceOf(Error);
  });

  it("mutation_malformed_result_is_boundary_error", async () => {
    // spec(§5.0/LESSON 22) — a non-ActionAck result from the daemon → BoundaryValidationError (the
    // parseAck fail-closed path), never a fabricated ack; symmetric with the reads' parse-don't-trust.
    const port = new UdsGatewayPort({ mutationsEnabled: true });
    mockInvoke.mockResolvedValueOnce({ not_an_ack: true });
    await expect(port.submit_action(sampleRequest)).rejects.toBeInstanceOf(
      BoundaryValidationError,
    );
  });

  it("mutation_malformed_preview_is_boundary_error", async () => {
    // spec(§5.0/LESSON 22) — a non-ActionPreview result → BoundaryValidationError (parsePreview
    // fail-closed); the human never approves against a fabricated/un-parsed preview.
    const port = new UdsGatewayPort({ mutationsEnabled: true });
    mockInvoke.mockResolvedValueOnce({ not_a_preview: true });
    await expect(port.preview_action("act_1")).rejects.toBeInstanceOf(
      BoundaryValidationError,
    );
  });

  it("create_session_invokes_omitting_absent_optionals_when_enabled", async () => {
    // spec(§6.1/W1-A) — enabled, createSession invokes gateway_create_session with the active
    // project_id (camelCase→snake_case) + absent optionals as null (Tauri→Rust None), and
    // boundary-parses the typed ActionAck (the DAEMON mints the id — no client-mint).
    const port = new UdsGatewayPort({ mutationsEnabled: true, enabledSessionLaunch: true });
    mockInvoke.mockResolvedValueOnce(validAck);
    const ack = await port.createSession({ project_id: "proj_1" });
    expect(ack.action_request_id).toBe("act_1");
    expect(mockInvoke).toHaveBeenCalledWith("gateway_create_session", {
      projectId: "proj_1",
      initialPrompt: null,
      executionProfileId: null,
    });
    // an initial_prompt threads through as initialPrompt (the dev-drive prompt).
    mockInvoke.mockResolvedValueOnce(validAck);
    await port.createSession({ project_id: "proj_2", initial_prompt: "drive me" });
    expect(mockInvoke).toHaveBeenCalledWith("gateway_create_session", {
      projectId: "proj_2",
      initialPrompt: "drive me",
      executionProfileId: null,
    });
  });

  it("create_session_wire_rejection_plain_and_malformed_is_boundary_error", async () => {
    // spec(LESSON 22) — a daemon WireError → the verbatim code as PLAIN {code} (not an Error); a
    // non-ActionAck result → BoundaryValidationError (parseAck fail-closed). Same classification as
    // submit_action (the shared invokeRead/handleError path).
    const port = new UdsGatewayPort({ mutationsEnabled: true, enabledSessionLaunch: true });
    mockInvoke.mockRejectedValueOnce({ kind: "wire", code: "precondition_stale" });
    const wireErr = await port
      .createSession({ project_id: "proj_1" })
      .then(() => null, (e: unknown) => e);
    expect(wireErr).not.toBeInstanceOf(Error);
    expect((wireErr as { code: string }).code).toBe("precondition_stale");

    mockInvoke.mockResolvedValueOnce({ not_an_ack: true });
    await expect(port.createSession({ project_id: "proj_1" })).rejects.toBeInstanceOf(
      BoundaryValidationError,
    );
  });

  it("create_session_held_when_session_launch_disabled_even_with_mutations_enabled", async () => {
    // spec(W1-A go-live gate / LESSON 36/37) — the PRODUCTION config (mutationsEnabled:true since the
    // ui-075 L2-C go-live) must NOT auto-launch agents: createSession throws-never-invokes unless
    // enabledSessionLaunch is ALSO on (default OFF). THE held-flip pin — launch awaits a cat-1 sign-off.
    const port = new UdsGatewayPort({ mutationsEnabled: true }); // enabledSessionLaunch defaults OFF
    await expect(port.createSession({ project_id: "proj_1" })).rejects.toThrow(/not enabled/i);
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it("uds_default_enabled_session_launch_is_off", () => {
    // spec(default-OFF) — the production default holds agent-launch (no auto-go-live).
    expect(new UdsGatewayPort().enabledSessionLaunch).toBe(false);
    expect(new UdsGatewayPort({ mutationsEnabled: true }).enabledSessionLaunch).toBe(false);
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

    port.notifyConnectionState("Session", "disconnected"); // the supervisor: the live stream is down
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
    port.notifyConnectionState("Session", "disconnected");
    port.notifyConnectionState("Session", "reconnecting"); // mid-recovery (refetch not yet confirmed)
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
    port.notifyConnectionState("Session", "disconnected");
    expect(port.getConnectionState()).toBe("disconnected");

    // recover via the supervisor, then axis 2 — a read transport fault still degrades:
    port.notifyConnectionState("Session", "reconnecting");
    port.notifyConnectionState("Session", "connected");
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

    port.notifyConnectionState("Session", "disconnected"); // stream down → upgrade suppressed
    mockInvoke.mockResolvedValueOnce(validCaps);
    await port.get_capabilities();
    expect(port.getConnectionState()).toBe("disconnected"); // suppressed

    port.notifyConnectionState("Session", "reconnecting");
    port.notifyConnectionState("Session", "connected"); // the supervisor recovered the stream
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

    port.notifyConnectionState("Session", "reconnecting"); // illegal from connecting → no-op
    expect(port.getConnectionState()).toBe("connecting");

    // the REJECTED hop must NOT have set streamDegraded for a state the port never entered — proven
    // behaviorally: a subsequent read SUCCESS still upgrades (not wrongly suppressed). streamDegraded
    // derives from the COMMITTED state (the aggregate after the guarded transition), NOT the rejected
    // requested arg — preserved verbatim under the ui-059 N-stream aggregate.
    mockInvoke.mockResolvedValueOnce(validCaps);
    await port.get_capabilities();
    expect(port.getConnectionState()).toBe("connected"); // read upgrade NOT suppressed
    expect(seen).toEqual(["connected"]); // the illegal hop never notified; only the read did
  });
});

// ── ui-059 — per-stream connection aggregation (the 2nd subscribe stream composes with 054) ──────
// notifyConnectionState is now (streamId, next): the port records each stream's last reported state and
// drives the global to the WORST-OF aggregate (disconnected > reconnecting > connecting > connected).
// LOAD-BEARING (not cosmetic): canSubmitIntent reads the global, so ANY degraded stream must keep the
// global non-connected (§11.1 fail-safe). A healthy stream can NEVER clear another stream's degrade.
describe("UdsGatewayPort — per-stream connection aggregation (ui-059)", () => {
  it("approvalqueue_stream_degrade_suppresses_read_upgrade", async () => {
    // spec(§11.1/§11.7) — 054's read-upgrade suppression generalized to N streams: with the
    // ApprovalQueue stream degraded, an ad-hoc read SUCCESS must NOT re-assert connected.
    const port = new UdsGatewayPort();
    mockInvoke.mockResolvedValueOnce(validCaps);
    await port.get_capabilities();
    expect(port.getConnectionState()).toBe("connected");

    port.notifyConnectionState("ApprovalQueue", "disconnected"); // the 2nd stream is down
    expect(port.getConnectionState()).toBe("disconnected");

    mockInvoke.mockResolvedValueOnce(validCaps);
    await port.get_capabilities(); // an unrelated read succeeds…
    expect(port.getConnectionState()).toBe("disconnected"); // …still suppressed (a stream is degraded)
  });

  it("healthy_stream_never_clears_other_streams_degrade", async () => {
    // spec(054/§11.1) — the load-bearing multi-stream property: a HEALTHY ApprovalQueue stream must NOT
    // clear a DEGRADED Session stream (and vice-versa). The global stays the worst-of (disconnected).
    const port = new UdsGatewayPort();
    mockInvoke.mockResolvedValueOnce(validCaps);
    await port.get_capabilities();
    expect(port.getConnectionState()).toBe("connected");

    port.notifyConnectionState("Session", "disconnected"); // Session down
    expect(port.getConnectionState()).toBe("disconnected");
    port.notifyConnectionState("ApprovalQueue", "connected"); // ApprovalQueue healthy…
    // …but the global stays disconnected (Session still degraded → worst-of). canSubmitIntent stays false.
    expect(port.getConnectionState()).toBe("disconnected");

    // and a read SUCCESS still can't upgrade while Session is degraded:
    mockInvoke.mockResolvedValueOnce(validCaps);
    await port.get_capabilities();
    expect(port.getConnectionState()).toBe("disconnected");
  });

  it("both_streams_healthy_clears_degrade", async () => {
    // spec(§11.7) — only when ALL reported streams are connected does the global return to connected and
    // the read-upgrade suppression clear (the aggregate-recovery path).
    const port = new UdsGatewayPort();
    mockInvoke.mockResolvedValueOnce(validCaps);
    await port.get_capabilities();

    port.notifyConnectionState("Session", "disconnected"); // Session down → global disconnected
    port.notifyConnectionState("ApprovalQueue", "connected"); // AQ healthy, but Session still degraded
    expect(port.getConnectionState()).toBe("disconnected");

    port.notifyConnectionState("Session", "reconnecting"); // Session recovering → worst-of reconnecting
    expect(port.getConnectionState()).toBe("reconnecting");
    port.notifyConnectionState("Session", "connected"); // both now connected → aggregate connected
    expect(port.getConnectionState()).toBe("connected");

    // suppression cleared — a read fault then success now upgrades normally (all streams healthy):
    mockInvoke.mockRejectedValueOnce({ kind: "io", message: "blip" });
    await expect(port.get_capabilities()).rejects.toBeInstanceOf(Error);
    expect(port.getConnectionState()).toBe("disconnected");
    mockInvoke.mockResolvedValueOnce(validCaps);
    await port.get_capabilities();
    expect(port.getConnectionState()).toBe("connected");
  });
});

// ─── ui-070/071 cat-1 — the per-action PR-mutation port guard (enabledPrMutations) ───────────────
describe("UdsGatewayPort — PR-mutation guard (cat-1 ui-070/071, per-action gate)", () => {
  it("enabled_pr_mutations_defaults_empty_uds", () => {
    // spec(ui-071/1b) — the per-action enablement set defaults EMPTY on the production transport (all PR
    // mutations HELD; SEPARATE from the already-live L2 mutationsEnabled).
    expect(new UdsGatewayPort().enabledPrMutations.size).toBe(0);
  });

  it("uds_submit_review_throws_never_invokes_when_not_enabled", async () => {
    // spec(cat-1 [[27]] + per-action independence) — a github.submit_review submit with submit_review NOT
    // in enabledPrMutations THROWS + NEVER invokes, EVEN with mutationsEnabled:true AND merge_pr enabled
    // (the per-action gate: enabling one PR mutation never enables another).
    const port = new UdsGatewayPort({
      mutationsEnabled: true,
      enabledPrMutations: new Set(["github.merge_pr"]),
    });
    const reviewReq = { action_type: "github.submit_review" } as ActionRequest;
    await expect(port.submit_action(reviewReq)).rejects.toThrow();
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it("uds_merge_pr_still_gated_after_refactor", async () => {
    // spec(fold-in correctness) — merge_pr throws-never-invokes unless merge_pr ∈ enabledPrMutations
    // (no regression from the ui-070 bool); enabling it (the future flip) lets it reach the wire.
    const held = new UdsGatewayPort({ mutationsEnabled: true, enabledPrMutations: new Set() });
    const mergeReq = { action_type: "github.merge_pr" } as ActionRequest;
    await expect(held.submit_action(mergeReq)).rejects.toThrow();
    expect(mockInvoke).not.toHaveBeenCalled();

    mockInvoke.mockResolvedValue(validAck);
    const enabled = new UdsGatewayPort({
      mutationsEnabled: true,
      enabledPrMutations: new Set(["github.merge_pr"]),
    });
    await enabled.submit_action(mergeReq);
    expect(mockInvoke).toHaveBeenCalledWith("gateway_submit_action", { request: mergeReq });
  });

  it("non_pr_mutation_submit_action_unaffected_by_pr_gate", async () => {
    // spec(no L2 regression) — an L2 (non-PR-mutation) submit_action is gated by mutationsEnabled ONLY,
    // NOT enabledPrMutations: mutationsEnabled:true + an EMPTY PR set still invokes.
    mockInvoke.mockResolvedValue(validAck);
    const port = new UdsGatewayPort({ mutationsEnabled: true, enabledPrMutations: new Set() });
    const l2Req = { action_type: "git.stage_hunk" } as ActionRequest;
    await port.submit_action(l2Req);
    expect(mockInvoke).toHaveBeenCalledWith("gateway_submit_action", { request: l2Req });
  });
});
