//! `git/` — git2 READ-ONLY repo introspection (P5.1 project detection; P5.2 dual-git read backend)
//! and, in P5.2, the git-CLI worktree/branch/commit/merge mutations exposed as typed Gateway actions.
//!
//! **Forbidden #6:** git2 is read-only here. Every git *mutation* goes through the git CLI as an
//! audited Gateway action — never a git2 mutating API. This slice (edges-001) lands the detection
//! reader only.

pub mod detect;
