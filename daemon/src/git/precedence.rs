//! §5.1-R7 two-axis worktree-status precedence.
//!
//! `derive_worktree_status` collapses the git-sync axis (`WorktreeGit`) and the lifecycle-overlay
//! axis (`WorktreeOverlay`) into the single most-salient §5.1 status. The order honors the LOCKED
//! §5.1-R7 partial spec (`locked/conflicts > dirty > ahead/behind > clean`, `ARCHITECTURE.md:53`),
//! extended with the overlay-lifecycle + terminal tiers §5.1 leaves unranked:
//!
//! ```text
//! deleted > conflicts > locked > merged > pr_open > prunable
//!     > dirty > untracked_files > behind_base > ahead_of_base > creating > clean
//! ```
//!
//! Pure: both axes are params (the overlay source is event-driven + gated; the `proj_worktree`
//! projector that supplies them lands in the gated 5.2-remainder slice).

use nexusops_shared::status::{WorktreeGit, WorktreeOverlay};

/// The single §5.1 status the precedence fn selects — a value from EITHER frozen axis (no new enum,
/// no contract). Serializes to the frozen `WorktreeGit ∪ WorktreeOverlay` snake_case wire string.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DerivedWorktreeStatus {
    Git(WorktreeGit),
    Overlay(WorktreeOverlay),
}

impl DerivedWorktreeStatus {
    /// The frozen §5.1 snake_case wire value (the `proj_worktree.status` column). Hand-mapped for a
    /// `&'static str`, but pinned byte-for-byte to the frozen serde serialization by the
    /// `derived_wire_str_matches_serde` test (LESSON §2 — the wire value is the contract). The
    /// exhaustive match means a new frozen-enum variant fails to compile here → forces a reconcile.
    pub fn as_wire_str(self) -> &'static str {
        match self {
            Self::Git(g) => match g {
                WorktreeGit::Clean => "clean",
                WorktreeGit::Dirty => "dirty",
                WorktreeGit::UntrackedFiles => "untracked_files",
                WorktreeGit::Conflicts => "conflicts",
                WorktreeGit::BehindBase => "behind_base",
                WorktreeGit::AheadOfBase => "ahead_of_base",
            },
            Self::Overlay(o) => match o {
                WorktreeOverlay::Creating => "creating",
                WorktreeOverlay::Locked => "locked",
                WorktreeOverlay::PrOpen => "pr_open",
                WorktreeOverlay::Merged => "merged",
                WorktreeOverlay::Prunable => "prunable",
                WorktreeOverlay::Deleted => "deleted",
            },
        }
    }

    /// Salience rank in the §5.1-R7 total order; 0 = most salient. Exhaustive over both axes (a new
    /// frozen variant fails to compile → forces a reconcile of the precedence order).
    fn rank(self) -> u8 {
        match self {
            Self::Overlay(WorktreeOverlay::Deleted) => 0,
            Self::Git(WorktreeGit::Conflicts) => 1,
            Self::Overlay(WorktreeOverlay::Locked) => 2,
            Self::Overlay(WorktreeOverlay::Merged) => 3,
            Self::Overlay(WorktreeOverlay::PrOpen) => 4,
            Self::Overlay(WorktreeOverlay::Prunable) => 5,
            Self::Git(WorktreeGit::Dirty) => 6,
            Self::Git(WorktreeGit::UntrackedFiles) => 7,
            Self::Git(WorktreeGit::BehindBase) => 8,
            Self::Git(WorktreeGit::AheadOfBase) => 9,
            Self::Overlay(WorktreeOverlay::Creating) => 10,
            Self::Git(WorktreeGit::Clean) => 11,
        }
    }
}

/// Collapse the git axis + an optional overlay into the most-salient §5.1 status (§5.1-R7). With no
/// overlay event yet, the git axis stands alone. On a salience tie the overlay wins; the two axes
/// never share a rank (a same-rank collision is architecturally forbidden, not merely accidentally
/// absent — every variant gets a distinct slot in `rank`'s exhaustive match), so the tie-break is
/// moot today and just keeps the fn total.
pub fn derive_worktree_status(
    git: WorktreeGit,
    overlay: Option<WorktreeOverlay>,
) -> DerivedWorktreeStatus {
    let git_status = DerivedWorktreeStatus::Git(git);
    match overlay {
        Some(o) => {
            let overlay_status = DerivedWorktreeStatus::Overlay(o);
            if overlay_status.rank() <= git_status.rank() {
                overlay_status
            } else {
                git_status
            }
        }
        None => git_status,
    }
}
