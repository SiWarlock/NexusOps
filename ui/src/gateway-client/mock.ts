// MockGatewayPort — the §14-sanctioned UI test/dev seam.
//
// Implements the §6.1 READ surface against contract-valid fixture projections,
// so every Phase-6 screen can be built + tested against the frozen contracts
// without a live daemon. It returns fixtures THROUGH the same boundary validator
// the real client uses, so the mock can never emit a payload the real boundary
// would reject (dogfoods parse-don't-trust). It also simulates the transport
// connection state + a skewable handshake version, so the degraded-mode surfaces
// can be exercised. The real UdsGatewayPort (§6.4) is a later per-slice
// integration gated on daemon Phase 1.5.
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
  ProjectionDelta,
  ProjectionName,
  ProjectionPageByName,
  WireError,
} from "../contracts/index";
import { CONTRACT_VERSION } from "../contracts/index";
import type { ConnectionState } from "../connection/state";
import { approvalQueueFixture } from "../projections/fixtures/proj_approval_queue";
import { auditTrailFixture } from "../projections/fixtures/proj_audit_trail";
import { projectActivityFixture } from "../projections/fixtures/proj_project_activity";
import { pullRequestFixture } from "../projections/fixtures/proj_pull_request";
import {
  sessionDeltaFixture,
  sessionPageFixture,
} from "../projections/fixtures/proj_session";
import { usageFixture } from "../projections/fixtures/proj_usage";
import { parseDelta, parseProjectionPage } from "./boundary";

const DEFAULT_PROTOCOL_VERSION = 1;

// Raw fixtures keyed by projection name; served THROUGH the boundary validator.
const FIXTURES: Record<ProjectionName, unknown> = {
  Session: sessionPageFixture,
  ProjectActivity: projectActivityFixture,
  PullRequest: pullRequestFixture,
  ApprovalQueue: approvalQueueFixture,
  AuditTrail: auditTrailFixture,
  UsageLedger: usageFixture,
};

export interface MockGatewayOptions {
  /** Initial connection state (default: a working "connected" handshake). */
  connection?: ConnectionState;
  /** Handshake protocol_version — set out of range to simulate version-skew. */
  protocolVersion?: number;
  /** When set, the mutation methods REJECT with this `WireError` (the daemon's §6.4
   *  error path) instead of resolving — so the seam's verbatim-error pin is exercised. */
  mutationError?: WireError;
}

export class MockGatewayPort implements GatewayPort {
  private connection: ConnectionState;
  private readonly protocolVersion: number;
  private readonly mutationError?: WireError;
  private readonly listeners = new Set<(state: ConnectionState) => void>();

  constructor(options: MockGatewayOptions = {}) {
    this.connection = options.connection ?? "connected";
    this.protocolVersion = options.protocolVersion ?? DEFAULT_PROTOCOL_VERSION;
    this.mutationError = options.mutationError;
  }

  async get_projection<K extends ProjectionName>(
    name: K,
    _scope?: ProjectionScope,
    _page?: ProjectionPageParams,
  ): Promise<ProjectionPageByName[K]> {
    const fixture = FIXTURES[name];
    // Validate the fixture through the real boundary; the registry guarantees
    // the parsed page matches ProjectionPageByName[K] for this name.
    return parseProjectionPage(name, fixture) as ProjectionPageByName[K];
  }

  async *subscribe(params: SubscribeParams): AsyncIterable<ProjectionDelta> {
    // Symmetric with get_projection: an unrecognized projection fails fast
    // rather than silently streaming nothing (which would mask wiring bugs).
    if (params.projection !== "Session") {
      throw new Error(
        `MockGatewayPort: no fixture for subscribe projection "${params.projection}"`,
      );
    }
    yield parseDelta(sessionDeltaFixture);
  }

  async get_capabilities(): Promise<Capabilities> {
    return {
      protocol_version: this.protocolVersion,
      contract_version: CONTRACT_VERSION,
    };
  }

  // §6.1 mutation-intent surface — deterministic fixtures. The daemon mints the
  // `action_request_id` + reports a lifecycle `status`. submit_action returns the
  // NON-terminal `submitted` (NEVER a synthesized `succeeded`) so the seam's
  // no-optimism pin has a real daemon-reported value to surface; approve/deny return
  // the daemon-reported decision status (`approved`/`denied`). `mutationError` rejects
  // with the daemon's `WireError` (the §6.4 error path).
  async submit_action(_request: ActionRequest): Promise<ActionAck> {
    if (this.mutationError) throw this.mutationError;
    return { action_request_id: "ar_mock_0001", status: "submitted" };
  }

  async preview_action(_action_request_id: string): Promise<ActionPreview> {
    if (this.mutationError) throw this.mutationError;
    return {
      action_request_id: "ar_mock_0001",
      generated_at: "2026-06-13T00:00:00Z",
      risk_level: 2,
      risk_reasons: ["touches tracked files"],
      summary: "Would modify 1 file in the worktree.",
      changed_resources: [{ type: "file", id: "src/main.rs" }],
      cannot_preview_reason: null,
    };
  }

  async approve(_approval_id: string, _step_id?: string): Promise<ActionAck> {
    if (this.mutationError) throw this.mutationError;
    return { action_request_id: "ar_mock_0001", status: "approved" };
  }

  async deny(_approval_id: string, _reason: string): Promise<ActionAck> {
    if (this.mutationError) throw this.mutationError;
    return { action_request_id: "ar_mock_0001", status: "denied" };
  }

  getConnectionState(): ConnectionState {
    return this.connection;
  }

  onConnectionChange(cb: (state: ConnectionState) => void): () => void {
    this.listeners.add(cb);
    return () => {
      this.listeners.delete(cb);
    };
  }

  reconnect(): void {
    // Simulated transport recovery: reconnecting → connected (both legal hops).
    this.setConnectionState("reconnecting");
    this.setConnectionState("connected");
  }

  // Test/dev helper: drive a connection transition and notify subscribers. This
  // is intentionally a RAW setter (no canTransition guard) so tests can stage
  // any state directly; the legal-transition spec is owned + enforced by
  // connection/state.ts (the real UdsGatewayPort drives these from the socket).
  setConnectionState(state: ConnectionState): void {
    this.connection = state;
    for (const cb of this.listeners) cb(state);
  }
}
