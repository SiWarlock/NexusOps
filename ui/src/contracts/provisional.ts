// PROVISIONAL — not frozen; reconciles when the daemon freezes object schemas
// (Phase 1/2 contract bump). The 0.5 freeze is ENUM-ONLY: projection-row shapes,
// GatewayPort params/results, and the ActionRequest/ActionPlan OBJECTS are
// Appendix-A prose, not yet a generated artifact. Until they freeze we type them
// here as minimal UI-local shapes — with EVERY enum-typed field delegated to the
// GENERATED zod enums (never a re-declared status string union). See ARCHITECTURE
// §5.0 / §6.1 / §7.2 and the brief's Step-2.5 Q2.
import { z } from "zod";
import bundle from "./generated";
// The §6.2 risk axis + the PolicyDecision shadow live in intent-contracts (which depends only on
// zod+generated — no cycle). The frozen ApprovalQueueRow carries both, so the row shadow reuses them.
import { PolicyDecision, RiskLevel } from "./intent-contracts";

// Delegate to the generated enums (no hand-declared status unions). The two R-5
// status enums were renamed to their `$def` names at the 0.19.0 Gateway freeze
// (`Approval`→`ApprovalStatus`, `ActionRequest`→`ActionRequestStatus`).
const Session = bundle.shape.Session;
const PullRequest = bundle.shape.PullRequest;
const ReviewState = bundle.shape.ReviewState;
const ApprovalStatus = bundle.shape.ApprovalStatus;
const ActorType = bundle.shape.ActorType;
const ActionRequestStatus = bundle.shape.ActionRequestStatus;
const RequesterType = bundle.shape.RequesterType;

// ─── Survival/recovery (GENERATED since ui-065) ──────────────────────────────
// The daemon froze the O-2 survival schema (`ResumeMode`/`RecoveryState`) as `oneOf`-of-`const` defs
// (doc-commented). Since ui-065 the generator emits oneOf-const, so these are REAL generated enums —
// exported from `contracts/index` (the §5.0 drift gate pins them); the prior hand-declared drift-pinned
// SHADOWS are retired. Here we take LOCAL handles from the generated bundle (the Session/PullRequest/
// ReviewState precedent above) for the internal `RecoveryStatus.state` / `SessionRow.resume_mode` uses.
// `RecoveryStatus` is a ui-local wrapper the daemon did NOT freeze → stays provisional, re-based over
// the generated `RecoveryState`.
const RecoveryState = bundle.shape.RecoveryState;
const ResumeMode = bundle.shape.ResumeMode;

/** The recovery status input (ui-local wrapper — the daemon froze RecoveryState but not this
 *  wrapper; re-based over the drift-pinned RecoveryState). The SessionRecovered-event→aggregate
 *  consumption (§11.4 recovery-UX) is a deferred follow-on. */
export const RecoveryStatus = z.object({
  state: RecoveryState,
  // sessions affected by a failed recovery (subject ids); empty/absent otherwise.
  affectedSessions: z.array(z.string()).optional(),
});
export type RecoveryStatus = z.infer<typeof RecoveryStatus>;

// ─── §17 safety-state (PROVISIONAL — 6.4d-2) ─────────────────────────────────
// The daemon's §17 failure-mode / conflict schema is NOT in the frozen contract.
// ConflictReason + FencingConflict are hand-declared provisional shapes (Lesson
// §2); they reconcile at the daemon §17/survival-schema freeze (Carry-forward
// provisional→generated spread). The audit-integrity treatments (L2) REUSE the
// frozen ActionRequest enum for partially_succeeded/rollback_failed — never
// re-declared here.

/**
 * Why a mutation was rejected at the fencing gate (§17 fencing row, safety #6).
 * Scoped to `fencing_conflict` ONLY — the never-auto-resolved hard conflict.
 * `stale_precondition` is a SEPARATE, RE-APPROVABLE flow (regenerate preview +
 * require fresh approval, §17/§6.2) — NOT a hard conflict; it belongs with the
 * approval/preview surface, not this card, so it is deliberately excluded here.
 * PROVISIONAL.
 */
export const ConflictReason = z.enum(["fencing_conflict"]);
export type ConflictReason = z.infer<typeof ConflictReason>;

