# OQ-INT-SPIKE-6 — libgit2 relative-worktree read re-check + octocrab/token spot-check

| | |
|---|---|
| **MVP task** | 0.3 (Phase 0) |
| **Open question** | `OQ-INT-SPIKE-6` — octocrab merge/GitHub-App ergonomics + libgit2 relative-worktree gap re-check |
| **Spec anchors** | `ARCHITECTURE.md §9`/`§9.1` (git + integrations), ADR-007 (git engine + integrations + credentials) |
| **Status** | ✅ RESOLVED — git2 reads **survive** relative-worktree repos (ADR-007 premise now stale); octocrab + token bootstrap **adequate** |
| **Date** | 2026-06-07 |
| **Gates** | Phase 5 git, Phase 7 GitHub |

---

## 1. Decision (TL;DR)

1. **libgit2 1.9.4 (bundled by `git2` 0.21) CAN fully read an `extensions.relativeWorktrees`
   repo** — every hot read the daemon needs passes (open, statuses, branches, head,
   worktree-list, `find_worktree`, diff). This **contradicts ADR-007's "libgit2 can't do
   `extensions.relativeworktrees` (git ≥2.48; fix unreleased)"** premise (written 2026-06-06)
   — the fix has shipped. ✅ → **a cross-doc flag for the orchestrator** (don't edit the
   locked ADR here; route via `/arch-finalize`).
2. **The dual-git design does NOT change.** Keep `git2` for hot reads + **git CLI for ALL
   mutations** (terminal parity + single Gateway chokepoint, ADR-007 / §15 INV-SEC-1). The
   relative-worktree gap was only *one* justification for the CLI-read fallback; the
   **sparse-checkout misreport is a separate, still-unverified gap** that keeps the
   CLI-read fallback warranted. The finding simply **widens git2's safe read scope**:
   relative-worktree repos no longer force CLI reads (on libgit2 ≥ 1.9.4).
3. **octocrab 0.53 merge + `gh auth token` bootstrap are adequate** (confirmed via the
   octocrab docs) — matches ADR-007; "octocrab ergonomics inadequate" is **not** triggered.

---

## 2. git2 relative-worktree read — empirical result

**Probe:** `daemon/spikes/git2-worktree-check/` (throwaway crate; `git2` 0.21,
`vendored-libgit2` = **libgit2 1.9.4**). **Fixture:** a real relative-worktree repo created
with `git 2.50.1` (≥ 2.48):

```bash
git -C main config worktree.useRelativePaths true
git -C main worktree add --relative-paths ../wt -b feature
# → core.repositoryformatversion=1, extensions.relativeWorktrees=true,
#   worktree .git pointer = "gitdir: ../main/.git/worktrees/wt" (relative)
```

**Result — all reads OK, on both the main repo and the worktree:**

```
# git2 crate 0.21 / bundled libgit2 1.9.4
[1] open the MAIN repo (extensions.relativeWorktrees=true, formatversion=1)
  OK   Repository::open(main)
[2] hot reads the daemon needs on the main repo
  OK   statuses()            OK   branches()            OK   head()
  OK   worktrees() list -> ["wt"]                       OK   find_worktree(wt)
  OK   diff_tree_to_workdir(main)
[3] open the WORKTREE directly + read its status/diff
  OK   Repository::open(worktree)   OK   worktree statuses()
  OK   worktree head()              OK   worktree diff_tree_to_workdir
# VERDICT: libgit2 CAN read a relative-worktrees repo   (exit 0)
```

Crucially, libgit2 did **not** reject the repo over the unknown `extensions.*` at
`repositoryformatversion=1` (the failure mode ADR-007 feared), and it **correctly resolved
the relative gitdir pointers** (opened the worktree and read its diff).

**Re-run:**
```bash
# (toolchain shims now repaired — plain cargo works; see OQ-DATA-SPIKE-3 §9)
cargo run --release --manifest-path daemon/spikes/git2-worktree-check/Cargo.toml -- <main-repo> <worktree>
```

**Scope of the claim / caveats:**
- Verified on **libgit2 1.9.4** only. The conclusion is **version-gated**: pin `git2 ≥ 0.21`
  (libgit2 ≥ 1.9.4) to *get* relative-worktree read support; below that, keep the CLI-read
  fallback for these repos.
- **Sparse-checkout misreporting (the other ADR-007 libgit2 gap) was NOT tested here** and is
  still assumed broken → the CLI-read fallback path stays in the design.
- Mutations are unaffected: still **CLI-only** per ADR-007 / INV-SEC-1 regardless.

---

## 3. octocrab merge + token bootstrap — spot-check

Confirmed against the octocrab docs (no live merge performed — none authorized):

```rust
// gh-token bootstrap (ADR-007 "reuse gh auth token"): `gh auth token` → stdout token
let octо = Octocrab::builder().personal_token(token).build()?;          // ✅ confirmed path
// future GitHub-App flow:
let app  = Octocrab::builder().app(app_id, EncodingKey::from_rsa_pem(pem)?).build()?;  // ✅

// merge a PR (Phase 7 PR-* / review.request_agent_fix downstream):
octо.pulls(owner, repo)
   .merge(pr_number)
   .method(params::pulls::MergeMethod::Squash)   // Squash | Rebase | Merge
   .title("…").message("…")                       // optional commit title/message
   .send().await?;                                // (.sha(expected_head) guard available)

// PR Review Workspace reads also confirmed present:
octо.pulls(owner, repo).get(n) / .get_diff(n) / .list_files(n) / .is_merged(n) / .list_reviews(n)
```

- **`gh auth token` works on this machine** (returns a token; account `SiWarlock`,
  github.com, SSH) — the ADR-007 bootstrap path is live. ✅
- Merge builder exposes method (squash/rebase/merge) + title/message + an optional head-sha
  guard → ergonomics are **adequate** (ADR-007's "inadequate ergonomics" trigger NOT met).
- **HITL boundary:** any *real* merge / write against a live repo needs the user's explicit
  authorization + their GitHub creds; only the API shape + token path were confirmed here.

---

## 4. What ran vs deferred

| Item | Status |
|---|---|
| git2 read on a real relative-worktree repo | ✅ ran — **all reads OK** (libgit2 1.9.4) |
| sparse-checkout misreport re-check | ⛔ not tested (still assumed broken → CLI fallback kept) |
| octocrab merge API shape | ✅ confirmed (docs; no live merge) |
| `gh auth token` bootstrap | ✅ ran — token returned OK |
| live PR merge against a real repo | ⛔ deferred — HITL (needs authorization + creds) |

---

## 5. Flags back to the orchestrator

- **Cross-doc (route via `/arch-finalize`, do not edit LOCKED text here):** ADR-007's
  "libgit2 can't do `extensions.relativeworktrees` … fix unreleased" and the §9 aside
  ("re-verifies whether git2 *reads* even survive on relative-worktree repos; if not, those
  repos fall back to CLI reads") are now **resolved favorably**: git2 reads **do** survive on
  libgit2 1.9.4. Suggested doc updates — (a) ADR-007 "What Would Change This" already names
  this; mark the relative-worktree gap **closed for reads on libgit2 ≥ 1.9.4**; (b) keep the
  CLI-read fallback for sparse-checkout; (c) add a **min `git2`/libgit2 pin** (≥ 0.21 /
  ≥ 1.9.4) to the §16 version-compat matrix as the floor that guarantees this.
- **No design change / no blocker:** dual-git (reads via git2, mutations via CLI) stands.
- **Deferred (HITL):** live PR merge verification (authorization + creds); sparse-checkout
  re-check if we ever want to widen git2 reads further.
