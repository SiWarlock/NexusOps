// UdsGatewayPort — the live §6.1 READ transport (P6.8 L1, slice 051, Layer A).
//
// The real GatewayPort the Shell's MockGatewayPort swaps to: each single-shot read
// (get_projection / get_diff / get_capabilities) `invoke`s a 050 Tauri read command
// (which calls the 049 nexusops-gateway-uds crate over the daemon's gateway.sock) and
// Zod-`.parse()`s the returned payload at the boundary.ts seam (parse-don't-trust —
// a malformed daemon payload FAILS CLOSED, never reaching view code).
//
// Error model (the security-load-bearing distinction — LESSON §16):
//   • a daemon WIRE rejection ({kind:"wire", code}) → thrown as PLAIN data {code} (a
//     WireError-shaped value, NEVER an Error instance) so the consumer (the intent
//     seam / DiffReview) classifies it via `!instanceof Error` + routes the §6.4 code
//     VERBATIM (a wire-rejection is daemon data, not a runtime fault).
//   • a TRANSPORT/host fault (io / protocol / serde / version_skew / internal) → an
//     Error instance (an honest degrade, §11.7) — never faked as a wire code.
//
// NON-cat-1: reads only. The mutation methods (submit_action/approve/deny/preview_action)
// + the streaming subscribe + subscribe_terminal are NOT wired here (no Tauri mutation/
// subscribe command exists) — they throw a not-wired error so the read client can NEVER
// reach a mutation (the L2 mutation transport is cat-1 HELD; subscribe streaming = 052).
import { invoke, Channel } from "@tauri-apps/api/core";
import type {
  GatewayPort,
  ProjectionPageParams,
  ProjectionScope,
  SubscribeParams,
} from "./types";
import type {
  ActionAck,
  ActionPreview,
  ActionRequest,
  Capabilities,
  DiffResult,
  ProjectionDelta,
  ProjectionName,
  ProjectionPageByName,
  TerminalOutputFrame,
} from "../contracts/index";
import {
  parseCapabilities,
  parseDelta,
  parseDiff,
  parseProjectionPage,
} from "./boundary";
import {
  canTransition,
  INITIAL_CONNECTION_STATE,
  type ConnectionState,
} from "../connection/state";

/** The serializable error the 050 bridge rejects with (`GatewayCommandError`, snake_case
 *  `kind` tag). Only `kind`/`code`/`message` are read here; the structural variants
 *  (version_skew/frame_too_large) carry extra fields we don't need. */
interface GatewayCommandError {
  kind: string;
  code?: string;
  message?: string;
}

function isGatewayCommandError(e: unknown): e is GatewayCommandError {
  return (
    typeof e === "object" &&
    e !== null &&
    "kind" in e &&
    typeof (e as { kind: unknown }).kind === "string"
  );
}

const NOT_WIRED_L2 =
  "UdsGatewayPort: the mutation transport is not wired (L2 — cat-1 HELD on the daemon 0.30.0 ②-mini)";

/** A frame received over the subscribe `Channel` (the TS mirror of the 050 bridge's
 *  `SubscriptionEvent`): `delta` carries a raw daemon delta (boundary-parsed before it's yielded),
 *  `closed` is the daemon's clean lag-close (→ the iterable ends), `error` is a transport fault
 *  (→ the iterable throws). The tag distinguishes ends-vs-errors unambiguously (§11.7). */
export type SubscriptionEvent =
  | { kind: "delta"; delta: unknown }
  | { kind: "closed" }
  | { kind: "error"; error: unknown };

/**
 * Turn a callback-push subscription source into an `AsyncIterable<ProjectionDelta>` — parse-don't-trust
 * (each delta is `parseDelta`-validated at the boundary before it's yielded; a malformed delta throws
 * `BoundaryValidationError` and is never yielded). `start` registers the message handler + returns the
 * transport promise; it is invoked EAGERLY so deltas buffer from the moment we subscribe (no gap
 * before the first `await`). The iterable ENDS on a `closed` event and THROWS on an `error` event
 * (honest degrade, never silent §11.7) — the Shell recovery treats either as "stream ended → reconnect".
 * Factored out so the streaming logic is unit-testable with a fake `start` (no real `Channel`/`invoke`).
 *
 * Single-threaded note: the `wake` resolve is assigned synchronously inside the Promise executor
 * before the `await` suspends, so a message that arrives while we're suspended can never be lost.
 */
