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
// TerminalOutputFrame + DiffResult imported as VALUES (the Zod shadows) — used both
// as the parse validators below and as the method return/element types.
import {
  CONTRACT_VERSION,
  DiffResult,
  TerminalOutputFrame,
} from "../contracts/index";
import { terminalOutputFixture } from "./terminal-fixture";
import {
  canTransition,
  worstOfConnection,
  type ConnectionState,
} from "../connection/state";
import {
  approvalQueueFixture,
  approvalQueueDeltaFixture,
} from "../projections/fixtures/proj_approval_queue";
import { auditTrailFixture } from "../projections/fixtures/proj_audit_trail";
import {
  projectActivityDeltaFixture,
  projectActivityFixture,
} from "../projections/fixtures/proj_project_activity";
import {
  pullRequestDeltaFixture,
  pullRequestFixture,
} from "../projections/fixtures/proj_pull_request";
import {
  reviewDeltaFixture,
  reviewFixture,
} from "../projections/fixtures/proj_review";
import {
  sessionDeltaFixture,
  sessionPageFixture,
} from "../projections/fixtures/proj_session";
import { usageDeltaFixture, usageFixture } from "../projections/fixtures/proj_usage";
import { parseDelta, parseProjectionPage } from "./boundary";

const DEFAULT_PROTOCOL_VERSION = 1;

// A small CONTRACT-shaped get_diff fixture (a DiffResult). Deliberately decoupled
// from any DiffReview.tsx render fixture — this is the §6.1 read-surface shape the
// 6.3e Code/Diff slice consumes + maps to the render (Q1). Served THROUGH the frozen
// shadow below so the mock can never emit a diff the real boundary would reject.
const diffFixture: unknown = {
  hunks: [
    {
      header: "@@ -1,3 +1,4 @@",
      old_start: 1,
      old_lines: 3,
      new_start: 1,
      new_lines: 4,
      // content EXCLUDES the +/- origin char (mirrors the daemon's git2 line.content();
      // the kind carries context|added|removed, and the kit DiffHunk renders the sign).
      lines: [
        { kind: "context", content: "fn main() {\n" },
        { kind: "removed", content: "    println!(\"hi\");\n" },
        { kind: "added", content: "    println!(\"hello\");\n" },
        { kind: "added", content: "    log::info!(\"started\");\n" },
        { kind: "context", content: "}\n" },
      ],
    },
  ],
};

// Raw fixtures keyed by projection name; served THROUGH the boundary validator.
const FIXTURES: Record<ProjectionName, unknown> = {
  Session: sessionPageFixture,
  ProjectActivity: projectActivityFixture,
  PullRequest: pullRequestFixture,
  Review: reviewFixture,
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
  /** The L2 go-live gate the UI controls consult (056). Default TRUE — the Mock is a fully-working
   *  test/dev port (its mutations resolve), so Mock-driven UI-flow tests keep enabled controls; set
   *  false to exercise the L2-B honest-disabled state (wired-but-not-enabled). */
  mutationsEnabled?: boolean;
}

export class MockGatewayPort implements GatewayPort {
  private connection: ConnectionState;
  private readonly protocolVersion: number;
  private readonly mutationError?: WireError;
  private readonly listeners = new Set<(state: ConnectionState) => void>();
  /** The L2 go-live gate the UI controls consult (056) — default TRUE (the Mock is a working port). */
  readonly mutationsEnabled: boolean;

  constructor(options: MockGatewayOptions = {}) {
    this.connection = options.connection ?? "connected";
    this.protocolVersion = options.protocolVersion ?? DEFAULT_PROTOCOL_VERSION;
    this.mutationError = options.mutationError;
    this.mutationsEnabled = options.mutationsEnabled ?? true;
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
    // Symmetric with get_projection: an unrecognized projection fails fast rather than silently
    // streaming nothing (which would mask wiring bugs). Session yields a row-bearing delta; the live
    // streams (ui-059 ApprovalQueue + ui-063 ProjectActivity/PullRequest/UsageLedger) yield daemon-shaped
    // `row:None` NUDGES (consumed via refetch-on-nudge, never row-apply — LESSON §29).
    if (params.projection === "Session") {
      yield parseDelta(sessionDeltaFixture);
    } else if (params.projection === "ApprovalQueue") {
      yield parseDelta(approvalQueueDeltaFixture);
    } else if (params.projection === "ProjectActivity") {
      yield parseDelta(projectActivityDeltaFixture);
    } else if (params.projection === "PullRequest") {
      yield parseDelta(pullRequestDeltaFixture);
    } else if (params.projection === "UsageLedger") {
      yield parseDelta(usageDeltaFixture);
    } else if (params.projection === "Review") {
      yield parseDelta(reviewDeltaFixture);
    } else {
      throw new Error(
        `MockGatewayPort: no fixture for subscribe projection "${params.projection}"`,
      );
    }
    // Then STAY OPEN — a real subscribe stream blocks on the next push until the daemon
    // closes it on lag (it does NOT end after one delta). Blocking here keeps the live
    // subscribe supervisor (052) from spinning recovery in Mock-backed tests; consumers
    // that only sample the stream break after the first delta (mock.test.ts).
    await new Promise<void>(() => {
      /* never resolves: a live subscribe stream stays open until the daemon closes it */
    });
  }

  async *subscribe_terminal(
    _terminal_id: string,
  ): AsyncIterable<TerminalOutputFrame> {
    // Yield the canned fixture frames THROUGH the frozen shadow (parse-don't-trust,
    // symmetric with subscribe/get_projection — the mock can never emit a frame the
    // real boundary would reject). Output ONLY: the §17 PTY-death is a daemon
    // event→projection, not a terminal-channel frame, so it's never yielded here.
    for (const frame of terminalOutputFixture) {
      yield TerminalOutputFrame.parse(frame);
    }
  }

  async get_capabilities(): Promise<Capabilities> {
    return {
      protocol_version: this.protocolVersion,
      contract_version: CONTRACT_VERSION,
    };
  }

  // §6.1 get_diff — read-only structured diff fixture, served THROUGH the frozen
  // DiffResult shadow (parse-don't-trust, symmetric with get_projection). The real
  // UdsGatewayPort reads git2; this returns a contract-valid fixture so 6.3e can be
  // built/tested without a live daemon. `worktree_id`/`file` are accepted but unused
  // (the fixture is fixed) — the daemon resolves the id→path + reads the real diff.
  async get_diff(_worktree_id: string, _file: string): Promise<DiffResult> {
    return DiffResult.parse(diffFixture);
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

  /** Per-stream last reported lifecycle (ui-059) — mirrors the real port so multi-stream behavior is
   *  faithful against the mock. */
  private readonly streamStates = new Map<string, ConnectionState>();

  /** The subscribe supervisors' connection-state drive (054; ui-059 per-stream) — mirrors the real port:
   *  records the stream's state, drives a GUARDED transition (illegal/no-op hops skipped) to the worst-of
   *  aggregate + notifies, so the Shell's supervisor binding behaves the same against the mock. DISTINCT
   *  from `setConnectionState` (the raw, unguarded test-staging setter below). The mock has no read-upgrade
   *  path, so it needs no streamDegraded suppression of its own. */
  notifyConnectionState(streamId: string, next: ConnectionState): void {
    this.streamStates.set(streamId, next);
    const aggregate = worstOfConnection([...this.streamStates.values()]);
    if (aggregate === null || this.connection === aggregate) return;
    if (!canTransition(this.connection, aggregate)) return;
    this.connection = aggregate;
    for (const cb of this.listeners) cb(aggregate);
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
