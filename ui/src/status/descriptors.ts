// The canonical status descriptor table (§11.3): every frozen §5.1 status, keyed
// by (machine, status), → { attentionRank 0–5, visualKind, label }. This is the
// single source the sidebar weight / queue / sort bind to. NO status falls
// through to idle — a completeness test (descriptors.test.ts) pins coverage to
// the generated enums, so a future daemon-added status fails CI until covered.
//
// visualKind names a kit StatusKind (StatusPill); the kit-kind mapping is
// rendered + refined in 6.2b. Ranks reviewed at Step-2.5 against the §11.1
// ladder — the four no-fall-through states (waiting_on_permission / conflicts /
// blocked / stale) + changes_ready (review-needed) are the load-bearing ones.
import type { AttentionRank } from "./attention";

export interface StatusDescriptor {
  machine: string;
  status: string;
  attentionRank: AttentionRank;
  visualKind: string;
  label: string;
}

type Entry = [rank: AttentionRank, visualKind: string];

const TABLE: Record<string, Record<string, Entry>> = {
  Session: {
    creating: [1, "active"],
    starting: [1, "active"],
    active: [1, "active"],
    thinking: [2, "running"],
    running_command: [2, "running"],
    editing_files: [2, "editing"],
    running_tests: [2, "testing"],
    waiting_on_permission: [4, "waiting-perm"],
    waiting_on_human_input: [5, "waiting-human"],
    waiting_on_external_service: [2, "running"],
    idle: [0, "idle"],
    stale: [3, "stale"],
    changes_ready: [4, "approval"],
    failed: [4, "failed"],
    completed: [0, "completed"],
    archived: [0, "archived"],
    killed: [0, "archived"],
  },
  Task: {
    unassigned: [1, "active"],
    queued: [1, "active"],
    assigned: [1, "active"],
    ready: [1, "active"],
    in_progress: [1, "active"],
    blocked: [4, "blocked"],
    needs_clarification: [4, "approval"],
    in_review: [1, "pr-open"],
    changes_ready: [4, "approval"],
    pr_opened: [1, "pr-open"],
    needs_review: [4, "approval"],
    requested_changes: [4, "blocked"],
    done: [0, "completed"],
    deferred: [0, "archived"],
    merged: [0, "merged"],
    closed: [0, "archived"],
    abandoned: [0, "archived"],
  },
  PullRequest: {
    draft: [1, "pr-open"],
    open: [1, "pr-open"],
    checks_pending: [2, "running"],
    checks_failing: [4, "failed"],
    needs_review: [4, "approval"],
    changes_requested: [4, "blocked"],
    approved: [2, "approved"],
    mergeable: [2, "approved"],
    conflict: [4, "conflict"],
    merged: [0, "merged"],
    closed: [0, "archived"],
  },
  WorkflowInstance: {
    not_detected: [0, "idle"],
    pack_available: [1, "active"],
    needs_personalization: [1, "active"],
    personalization_in_progress: [2, "running"],
    generated_review_required: [4, "approval"],
    active: [1, "active"],
    ready_for_team_run: [1, "active"],
    degraded: [3, "degraded"],
    drift_detected: [3, "degraded"],
    upgrade_available: [1, "active"],
    archived: [0, "archived"],
    // "stale" (not "idle") — detached is the only non-zero-rank entry, so it must
    // not share the settled-idle visual kind (6.2b maps idle → settled rendering).
    detached: [1, "stale"],
  },
  ProjectBrain: {
    not_configured: [0, "idle"],
    indexing: [2, "running"],
    ready: [0, "completed"],
    partial_index: [3, "degraded"],
    stale: [3, "stale"],
    graph_degraded: [3, "degraded"],
    transcript_ingestion_off: [0, "idle"],
    transcript_ingestion_active: [2, "running"],
    reindex_required: [3, "degraded"],
    error: [4, "failed"],
  },
  Approval: {
    requested: [4, "approval"],
    previewed: [4, "approval"],
    awaiting_approval: [4, "approval"],
    approved: [0, "approved"],
    denied: [0, "archived"],
    edited: [4, "approval"],
    auto_approved_by_policy: [0, "approved"],
    expired: [1, "stale"],
    cancelled: [0, "archived"],
    escalated: [4, "approval"],
  },
  ActionRequest: {
    submitted: [1, "active"],
    previewed: [1, "active"],
    policy_decided: [1, "active"],
    awaiting_approval: [4, "approval"],
    approved: [1, "approved"],
    denied: [0, "archived"],
    queued: [2, "running"],
    executing: [2, "running"],
    succeeded: [0, "completed"],
    failed: [4, "failed"],
    partially_succeeded: [4, "failed"],
    rolled_back: [1, "archived"],
    rollback_failed: [4, "critical"],
    cancelled: [0, "archived"],
    expired: [0, "archived"],
  },
  AgentTeam: {
    draft: [1, "active"],
    starting: [1, "active"],
    active: [1, "active"],
    waiting_on_human: [5, "waiting-human"],
    blocked: [4, "blocked"],
    reconciling_outputs: [2, "running"],
    completed: [0, "completed"],
    failed: [4, "failed"],
    archived: [0, "archived"],
  },
  WorktreeGit: {
    clean: [0, "completed"],
    dirty: [1, "active"],
    untracked_files: [1, "active"],
    conflicts: [4, "conflict"],
    behind_base: [1, "active"],
    ahead_of_base: [1, "active"],
  },
  WorktreeOverlay: {
    creating: [1, "active"],
    locked: [1, "active"],
    pr_open: [1, "pr-open"],
    merged: [0, "merged"],
    prunable: [0, "archived"],
    deleted: [0, "archived"],
  },
};

function humanize(status: string): string {
  const spaced = status.replace(/_/g, " ");
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

// Unknown status → a VISIBLE descriptor (non-zero rank + a flagged kind), never
// a silent idle/0 (§11.3 "unknown → visible, not idle"). Defensive: in normal
// operation the completeness test guarantees every frozen status is covered.
const UNKNOWN_RANK: AttentionRank = 3;

export function describeStatus(
  machine: string,
  status: string,
): StatusDescriptor {
  const entry = TABLE[machine]?.[status];
  if (!entry) {
    return {
      machine,
      status,
      attentionRank: UNKNOWN_RANK,
      visualKind: "unknown",
      label: `Unknown (${machine}/${status})`,
    };
  }
  const [attentionRank, visualKind] = entry;
  return { machine, status, attentionRank, visualKind, label: humanize(status) };
}
