//! P4.7 — the ONE GitHub `(owner, repo)` resolution authority (the confused-deputy closure).
//!
//! Every github-write executor (`merge_pr`/`submit_review`/`create_pr`/`sync_reviews`) + the D7
//! `get_pr_diff` read path resolve the GitHub target from the **audited** identity (the resource_ref
//! `repo_id` / the envelope `project_id`) — NEVER from attacker-controllable `inputs["owner"/"repo"]`.
//! After this, the executed target == the audited identity by construction (INV-SEC-1 / §15).
//!
//! Two resolution paths share [`parse_owner_repo`]:
//! - [`resolve_owner_repo_by_pr`] — `(repo_id, pr_number) → proj_pull_request → project_id →
//!   proj_repository.remote_url → owner/repo` (the D7/LESSON-59 path, moved here verbatim). Used by the
//!   3 PR-scoped sites (merge/submit/sync — the resource_ref repo_id is validated against the PR row).
//! - [`resolve_owner_repo_by_project`] — `project_id → proj_repository.remote_url → owner/repo` (the
//!   repo-only path). Used by `create_pr` (no PR row exists yet; `proj_repository` is keyed by
//!   `project_id`, which is the create_pr audited identity — orchestrator-verified, ipc/methods.rs:313).
//!
//! **`repo_id` is not first-class in `proj_repository`** (keyed by `project_id`) — a direct
//! `repo_id → owner/repo` projection link is a future-TODO (removes the PR-row detour; especially for the
//! repo-only path). The `FakeRepoResolver` mirrors the in-src `FakeGithubWriteClient` test-fake precedent.

use std::path::{Path, PathBuf};

use crate::eventstore::open_read_only;

/// A resolution failure — fail-closed (the caller refuses the action / returns NotFound).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RepoResolveError {
    /// no PR row / no project_id / no remote_url / an unparseable URL — the target cannot be resolved.
    NotFound,
    /// a read/DB error opening or querying the projection.
    Internal,
}

/// The read seam the `GithubExecutor` resolves a github-write target against (§15 #8 / the confused-deputy
/// closure). Injected so resolution is deterministically unit-testable (a fake in tests; the sqlite-backed
/// [`DbRepoResolver`] in production over a read-only WAL conn).
pub trait RepoOwnerRepoResolver: Send + Sync {
    /// PR-scoped: `(repo_id, pr_number) → (owner, repo)` (merge/submit/sync — validates repo_id vs the PR row).
    fn by_pr(&self, repo_id: &str, pr_number: u64) -> Result<(String, String), RepoResolveError>;
    /// repo-scoped: `project_id → (owner, repo)` (create_pr — no PR row; project_id is the audited identity).
    fn by_project(&self, project_id: &str) -> Result<(String, String), RepoResolveError>;
}

/// Resolve `(repo_id, pr_number)` → the GitHub `(owner, repo)` over a READ-ONLY WAL conn: the EXACT PR
/// row (`proj_pull_request WHERE repo_id=? AND pr_number=?`) → its `project_id` → `proj_repository`'s
/// `remote_url` → parsed owner/repo. Any missing link or an unparseable URL → [`RepoResolveError::NotFound`].
/// The D7 path (moved verbatim from `ipc/methods.rs`); `get_pr_diff` calls it through the ipc wrapper.
pub fn resolve_owner_repo_by_pr(
    db_path: &Path,
    repo_id: &str,
    pr_number: u64,
) -> Result<(String, String), RepoResolveError> {
    use rusqlite::OptionalExtension as _;
    // ONE read-only conn for BOTH queries (the PR-row lookup + the repository lookup) — the executor runs
    // on the write-actor's std::thread, so a single read conn per resolution is the leaner posture.
    let conn = open_read_only(db_path).map_err(|_| RepoResolveError::Internal)?;
    // pr_number is `u64` on the wire but stored as a SQLite INTEGER (i64). A value above i64::MAX can't
    // be a real GitHub PR number → it matches no row → NotFound (fail-fast, never a silent wrap).
    let pr_number = i64::try_from(pr_number).map_err(|_| RepoResolveError::NotFound)?;
    let project_id: Option<String> = conn
        .query_row(
            "SELECT project_id FROM proj_pull_request WHERE repo_id = ?1 AND pr_number = ?2",
            rusqlite::params![repo_id, pr_number],
            |r| r.get(0),
        )
        .optional()
        .map_err(|_| RepoResolveError::Internal)?;
    let Some(project_id) = project_id else {
        return Err(RepoResolveError::NotFound);
    };
    owner_repo_for_project(&conn, &project_id)
}

/// Resolve `project_id` → the GitHub `(owner, repo)` over a READ-ONLY WAL conn: `proj_repository`'s
/// `remote_url` → parsed owner/repo. The repo-only path (no PR row); a missing row / unparseable URL →
/// [`RepoResolveError::NotFound`]. The create_pr site.
pub fn resolve_owner_repo_by_project(
    db_path: &Path,
    project_id: &str,
) -> Result<(String, String), RepoResolveError> {
    let conn = open_read_only(db_path).map_err(|_| RepoResolveError::Internal)?;
    owner_repo_for_project(&conn, project_id)
}