/**
 * A stale-token / lease-expiry rejected mutation → `ActionFailed(fencing_conflict)`
 * + conflict event (§17). The UI DISPLAYS it as the never-auto-resolved hard-
 * conflict card (safety #6); resolution is a daemon-side typed Action (parked,
 * daemon-1.5 — single mutator / INV-SEC-1 is daemon-side). PROVISIONAL.
 */
export const FencingConflict = z.object({
  // The affected action (frozen IdKind `action_request_id`; the value is the `act_…` id string).
  action_request_id: z.string(),
  // The affected session when session-scoped (frozen IdKind `session_id`).
  session_id: z.string().optional(),
  reason: ConflictReason,
  // Human-readable summary of what was rejected (daemon-supplied; never invented — forbidden #2).
  summary: z.string(),
});
export type FencingConflict = z.infer<typeof FencingConflict>;

// ─── §17 audit-integrity (PROVISIONAL — 6.4d-2 L2) ───────────────────────────
// Fail-closed / audit-integrity treatments (§11.4, §15/§17, safety #5). The two
// action-outcome treatments REUSE the frozen ActionRequest enum (drift-pinned via
// `.extract` — it THROWS at load if the frozen enum renames them; never a re-
// declaration, Lesson §2). Only the net-new integrity signals are provisional.

/** The two frozen ActionRequestStatus outcomes that raise an audit-integrity alert. */
export const AuditOutcomeStatus = ActionRequestStatus.extract([
  "partially_succeeded",
  "rollback_failed",
]);
export type AuditOutcomeStatus = z.infer<typeof AuditOutcomeStatus>;

/** Net-new audit-integrity signals NOT in the frozen contract (§17). PROVISIONAL. */
export const AuditIntegrityKind = z.enum([
  "unknown_outcome",
  "audit_write_failed",
  "corrupt_payload",
]);
export type AuditIntegrityKind = z.infer<typeof AuditIntegrityKind>;

/**
 * A fail-closed / audit-integrity signal (§15/§17, safety #5) — a discriminated
 * union: `action_status` reuses the frozen ActionRequest outcome enum, `integrity`
 * carries the provisional net-new signals (daemon-crash unknown-outcome, event-
 * write-fail, corrupt-payload quarantine). PROVISIONAL.
 */
export const AuditIntegrityState = z.discriminatedUnion("source", [
  z.object({ source: z.literal("action_status"), status: AuditOutcomeStatus }),
  z.object({ source: z.literal("integrity"), kind: AuditIntegrityKind }),
]);
export type AuditIntegrityState = z.infer<typeof AuditIntegrityState>;

/**
 * The aggregate §17 safety state the Shell renders (fixture-driven; the daemon
 * §17 failure-mode logic isn't built). PROVISIONAL — mirrors the `RecoveryStatus`
 * input-prop shape (a Zod schema, validated at the boundary when real state lands).
 */
export const SafetyState = z.object({
  // A pending fencing/hard-conflict, or null when clean (non-intrusive default).
  conflict: FencingConflict.nullable(),
  // A pending fail-closed/audit-integrity signal, or null when clean. REQUIRED
  // (not optional) so a #5 caller can't silently omit a must-be-seen alert.
  integrity: AuditIntegrityState.nullable(),
});
export type SafetyState = z.infer<typeof SafetyState>;

/** A row of the Session projection — the 3rd frozen projection-row (@0.35, D2,
 *  `shared/src/projections.rs`). Reconciled 5→10 (ui-062): a drift-pinned frozen-shadow snapshot-pinned
 *  to `$defs.SessionRow` (the ApprovalQueueRow precedent §37). `session_id`/`project_id`/`status` are the
 *  non-Option required core (the daemon's NOT-NULL columns — `project_id` was wrongly optional); the rest
 *  present-and-nullable. `display_name` is the daemon-canonical name (was the ui-provisional `title`). The
 *  §8.1/§11.4 survival fields (`resume_mode`/`replayed_event_count`/`recovered_at`) are now consumed (the
 *  per-session resume-mode indicator). `.strict()` per the frozen `deny_unknown_fields`. */
export const SessionRow = z
  .object({
    session_id: z.string(),
    project_id: z.string(),
    status: Session,
    display_name: z.string().nullable().optional(),
    harness: z.string().nullable().optional(),
    model: z.string().nullable().optional(),
    execution_profile_id: z.string().nullable().optional(),
    // O-2 survival display: how the session came back after a daemon restart.
    resume_mode: ResumeMode.nullable().optional(),
    replayed_event_count: z.number().int().nonnegative().nullable().optional(),
    recovered_at: z.string().nullable().optional(),
  })
  .strict();
