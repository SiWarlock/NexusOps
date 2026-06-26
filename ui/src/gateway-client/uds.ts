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
// L2-B: the mutation methods (submit_action/approve/deny/preview_action) are WIRED + GATED behind
// `mutationsEnabled` — which is LIVE in production since the USER-gated L2-C go-live (ui-075). PR
// mutations + createSession (WAVE-1 agent-launch) therefore carry their OWN default-OFF per-action
// gates (`enabledPrMutations` / `enabledSessionLaunch`) so a new high-consequence capability does NOT
// auto-ride the now-true `mutationsEnabled` flip — it stays throw-never-invoke until its own USER cat-1
// sign-off. The streaming subscribe is wired (052); subscribe_terminal stays a P4 not-wired surface.
// INV-SEC-1 stays daemon-side regardless.
import { invoke, Channel } from "@tauri-apps/api/core";
import type {
  CreateSessionParams,
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
  GetExecutionProfilesResult,
  ProjectionDelta,
  ProjectionName,
  ProjectionPageByName,
  TerminalOutputFrame,
} from "../contracts/index";
import {
  parseAck,
  parseCapabilities,
  parseDelta,
  parseDiff,
  parseExecutionProfilesResult,
  parsePreview,
  parseProjectionPage,
} from "./boundary";
import {
  canTransition,
  INITIAL_CONNECTION_STATE,
  worstOfConnection,
  type ConnectionState,
} from "../connection/state";
import { PR_MUTATION_ACTION_TYPES } from "../intent/pr-mutation-request";
import { SESSION_KILL_ACTION_TYPE } from "../intent/kill-session-request";
import { SESSION_PROFILE_CHANGE_ACTION_TYPE } from "../intent/profile-change-request";
import { SESSION_DRIVE_ACTION_TYPES } from "../intent/session-drive-request";

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

const MUTATIONS_NOT_ENABLED =
  "UdsGatewayPort: L2 mutation submit is not enabled (the wire is built but gated off until the USER-gated L2-C go-live; mutationsEnabled=false)";
const PR_MUTATION_NOT_ENABLED = (actionType: string) =>
  `UdsGatewayPort: the PR mutation '${actionType}' is not enabled (guarded off until a future USER-signed-off go-live + the daemon auth-bootstrap re-review; not in enabledPrMutations)`;
const SESSION_LAUNCH_NOT_ENABLED =
  "UdsGatewayPort: agent-launch (session.create) is not enabled (held off until a USER cat-1 sign-off + visual gate; enabledSessionLaunch=false — it does NOT auto-ride the live mutationsEnabled flip)";
const SESSION_KILL_NOT_ENABLED =
  "UdsGatewayPort: agent-kill (session.kill) is not enabled (held off until a USER cat-1 sign-off + visual gate; enabledSessionKill=false — a destructive write does NOT auto-ride the live mutationsEnabled flip)";
const PROFILE_CHANGE_NOT_ENABLED =
  "UdsGatewayPort: profile-change (session.profile_change) is not enabled (held off until a USER cat-1 sign-off + visual gate; enabledProfileChange=false — a session live-write does NOT auto-ride the live mutationsEnabled flip; defense-in-depth to the daemon risk-2 approval)";
