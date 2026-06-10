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
import type { SessionRow, UsageRow } from "../contracts/index";

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
  },
};

/** Display meta for the §14 fixture projects (keyed by project_id). */
export const projectDisplayFixture: Record<string, ProjectDisplayMeta> = {
  project_fixture_1: { repo: "org/auth-service", workflow: "active" },
  project_fixture_2: { repo: "org/billing", workflow: "drift" },
  project_fixture_3: { repo: "org/docs-site", workflow: "none" },
};

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