export type SessionRow = z.infer<typeof SessionRow>;

/** A page of the Session projection returned by get_projection (provisional). */
export const SessionProjectionPage = z.object({
  projection: z.literal("Session"),
  rows: z.array(SessionRow),
  cursor: z.string().nullable().optional(),
});
export type SessionProjectionPage = z.infer<typeof SessionProjectionPage>;

/** A single row of the ProjectActivity projection — a project (provisional). */
export const ProjectActivityRow = z.object({
  project_id: z.string(),
  name: z.string(),
});
export type ProjectActivityRow = z.infer<typeof ProjectActivityRow>;

export const ProjectActivityPage = z.object({
  projection: z.literal("ProjectActivity"),
  rows: z.array(ProjectActivityRow),
  cursor: z.string().nullable().optional(),
});
export type ProjectActivityPage = z.infer<typeof ProjectActivityPage>;

/** A row of the PullRequest projection — the 2nd frozen projection-row (@0.34/0.36,
 *  `shared/src/projections.rs`). A drift-pinned frozen-shadow reconciled 4→11 (P7.2/D5a): the field
 *  set + types are snapshot-pinned to the schema `$defs.PullRequestRow` (provisional.test); `status`
 *  delegates to the generated `PullRequest` enum. `.strict()` per the frozen `deny_unknown_fields`;
 *  `pr_id` (PK) + `status` are non-Option (the daemon serves a NOT-NULL PK), the rest present-and-
 *  nullable (serialized as explicit `null`, tolerated-absent on read). `pr_number` is the GitHub-native
 *  PR number — a u64 shadow (NOT a string — the work-order str→number drift), `mergeable` a bool. */
export const PullRequestRow = z
  .object({
    pr_id: z.string(),
    project_id: z.string().nullable().optional(),
    repo_id: z.string().nullable().optional(),
    pr_number: z.number().int().nonnegative().nullable().optional(),
    title: z.string().nullable().optional(),
    status: PullRequest,
    head_branch: z.string().nullable().optional(),
    base_branch: z.string().nullable().optional(),
    pr_checked_at: z.string().nullable().optional(),
    mergeable: z.boolean().nullable().optional(),
    checks_summary: z.string().nullable().optional(),
  })
  .strict();
export type PullRequestRow = z.infer<typeof PullRequestRow>;

export const PullRequestProjectionPage = z.object({
  projection: z.literal("PullRequest"),
  rows: z.array(PullRequestRow),
  cursor: z.string().nullable().optional(),
});
export type PullRequestProjectionPage = z.infer<typeof PullRequestProjectionPage>;

/** A row of the Review projection — the 4th frozen projection-row (@0.37, D5b-1,
 *  `shared/src/projections.rs`). A single structured GitHub PR review the L2 PR Review Workspace
 *  (§11.2) renders, a separate `get_projection("Review")` page joined to a PR client-side on
 *  `pr_number`. Drift-pinned frozen-shadow (8 fields snapshot-pinned to `$defs.ReviewRow`); `state`
 *  delegates to the generated `ReviewState` VALUE enum (a fixed verdict, reject-unknown). `.strict()`;
 *  `review_id` (PK) + `state` non-Option, the rest present-and-nullable. `review_id`/`pr_number` are
 *  u64 shadows; `body` is the §15-redacted review text the row serves (the redacted value). */
export const ReviewRow = z
  .object({
    review_id: z.number().int().nonnegative(),
    pr_number: z.number().int().nonnegative().nullable().optional(),
    project_id: z.string().nullable().optional(),
    repo_id: z.string().nullable().optional(),
    reviewer: z.string().nullable().optional(),
    state: ReviewState,
    submitted_at: z.string().nullable().optional(),
    body: z.string().nullable().optional(),
  })
  .strict();
export type ReviewRow = z.infer<typeof ReviewRow>;

export const ReviewProjectionPage = z.object({
  projection: z.literal("Review"),
  rows: z.array(ReviewRow),
  cursor: z.string().nullable().optional(),
});
export type ReviewProjectionPage = z.infer<typeof ReviewProjectionPage>;

