//! P4.0b-ui1 (brief 052) — the daemon's git2 READ-ONLY introspection (§6.1/§7.2). The FIRST git2
//! use in the daemon: the `get_diff` RPC's live diff read for the ui's 6.3e per-hunk review.
//!
//! **READ-ONLY (forbidden #6).** git2 is used here ONLY to READ (the HEAD→workdir diff). All git
//! MUTATIONS (stage/unstage/discard/worktree/commit/merge) go through the **git CLI as Gateway
//! actions** — never a git2 mutating API. This module opens a repo + computes a diff; it never writes.

use std::path::Path;

use nexusops_shared::ipc::{DiffLine, DiffLineKind, DiffResult, Hunk};

/// A read-only git introspection failure (the diff couldn't be read). Never a mutation error — this
/// module only reads.
#[derive(Debug, thiserror::Error)]
pub enum GitReadError {
    #[error("git open failed for {path}: {source}")]
    Open {
        path: String,
        #[source]
        source: git2::Error,
    },
    #[error("git diff read failed: {0}")]
    Diff(#[source] git2::Error),
}

/// Read `file`'s **HEAD→workdir** diff (all uncommitted changes, staged + unstaged) LIVE via git2,
/// as structured [`Hunk`]s. READ-ONLY. A clean (unmodified) file → no hunks. An unborn HEAD (a repo
/// with no commits yet) diffs against the empty tree (everything is "added").
///
/// **A modified BINARY file → no hunks** (a line-based diff is N/A) — indistinguishable here from a
/// clean file. The ui-6.3e per-hunk surface is text-oriented; a distinct binary signal (e.g. a
/// `DiffResult.is_binary` flag) is additive-later (deferred, like the "\ No newline at eof" marker).
///
/// The `old_start`/`new_start` hunk positions are the hunk-identity the ui packs into the git.*
/// action resource_ref id (read↔mutate consistency, §17). `context_lines(3)` = the standard window.
pub fn read_diff(repo_path: &Path, file: &str) -> Result<DiffResult, GitReadError> {
    let repo = git2::Repository::open(repo_path).map_err(|e| GitReadError::Open {
        path: repo_path.display().to_string(),
        source: e,
    })?;
    // the HEAD tree. An UNBORN branch (a repo with no commits yet) → None → diff against the empty
    // tree (everything "added"). Any OTHER head() error (a corrupt/broken HEAD ref) is a real fault —
    // propagate it, never silently treat a broken HEAD as "empty repo" (which would mis-report every
    // line as added).
    let head_tree = match repo.head() {
        Ok(head) => Some(head.peel_to_tree().map_err(GitReadError::Diff)?),
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => None,
        Err(e) => return Err(GitReadError::Diff(e)),
    };
    let mut opts = git2::DiffOptions::new();
    opts.pathspec(file);
    opts.context_lines(3);
    let diff = repo
        .diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut opts))
        .map_err(GitReadError::Diff)?;

    // ONE `print` callback (FnMut) builds the structured hunks — avoids the multi-callback borrow
    // conflict of `foreach`. A 'H' line starts a new Hunk (from the DiffHunk); a content line
    // (' '/'+'/'-' and the eofnl variants) appends a typed DiffLine to the current hunk; 'F'/'B'
    // (file/binary headers) are skipped.
    let mut hunks: Vec<Hunk> = Vec::new();
    diff.print(git2::DiffFormat::Patch, |_delta, hunk, line| {
        match line.origin() {
            'H' => {
                if let Some(h) = hunk {
                    hunks.push(Hunk {
                        header: String::from_utf8_lossy(h.header()).trim_end().to_string(),
                        old_start: h.old_start(),
                        old_lines: h.old_lines(),
                        new_start: h.new_start(),
                        new_lines: h.new_lines(),
                        lines: Vec::new(),
                    });
                }
            }
            'F' | 'B' => {} // file / binary header — not a content line
            origin => {
                let kind = match origin {
                    '+' | '>' => DiffLineKind::Added, // addition (+ the ADD_EOFNL variant)
                    '-' | '<' => DiffLineKind::Removed, // deletion (+ the DEL_EOFNL variant)
                    _ => DiffLineKind::Context,       // ' ' context (+ the CONTEXT_EOFNL variant)
                };
                let content = String::from_utf8_lossy(line.content()).to_string();
                if let Some(h) = hunks.last_mut() {
                    h.lines.push(DiffLine { kind, content });
                }
            }
        }
        true
    })
    .map_err(GitReadError::Diff)?;

    Ok(DiffResult { hunks })
}
