//! `git/` — git2 READ-ONLY repo introspection (P5.1 project detection; P5.2 dual-git read backend)
//! and, in P5.2, the git-CLI worktree/branch/commit/merge mutations exposed as typed Gateway actions.
//!
//! **Forbidden #6:** git2 is read-only here. Every git *mutation* goes through the git CLI as an
//! audited Gateway action — never a git2 mutating API. `detect` (edges-001) is the project-detection
//! reader; `reads` + `precedence` (edges-002) are the worktree-status read backend + the §5.1-R7
//! two-axis precedence fn. The git-CLI worktree/branch mutation executors land in the gated
//! 5.2-remainder slice.

pub mod detect;
pub mod precedence;
pub mod reads;