/** A row of the ApprovalQueue projection — the FIRST frozen projection-row (@0.30,
 *  `shared/src/projections.rs`). A drift-pinned frozen-shadow: the 14-field set + types are
 *  snapshot-pinned to the schema `$defs.ApprovalQueueRow` (provisional.test); enum fields delegate
 *  to the generated value-sets (`risk_level`→`RiskLevel` int 0–4, `status`→`ApprovalStatus`,
 *  `requester_type`→`RequesterType`); `policy_decision` delegates to the `PolicyDecision` shadow.
 *  `.strict()` per the frozen `deny_unknown_fields`; the `Option<>` fields are present-and-nullable
 *  (the daemon serializes them as explicit `null` — no `skip_serializing_if`), tolerated-absent on read.
 *  The card sources real `risk_level`/`policy_decision` from this row (053 Layer C — no fixture risk). */
export const ApprovalQueueRow = z
  .object({
    approval_id: z.string(),
    action_request_id: z.string().nullable().optional(),
    plan_id: z.string().nullable().optional(),
    project_id: z.string().nullable().optional(),
    session_id: z.string().nullable().optional(),
    agent_team_id: z.string().nullable().optional(),
    risk_level: RiskLevel,
    status: ApprovalStatus,
    requester_type: RequesterType,
    requester_id: z.string(),
    preview_summary: z.string().nullable().optional(),
    requested_at: z.string(),
    expires_at: z.string().nullable().optional(),
    policy_decision: PolicyDecision.nullable().optional(),
  })
  .strict();
export type ApprovalQueueRow = z.infer<typeof ApprovalQueueRow>;

export const ApprovalQueuePage = z.object({
  projection: z.literal("ApprovalQueue"),
  rows: z.array(ApprovalQueueRow),
  cursor: z.string().nullable().optional(),
});
export type ApprovalQueuePage = z.infer<typeof ApprovalQueuePage>;

/** A row of the AuditTrail projection — one event (provisional; actor delegates to the frozen enum). */
export const AuditEventRow = z.object({
  event_id: z.string(),
  seq: z.number(),
  project_id: z.string().optional(),
  actor_type: ActorType,
  event_type: z.string(),
  summary: z.string().optional(),
});
export type AuditEventRow = z.infer<typeof AuditEventRow>;

export const AuditTrailPage = z.object({
  projection: z.literal("AuditTrail"),
  rows: z.array(AuditEventRow),
  cursor: z.string().nullable().optional(),
});
export type AuditTrailPage = z.infer<typeof AuditTrailPage>;

// ─── Usage (PROVISIONAL — 6.4b) ──────────────────────────────────────────────
// The daemon's usage/telemetry schema is NOT in the frozen contract. UsageRow +
// metric_quality + Harness are hand-declared provisional shapes (Lesson §2); they
// (incl. the credit-pool thresholds + enum delegation) reconcile at the daemon
// usage-schema freeze — tracked in the MVP_TASKS Carry-forward provisional→generated
// spread. `MetricQuality` IS frozen at 0.23.0 as a `oneOf`-of-`const` (its variants carry
// doc-comments); since ui-065 the generator emits oneOf-const, so it is a REAL generated enum
// (exported from `contracts/index`, drift-gate-pinned) — a LOCAL handle here for `UsageRow.metric_quality`.
// `Harness` has no frozen enum yet, so it stays local.
const MetricQuality = bundle.shape.MetricQuality;

/** The agent harness a session runs under. PROVISIONAL (no frozen Harness enum). */
export const Harness = z.enum(["claude", "codex"]);
export type Harness = z.infer<typeof Harness>;

/** Per-subject token/cost usage (§11.4). PROVISIONAL. */
export const UsageRow = z.object({
  subject_id: z.string(),
  harness: Harness,
  tokens: z.number(),
  cost: z.number(),
  metric_quality: MetricQuality,
  // Codex reports no context metadata (§9.1 supportsContextMetadata=false) → null.
  context_pct: z.number().nullable().optional(),
});
export type UsageRow = z.infer<typeof UsageRow>;

