// PROVISIONAL DISPLAY FIXTURES — the cockpit's per-entity display side-maps.
//
// The frozen/provisional projection rows are THIN (SessionRow carries no harness/
// profile/branch/worktree/current-activity; ProjectActivityRow no repo slug or
// workflow state), but the §11 cockpit anatomy renders all of them. Until the
// daemon enriches those projections (Carry-forward: projection-enrichment spread),
// this module supplies the display data as id-keyed SIDE MAPS (Lesson §8 — never a
// widened projection row), fixture-driven exactly like recoveryStatusFixture /
// safetyCleanFixture. The render path is real; only this data source is a fixture.
//
// FLAG (not faked): every field here needs a daemon projection field to go live.
// When the daemon lands them, these maps are built from the projection rows and
// this fixture is deleted — no view-layer change.
import type {
  ActionAck,
  ActionPlan,
  Approval,
  ApprovalQueueRow,
  PolicyDecision,
  SessionRow,
  UsageRow,
} from "../contracts/index";
import type { GatewayPort } from "../gateway-client/types";

/** Kit HarnessBadge kinds (NexusOps-ui-kit components/badges/HarnessBadge). */
export type HarnessKind = "claude-code" | "codex-cli" | "codex-cloud" | "shell";

/** Workflow-instance tone for the sidebar/switcher project dot (§11.3). */
export type WorkflowTone = "active" | "drift" | "needs-personalization" | "none";

export interface SessionDisplayMeta {
  harness?: HarnessKind;
  provider?: "claude" | "codex";
  profile?: string;
  task?: { id: string; tone?: "linear" | "github" | "accent" };
  branch?: string;
  worktree?: string;
  pr?: string;
  /** Current command / activity line (mono). */
  current?: string;
  /** Last-activity text (the projection has no timestamps yet). */
  activity?: string;
  /** Session is an agent-team lead (AgentTeam projection not wired yet). */
  team?: boolean;
  /**
   * The §6.4 terminal runtime handle for a session that has a LIVE terminal stream
   * (6.3d). FIXTURE STAND-IN — the real handle is the opaque daemon-minted
   * `terminal_id` (re-minted on resume); the daemon will surface it on the session
   * projection at P4, and this side-map entry is built from it then (Lesson §8).
   * Absent → the well shows the honest placeholder (no live terminal).
   */
  terminalId?: string;
}

export interface ProjectDisplayMeta {
  /** Repo slug, e.g. "org/auth-service". */
  repo?: string;
  workflow?: WorkflowTone;
}

/** Display meta for the §14 fixture sessions (keyed by session_id). */
export const sessionDisplayFixture: Record<string, SessionDisplayMeta> = {
  session_fixture_1: {
    harness: "claude-code",
    provider: "claude",
    profile: "Claude Max Main",
    task: { id: "ENG-310", tone: "linear" },
    branch: "agent/auth-refactor",
    worktree: "~/wt/auth-refactor",
    current: "editing src/auth/session.ts",
    activity: "40s ago",
    // Team lead (prototype: a team session opens the Agent Team view) — display
    // fixture until the AgentTeam projection lands.
    team: true,
  },
  session_fixture_2: {
    harness: "claude-code",
    provider: "claude",
    profile: "Claude Max Main",
    task: { id: "ENG-284", tone: "linear" },
    branch: "agent/rate-limit",
    worktree: "~/wt/rate-limit",
    current: "$ npm test — awaiting permission",
    activity: "2m ago",
    // Live terminal + a pending permission card (the real cockpit's "streaming
    // output behind an approval" case): the well renders the xterm stream AND the
    // permission prompt. Sidebar-reachable (the openable demo of TerminalDisplay).
    terminalId: "term_fixture_2", // fixture stand-in for the P4 daemon handle
  },
  session_fixture_3: {
    harness: "codex-cli",
    provider: "codex",
    profile: "Codex CLI Main",
    task: { id: "#214", tone: "github" },
    branch: "fix/flaky-integration",
    worktree: "~/wt/flaky-it",
    current: "diff ready for review",
    activity: "6m ago",
    terminalId: "term_fixture_3", // live terminal (fixture stand-in for the P4 daemon handle)
  },
  session_fixture_4: {
    harness: "claude-code",
    provider: "claude",
    profile: "Claude Team Work",
    branch: "chore/deps",
    pr: "#102",
    current: "summarized to Project Brain",
    activity: "14m ago",
  },
  session_fixture_5: {
    harness: "claude-code",
    provider: "claude",
    profile: "Claude Team Work",
    task: { id: "ENG-301", tone: "linear" },
    branch: "agent/migration-plan",
    worktree: "~/wt/migration",
    current: "needs a decision on plan step 2",
    activity: "1m ago",
    terminalId: "term_fixture_5", // live terminal (fixture stand-in for the P4 daemon handle)
  },
};

