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
  Approval,
  ApprovalQueueRow,
  PolicyDecision,
  SessionRow,
  UsageRow,
} from "../contracts/index";

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

// ─── GatewayModal enrichment (PROVISIONAL display side-map) ───────────────────
// The GatewayModal renders the daemon's full Approval + PolicyDecision, but the
// ApprovalQueue projection row is THIN ({approval_id, project_id, status, title?}).
// Until the daemon enriches the projection (Carry-forward: projection-enrichment +
// a preview/policy RPC), this side-map supplies the full shapes — fixture-driven,
// keyed by approval_id, with a default for any unlisted row. The render path is real
// (the modal is a pure renderer of this daemon-shaped data); only the source is a fixture.
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

/** Enrich a thin ApprovalQueue row → the full {Approval, PolicyDecision} the modal
 *  renders. A default covers any unlisted row (still daemon-SHAPED, never UI-derived risk). */
export function enrichApproval(row: ApprovalQueueRow): GatewayApprovalEnrichment {
  return (
    gatewayApprovalEnrichment[row.approval_id] ?? {
      approval: {
        approval_id: row.approval_id,
        required_approver: { kind: "current_user" },
        status: row.status,
        scope: "single_action",
        risk_level: 2,
        // NULL (not the approval_id) for an unlisted row — an approval_id is NOT an
        // action_request_id (distinct daemon namespaces); the modal suppresses the
        // preview fetch honestly until the real daemon projection supplies the link.
        action_request_id: null,
      },
      policyDecision: {
        status: "require_approval",
        reasons: ["This action requires human approval."],
        required_approvals: [{ kind: "current_user" }],
        constraints: [],
        safer_alt: null,
      },
    }
  );
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
 * Context-ring input for a session, from the REAL Usage projection (subject_id ↔
 * session_id). Codex / unavailable rows report no context (§9.1) → null — the
 * caller renders "unknown", NEVER a fabricated number (forbidden #4).
 */
export function contextForSession(
  usage: UsageRow[],
  sessionId: string,
): { pct: number | null; accuracy: UsageRow["metric_quality"] } | null {
  const row = usage.find((u) => u.subject_id === sessionId);
  if (!row) return null;
  return { pct: row.context_pct ?? null, accuracy: row.metric_quality };
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
