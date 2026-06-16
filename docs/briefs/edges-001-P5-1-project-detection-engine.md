# /tdd brief — project_detection_engine

## Feature
A deterministic **project-detection engine** — git2 read-only repo introspection (`git/detect.rs`) + workflow/cc-crew/plan/Brain file-signal detection (`workflow/detect.rs`) — producing daemon-internal `GitDetection` + `WorkflowDetection` results. This is the pure logic the `project.rescan` Gateway executor will call; **the executor wiring + event emission + registry migration + projector are deferred to `edges-002`** (cross-track-gated — see Dependencies).

## Use case + traceability
- **Task ID:** P5.1 (the detection-engine portion; the registry/event/projector portion = `edges-002`)
- **Architecture sections it implements:** `ARCHITECTURE.md §9` (git2 reads for status/branch/remote/worktree; relative-worktree reads resolved per OQ-INT-SPIKE-6), `§7.2` (Git/FS SoT = the repo, read git2 live; WorkflowInstance SoT = `.scaffolding/manifest.json`; PlanTask SoT = the parsed plan file), `§6.3` (`project.rescan` = risk-0, `ExecutorKind::Project` — context for the deferred `edges-002` wiring).
- **Related context:** the OQ-INT-SPIKE-6 spike (`daemon/spikes/git2-worktree-check/`) proved git2 0.21 reads relative-worktree repos; match its git2 version. Forbidden-pattern **#6** (git2 is read-only; mutations are git-CLI Gateway actions) is the governing safety rule for this slice.