/**
 * A harness billing pool (§11.4/§9.1), distinct from token spend. The `kind`
 * discriminator encodes the confirmed two-pool asymmetry: `"sdk"` is the capped
 * monthly SDK/`-p` pool that HARD-STOPS with no fallback; `"interactive"` is the
 * auto-resetting rolling-window pool that NEVER hard-stops. REQUIRED — every pool
 * declares its kind (no silent default; a future interactive pool can't inherit
 * `"sdk"` and false-alarm a hard-stop, §11 never-a-silent-false-safety-state).
 * PROVISIONAL — the daemon has not frozen a credit-pool schema.
 */
export const CreditPool = z.object({
  kind: z.enum(["sdk", "interactive"]),
  used: z.number(),
  limit: z.number(),
});
export type CreditPool = z.infer<typeof CreditPool>;

export const UsageProjectionPage = z.object({
  // 0.12.0: the frozen ProjectionName enum fixes the canonical name as
  // "UsageLedger" (was provisional "Usage").
  projection: z.literal("UsageLedger"),
  rows: z.array(UsageRow),
  creditPool: CreditPool.nullable().optional(),
  cursor: z.string().nullable().optional(),
});
export type UsageProjectionPage = z.infer<typeof UsageProjectionPage>;

/**
 * Typed projection registry (provisional). Maps each projection name to its page
 * shape so get_projection / parseProjectionPage give precise per-name types.
 */
export type ProjectionPageByName = {
  Session: SessionProjectionPage;
  ProjectActivity: ProjectActivityPage;
  PullRequest: PullRequestProjectionPage;
  Review: ReviewProjectionPage;
  ApprovalQueue: ApprovalQueuePage;
  AuditTrail: AuditTrailPage;
  UsageLedger: UsageProjectionPage;
};
export type ProjectionName = keyof ProjectionPageByName;

/** Any projection page (the union over the registry). */
export type ProjectionPage = ProjectionPageByName[ProjectionName];

/** A streamed projection delta from subscribe (provisional; kind delegates to
 *  the frozen 0.12.0 DeltaKind enum — never re-declared, Lesson §1/§2). */
export const ProjectionDelta = z.object({
  projection: z.string(),
  kind: bundle.shape.DeltaKind,
  row: SessionRow.optional(),
  id: z.string().optional(),
});
export type ProjectionDelta = z.infer<typeof ProjectionDelta>;

// ─── §6.4 frame-mux (PROVISIONAL — contract-ahead) ───────────────────────────
// The server→client multiplexed frame surface. Hand-modeled here (the GatewayPort
// frame types are Appendix-A prose, not a generated artifact); the enum-typed
// fields delegate to the generated enums (never re-declared — Lesson §1/§2). Both
// shapes are pinned to the frozen schema by the provisional.test §2.5-seam snapshot.

/** A structured daemon→client error frame (§6.4) — carries one closed
 *  [`IpcErrorCode`]. Frozen §6.4 $def (`additionalProperties:false`), hand-modeled;
 *  field-set == {code}. `.strict()` MATCHES the frozen closed shape and is
 *  security-load-bearing for the intent seam: a thrown runtime/transport `Error`
 *  that happens to carry a colliding `code` (plus `message`/`stack`) is REJECTED, so
 *  the seam re-throws it as a real bug instead of misclassifying it as a daemon error. */
export const WireError = z
  .object({
    code: bundle.shape.IpcErrorCode,
  })
  .strict();
export type WireError = z.infer<typeof WireError>;

/**
 * The §6.4 Terminal-Channel OUTPUT frame (P3.4) — the daemon→client raw PTY output
 * push, the `terminal_output` `ServerFrame` variant EXTRACTED so the 6.3d Session
 * Terminal well + its `subscribe_terminal` consumer take it directly (one source;
 * still a `ServerFrame` member below). DISPLAY-ONLY #9: `data` is opaque base64 the
 * well DECODES for display, NEVER scraped for state. All 4 fields REQUIRED;
 * field-set drift-pinned to the frozen `ServerFrame.terminal_output`
 * (provisional.test.ts). Output ONLY — the §17 PTY-death is a daemon
 * event→projection (`TerminalProcessExited`), not pushed over this channel.
 */
export const TerminalOutputFrame = z.object({
  frame_type: z.literal("terminal_output"),
  terminal_id: z.string(), // opaque daemon-minted handle — NOT a frozen-22 ID; re-minted on resume.
  seq: z.number().int().nonnegative(), // uint64 PTY chunk sequence (frozen: integer ≥0).
  data: z.string(), // base64-encoded raw PTY bytes — opaque; the 6.3d well decodes, not the contract layer.
});
export type TerminalOutputFrame = z.infer<typeof TerminalOutputFrame>;

