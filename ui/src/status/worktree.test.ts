import { describe, it, expect } from "vitest";
import { deriveWorktreeStatus } from "./worktree";
import { describeStatus } from "./descriptors";

describe("worktree two-axis precedence", () => {
  it("worktree_overlay_terminal_precedence", () => {
    // overlay terminal states win over the git axis
    expect(deriveWorktreeStatus("dirty", "deleted")).toEqual({
      machine: "WorktreeOverlay",
      status: "deleted",
    });
    expect(deriveWorktreeStatus("dirty", "merged")).toEqual({
      machine: "WorktreeOverlay",
      status: "merged",
    });
    // pr_open overlay surfaces
    expect(deriveWorktreeStatus("clean", "pr_open")).toEqual({
      machine: "WorktreeOverlay",
      status: "pr_open",
    });
    // clean git + no overlay → clean
    expect(deriveWorktreeStatus("clean", null)).toEqual({
      machine: "WorktreeGit",
      status: "clean",
    });
    // terminal overlay wins even over git conflicts (worktree is gone/merged) —
    // rule 1 precedes rule 2.
    expect(deriveWorktreeStatus("conflicts", "deleted")).toEqual({
      machine: "WorktreeOverlay",
      status: "deleted",
    });
    expect(deriveWorktreeStatus("conflicts", "merged")).toEqual({
      machine: "WorktreeOverlay",
      status: "merged",
    });
  });

  it("worktree_conflicts_surface_loud", () => {
    // conflicts surface even under a benign overlay (pr_open / locked), not masked
    expect(deriveWorktreeStatus("conflicts", "pr_open")).toEqual({
      machine: "WorktreeGit",
      status: "conflicts",
    });
    expect(deriveWorktreeStatus("conflicts", "locked")).toEqual({
      machine: "WorktreeGit",
      status: "conflicts",
    });
    // …and the derived conflicts resolves to a high-attention descriptor
    const derived = deriveWorktreeStatus("conflicts", "pr_open");
    expect(describeStatus(derived.machine, derived.status).attentionRank).toBe(4);
  });
});