/// `project_id → proj_repository.remote_url → (owner, repo)` over an ALREADY-OPEN conn (the shared inner
/// of both resolution paths — one conn, no re-open).
fn owner_repo_for_project(
    conn: &rusqlite::Connection,
    project_id: &str,
) -> Result<(String, String), RepoResolveError> {
    use rusqlite::OptionalExtension as _;
    let remote_url: Option<String> = conn
        .query_row(
            "SELECT remote_url FROM proj_repository WHERE project_id = ?1",
            [project_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|_| RepoResolveError::Internal)?;
    let Some(remote_url) = remote_url else {
        return Err(RepoResolveError::NotFound);
    };
    parse_owner_repo(&remote_url).ok_or(RepoResolveError::NotFound)
}

/// Parse a GitHub `(owner, repo)` from a git `remote_url` — `https://[user:tok@]github.com/owner/repo[.git]`
/// (the `user:token@` userinfo STRIPPED, LESSON §44) OR scp-style `git@github.com:owner/repo[.git]`. The
/// `.git` suffix + any trailing path are dropped. Returns `None` for an unrecognizable URL (→ NotFound).
pub fn parse_owner_repo(remote_url: &str) -> Option<(String, String)> {
    let path = if let Some(after_scheme) = remote_url.split_once("://").map(|(_, r)| r) {
        // https://[userinfo@]host/owner/repo… → strip the userinfo, then drop the host segment.
        let host_and_path = after_scheme
            .split_once('@')
            .map_or(after_scheme, |(_, r)| r);
        host_and_path.split_once('/').map(|(_, p)| p)?.to_string()
    } else {
        // scp-style git@host:owner/repo… → the part after the first ':'.
        remote_url.split_once(':').map(|(_, p)| p)?.to_string()
    };
    let (owner, rest) = path.trim_end_matches('/').split_once('/')?;
    // repo = the first path segment after the owner, sans the `.git` suffix.
    let repo = rest.split('/').next().unwrap_or(rest);
    let repo = repo.strip_suffix(".git").unwrap_or(repo);
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some((owner.to_string(), repo.to_string()))
}

/// The production [`RepoOwnerRepoResolver`]: resolves over a fresh read-only WAL conn at `db_path` (the
/// executor runs on the write-actor; a 2nd READ conn is single-writer-safe — forbidden #3 governs WRITES).
pub struct DbRepoResolver {
    db_path: PathBuf,
}

impl DbRepoResolver {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }
}

impl RepoOwnerRepoResolver for DbRepoResolver {
    fn by_pr(&self, repo_id: &str, pr_number: u64) -> Result<(String, String), RepoResolveError> {
        resolve_owner_repo_by_pr(&self.db_path, repo_id, pr_number)
    }
    fn by_project(&self, project_id: &str) -> Result<(String, String), RepoResolveError> {
        resolve_owner_repo_by_project(&self.db_path, project_id)
    }
}

/// A canned [`RepoOwnerRepoResolver`] for tests (the `FakeGithubWriteClient` in-src-fake precedent):
/// every resolution returns the same canned `Ok((owner, repo))` or `Err`. The behavior tests inject
/// [`FakeRepoResolver::ok`] (resolve / ignore-divergent) or [`FakeRepoResolver::not_found`] (fail-closed).
pub struct FakeRepoResolver {
    result: Result<(String, String), RepoResolveError>,
}

impl FakeRepoResolver {
    /// Resolve every call to `(owner, repo)`.
    pub fn ok(owner: &str, repo: &str) -> Self {
        Self {
            result: Ok((owner.to_string(), repo.to_string())),
        }
    }
    /// Fail every call with `NotFound` (the fail-closed path).
    pub fn not_found() -> Self {
        Self {
            result: Err(RepoResolveError::NotFound),
        }
    }
}

impl RepoOwnerRepoResolver for FakeRepoResolver {
    fn by_pr(&self, _repo_id: &str, _pr_number: u64) -> Result<(String, String), RepoResolveError> {
        self.result.clone()
    }
    fn by_project(&self, _project_id: &str) -> Result<(String, String), RepoResolveError> {
        self.result.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::parse_owner_repo;

    #[test]
    fn parse_owner_repo_handles_url_forms() {
        // spec(D7 / LESSON 44): https (with/without `.git`, a port) + scp-style → (owner, repo); a
        // `user:token@` userinfo is STRIPPED before the parse (defense-in-depth — the canonical strip is
        // upstream at the ProjectExecutor emit-source); an unrecognizable / owner-only URL → None (→ NotFound).
        // Moved here verbatim with `parse_owner_repo` at the P4.7 extraction (was ipc/methods.rs).
        let ok = |o: &str, r: &str| Some((o.to_string(), r.to_string()));
        assert_eq!(
            parse_owner_repo("https://github.com/acme/widget.git"),
            ok("acme", "widget")
        );
        assert_eq!(
            parse_owner_repo("https://github.com/acme/widget"),
            ok("acme", "widget")
        );
        assert_eq!(
            parse_owner_repo("git@github.com:acme/widget.git"),
            ok("acme", "widget")
        );
        // a credential in the userinfo is stripped, never reaching the parsed owner/repo (LESSON 44).
        assert_eq!(
            parse_owner_repo("https://user:ghp_TOKEN@github.com/acme/widget.git"),
            ok("acme", "widget")
        );
        assert_eq!(
            parse_owner_repo("https://ghp_TOKEN@github.com/acme/widget"),
            ok("acme", "widget")
        );
        // a host:port is dropped as the host segment.
        assert_eq!(
            parse_owner_repo("https://github.example.com:8080/acme/widget"),
            ok("acme", "widget")
        );
        // unrecognizable / incomplete → None (→ NotFound).
        assert_eq!(parse_owner_repo("not a url"), None);
        assert_eq!(parse_owner_repo("https://github.com/justowner"), None);
    }
}