/**
 * The server→client multiplexed frame envelope (§6.4 `frame_type` tag). The
 * internally-tagged discriminant demuxes one connection: an RPC response
 * (`rpc_response`, correlated by `id`, one of result/error) vs a subscription
 * push (`subscription_push`, the changed projection/kind/row). PROVISIONAL +
 * contract-ahead — adopted for the intent-seam/transport slice that actually
 * demuxes; MVP subscribe stays a dedicated single-connection `ProjectionDelta`
 * stream (no demux yet). The Terminal-Channel `terminal_output` variant
 * (`TerminalOutputFrame` above) was defined at 0.23.0 (P3.4 §6.4) — the 6.3d
 * Session Terminal well consumes it. Variant field-sets pinned to `ServerFrame.oneOf`.
 */
export const ServerFrame = z.discriminatedUnion("frame_type", [
  z.object({
    frame_type: z.literal("rpc_response"),
    // uint64 correlation id (frozen schema: integer, minimum 0, REQUIRED) — a
    // numeric id, NOT a string; the JSON-RPC client matches the daemon's id type.
    id: z.number().int().nonnegative(),
    result: z.unknown().optional(),
    error: WireError.nullable().optional(),
  }),
  z.object({
    frame_type: z.literal("subscription_push"),
    projection: bundle.shape.ProjectionName,
    kind: bundle.shape.DeltaKind,
    id: z.string().nullable().optional(),
    row: SessionRow.optional(),
  }),
  TerminalOutputFrame,
]);
export type ServerFrame = z.infer<typeof ServerFrame>;

// ─── §6.1 diff read surface (PROVISIONAL — the 6.3e get_diff result) ──────────
// The `get_diff` RPC's structured result (HEAD→workdir), adopted AHEAD of its
// consumer (the 6.3e Code/Diff slice maps DiffResult → the render + targets the
// per-hunk git.* actions by old_start/new_start hunk-identity). Hand-modeled
// provisional shadows of the frozen §6.1 $defs (objects aren't generated — Lesson
// §2); the enum field (DiffLine.kind) delegates to the GENERATED DiffLineKind
// (never re-declared, Lesson §1/§2). All four are `.strict()` (the frozen $defs are
// `additionalProperties:false`) + field-set/-type drift-pinned (provisional.test).

/** One line within a [`Hunk`] (§6.1): kind + verbatim content. PROVISIONAL. */
export const DiffLine = z
  .object({
    kind: bundle.shape.DiffLineKind,
    content: z.string(),
  })
  .strict();
export type DiffLine = z.infer<typeof DiffLine>;

/** One unified-diff hunk (§6.1). The old_start/new_start POSITION fields are the
 *  hunk-identity the per-hunk git.* actions target; offsets are frozen uint32
 *  (integer, minimum 0). PROVISIONAL. */
export const Hunk = z
  .object({
    header: z.string(),
    old_start: z.number().int().nonnegative(),
    old_lines: z.number().int().nonnegative(),
    new_start: z.number().int().nonnegative(),
    new_lines: z.number().int().nonnegative(),
    lines: z.array(DiffLine),
  })
  .strict();
export type Hunk = z.infer<typeof Hunk>;

/** `get_diff` result (§6.1) — the file's structured diff (HEAD→workdir). PROVISIONAL. */
export const DiffResult = z
  .object({
    hunks: z.array(Hunk),
  })
  .strict();
export type DiffResult = z.infer<typeof DiffResult>;

/** `get_diff` params (§6.1) — keyed off the stable `wt_` worktree id (NOT a path;
 *  the git.* actions ALSO target the id → read-by-id ↔ mutate-by-id consistency,
 *  §17). The daemon resolves worktree_id → proj_worktree.path. PROVISIONAL. */
export const GetDiffParams = z
  .object({
    worktree_id: z.string(),
    file: z.string(),
  })
  .strict();
export type GetDiffParams = z.infer<typeof GetDiffParams>;

/** get_capabilities result (provisional; §6.4 handshake surface). */
export const Capabilities = z.object({
  protocol_version: z.number(),
  contract_version: z.string(),
});
export type Capabilities = z.infer<typeof Capabilities>;