export function subscriptionIterable(
  start: (onMessage: (e: SubscriptionEvent) => void) => Promise<unknown>,
): AsyncIterable<ProjectionDelta> {
  const queue: SubscriptionEvent[] = [];
  let wake: (() => void) | null = null;
  const onMessage = (e: SubscriptionEvent): void => {
    queue.push(e);
    const w = wake;
    wake = null;
    w?.();
  };
  // start eagerly; a rejected transport promise surfaces as an error event (never an unhandled reject).
  const startPromise = start(onMessage).catch((err: unknown) => {
    onMessage({ kind: "error", error: err });
  });
  return {
    async *[Symbol.asyncIterator](): AsyncGenerator<ProjectionDelta> {
      // `startPromise`'s `.catch` (an invoke rejection → an error event) is attached eagerly at the
      // `start()` call above; reference it here so it isn't flagged unused (no floating rejection).
      void startPromise;
      for (;;) {
        while (queue.length > 0) {
          const e = queue.shift()!;
          if (e.kind === "delta") {
            yield parseDelta(e.delta); // throws BoundaryValidationError on a malformed delta
          } else if (e.kind === "closed") {
            return;
          } else {
            throw e.error instanceof Error
              ? e.error
              : new Error(`subscription stream error: ${JSON.stringify(e.error)}`);
          }
        }
        await new Promise<void>((resolve) => {
          wake = resolve;
        });
      }
    },
  };
}

export class UdsGatewayPort implements GatewayPort {
  // Transport liveness, driven from the read outcomes (the real socket's success/failure).
  // Fail-safe: starts "connecting" (read-only) until a daemon response confirms it (LESSON §4).
  private connection: ConnectionState = INITIAL_CONNECTION_STATE;
  private readonly listeners = new Set<(state: ConnectionState) => void>();

  // ── the §6.1 read surface (single-shot — invoke + boundary-parse) ──────────────────

  async get_projection<K extends ProjectionName>(
    name: K,
    scope?: ProjectionScope,
    _page?: ProjectionPageParams,
  ): Promise<ProjectionPageByName[K]> {
    return this.invokeRead(
      (raw) => parseProjectionPage(name, raw) as ProjectionPageByName[K],
      "gateway_get_projection",
      { name, scope },
    );
  }

  async get_diff(worktree_id: string, file: string): Promise<DiffResult> {
    // Tauri auto-converts JS camelCase args → the Rust snake_case command params.
    return this.invokeRead(parseDiff, "gateway_get_diff", {
      worktreeId: worktree_id,
      file,
    });
  }

  async get_capabilities(): Promise<Capabilities> {
    return this.invokeRead(parseCapabilities, "gateway_get_capabilities");
  }

  /** Invoke a read command, mark the connection live on a daemon response, then
   *  boundary-parse the payload (parse OUTSIDE the try so a BoundaryValidationError
   *  propagates verbatim — a malformed payload is a fail-closed read error, NOT a
   *  transport fault to remap). */
  private async invokeRead<T>(
    parse: (raw: unknown) => T,
    cmd: string,
    args?: Record<string, unknown>,
  ): Promise<T> {
    let raw: unknown;
    try {
      // omit the args object entirely for a no-param command (get_capabilities) — a
      // bare invoke(cmd), not invoke(cmd, undefined).
      raw = args === undefined ? await invoke(cmd) : await invoke(cmd, args);
    } catch (e) {
      this.handleError(e); // : never — maps wire-vs-transport + drives connection state
    }
    this.markConnected();
    return parse(raw);
  }