const SESSION_DRIVE_NOT_ENABLED = (actionType: string) =>
  `UdsGatewayPort: session-drive '${actionType}' is not enabled (held off until a USER cat-1 sign-off + visual gate; not in enabledSessionControls — an approval-gated drive write does NOT auto-ride the live mutationsEnabled flip; defense-in-depth to the daemon approval)`;

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

  /** The L2 go-live gate (056) — default FALSE: no live mutation reaches the daemon until the
   *  USER-gated L2-C constructs the port with `{mutationsEnabled:true}`. Read by BOTH the mutation
   *  methods (throw-not-enabled + never invoke when false) AND the UI submit controls (disabled when
   *  false) — the single switch L2-C flips to light up the wire + the controls together. */
  readonly mutationsEnabled: boolean;

  /** The PER-ACTION PR-mutation go-live gate (cat-1, ui-070/071 fork-1b) — the SET of enabled PR-mutation
   *  action types; default EMPTY (all HELD); SEPARATE from `mutationsEnabled` (already true in production).
   *  A PR mutation reaches the wire only when its action_type is in this set (throw-never-invoke in
   *  `submit_action` + the UI control disabled). The flip is a future USER-signed-off slice (per-action,
   *  + auth-bootstrap re-review) — never populated in production today. */
  readonly enabledPrMutations: ReadonlySet<string>;

  /** The per-action AGENT-LAUNCH go-live gate (WAVE-1 Slice A; the `enabledPrMutations` mirror).
   *  default FALSE: createSession throws-never-invokes (+ the Launch control disabled) until a USER
   *  cat-1 sign-off flips it — SEPARATE from `mutationsEnabled` (already TRUE in prod since ui-075). */
  readonly enabledSessionLaunch: boolean;

  /** The per-action AGENT-KILL go-live gate (WAVE-1 Slice B; the `enabledSessionLaunch` mirror).
   *  default FALSE: a session.kill submit_action throws-never-invokes (+ the Kill control disabled) until
   *  a USER cat-1 sign-off flips it — a DESTRUCTIVE write does not auto-ride the already-live mutationsEnabled. */
  readonly enabledSessionKill: boolean;

  /** The per-action PROFILE-CHANGE go-live gate (WAVE-1 W1-C-a; the `enabledSessionKill` mirror).
   *  default FALSE: a session.profile_change submit_action throws-never-invokes (+ the Change-profile
   *  control disabled) until a USER cat-1 sign-off flips it — defense-in-depth to the daemon risk-2 approval. */
  readonly enabledProfileChange: boolean;

  /** The per-action SESSION-DRIVE go-live gate (WAVE-1 W1-C-b; the `enabledPrMutations` Set mirror).
   *  default EMPTY: a drive submit_action (send_message/pause/resume) throws-never-invokes (+ the control
   *  disabled) until a USER cat-1 sign-off adds its type to the set — seeds the per-control consolidation. */
  readonly enabledSessionControls: ReadonlySet<string>;

  constructor(
    opts: {
      mutationsEnabled?: boolean;
      enabledPrMutations?: ReadonlySet<string>;
      enabledSessionLaunch?: boolean;
      enabledSessionKill?: boolean;
      enabledProfileChange?: boolean;
      enabledSessionControls?: ReadonlySet<string>;
    } = {},
  ) {
    this.mutationsEnabled = opts.mutationsEnabled ?? false;
    this.enabledPrMutations = opts.enabledPrMutations ?? new Set();
    this.enabledSessionLaunch = opts.enabledSessionLaunch ?? false;
    this.enabledSessionKill = opts.enabledSessionKill ?? false;
    this.enabledProfileChange = opts.enabledProfileChange ?? false;
    this.enabledSessionControls = opts.enabledSessionControls ?? new Set();
  }

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

  async get_pr_diff(
    repo_id: string,
    pr_number: number,
    file: string | null,
  ): Promise<DiffResult> {
    // D7 — the remote-PR code-diff; REUSE parseDiff (the return shape is the frozen DiffResult).
    // Tauri auto-converts camelCase → snake_case (repoId → repo_id, prNumber → pr_number).
    return this.invokeRead(parseDiff, "gateway_get_pr_diff", {
      repoId: repo_id,
      prNumber: pr_number,
      file,
    });
  }

  async get_capabilities(): Promise<Capabilities> {
    return this.invokeRead(parseCapabilities, "gateway_get_capabilities");
  }

  // §6.1 get_execution_profiles (W1-prof) — the no-param read RPC; invoke + boundary-parse the typed
  // secret-free result. A READ → NOT gated on `mutationsEnabled` (the get_diff/get_pr_diff precedent;
  // the profile list is read-only display data). A `WireError` routes VERBATIM via the shared
  // invokeRead/handleError path (LESSON §16); a malformed payload → BoundaryValidationError (fail-closed).
  async get_execution_profiles(): Promise<GetExecutionProfilesResult> {
    return this.invokeRead(parseExecutionProfilesResult, "gateway_get_execution_profiles");
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

  /** True while ANY supervised subscribe stream reports down/recovering (054 → ui-059 N-stream). The
   *  read-path UPGRADE is suppressed while set, so an ad-hoc read can't mask a degraded stream. */
  private streamDegraded = false;

  /** Each live subscribe stream's last reported lifecycle, keyed by streamId (054 → ui-059). The global
   *  connection is driven to the WORST-OF these (worstOfConnection), so a 2nd stream can neither mask
   *  the 1st's degrade nor be masked by it. */
  private readonly streamStates = new Map<string, ConnectionState>();

  /** The subscribe supervisors' connection-state drive — the SINGLE authority (054; ui-059 per-stream).
   *  Records this stream's reported lifecycle, then drives the one guarded `setConnection` to the
   *  worst-of aggregate across ALL streams. Setting it here (the port), not a second raw React setter in
   *  the Shell, is what makes the read-path suppression + cross-stream non-masking possible. */
  notifyConnectionState(streamId: string, next: ConnectionState): void {
    this.streamStates.set(streamId, next);
    // Drive the global to the worst-of aggregate of all reported streams (LOAD-BEARING — canSubmitIntent
    // reads the global, so any-degraded must keep it non-connected, §11.1 fail-safe). An empty set can't
    // occur here (we just set one), but worstOfConnection is null-safe; keep the current state if so.
    const aggregate = worstOfConnection([...this.streamStates.values()]);
    if (aggregate !== null) this.setConnection(aggregate);
    // Derive streamDegraded from the COMMITTED state (this.connection after the guarded transition),
    // NOT the requested aggregate — a rejected/no-op hop must never flip the suppression flag for a
    // state the port never actually entered (else a later read upgrade would be wrongly suppressed).
    // Preserved verbatim from 054, now over the N-stream aggregate.
    this.streamDegraded =
      this.connection === "disconnected" || this.connection === "reconnecting";
  }

  private markConnected(): void {
    // Suppress the read-path UPGRADE while the subscribe stream is supervisor-degraded — a successful
    // ad-hoc read (e.g. a Code-view get_diff) must NOT re-assert `connected` over a down/recovering
    // live stream (the 052 masking; forbidden #6 / LESSON 4). DEGRADE is never suppressed (below).
    if (this.streamDegraded) return;
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

  // ── the §6.1 mutation-intent surface (L2-B) — invoke the typed command + boundary-parse, GATED ──
  // `mutationsEnabled` is LIVE in production (the USER-gated L2-C go-live, ui-075), so it is no longer
  // the sole guard: each method THROWS + NEVER `invoke`s when its applicable gate is off — the general
  // `mutationsEnabled` for the L2 set, the per-action `enabledPrMutations` for PR writes, and
  // `enabledSessionLaunch` for agent-launch (each default-OFF capability HELD until its own USER cat-1
  // sign-off, so a new high-consequence write never auto-rides the live `mutationsEnabled` flip).
  // When enabled, each reuses the read-command invoke path (`invokeRead` — invoke + markConnected
  // [054-suppressed] + boundary-parse), so the wire-rejection (plain {code}) vs transport-fault (Error)
  // classification is inherited identically (LESSON 22). The UI still never mutates: a submit SENDS a
  // typed intent; the daemon Gateway is the INV-SEC-1 chokepoint (defense-in-depth: the control is
  // disabled + the seam/gate + this throws — layered when not enabled).
  private assertMutationsEnabled(): void {
    if (!this.mutationsEnabled) throw new Error(MUTATIONS_NOT_ENABLED);
  }

  private assertSessionLaunchEnabled(): void {
    if (!this.enabledSessionLaunch) throw new Error(SESSION_LAUNCH_NOT_ENABLED);
  }

  /** cat-1 (ui-070/071 fork-1b) — a PR-mutation action_type (github.merge_pr / github.submit_review)
   *  reaches the wire ONLY when its type is in the SEPARATE `enabledPrMutations` set. Per-action +
   *  independent of `mutationsEnabled` (a PR mutation can't ride the already-live L2 flag, and enabling
   *  one PR mutation never enables another); throws BEFORE invoke → the provably-unreachable layer ([[27]]). */
  private assertPrMutationsEnabledFor(actionType: string): void {
    if (PR_MUTATION_ACTION_TYPES.has(actionType) && !this.enabledPrMutations.has(actionType)) {
      throw new Error(PR_MUTATION_NOT_ENABLED(actionType));
    }
  }

  /** WAVE-1 Slice B — a `session.kill` submit reaches the wire ONLY when `enabledSessionKill` is true.
   *  Action-type-SCOPED (keyed on `session.kill`) + independent of `mutationsEnabled` (a destructive
   *  write can't ride the already-live L2 flag); throws BEFORE invoke → the provably-unreachable layer
   *  ([[27]]). Other action types are unaffected (no L2 regression). The `assertPrMutationsEnabledFor` mirror. */
  private assertSessionKillEnabledFor(actionType: string): void {
    if (actionType === SESSION_KILL_ACTION_TYPE && !this.enabledSessionKill) {
      throw new Error(SESSION_KILL_NOT_ENABLED);
    }
  }

  /** WAVE-1 W1-C-a — a `session.profile_change` submit reaches the wire ONLY when `enabledProfileChange`
   *  is true. Action-type-SCOPED + independent of `mutationsEnabled` (an approval-gated session live-write
   *  can't ride the live L2 flag); throws BEFORE invoke → the provably-unreachable layer ([[27]]).
   *  Defense-in-depth to the daemon risk-2 §15 #8 approval. The `assertSessionKillEnabledFor` mirror. */
  private assertProfileChangeEnabledFor(actionType: string): void {
    if (actionType === SESSION_PROFILE_CHANGE_ACTION_TYPE && !this.enabledProfileChange) {
      throw new Error(PROFILE_CHANGE_NOT_ENABLED);
    }
  }

  /** WAVE-1 W1-C-b — a session-drive submit (send_message/pause/resume) reaches the wire ONLY when its
   *  action_type is in the `enabledSessionControls` set. Per-action (enabling pause ≠ enabling resume) +
   *  independent of `mutationsEnabled` (an approval-gated drive write can't ride the live L2 flag); throws
   *  BEFORE invoke → the provably-unreachable layer ([[27]]). Non-drive types are unaffected. The
   *  `assertPrMutationsEnabledFor` Set mirror. */
  private assertSessionControlEnabledFor(actionType: string): void {
    if (SESSION_DRIVE_ACTION_TYPES.has(actionType) && !this.enabledSessionControls.has(actionType)) {
      throw new Error(SESSION_DRIVE_NOT_ENABLED(actionType));
    }
  }

  async submit_action(request: ActionRequest): Promise<ActionAck> {
    this.assertMutationsEnabled();
    this.assertPrMutationsEnabledFor(request.action_type);
    this.assertSessionKillEnabledFor(request.action_type);
    this.assertProfileChangeEnabledFor(request.action_type);
    this.assertSessionControlEnabledFor(request.action_type);
    return this.invokeRead(parseAck, "gateway_submit_action", { request });
  }
  // No PR-mutation guard here by design: preview_action takes only an opaque action_request_id (no
  // action_type to gate on); the PR-mutation gate lives on submit_action (where the typed request is) +
  // the daemon adjudicates. (The asymmetry is intentional — preview is a read-shaped daemon fetch.)
  async preview_action(action_request_id: string): Promise<ActionPreview> {
    this.assertMutationsEnabled();
    return this.invokeRead(parsePreview, "gateway_preview_action", {
      actionRequestId: action_request_id,
    });
  }
  async approve(approval_id: string, step_id?: string): Promise<ActionAck> {
    this.assertMutationsEnabled();
    return this.invokeRead(parseAck, "gateway_approve", {
      approvalId: approval_id,
      stepId: step_id ?? null,
    });
  }
  async deny(approval_id: string, reason: string): Promise<ActionAck> {
    this.assertMutationsEnabled();
    return this.invokeRead(parseAck, "gateway_deny", { approvalId: approval_id, reason });
  }
  // §6.1 session.create (W1-A) — gated like submit_action (throws-never-invokes when disabled).
  // camelCase args → the Tauri command's snake_case Rust params; absent optionals → null (→ Rust
  // None, omitted). Boundary-parses the typed ActionAck (the daemon mints the id — no client-mint).
  async createSession(params: CreateSessionParams): Promise<ActionAck> {
    this.assertMutationsEnabled();
    this.assertSessionLaunchEnabled(); // the default-OFF per-action gate (agent-launch HELD)
    return this.invokeRead(parseAck, "gateway_create_session", {
      projectId: params.project_id,
      initialPrompt: params.initial_prompt ?? null,
      executionProfileId: params.execution_profile_id ?? null,
    });
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
