// Pure projection → shell-chrome derivations. No authoritative state is invented
// here — every value is computed from the validated projection rows (§4.2 law 2).
import type {
  ApprovalQueueRow,
  AuditEventRow,
  ProjectActivityRow,
  PullRequestRow,
  SessionRow,
} from "../contracts/index";

// Session states that no longer count as "active work" (§5.1 terminal set).
// NOTE: `stale` is deliberately NOT terminal — it is time-derived (§5.1), a
// degraded-but-still-active state, so it counts toward activeSessions here;
// 6.2's attention-rank table is where stale gets demoted.
const TERMINAL_SESSION = new Set(["failed", "completed", "archived", "killed"]);
// PR states that are no longer open (terminal); everything else is "open-ish".
const CLOSED_PR = new Set(["merged", "closed"]);
// Approval states still awaiting a human decision.
const PENDING_APPROVAL = new Set([
  "requested",
  "previewed",
  "awaiting_approval",
  "escalated",
]);
// Session states that mean the agent is waiting on the HUMAN (→ waitingOnYou).
// `waiting_on_external_service` is intentionally excluded: that session is
// blocked by an external service (CI/GitHub/Linear), not by the human, so it is
// not a "waiting on you" item (orchestrator-confirmed Q3 semantics).
const WAITING_SESSION = new Set([
  "waiting_on_permission",
  "waiting_on_human_input",
]);

export const ACTIVITY_FEED_DEFAULT_LIMIT = 50;

/** Approvals still awaiting a human decision (the HIQ queue membership). */
export function pendingApprovals(approvals: ApprovalQueueRow[]): ApprovalQueueRow[] {
  return approvals.filter((a) => PENDING_APPROVAL.has(a.status));
}

/** Sessions waiting on the HUMAN (permission / input — §5.1 waiting set). */
export function waitingSessions(sessions: SessionRow[]): SessionRow[] {
  return sessions.filter((s) => WAITING_SESSION.has(s.status));
}

export interface ProjectSwitcherCounts {
  activeSessions: number;
  openPRs: number;
  waitingOnYou: number;
}

/**
 * Per-project live counts for the project switcher. Keyed by project_id, with an
 * explicit entry (zeros) for EVERY project in the projection — never absent.
 */
export function deriveProjectSwitcherCounts(input: {
  projects: ProjectActivityRow[];
  sessions: SessionRow[];
  pullRequests: PullRequestRow[];
  approvals: ApprovalQueueRow[];
}): Record<string, ProjectSwitcherCounts> {
  const out: Record<string, ProjectSwitcherCounts> = {};
  for (const project of input.projects) {
    const pid = project.project_id;
    const projectSessions = input.sessions.filter((s) => s.project_id === pid);
    const activeSessions = projectSessions.filter(
      (s) => !TERMINAL_SESSION.has(s.status),
    ).length;
    const openPRs = input.pullRequests.filter(
      (pr) => pr.project_id === pid && !CLOSED_PR.has(pr.status),
    ).length;
    const pendingApprovalCount = input.approvals.filter(
      (a) => a.project_id === pid && PENDING_APPROVAL.has(a.status),
    ).length;
    const waitingSessionCount = projectSessions.filter((s) =>
      WAITING_SESSION.has(s.status),
    ).length;
    out[pid] = {
      activeSessions,
      openPRs,
      waitingOnYou: pendingApprovalCount + waitingSessionCount,
    };
  }
  return out;
}

/**
 * The activity-dock event feed: optionally scoped to one project, ordered by the
 * canonical event seq (descending — most recent first; §7.1), capped at `limit`.
 */
export function deriveActivityFeed(
  events: AuditEventRow[],
  opts: { projectId?: string; limit?: number } = {},
): AuditEventRow[] {
  const { projectId, limit = ACTIVITY_FEED_DEFAULT_LIMIT } = opts;
  const scoped = projectId
    ? events.filter((e) => e.project_id === projectId)
    : events;
  return [...scoped].toSorted((a, b) => b.seq - a.seq).slice(0, limit);
}
