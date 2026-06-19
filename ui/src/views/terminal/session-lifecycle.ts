// The lifecycle partition of the frozen §5.1 Session statuses for the terminal
// well: ENDED (the agent process has stopped → an honest ended state) vs LIVE (the
// terminal may still be streaming). The partition is COMPLETENESS-drift-pinned
// against the generated `Session` enum (session-lifecycle.test) — Lesson §5: a NEW
// daemon status must be consciously classified here, never a silent fall-through to
// "live" (which would render a live terminal for an already-ended session — #9
// honesty: never show "still running" for a dead process).

/** §5.1 Session statuses where the agent process has ENDED. */
export const ENDED_SESSION_STATUSES: ReadonlySet<string> = new Set([
  "completed",
  "failed",
  "killed",
  "archived",
]);

/** §5.1 Session statuses where the session is LIVE (terminal may be streaming). */
export const LIVE_SESSION_STATUSES: ReadonlySet<string> = new Set([
  "creating",
  "starting",
  "active",
  "thinking",
  "running_command",
  "editing_files",
  "running_tests",
  "waiting_on_permission",
  "waiting_on_human_input",
  "waiting_on_external_service",
  "idle",
  "stale",
  "changes_ready",
]);

/** True when the session's process has ended (the well shows the honest ended state). */
export const isEndedSession = (status: string): boolean =>
  ENDED_SESSION_STATUSES.has(status);
