import { describe, it, expect } from "vitest";
import { describeStatus } from "./descriptors";
import { triageBucket } from "./attention";
import { validators } from "../contracts/index";

// The 9 frozen STATUS machines (Worktree = its two axes). ActorType / IdKind /
// DesktopObjectKind are not status machines; ExecutionProfile is held (0.5b).
const STATUS_MACHINES = [
  "Session",
  "Task",
  "PullRequest",
  "WorkflowInstance",
  "ProjectBrain",
  "Approval",
  "ActionRequest",
  "AgentTeam",
  "WorktreeGit",
  "WorktreeOverlay",
] as const;

// The UI status-machine identifier is UI-render-policy naming (descriptor table
// keys) and is DECOUPLED from the frozen enum `$def` name. The 0.19.0 Gateway
// freeze renamed the two R-5 value-sets to their `$def` names; the validators
// record is keyed by `$def` name, so the two diverging machines alias here (the
// other 8 machine names == their `$def` name). Drift-pin stays intact: a daemon
// status added to the aliased validator still fails the completeness sweep.
const VALIDATOR_KEY: Record<string, string> = {
  Approval: "ApprovalStatus",
  ActionRequest: "ActionRequestStatus",
};

describe("status descriptor table", () => {
  it("descriptor_covers_every_frozen_status", () => {
    // [load-bearing] drift-pinned to the generated enums: a future daemon-added
    // status has no descriptor → falls to the "unknown" fallback → this fails.
    for (const machine of STATUS_MACHINES) {
      const validator = validators[VALIDATOR_KEY[machine] ?? machine];
      expect(validator, `missing generated validator for ${machine}`).toBeDefined();
      for (const status of validator!.options as readonly string[]) {
        const d = describeStatus(machine, status);
        expect(
          d.visualKind,
          `${machine}/${status} fell through to the unknown fallback`,
        ).not.toBe("unknown");
        expect(d.attentionRank).toBeGreaterThanOrEqual(0);
        expect(d.attentionRank).toBeLessThanOrEqual(5);
      }
    }
  });

  it("attention_states_not_floored_to_idle", () => {
    // [load-bearing correctness] the §11.3 fix — these never fall to idle/0.
    expect(describeStatus("Session", "waiting_on_permission").attentionRank).toBe(4);
    expect(describeStatus("WorktreeGit", "conflicts").attentionRank).toBe(4);
    expect(describeStatus("Task", "blocked").attentionRank).toBe(4);
    expect(describeStatus("Session", "stale").attentionRank).toBe(3);
    // changes_ready is review-needed on every machine that carries it — pin both
    expect(describeStatus("Session", "changes_ready").attentionRank).toBe(4);
    expect(describeStatus("Task", "changes_ready").attentionRank).toBe(4);
  });

  it("attention_ranks_match_ladder", () => {
    expect(describeStatus("Session", "waiting_on_human_input").attentionRank).toBe(5);
    expect(describeStatus("AgentTeam", "waiting_on_human").attentionRank).toBe(5);
    expect(describeStatus("Session", "failed").attentionRank).toBe(4);
    expect(describeStatus("Session", "changes_ready").attentionRank).toBe(4);
    expect(describeStatus("Session", "running_command").attentionRank).toBe(2);
    expect(describeStatus("Session", "running_tests").attentionRank).toBe(2);
    expect(describeStatus("Session", "active").attentionRank).toBe(1);
    expect(describeStatus("Session", "idle").attentionRank).toBe(0);
    expect(describeStatus("Session", "completed").attentionRank).toBe(0);
    expect(describeStatus("Session", "archived").attentionRank).toBe(0);
  });

  it("unknown_status_is_visible_not_idle", () => {
    const d = describeStatus("Session", "bogus_status");
    expect(d.visualKind).toBe("unknown");
    expect(d.attentionRank).toBeGreaterThan(0); // visible, not a silent idle/0
    // pin the exact bucket so a future UNKNOWN_RANK change can't silently shift it
    expect(d.attentionRank).toBe(3);
    expect(triageBucket(d.attentionRank)).toBe("working");
  });
});
