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
  ProjectionDelta,
  ProjectionName,
  ProjectionPageByName,
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
export interface GatewayPort {
  get_projection<K extends ProjectionName>(
    name: K,
    scope?: ProjectionScope,
    page?: ProjectionPageParams,
  ): Promise<ProjectionPageByName[K]>;
  subscribe(params: SubscribeParams): AsyncIterable<ProjectionDelta>;
  get_capabilities(): Promise<Capabilities>;

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

  // Connection management (transport liveness; §11.4). These are UI-client
  // transport concerns, NOT part of the frozen §6.1 RPC method surface.
  getConnectionState(): ConnectionState;
  onConnectionChange(cb: (state: ConnectionState) => void): () => void;
  /** Attempt to (re)establish the transport — the Retry/Repair action. */
  reconnect(): void;
}