/** Display meta for the §14 fixture projects (keyed by project_id). */
export const projectDisplayFixture: Record<string, ProjectDisplayMeta> = {
  project_fixture_1: { repo: "org/auth-service", workflow: "active" },
  project_fixture_2: { repo: "org/billing", workflow: "drift" },
  project_fixture_3: { repo: "org/docs-site", workflow: "none" },
};

/** Kit RiskBadge levels (Action Gateway risk taxonomy, §5.1). */
export type RiskLevel = "readonly" | "low" | "medium" | "high" | "critical";

export interface ApprovalDisplayMeta {
  /** Gateway risk classification — NOT in the ApprovalQueue projection yet. */
  risk?: RiskLevel;
  /** Requesting actor line (e.g. "Claude · ENG-310 · Claude Max Main"). */
  who?: string;
}

/** Display meta for the §14 fixture approvals (keyed by approval_id). */
export const approvalDisplayFixture: Record<string, ApprovalDisplayMeta> = {
  approval_fixture_1: { risk: "medium", who: "Claude · ENG-310 · Claude Max Main" },
  approval_fixture_2: { risk: "low", who: "Claude · docs · Claude Team Work" },
};

// ─── GatewayModal {Approval, PolicyDecision} SAMPLE (test fixture, post-053) ──
// As of 053 Layer C the PRODUCTION path no longer uses this side-map: `enrichApproval`
// reads the daemon's real `risk_level`/`policy_decision` from the now-frozen 14-field
// ApprovalQueueRow (the ②-mini-enriched proj_approval_queue). This map remains only as a
// daemon-SHAPED SAMPLE for GatewayModal.test.tsx (a valid {Approval, PolicyDecision} to
// render). A later cleanup may relocate it to a test fixture / build it via enrichApproval.
export interface GatewayApprovalEnrichment {
  approval: Approval;
  policyDecision: PolicyDecision;
}

export const gatewayApprovalEnrichment: Record<string, GatewayApprovalEnrichment> = {
  approval_fixture_1: {
    approval: {
      approval_id: "approval_fixture_1",
      required_approver: { kind: "current_user" },
      status: "awaiting_approval",
      scope: "single_action",
      risk_level: 3,
      action_request_id: "ar_fixture_1",
    },
    policyDecision: {
      status: "require_approval",
      reasons: ["Writes to a tracked file outside the session worktree."],
      required_approvals: [{ kind: "current_user" }],
      constraints: [],
      safer_alt: null,
    },
  },
  approval_fixture_2: {
    approval: {
      approval_id: "approval_fixture_2",
      required_approver: { kind: "project_owner" },
      status: "awaiting_approval",
      scope: "single_action",
      risk_level: 1,
      action_request_id: "ar_fixture_2",
    },
    policyDecision: {
      status: "require_approval",
      reasons: ["Edits documentation."],
      required_approvals: [{ kind: "project_owner" }],
      constraints: [],
      safer_alt: null,
    },
  },
};

