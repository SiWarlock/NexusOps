// The GatewayPort interface — the §6.1 method surface, mirrored on the UI side.
//
// 6.1a implements the READ surface only (get_projection / subscribe /
// get_capabilities). The mutation-intent methods (submit_action, approve, …) and
// the real UDS transport (§6.4 framing/handshake, UdsGatewayPort) are later
// per-slice integrations, gated on daemon Phase 1.5 — OUT of scope here.
// All daemon access in the UI flows through a single implementation of this
// interface (ui/CLAUDE.md forbidden #3).
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
import type { ConnectionState } from "../connection/state";

/** Scope filter for a projection query (provisional; widens with §6.1). */
export interface ProjectionScope {
  project_id?: string;
}

/** Pagination cursor for a projection query (provisional). */
export interface ProjectionPageParams {
  cursor?: string;
  limit?: number;
}

/** subscribe() params — the `projection` variant of §6.1's subscribe. */
export interface SubscribeParams {
  projection: string;
  filter?: Record<string, unknown>;
}

/** The read surface of the §6.1 GatewayPort contract. */
/** §6.1 `session.create` params — the cockpit's agent-launch (WAVE-1 Slice A). `project_id` is the
 *  ALREADY-REGISTERED active project (required; the daemon validates it). `initial_prompt` is the
 *  optional dev-drive prompt fed to the Claude TUI; `execution_profile_id` omitted → the seeded
 *  default. The DAEMON mints the action_request_id (dedicated `session.create` method) → no client-mint. */
export interface CreateSessionParams {
  project_id: string;
  initial_prompt?: string;
  execution_profile_id?: string;
}

export interface GatewayPort {
  get_projection<K extends ProjectionName>(
    name: K,
    scope?: ProjectionScope,
    page?: ProjectionPageParams,
  ): Promise<ProjectionPageByName[K]>;
  subscribe(params: SubscribeParams): AsyncIterable<ProjectionDelta>;

  // §6.4 Terminal-Channel display READ: subscribe to a terminal's output stream.
  // A READ (NOT a mutation — qualified to build provisionally, unlike the 043
  // intent path). Yields frozen `terminal_output` frames, OUTPUT ONLY (the §17
  // PTY-death is a daemon event→projection, never pushed over this channel — the
  // well's ended-state reads `session.status`). DISPLAY-ONLY #9: the UI renders
  // these bytes and never sends keystroke input to the PTY (no input method here).
  // The real UdsGatewayPort demux of `ServerFrame.terminal_output` is a transport/P4 spread.
  subscribe_terminal(terminal_id: string): AsyncIterable<TerminalOutputFrame>;

  get_capabilities(): Promise<Capabilities>;

  // §6.1 get_diff — a read-only structured diff (HEAD→workdir) for a file in a
  // worktree (the 6.3e Code/Diff surface). A READ (NOT a mutation — qualified to
  // adopt ahead of its consumer): the daemon resolves `worktree_id → proj_worktree.path`
  // then reads git2. Errors surface a `WireError` (e.g. the §6.4 `not_found` read
  // code) the same as the other methods; there is NO diff-mutation method here (the
  // per-hunk git.* actions go through the submit_action intent path, not this read).
  get_diff(worktree_id: string, file: string): Promise<DiffResult>;

  // §6.1 get_pr_diff (D7) — a read-only remote-PR code-diff (head-vs-base) for a
  // selected PR (the §11.2 Review-tab code panel). A READ: the daemon resolves
  // `(repo_id, pr_number) → owner/repo` then fetches from GitHub. `file=null` = the
  // whole changeset (all files' hunks flattened, no per-file attribution; a per-file
  // file-tree is a post-D7 follow-on). Returns the SAME frozen `DiffResult` as get_diff
  // (REUSED). Errors surface a `WireError` (e.g. `not_found`) like the other reads.
  get_pr_diff(repo_id: string, pr_number: number, file: string | null): Promise<DiffResult>;

