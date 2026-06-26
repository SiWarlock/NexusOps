//! `git/` — git2 READ-ONLY repo introspection. Edges (P5.1/P5.2): project detection (`detect`) + the
//! dual-git worktree-status read backend (`reads` + the §5.1-R7 `precedence` fn) + the git-CLI
//! worktree/branch mutation executors exposed as typed Gateway actions (`cli` + `executor`). Main
//! (P4.0b-ui1, brief 052): the `get_diff` RPC's live HEAD→workdir diff read (`read_diff` below) for
//! the ui 6.3e per-hunk review.
//!
//! **Forbidden #6:** git2 is READ-ONLY everywhere in this module. Every git *mutation* (stage/unstage/
//! discard/worktree/branch/commit/merge) goes through the **git CLI as an audited Gateway action** —
//! never a git2 mutating API. (`git::read_diff` here is distinct from `git::reads::read_diff` — the
//! former returns the ui `ipc::DiffResult` for get_diff; the latter the edges `reads::FileChange` set.)

pub mod cli;
pub mod detect;
pub mod executor;
pub mod precedence;
pub mod reads;

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

/// Parse a GitHub PR **unified-diff string** (octocrab `pulls().get_diff(pr) → String`, D7) into the
/// frozen [`DiffResult`] — the SAME shape `read_diff` produces from git2, reused for ui consistency.
/// `file: Some(path)` keeps only that file's hunks (matched against the `diff --git … b/<path>`); `None`
/// keeps EVERY file's hunks FLATTENED (the flat `DiffResult` carries no per-file attribution — a per-file
/// file-tree is a post-D7 follow-on). Tolerant (the §42 no-panic posture): a malformed input parses what
/// it can; the `\ No newline at eof` marker + all metadata (`index`/`---`/`+++`/`new file`) are skipped
/// (the `Hunk` doc defers the no-newline marker). `DiffLine.content` retains the trailing newline.
pub fn parse_unified_diff(diff: &str, file: Option<&str>) -> DiffResult {
    let mut hunks: Vec<Hunk> = Vec::new();
    let mut in_selected_file = file.is_none(); // None → every file is selected
    let mut cur: Option<Hunk> = None;

    for line in diff.split_inclusive('\n') {
        // `trimmed` is the line WITHOUT its terminator, for prefix/header matching (CRLF-tolerant — a
        // GitHub diff is LF, but a stray `\r` must not leak into a `diff --git … b/<path>` file match or
        // a hunk header). The CONTENT lines below use `line` verbatim (the trailing newline retained).
        let trimmed = line.strip_suffix('\n').unwrap_or(line);
        let trimmed = trimmed.strip_suffix('\r').unwrap_or(trimmed);
        if let Some(rest) = trimmed.strip_prefix("diff --git ") {
            // a new file section → flush the prior hunk under the PRIOR selection, then re-select.
            if let Some(h) = cur.take() {
                if in_selected_file {
                    hunks.push(h);
                }
            }
            let b_path = diff_git_b_path(rest);
            in_selected_file = match file {
                None => true,
                Some(want) => b_path.as_deref() == Some(want),
            };
        } else if trimmed.starts_with("@@") {
            if let Some(h) = cur.take() {
                if in_selected_file {
                    hunks.push(h);
                }
            }
            if in_selected_file {
                cur = Some(parse_hunk_header(trimmed));
            }
        } else if in_selected_file {
            if let Some(h) = cur.as_mut() {
                // a line within a hunk — classify by the first byte (the +/-/space marker). `---`/`+++`
                // file-header lines appear BEFORE the first `@@` (cur is None) → never misread here.
                let kind = match line.as_bytes().first() {
                    Some(b' ') => DiffLineKind::Context,
                    Some(b'+') => DiffLineKind::Added,
                    Some(b'-') => DiffLineKind::Removed,
                    _ => continue, // `\ No newline…`, a stray blank, or EOF marker → not a diff line.
                };
                // strip the 1-byte ASCII marker; the rest (incl. the trailing newline) is the content.
                h.lines.push(DiffLine {
                    kind,
                    content: line.get(1..).unwrap_or("").to_string(),
                });
            }
        }
    }
    if let Some(h) = cur.take() {
        if in_selected_file {
            hunks.push(h);
        }
    }
    DiffResult { hunks }
}