/** Build the {Approval, PolicyDecision} the GatewayModal renders from the FROZEN ApprovalQueueRow
 *  (053 Layer C — the real-row swap). The card sources the daemon's AUTHORITATIVE `risk_level` +
 *  `policy_decision` directly from the row (no fixture side-map, no UI-derived risk — LESSON 17;
 *  resolves the 044 [med]: no real human approves against fixture risk). The row carries
 *  `risk_level` (always) + `policy_decision` (the ②-mini-enriched proj_approval_queue); an absent
 *  policy is a transparent "awaiting" placeholder — never a fabricated decision (the risk stays the
 *  row's real value, forbidden #2/#4). The projection row carries no `scope` → default `single_action`
 *  (not risk-bearing; the modal renders risk/policy, never a scope-derived risk). */
export function enrichApproval(row: ApprovalQueueRow): GatewayApprovalEnrichment {
  // The Approval's single required_approver is a best-effort mirror of the policy's first approver
  // (falls back to current_user when policy is absent OR carries an empty list). NOTE: the modal
  // renders the daemon's full `policyDecision.required_approvals` list VERBATIM (below) — this single
  // field is not the displayed approval requirement, so the fallback never fabricates the shown list.
  const requiredApprover = row.policy_decision?.required_approvals?.[0] ?? {
    kind: "current_user",
  };
  return {
    approval: {
      approval_id: row.approval_id,
      required_approver: requiredApprover,
      status: row.status,
      scope: "single_action",
      risk_level: row.risk_level, // REAL daemon risk (was the fixture side-map)
      action_request_id: row.action_request_id ?? null,
      expires_at: row.expires_at ?? null,
      plan_id: row.plan_id ?? null,
    },
    policyDecision: row.policy_decision ?? {
      // absent (the daemon hasn't enriched yet) → an honest "awaiting" placeholder, NOT a
      // fabricated specific decision; the risk above is still the row's real value.
      status: "require_approval",
      reasons: ["Awaiting the daemon's policy decision."],
      required_approvals: [requiredApprover],
      constraints: [],
      safer_alt: null,
    },
  };
}

// ─── Per-hunk git-action enrichment (053b — the real-row swap completes the 044 [med]) ──
// The DiffReview per-hunk git action (stage/unstage/discard) has NO ApprovalQueueRow in hand — the
// submit ack carries an `action_request_id`, not an `approval_id`. So sourcing the daemon's
// AUTHORITATIVE risk/policy means RE-FETCHING get_projection("ApprovalQueue") and matching the row by
// EXACT action_request_id (the ApprovalQueue isn't subscribed — 052 Q3). On a match → enrichApproval
// (the 053 Layer C real-row builder, reused for DRY): the card renders the daemon's real risk/policy,
// no fixture (LESSON 17 — resolves the 044 [med] on the per-hunk path too). On NO match (timing — the
// daemon mints the approval row async) → an HONEST awaiting placeholder, never a fabricated/UI-derived
// decision (forbidden #2). Parse-don't-trust is intrinsic: get_projection boundary-validates the page
// (a malformed payload → BoundaryValidationError, never a fabricated row — LESSON 22). The handle is
// typed `Pick<…,"get_projection">` → compile-time NO mutation reach; the per-hunk submit stays L2-HELD.
export async function enrichActionApproval(
  gateway: Pick<GatewayPort, "get_projection">,
  ack: ActionAck,
): Promise<GatewayApprovalEnrichment> {
  const page = await gateway.get_projection("ApprovalQueue");
  // EXACT, non-null string match: ApprovalQueueRow.action_request_id is Option<String>; a null/absent
  // row id must NEVER collapse-match the ack (a wrong-approval false match). The ack id is always a
  // non-null string and `===` makes null/undefined never equal it — no fuzzy/first-row fallback.
  const row = page.rows.find((r) => r.action_request_id === ack.action_request_id);
  if (row) return enrichApproval(row); // the daemon's real risk + policy (DRY w/ 053 Layer C)
  // Absent: the daemon hasn't surfaced the approval row in the queue snapshot yet. An honest awaiting
  // placeholder — the card's SHOWN risk comes from the live preview_action (previewRisk), so this
  // risk_level is a NON-displayed structural field, set FAIL-SAFE to 4 (over-warn, never under-warn;
  // §17 spirit) — it can never surface as a fabricated shown number (the modal reads the preview).
  return {
    approval: {
      approval_id: `appr_for_${ack.action_request_id}`,
      required_approver: { kind: "current_user" },
      // single_action ONLY — a per-hunk action is never standing-/bulk-granted here
      // (git.discard_hunk is non-standing-grantable daemon-side regardless; LESSON §32).
      scope: "single_action",
      status: "awaiting_approval",
      risk_level: 4,
      action_request_id: ack.action_request_id,
    },
    policyDecision: {
      status: "require_approval",
      reasons: ["Awaiting the daemon's policy decision."],
      required_approvals: [{ kind: "current_user" }],
      constraints: [],
      safer_alt: null,
    },
  };
}

