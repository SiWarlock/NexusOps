// Worktree is the one derived, two-axis status (§5.1/§7.2): a git-sync axis
// (WorktreeGit) + a lifecycle-overlay axis (WorktreeOverlay). This collapses the
// two into the single status to render, by precedence:
//   1. overlay terminal (deleted / merged) wins — the worktree is gone/merged.
//   2. git conflicts surface LOUD — never masked by a benign overlay.
//   3. pr_open overlay surfaces.
//   4. any other active overlay (creating / locked / prunable).
//   5. otherwise the git axis (clean / dirty / untracked / ahead / behind).
import { z } from "zod";
import { WorktreeGit, WorktreeOverlay } from "../contracts/index";

export type WorktreeGitStatus = z.infer<typeof WorktreeGit>;
export type WorktreeOverlayStatus = z.infer<typeof WorktreeOverlay>;

export interface DerivedWorktreeStatus {
  machine: "WorktreeGit" | "WorktreeOverlay";
  status: WorktreeGitStatus | WorktreeOverlayStatus;
}

export function deriveWorktreeStatus(
  git: WorktreeGitStatus,
  overlay: WorktreeOverlayStatus | null,
): DerivedWorktreeStatus {
  if (overlay === "deleted" || overlay === "merged") {
    return { machine: "WorktreeOverlay", status: overlay };
  }
  if (git === "conflicts") {
    return { machine: "WorktreeGit", status: "conflicts" };
  }
  if (overlay === "pr_open") {
    return { machine: "WorktreeOverlay", status: "pr_open" };
  }
  if (overlay !== null) {
    return { machine: "WorktreeOverlay", status: overlay };
  }
  return { machine: "WorktreeGit", status: git };
}
