//! OQ-INT-SPIKE-6 — does libgit2 (git2-rs) READ an `extensions.relativeWorktrees`
//! repo? (MVP task 0.3). THROWAWAY probe.
//!
//! Usage: git2-worktree-check <main-repo-path> <worktree-path>
//! Prints OK/ERR for each read op the daemon's hot-read path needs (§9 git).
//! Also reports the bundled libgit2 version so the result is pinned.

use std::path::Path;

fn report<T>(label: &str, r: Result<T, git2::Error>) -> bool {
    match r {
        Ok(_) => {
            println!("  OK   {label}");
            true
        }
        Err(e) => {
            println!(
                "  ERR  {label}  -> [{:?}/{:?}] {}",
                e.code(),
                e.class(),
                e.message()
            );
            false
        }
    }
}

fn main() {
    let (major, minor, patch) = git2::Version::get().libgit2_version();
    println!("# git2 crate 0.21 / bundled libgit2 {major}.{minor}.{patch}");

    let args: Vec<String> = std::env::args().collect();
    let main_path = args.get(1).expect("arg1 = main repo path");
    let wt_path = args.get(2).expect("arg2 = worktree path");

    let mut all_ok = true;

    println!("\n[1] open the MAIN repo (extensions.relativeWorktrees=true, formatversion=1)");
    let repo = match git2::Repository::open(Path::new(main_path)) {
        Ok(r) => {
            println!("  OK   Repository::open(main)");
            Some(r)
        }
        Err(e) => {
            println!(
                "  ERR  Repository::open(main) -> [{:?}/{:?}] {}",
                e.code(),
                e.class(),
                e.message()
            );
            all_ok = false;
            None
        }
    };

    if let Some(repo) = repo.as_ref() {
        println!("[2] hot reads the daemon needs on the main repo");
        all_ok &= report("statuses()", repo.statuses(None).map(|s| s.len()));
        all_ok &= report("branches()", repo.branches(None).map(|b| b.count()));
        all_ok &= report(
            "head()",
            repo.head().map(|h| h.shorthand().map(String::from)),
        );
        match repo.worktrees() {
            Ok(names) => {
                let names_owned: Vec<String> = names
                    .iter()
                    .filter_map(|r| r.ok().flatten().map(|s| s.to_string()))
                    .collect();
                println!("  OK   worktrees() list -> {names_owned:?}");
                for n in &names_owned {
                    all_ok &= report(
                        &format!("find_worktree({n})"),
                        repo.find_worktree(n).map(|_| ()),
                    );
                }
            }
            Err(e) => {
                println!(
                    "  ERR  worktrees() list -> [{:?}/{:?}] {}",
                    e.code(),
                    e.class(),
                    e.message()
                );
                all_ok = false;
            }
        }
        // a real diff (workdir vs HEAD tree) — the diff-open hot path (§18)
        all_ok &= report(
            "diff_tree_to_workdir(main)",
            repo.head()
                .and_then(|h| h.peel_to_tree())
                .and_then(|t| repo.diff_tree_to_workdir(Some(&t), None))
                .map(|d| d.deltas().len()),
        );
    }

    println!("[3] open the WORKTREE directly + read its status/diff");
    match git2::Repository::open(Path::new(wt_path)) {
        Ok(wt) => {
            println!("  OK   Repository::open(worktree)");
            all_ok &= report("worktree statuses()", wt.statuses(None).map(|s| s.len()));
            all_ok &= report(
                "worktree head()",
                wt.head().map(|h| h.shorthand().map(String::from)),
            );
            all_ok &= report(
                "worktree diff_tree_to_workdir",
                wt.head()
                    .and_then(|h| h.peel_to_tree())
                    .and_then(|t| wt.diff_tree_to_workdir(Some(&t), None))
                    .map(|d| d.deltas().len()),
            );
        }
        Err(e) => {
            println!(
                "  ERR  Repository::open(worktree) -> [{:?}/{:?}] {}",
                e.code(),
                e.class(),
                e.message()
            );
            all_ok = false;
        }
    }

    println!(
        "\n# VERDICT: libgit2 {} read a relative-worktrees repo",
        if all_ok { "CAN" } else { "CANNOT fully" }
    );
    std::process::exit(if all_ok { 0 } else { 1 });
}