// ─── ActionPlan SAMPLE (ui-073 — the exposed-ahead N-step plan-render fixture) ──
// There is NO live plan-submitter in production: the Brain "Run via Gateway" submit path is 8.1-gated
// + guarded-disabled (slice A), and `proj_approval_queue` carries per-approval rows WITHOUT the full
// ActionPlan model (no title / approval_mode / dependencies / per-step label). So PlanModal renders
// this daemon-SHAPED sample (the intent-seam-shadow precedent — the render path is real, only the
// data source is a fixture). FLAG (not faked): the live plan-data feed — assemble from
// proj_approval_queue grouped by plan_id, OR a future `get_action_plan` RPC serving the full model —
// is a deferred follow-on; when it lands this sample is deleted and the plan is built from the
// projection (no view-layer change). `overall_risk` + per-step `risk_level` here are the daemon's
// catalog-authoritative values (a daemon-formed plan), NOT UI-derived (Q3).
export const samplePlan: ActionPlan = {
  plan_id: "plan_sample_1",
  title: "Land the auth-refactor across three steps",
  approval_mode: "step_by_step",
  overall_risk: 3,
  dependencies: [
    { step_id: "step_apply", depends_on_step_ids: ["step_worktree"] },
    { step_id: "step_pr", depends_on_step_ids: ["step_apply"] },
  ],
  steps: [
    {
      step_id: "step_worktree",
      label: "Create the refactor worktree",
      required: true,
      can_skip: false,
      rollback_action_type: null,
      status: "awaiting_approval",
      action_request: {
        action_request_id: "ar_plan_worktree",
        action_type: "git.create_worktree",
        requester_type: "project_brain",
        requester_id: "brain_main",
        resource_refs: [{ type: "repo", id: "repo_auth" }],
        inputs: {},
        risk_level: 1,
        status: "awaiting_approval",
        created_at: "2026-06-21T00:00:00Z",
      },
    },
    {
      step_id: "step_apply",
      label: "Apply the refactor patch",
      required: true,
      can_skip: false,
      rollback_action_type: "git.revert_commit",
      status: "awaiting_approval",
      action_request: {
        action_request_id: "ar_plan_apply",
        action_type: "git.commit",
        requester_type: "project_brain",
        requester_id: "brain_main",
        resource_refs: [{ type: "worktree", id: "wt_auth_refactor" }],
        inputs: {},
        risk_level: 3,
        status: "awaiting_approval",
        created_at: "2026-06-21T00:00:00Z",
      },
    },
    {
      step_id: "step_pr",
      label: "Open the pull request",
      required: false,
      can_skip: true,
      rollback_action_type: null,
      status: "awaiting_approval",
      action_request: {
        action_request_id: "ar_plan_pr",
        action_type: "github.create_pr",
        requester_type: "project_brain",
        requester_id: "brain_main",
        resource_refs: [{ type: "repo", id: "repo_auth" }],
        inputs: {},
        risk_level: 2,
        status: "awaiting_approval",
        created_at: "2026-06-21T00:00:00Z",
      },
    },
  ],
};

/** Build the ActionPlan PlanModal renders for a plan-bearing approval (ui-073). EXPOSED-AHEAD:
 *  with no live plan-submitter, this returns the daemon-SHAPED `samplePlan` stamped with the
 *  approval's `plan_id` (a single fixture, NOT a per-id lookup). The live plan-data feed (group
 *  proj_approval_queue by plan_id, or a get_action_plan RPC) is a deferred follow-on — wired here
 *  the day a plan-submitter exists (no view change). */