  /** Map a caught read error to the GatewayPort error model (see the module header). */
  private handleError(e: unknown): never {
    if (isGatewayCommandError(e)) {
      if (e.kind === "wire" && typeof e.code === "string") {
        // PLAIN data (NOT an Error) so the §6.4 code routes verbatim (LESSON §16). The
        // connection is left unchanged: a wire-rejection is a routed read outcome, not
        // a transport-liveness signal (the load path confirms connectivity via success).
        throw { code: e.code };
      }
      // A non-wire fault is a transport/host fault — degrade the connection (fail-safe:
      // drops `canSubmitIntent` per §11.4) for any fault that means the link can't be
      // trusted as "connected": io (socket down), protocol/serde/frame_too_large (the
      // daemon produced an untrustworthy frame), internal (host fault). EXCEPT
      // `version_skew` — the daemon answered the handshake; that's the version axis
      // (checkVersionCompat), not transport liveness. (A `wire{code}` without a string
      // code also lands here — a malformed bridge response → treat as a transport fault,
      // NOT a fabricated §6.4 code.)
      if (e.kind !== "version_skew") this.markDisconnected();
      const detail = typeof e.message === "string" ? `: ${e.message}` : "";
      throw new Error(
        `UdsGatewayPort: gateway transport error (${e.kind})${detail}`,
      );
    }
    // an unexpected rejection (a real JS Error / a non-object throw) — surface as an Error.
    throw e instanceof Error
      ? e
      : new Error(`UdsGatewayPort: unexpected error: ${String(e)}`);
  }

  // ── connection management (§11.4 transport liveness; NOT part of the §6.1 RPC set) ──

  getConnectionState(): ConnectionState {
    return this.connection;
  }

  onConnectionChange(cb: (state: ConnectionState) => void): () => void {
    this.listeners.add(cb);
    return () => {
      this.listeners.delete(cb);
    };
  }

  /** The Retry/Repair affordance — re-arm the transport; the next read confirms or fails it.
   *  From the pre-connection "connecting" state this is a no-op (already attempting first
   *  contact, and connecting→reconnecting is not a legal hop) — not reachable in L1 (the
   *  Retry banner needs loaded `data`, which implies a prior `connected`). The full
   *  cross-state reconnect drives the 052 subscribe-recovery machine. */
  reconnect(): void {
    this.setConnection("reconnecting");
  }

  private markConnected(): void {
    this.setConnection("connected");
  }

  private markDisconnected(): void {
    this.setConnection("disconnected");
  }

  /** Drive a connection transition, respecting the legal-transition spec (illegal/no-op
   *  hops are skipped) + notifying subscribers only on an actual change. */
  private setConnection(next: ConnectionState): void {
    if (this.connection === next) return;
    if (!canTransition(this.connection, next)) return;
    this.connection = next;
    for (const cb of this.listeners) cb(next);
  }

  // ── un-wired surfaces (reads-only L1) — the read client can NEVER reach these ────────

  // The §6.1 mutation-intent surface — L2, cat-1 HELD. No Tauri mutation command exists;
  // these reject so a mutation can never be submitted through the read client.
  async submit_action(_request: ActionRequest): Promise<ActionAck> {
    throw new Error(NOT_WIRED_L2);
  }
  async preview_action(_action_request_id: string): Promise<ActionPreview> {
    throw new Error(NOT_WIRED_L2);
  }
  async approve(_approval_id: string, _step_id?: string): Promise<ActionAck> {
    throw new Error(NOT_WIRED_L2);
  }
  async deny(_approval_id: string, _reason: string): Promise<ActionAck> {
    throw new Error(NOT_WIRED_L2);
  }

  // The streaming subscribe (§6.1 ProjectionDelta) — WIRED at 052. Opens a dedicated persistent
  // connection via the `gateway_subscribe` Tauri Channel command (the 049 `subscribe_stream` runs
  // off the async runtime), and returns an AsyncIterable that boundary-`parseDelta`s each pushed
  // frame. The iterable ends on the daemon's lag-close + throws on a transport error (§11.7); the
  // Shell recovery reconnects + re-`get_projection`s on either (the seq-less stream can't gap-fill).
  subscribe(params: SubscribeParams): AsyncIterable<ProjectionDelta> {
    return subscriptionIterable((onMessage) => {
      const channel = new Channel<SubscriptionEvent>();
      // eslint-disable-next-line unicorn/prefer-add-event-listener -- the Tauri ipc::Channel exposes only an `onmessage` setter (one handler per channel), not addEventListener.
      channel.onmessage = onMessage;
      return invoke("gateway_subscribe", {
        projection: params.projection,
        onEvent: channel,
      });
    });
  }

  // The §6.4 terminal-output demux is a P4 transport surface (LESSON §18) — not wired in L1.
  subscribe_terminal(_terminal_id: string): AsyncIterable<TerminalOutputFrame> {
    throw new Error(
      "UdsGatewayPort.subscribe_terminal: the terminal channel is not wired (P4)",
    );
  }
}