## Acceptance criteria (what "done" means)
**Git detection (`daemon/src/git/detect.rs`):**
- [ ] Detecting a git repo at a path returns `is_git = true` + the resolved repo root.
- [ ] The `origin` remote URL is extracted when present (`Some(url)`), `None` when no remote (feeds P7.1 GitHub linking).
- [ ] Current branch / HEAD is reported; **detached HEAD** is handled (reported as detached, never a panic).
- [ ] Working-tree dirty/clean state is reported (the minimal §7.2 git-state set).
- [ ] A **non-git path** → `is_git = false`, a valid degraded result (no error/panic) — "basic project w/ no git still works."
- [ ] A **missing/nonexistent path** → a typed degraded result, **never** a panic/`unwrap`/`expect` in non-test code.
- [ ] Detection **does not mutate the repo** (forbidden #6 — git2 read APIs only; the repo HEAD/state is unchanged after `detect`).

**Workflow/signal detection (`daemon/src/workflow/detect.rs`):**
- [ ] Workflow-pack signal detected from `.scaffolding/manifest.json` presence (§7.2 WorkflowInstance SoT) — exact marker is a Step-2.5 question.
- [ ] cc-crew signal detected from the `.claude/` scaffolding presence — exact marker is a Step-2.5 question.
- [ ] Plan-file signal detected from a plan-file marker (§7.2 PlanTask SoT names `MVP_TASKS.md`) + its path — exact glob is a Step-2.5 question.
- [ ] Brain signal detected as **presence-only** (full Brain status = Phase 8) — Step-2.5 question.
- [ ] A bare directory → all signals absent, a valid result — "basic project w/ no pack still works."

**General:**
- [ ] `git2` added to `daemon/Cargo.toml` (read-only use); `pub mod git;` + `pub mod workflow;` registered in the crate root.
- [ ] All unit tests pass; `/preflight` clean (fmt + clippy `-D warnings` + check + test).

## Wiring / entry point (Step 7.5)
**`none — wiring lands in `edges-002`.`** The detection engine is a pure library with no production caller in this slice. Its consumer is the `project.rescan` executor arm (`ExecutorKind::Project`), which lives in `gateway/executor.rs` — **sealed Phase-2 daemon-track territory** — and whose detection-result event (a new `EventTypeRegistry` type) is a CONTRACT-bearing, daemon-track-owned addition. Both are cross-track-gated (flagged as a Finding this round). `edges-002` wires the arm + emits the event + adds the `proj_projects`/`proj_repositories` migration + the projector once those land. This slice's reachability is therefore **intentionally deferred** (named), not an oversight.

## Files expected to touch
**New:**
- `daemon/src/git/mod.rs` — `git` module decl (NEW module; `pub mod detect;`)
- `daemon/src/git/detect.rs` — git2 read-only repo introspection → `GitDetection { is_git, repo_root, remote_url, branch, detached, is_dirty }` (final shape is a Step-1 detail)
- `daemon/src/workflow/mod.rs` — `workflow` module decl (NEW module; `pub mod detect;`)
- `daemon/src/workflow/detect.rs` — file-signal detection → `WorkflowDetection { workflow_pack, cc_crew, plan_file: Option<path>, brain }`
- Test file(s): `daemon/tests/detect.rs` (or inline `#[cfg(test)]` units per the impl's Step-1 choice)

**Modified:**
- `daemon/src/lib.rs` (or the crate-root module list) — register `pub mod git;` + `pub mod workflow;`
- `daemon/Cargo.toml` — add `git2` (0.21+, matching the OQ-INT-SPIKE-6 spike; read-only)

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN. **Do NOT touch `gateway/` (sealed), `shared/` (daemon-track contract), or any migration** in this slice.

## RED test outline (Step 2)
Tests (placement per Step-1 — `daemon/tests/detect.rs` or inline units):

1. **`git_detect_repo_root`** — a `tempdir` + `git2::Repository::init` → `is_git=true`, `repo_root` resolves to the repo dir.
   - Asserts: `is_git == true && repo_root == <tempdir canonical>`. Why: §9 git2 read.
2. **`git_detect_origin_remote_url`** — repo with an `origin` remote set → `remote_url == Some(url)`.
   - Asserts: remote URL extracted. Why: §9 GitHub-remote detection (feeds P7.1 linking).
3. **`git_detect_no_remote`** — repo without a remote → `remote_url == None`.
   - Asserts: absent remote is `None`, not an error. Why: edge.
4. **`git_detect_current_branch`** — repo with one commit on `main` → `branch == Some("main")`, `detached == false`.
   - Asserts: branch name read. Why: §7.2 git state.
5. **`git_detect_detached_head`** — detached HEAD → `detached == true`, no panic.
   - Asserts: detached reported, not crashed. Why: edge robustness.
6. **`git_detect_dirty_clean`** — clean tree → `is_dirty=false`; add an untracked/modified file → `is_dirty=true`.
   - Asserts: working-tree state. Why: §7.2 git-state minimal set.
7. **`git_detect_non_git_path`** — a tempdir with no git → `is_git=false`, valid degraded result (no `Err`/panic).
   - Asserts: degraded-but-valid. Why: edge "basic project w/ no git still works."
8. **`git_detect_missing_path`** — a nonexistent path → typed degraded result, never panic/unwrap.
   - Asserts: no panic; a typed result. Why: error bullet (no `unwrap`/`expect` — daemon CLAUDE.md typing posture).
9. **`git_detect_does_not_mutate_repo`** — capture HEAD/state before + after `detect` → unchanged.
   - Asserts: repo state identical post-detect. Why: forbidden #6 (git2 read-only).
10. **`workflow_detect_pack_manifest`** — dir with `.scaffolding/manifest.json` → `workflow_pack == true`.
    - Asserts: pack signal present. Why: §7.2 WorkflowInstance SoT.
11. **`workflow_detect_cc_crew`** — dir with `.claude/` → `cc_crew == true`.
    - Asserts: cc-crew signal present. Why: plan (cc-crew detection).
12. **`workflow_detect_plan_file`** — dir with a plan file → `plan_file == Some(path)`.
    - Asserts: plan signal + path. Why: §7.2 PlanTask SoT.
13. **`workflow_detect_none`** — a bare dir → all signals absent, valid result.
    - Asserts: all-false valid. Why: edge "basic project w/ no pack still works."

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** **none.** `GitDetection` / `WorkflowDetection` are **daemon-internal** types (not a `shared/` contract, not an Appendix-A model).
- **Shared-contract seam model touched?** (the subsystem-boundary list — EventEnvelope / IDs / status-machines / catalog / `EventTypeRegistry`, etc.) **NO.** No envelope / ID / status-machine / catalog / `EventTypeRegistry` change → **no schema-snapshot test required** (confirms this slice is cleanly in-lane: zero `shared/` touch, zero CONTRACT_VERSION implication).
- **Orchestrator doc rows to write hot:** none this slice. (When `edges-002` lands the `projects`/`repositories` registry + the `ProjectRescanned` event, a `daemon/CLAUDE.md` cross-doc row + DATA_MODEL extension follow — daemon-track-coordinated.)

## Things to flag at Step 2.5
1. **Exact signal markers for `workflow/detect.rs`.** What concretely marks each signal? My default votes: (a) workflow pack = `.scaffolding/manifest.json` exists (§7.2 SoT); (b) cc-crew = a `.claude/` dir exists (agents/commands scaffolding); (c) plan file = first match of `IMPLEMENTATION_PLAN.md` | `MVP_TASKS.md` at the project root (§7.2 names `MVP_TASKS.md`); (d) Brain = presence of a Brain marker (`brain/` sibling or a `.brain` config) as a **boolean only** — full Brain status is Phase 8. **Default vote: the above; confirm or correct the exact paths** (you have the live repo to check).
2. **`ProjectDetection` aggregate now, or in `edges-002`?** My default vote: **not now** — ship the two independent detectors (`GitDetection`, `WorkflowDetection`); the executor assembles the aggregate in `edges-002` when it emits the event. Keeps this slice focused + avoids a premature shape. (If a thin aggregate struct now reads cleaner, that's acceptable — flag it.)
3. **git-detection breadth — where's the 5.1/5.2 line?** My default vote: the minimal load-bearing set only — `is_git, repo_root, remote_url, branch (+ detached), is_dirty`. **Worktree-list, diff, and log reads belong to 5.2** (the dual-git read backend). Confirm the boundary so detection doesn't pre-build 5.2.
4. **Git test-fixture strategy.** My default vote: `tempfile::tempdir()` + `git2::Repository::init` + programmatic commits/remotes **in-test** (no shelling to `git`, no committed fixture repos) — hermetic + deterministic. Confirm.

## Dependencies + sequencing
- **Depends on:** nothing blocking — this slice touches neither the Gateway nor `shared/`. Adds the `git2` dep (first daemon use). (The frozen 2.1 Gateway iface is irrelevant here; the detection engine is a pure library.)
- **Blocks:** **`edges-002`** (the `project.rescan` executor arm + the `ProjectRescanned`/registry event + the `proj_projects`/`proj_repositories` M9 migration + the projector feeding `proj_project_activity` + graph). **`edges-002` is itself CROSS-TRACK-GATED** on (i) the daemon track adding the Phase-5 `EventTypeRegistry` event type(s) to `shared/src/events.rs` (CONTRACT bump — daemon-track-owned), and (ii) the executor-arm wiring seam in `gateway/executor.rs` (sealed Phase-2 territory). **Both flagged to the lead as a Finding this round.**

## Estimated commit count
**1–2.** Bundle both detectors (same detection concern, deterministic, **no safety-invariant pin** — forbidden #6 is enforced by a behavior test + review, not a gateway change). Split into a git-detect commit + a workflow-detect commit only if the diff grows large. One logical unit; bisectable either way.

## Lessons-logged candidates anticipated
- **Convention candidate** — "git2 detection is read-only (forbidden #6): pin non-mutation with a before/after state assertion; test git logic via `tempfile`-init'd hermetic repos, never committed fixtures or shelling to `git`."
- **Architecture-doc note candidate** — the concrete signal markers (the exact `.scaffolding`/`.claude`/plan/Brain paths), once confirmed at Step 2.5, are a §9 detection note worth recording.
- **Future TODO — operational** — Brain-status detection is presence-only in 5.1; full Brain status lands Phase 8 (brainclient).

## How to invoke
1. **Read this brief end-to-end** — especially "Things to flag at Step 2.5" (the signal markers + the 5.1/5.2 line need answers before GREEN).
2. **Run `/tdd project_detection_engine`** in the implementer session.
3. **Step 0 (Restate)** — confirm the restatement matches the Feature line (detection engine; wiring deferred to `edges-002`).
4. **Step 1 (Identify files)** — confirm against "Files expected to touch"; do NOT touch `gateway/`, `shared/`, or migrations.
5. **Step 2.5** — send the test-design write-up + the 4 design-question answers; wait for `APPROVED.` before GREEN.
6. **Step 9** — surface anything beyond the anticipated lessons-logged candidates.