  // §6.1 mutation-intent surface (daemon/src/ipc/methods.rs:169-211). INV-SEC-1 /
  // §4.2 law 1: the UI SUBMITS intents only — the daemon's Action Gateway is the
  // single executor + DB writer; the daemon mints `action_request_id`. The wire
  // params are IDs (not objects) for preview/approve/deny (the daemon owns the
  // record). Each method REJECTS with a `WireError` (the daemon's §6.4
  // `IpcErrorCode`) on the daemon error path; the intent seam surfaces that code
  // VERBATIM (never collapsed/remapped). There is NO execution method here.
  submit_action(request: ActionRequest): Promise<ActionAck>;
  preview_action(action_request_id: string): Promise<ActionPreview>;
  approve(approval_id: string, step_id?: string): Promise<ActionAck>;
  deny(approval_id: string, reason: string): Promise<ActionAck>;
  // §6.1 session.create (W1-A) — the agent-launch path. A mutation (spawns a live agent) → gated
  // on `mutationsEnabled` like submit_action; the daemon mints the id + drives the SessionExecutor.
  createSession(params: CreateSessionParams): Promise<ActionAck>;

  // Connection management (transport liveness; §11.4). These are UI-client
  // transport concerns, NOT part of the frozen §6.1 RPC method surface.
  getConnectionState(): ConnectionState;
  onConnectionChange(cb: (state: ConnectionState) => void): () => void;
  /** Attempt to (re)establish the transport — the Retry/Repair action. */
  reconnect(): void;
  /**
   * The subscribe supervisor's connection-state drive (054 — the single authority; ui-059 per-stream).
   * Each live subscribe stream reports its computed lifecycle (`disconnected`/`reconnecting`/`connected`)
   * keyed by `streamId` (e.g. "Session", "ApprovalQueue"). The port is the SINGLE writer of connection
   * state (no second raw React setter), records each stream's last state, and drives the global to the
   * WORST-OF aggregate — so an ad-hoc read can never mask a down stream AND a healthy stream can never
   * clear another stream's degrade (forbidden #6 / LESSON 4). The stream-degraded axis derives from the
   * committed aggregate; it gates the read-path UPGRADE while degraded; DEGRADE always flows (fail-safe).
   */
  notifyConnectionState(streamId: string, next: ConnectionState): void;

  /**
   * The L2 go-live gate (cat-1, 056/L2-B). The SINGLE flag both the transport (a mutation method
   * throws-not-enabled + never reaches the daemon when false) AND the UI submit controls (disabled
   * when false) honor — so L2-C is one flip that lights up the wire + the controls together.
   * `UdsGatewayPort` defaults it FALSE (production: no live mutation until the USER-gated L2-C enable);
   * `MockGatewayPort` defaults it TRUE (a fully-working test/dev port). NOT a §6.1 RPC method.
   */
  readonly mutationsEnabled: boolean;

  /**
   * The PER-ACTION PR-mutation go-live gate (cat-1, ui-070/071 fork-1b) — the SET of enabled PR-mutation
   * action types (`github.merge_pr`, `github.submit_review`). SEPARATE from `mutationsEnabled` (already
   * TRUE in production). A PR mutation reaches the wire ONLY when its action_type is in this set; the
   * transport throws-never-invokes AND the UI control is disabled otherwise. Per-action so the future
   * go-live can stage lowest-risk-first. `UdsGatewayPort` defaults it EMPTY (all HELD until a USER-signed-off
   * go-live + the daemon auth-bootstrap re-review); `MockGatewayPort` defaults it to the full set. NOT a
   * §6.1 RPC method. Read via `isPrMutationEnabled` (pr-mutation-request.ts).
   */
  readonly enabledPrMutations: ReadonlySet<string>;

  /**
   * The per-action AGENT-LAUNCH go-live gate (WAVE-1 Slice A; the `enabledPrMutations` mirror, LESSON
   * 36/37). `createSession` (spawning a live agent) reaches the wire ONLY when this is TRUE — the
   * transport throws-never-invokes AND the Launch control is disabled otherwise. SEPARATE from
   * `mutationsEnabled` (already TRUE in production since the ui-075 L2-C go-live), so a new
   * high-consequence capability does NOT auto-ride that flip. `UdsGatewayPort` defaults it FALSE (HELD
   * until a USER cat-1 sign-off + visual gate); `MockGatewayPort` defaults it TRUE. NOT a §6.1 RPC method.
   */
  readonly enabledSessionLaunch: boolean;
}
