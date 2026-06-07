// The GatewayPort interface — the §6.1 method surface, mirrored on the UI side.
//
// 6.1a implements the READ surface only (get_projection / subscribe /
// get_capabilities). The mutation-intent methods (submit_action, approve, …) and
// the real UDS transport (§6.4 framing/handshake, UdsGatewayPort) are later
// per-slice integrations, gated on daemon Phase 1.5 — OUT of scope here.
// All daemon access in the UI flows through a single implementation of this
// interface (ui/CLAUDE.md forbidden #3).
import type {
  Capabilities,
  ProjectionDelta,
  ProjectionPage,
} from "../contracts/index";

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
  get_projection(
    name: string,
    scope?: ProjectionScope,
    page?: ProjectionPageParams,
  ): Promise<ProjectionPage>;
  subscribe(params: SubscribeParams): AsyncIterable<ProjectionDelta>;
  get_capabilities(): Promise<Capabilities>;
}