/// The `b/<path>` (new-side) file path from a `diff --git a/<path> b/<path>` operand string. Renames
/// (`a/old b/new`) yield the NEW path; a path containing ` b/` resolves to the LAST occurrence.
fn diff_git_b_path(rest: &str) -> Option<String> {
    rest.rsplit_once(" b/").map(|(_, b)| b.to_string())
}

/// Parse a `@@ -old_start[,old_lines] +new_start[,new_lines] @@ [section]` header into a `Hunk` (no
/// lines yet). An OMITTED length means 1 (the unified-diff grammar); an unparseable count degrades to 0
/// start / 1 length (tolerant). `header` is the verbatim `@@` line (the ui's hunk-identity, §17).
fn parse_hunk_header(line: &str) -> Hunk {
    // strip the leading `@@`, take the ranges up to the closing `@@`.
    let ranges = line
        .trim_start_matches('@')
        .split("@@")
        .next()
        .unwrap_or("")
        .trim();
    let (mut old_start, mut old_lines, mut new_start, mut new_lines) = (0u32, 1u32, 0u32, 1u32);
    for tok in ranges.split_whitespace() {
        if let Some(old) = tok.strip_prefix('-') {
            (old_start, old_lines) = parse_range(old);
        } else if let Some(new) = tok.strip_prefix('+') {
            (new_start, new_lines) = parse_range(new);
        }
    }
    Hunk {
        header: line.to_string(),
        old_start,
        old_lines,
        new_start,
        new_lines,
        lines: Vec::new(),
    }
}

/// `start[,len]` → `(start, len)`; an omitted `len` is 1 (unified-diff grammar). Unparseable → (0, 1).
fn parse_range(s: &str) -> (u32, u32) {
    match s.split_once(',') {
        Some((start, len)) => (start.parse().unwrap_or(0), len.parse().unwrap_or(1)),
        None => (s.parse().unwrap_or(0), 1),
    }
}

/// (W1-git-stage/095) Slice a ONE-HUNK patch out of a raw unified diff (`git diff … -- <file>`): the
/// FILE PREAMBLE (the `diff --git`/`index`/`---`/`+++` lines before the first `@@`) + the SINGLE hunk
/// whose `@@` header matches the position quad `(old_start, old_lines, new_start, new_lines)`. Returns
/// `None` if NO hunk matches those positions — the §17 read↔mutate guard (the file changed since the UI
/// read it / the displayed hunk is gone → fail-closed, no apply). **Byte-faithful** (`split_inclusive`
/// preserves the exact bytes incl. trailing newlines + the `\ No newline at eof` marker), so the sliced
/// patch re-applies via `git apply` without any DiffLine→patch round-trip fidelity risk. The position
/// quad IS the frozen position-only resource-ref (4.0b-ui1) — the AUDITED unit.
pub fn find_hunk_patch(
    diff: &str,
    old_start: u32,
    old_lines: u32,
    new_start: u32,
    new_lines: u32,
) -> Option<String> {
    // the byte offset of every `@@` (hunk-start) line, preserving exact bytes via split_inclusive.
    let mut hunk_starts: Vec<usize> = Vec::new();
    let mut offset = 0usize;
    for line in diff.split_inclusive('\n') {
        if line.starts_with("@@") {
            hunk_starts.push(offset);
        }
        offset += line.len();
    }
    let &first = hunk_starts.first()?;
    let preamble = &diff[..first];
    for (i, &start) in hunk_starts.iter().enumerate() {
        let end = hunk_starts.get(i + 1).copied().unwrap_or(diff.len());
        let block = &diff[start..end];
        let header_line = block.lines().next().unwrap_or("");
        let h = parse_hunk_header(header_line);
        if h.old_start == old_start
            && h.old_lines == old_lines
            && h.new_start == new_start
            && h.new_lines == new_lines
        {
            // preamble + the EXACT matching hunk block = a valid one-hunk patch for `git apply`.
            return Some(format!("{preamble}{block}"));
        }
    }
    None
}