export function enrichPlan(planId: string): ActionPlan {
  return { ...samplePlan, plan_id: planId };
}

// ─── Settings display fixtures (Integrations / Execution profiles) ──────────
// No Integration or ExecutionProfile projection exists yet (Phase 7 connectors;
// ExecutionProfile enum is 0.5b-gated). These card sets are PROVISIONAL DISPLAY
// FIXTURES (visual treatment per the prototype; flagged, not faked as live).

export interface IntegrationDisplay {
  id: string;
  name: string;
  /** lucide icon key: cpu | github | square-kanban | brain. */
  icon: "cpu" | "github" | "square-kanban" | "brain";
  connected: boolean;
  detail: string;
  scope?: string;
  action: "Manage" | "Connect";
}

export const integrationsDisplayFixture: IntegrationDisplay[] = [
  {
    id: "runtime",
    name: "Local runtime",
    icon: "cpu",
    connected: true,
    detail: "Healthy · sessions via the daemon · worktree root ~/wt",
    action: "Manage",
  },
  {
    id: "github",
    name: "GitHub",
    icon: "github",
    connected: true,
    detail: "org · mapped org/auth-service",
    scope: "Issues · PRs · checks",
    action: "Manage",
  },
  {
    id: "linear",
    name: "Linear",
    icon: "square-kanban",
    connected: false,
    detail: "Not connected — link a team to pull tickets into the Task Inbox",
    action: "Connect",
  },
  {
    id: "brain",
    name: "Project Brain store",
    icon: "brain",
    connected: true,
    detail: "Ready · indexed · grounded at the last commit",
    action: "Manage",
  },
];

export type ProfileHealth = "active" | "available" | "rate-limited" | "auth-expired";

export interface ProfileDisplay {
  name: string;
  provider: "claude" | "codex";
  health: ProfileHealth;
  sessions: number;
  usage: number;
  limit: number;
  resets: string;
  note: string;
}

export const profilesDisplayFixture: ProfileDisplay[] = [
  {
    name: "Claude Max Main",
    provider: "claude",
    health: "active",
    sessions: 2,
    usage: 62,
    limit: 100,
    resets: "—",
    note: "Primary interactive profile",
  },
  {
    name: "Claude Team Work",
    provider: "claude",
    health: "rate-limited",
    sessions: 1,
    usage: 98,
    limit: 100,
    resets: "in 24m",
    note: "Shared team seat",
  },
  {
    name: "Codex CLI Main",
    provider: "codex",
    health: "active",
    sessions: 1,
    usage: 34,
    limit: 80,
    resets: "—",
    note: "Local CLI harness",
  },
];

/**
 * Context-ring input for a session, from the REAL Usage projection — matched by the row's
 * `session_id`. A row with no context metadata (`context_pct_max` null, §9.1) → pct null — the
 * caller renders "unknown", NEVER a fabricated number (forbidden #4).
 */
export function contextForSession(
  usage: UsageRow[],
  sessionId: string,
): { pct: number | null; accuracy: NonNullable<UsageRow["metric_quality"]> } | null {
  const row = usage.find((u) => u.session_id === sessionId);
  if (!row) return null;
  // accuracy is Option<MetricQuality> — null coalesces to the honest "unavailable" (§11.7), so the
  // kit meter always gets a concrete accuracy (never null); context_pct_max null → the caller's "unknown".
  return { pct: row.context_pct_max ?? null, accuracy: row.metric_quality ?? "unavailable" };
}

/** Sessions grouped by project_id (tree order helper for the sidebar). */
export function sessionsByProject(
  sessions: SessionRow[],
): Record<string, SessionRow[]> {
  const out: Record<string, SessionRow[]> = {};
  for (const s of sessions) {
    const pid = s.project_id ?? "";
    (out[pid] ??= []).push(s);
  }
  return out;
}
