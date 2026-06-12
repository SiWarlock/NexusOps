# /tdd brief — ci_merge_gates (§14 CI workflow + §5.0 contract gates + audit/spec-lint baseline)

> **NON-TDD slice — infrastructure (CI config), not a RED→GREEN behavioral slice.** The Phase-2 `Spec anchors:` line carries `§14 (non-TDD: CI pipeline config; the §14 tiers run in CI)` and `§5.0 (non-TDD: the CI schema-diff/3-way gates ARE the verification)` as **explicit waiver classes**. There is **no RED test outline** — the deliverable is `.github/workflows/ci.yml` + supporting wiring, and "the gates running green in CI" is the coverage. Per `docs/tdd-brief-template.md` "When NOT to use a /tdd brief → Infrastructure work," this is brief-as-design-record + Step-2.5 design review; the impl builds the workflow and verifies each job locally where possible (the contract-gate harnesses already run standalone today).

## Feature
Create the first `.github/workflows/ci.yml` standing up the §14 merge-gate tiers (daemon lint/type/test · ui lint/type/test · the §5.0 contract gates · dependency-audit · spec-lint) so every later phase's merge is gated, plus a nightly job for the heavier runs (the 2.5 perf benchmark + the @1M/N=100 sweep). CI ≡ the local `/preflight` gate, extended with the cross-language contract verification that can't run offline.

## Use case + traceability
- **Task ID:** P2.6
- **Architecture sections it implements:** `ARCHITECTURE.md §14` (the testing-tier merge gates), `§5.0` (the contract source-of-truth gates: schema-diff · 3-way verify · `CONTRACT_VERSION` pin), `§16` (the dependency-audit baseline is part of the supply-chain posture).
- **Related context — folds in the Carry-forward "Wire the §5.0 contract gates into CI" item** (its full checklist): schema-diff (test 9) + 3-way verify (test 8) + the ui TS drift test + `CONTRACT_VERSION`===`x-contract-version` pin + the corepack/pnpm note (`ui/.npmrc verify-deps-before-run=false`; corepack shim flaky → `npm i -g pnpm`).
- **Depends on 2.5** for the nightly perf job (it calls 2.5's `cargo bench --bench event_write` runner) — so 2.5 lands first; 2.6 schedules it.
- **OUT of scope:** the **Codex schema-regen gate** (an OQ-HARN-SPIKE-4 / harness-adapter-contract concern) joins at **P3**, not here.

## The §14 merge-gate tiers to wire (from `ARCHITECTURE.md §14`)
Merge gates (fire on every PR/push): **Unit · Contract-with-fixtures · Daemon-integration-with-fakes · Frontend · Security · Performance(-vs-§18)**. Non-merge: **Live-agent smoke** (nightly) · **Demo e2e** (release gate). For 2.6 the daemon Unit/Contract/Integration/Security tiers all run under `cargo test --workspace` today; the Frontend tier under the ui `vitest`; Performance is the 2.5 benchmark (nightly). 2.6 wires the *runners*, not new tests.

## Acceptance criteria (what "done" means)
- [ ] `.github/workflows/ci.yml` exists and defines a **PR/push-triggered** workflow with these jobs (parallelized where independent):
  - [ ] **daemon**: `cargo fmt --all --check` · `cargo clippy --workspace --all-targets -- -D warnings` · `cargo test --workspace` (mirrors local `/preflight`; this run includes the §5.0 schema-diff **test 9** — the `emit_schema`-vs-checked-in-artifact test in the daemon/shared suite).
  - [ ] **ui**: `pnpm install` · `pnpm typecheck` (`tsc --noEmit`) · `pnpm oxlint` · `pnpm test:run` (`vitest run`) — with the corepack workaround applied (see Q1); this run includes the **ui TS drift test** (`ui/src/contracts/generated.test.ts`).
  - [ ] **contract-3way**: the cross-language **test 8** — `shared/contracts/verify/run.sh` (regenerates the schema via `cargo run --bin emit_schema`, codegens Pydantic via `uvx datamodel-codegen` + Zod via `npx json-schema-to-zod`, asserts the 3 enum value-sets agree + `x-contract-version` present). Needs Rust+Node+Python toolchains + network for the codegen tools (see Q2).
  - [ ] **dep-audit**: `cargo audit` + `(cd ui && pnpm audit --prod)`; gate = no vulnerabilities outside the accepted-risk ignore-list; baseline is `docs/audits/P2-cargo-audit.txt` (currently 0 vulns) (see Q3).
  - [ ] **spec-lint**: wires `scripts/spec-lint.sh` into CI (subcommand + scope per Q4).
- [ ] A **nightly** schedule (`schedule: cron`) workflow (or a `nightly` job gated on the schedule trigger) runs: the **2.5 perf benchmark** (`cargo bench --bench event_write`, asserting the §18 N=20 guards) + the documented heavier sweep (N=100 ceiling / @1M) — NON-merge-gating (a regression here is a Finding, surfaced at `/phase-exit`/nightly, not a per-PR block).
- [ ] **Toolchain setup pinned**: Rust stable (clippy+rustfmt components), Node 22 + pnpm (via the corepack workaround), Python 3 + the codegen tools — in the workflow's setup steps (see Q5).
- [ ] **CI ≡ local gate parity**: the daemon + ui jobs run the *same* commands `/preflight` runs locally (no drift between what CI checks and what the implementer checks pre-commit).
- [ ] **`/preflight` clean** locally after the change (the workflow yaml is not executed by `cargo test`/`vitest`; adding it must not perturb the 247-test daemon suite or the ui suite). The workflow is **lint-validated** (valid YAML; `actionlint` if available, else a documented manual review).
- [ ] No secrets committed; the workflow uses `GITHUB_TOKEN`/repo-scoped perms only (the push remote is `git@github.com:SiWarlock/NexusOps.git`).

## Wiring / entry point (Step 7.5)
The "production entry point" is **GitHub Actions itself** — the workflow triggers on `push` + `pull_request` (merge gates) and `schedule` (nightly). The contract-gate job's entry is the existing standalone `shared/contracts/verify/run.sh`; the daemon/ui jobs' entries are the existing `cargo`/`pnpm` commands. **This slice wires runners that already exist** (the 3-way harness, the emit_schema bin, the gen-contracts drift test, spec-lint, cargo/pnpm audit) into a CI workflow — it does not author new test logic. The nightly perf job's entry is **2.5's `cargo bench --bench event_write`** runner (so 2.5 must have landed).

## Files expected to touch
**New:**
- `.github/workflows/ci.yml` — the merge-gate workflow (daemon · ui · contract-3way · dep-audit · spec-lint jobs).
- Possibly `.github/workflows/nightly.yml` (or a `schedule`-gated job in ci.yml) — the perf + heavy-sweep run (Q6: one file vs two).
- Possibly `docs/audits/.gitkeep` or a baseline README documenting the audit-baseline semantics (the `docs/audits/` dir now holds `P2-*.md` + `P2-cargo-audit.txt`).

**Modified:**
- `ui/.npmrc` — already has `verify-deps-before-run=false`; confirm CI honors it (or replicate in the runner). Possibly `ui/package.json` if a `prettier`/`format:check` script is added (Q7 — only if prettier is adopted).
- (Reference, do NOT re-author) the existing gate harnesses: `shared/contracts/verify/{run.sh,verify.py}`, `shared/src/bin/emit_schema.rs`, `ui/scripts/gen-contracts.mjs`, `ui/src/contracts/generated.test.ts`, `scripts/spec-lint.sh`.

If wiring the 3-way verify in CI needs a change to `run.sh`/`verify.py` (e.g. a `--ci` flag, or pre-caching the codegen tools), **flag at Step 2.5** before editing — those are frozen-since-0.5 contract harnesses (touch minimally; a CI-only wrapper is preferred over editing them).

## CI-design review surface (Step 2.5 — the "test outline" equivalent)
The implementer sends, instead of a RED outline:
1. **The job graph** — the jobs, their triggers (push/PR vs schedule), what runs in parallel vs serial, and which are merge-gating vs advisory/nightly.
2. **The exact command list per job** — proving CI ≡ local `/preflight` for daemon + ui, plus the contract-gate + audit + spec-lint commands.
3. **The toolchain setup** — the `setup-*` actions + versions + the corepack/pnpm + codegen-tool install steps.
4. **The answers to Q1–Q7** below.
5. **Coverage map** — each acceptance bullet → the job/step that satisfies it (or a `deferred-because:` note, e.g. the Codex gate → P3).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none.
- **Orchestrator doc rows to write hot (Step 9 routing):** the **Carry-forward "Wire the §5.0 contract gates into CI" item is RESOLVED** by this slice → orchestrator marks it done/removes it at `/orchestrate-end`. A possible `ARCHITECTURE.md §14` note that the tiers are now CI-realized (orchestrator-written). No Appendix A change.
- **§2.5-seam (shared-contract) model touched?** No — CI config, no `shared/` model, no schema-snapshot test.

## Things to flag at Step 2.5
1. **Corepack/pnpm in CI.** The repo's `ui/.npmrc` has `verify-deps-before-run=false` (corepack shim flaky locally). My default vote: in CI use **`npm i -g pnpm@<pinned>`** (not corepack) + rely on the committed `.npmrc`; pin the pnpm major to the lockfile's. Confirm the lockfile's pnpm version.
2. **The 3-way verify "needs network."** `verify.py` shells `uvx datamodel-codegen` + `npx json-schema-to-zod`, which download on first run. GitHub runners HAVE network, so this is fine in CI (it's the LOCAL offline case that can't run it). My default vote: **run contract-3way as a real merge-gating job** with Rust+Node+Python setup + the codegen tools installed in a setup step (cache them via `actions/cache` keyed on the tool versions). Do NOT mark it `continue-on-error`. Raise if you'd rather pre-vendor the generators.
3. **Dep-audit gate semantics.** `cargo audit` exits non-zero on ANY vuln. Baseline is currently 0 vulns. My default vote: **gate = `cargo audit` exits 0** (and `pnpm audit --prod` clean); when a vuln must be ACCEPTED, record it in Decisions-tabled + add `cargo audit --ignore RUSTSEC-xxxx` (the `docs/audits/` baseline is the human-readable record of accepted advisories). Don't build a text-diff-against-snapshot parser — the ignore-list IS the baseline mechanism.
4. **spec-lint in CI — which subcommand + scope?** `spec-lint.sh tests <phase>` needs a phase arg (awkward as a per-PR gate); `brief` is per-brief; `reqs` is warn-only. My default vote: a **`spec-lint` job that runs `scripts/spec-lint.sh tests <active-phase>`** parameterized via a workflow var (default the current phase), **non-blocking/advisory for now** (it's a phase-exit-time check, not a per-commit invariant), promoted to blocking when phase-exit is CI-automated. Alternatively run `reqs` (warn-only) per-PR. Your call — note that spec-lint was just fixed to exclude `target/` (fast now) + parse backtick-wrapped waivers.
5. **Toolchain pinning.** No `rust-toolchain.toml`, no `.nvmrc`/`engines` today. My default vote: pin in the workflow (`dtolnay/rust-toolchain@stable` + `actions/setup-node@v4` node-version 22 + `actions/setup-python@v5` 3.x) rather than adding repo-root toolchain files in a CI slice — keep the slice's blast radius in `.github/`. Add `engines`/`rust-toolchain.toml` only if you judge it cleaner (flag it).
6. **One workflow file or two (ci.yml + nightly.yml)?** My default vote: **two files** — `ci.yml` (push/PR merge gates) + `nightly.yml` (`schedule` cron → the 2.5 perf bench + heavy sweep + live-agent smoke placeholder). Cleaner separation of merge-gating vs nightly; the nightly references 2.5's runner.
7. **Prettier.** The tracker's 2.6 line lists `prettier --check` as a ui gate, but **prettier is NOT in `ui/package.json`** (the ui formats via oxlint). My default vote: **drop `prettier --check` from the gate** — don't introduce a new formatter in a CI slice; oxlint is the project's lint/format authority (CLAUDE.md ui stack: "Lint = oxlint"). If the user wants prettier, that's a separate tooling decision. Flag this as the one place the brief diverges from the tracker's literal command list (clerical — the tracker pre-dated the oxlint-only posture).

## Dependencies + sequencing
- **Depends on:** the existing §5.0 gate harnesses (✅ 0.5: `verify/run.sh`, `emit_schema`, `gen-contracts.mjs`, the drift test), `cargo audit` baseline (✅ established this gate), spec-lint (✅ fixed this gate), and **2.5's perf runner** (for the nightly job — land 2.5 first).
- **Blocks:** the `/phase-exit 2` checkbox-row completion (2.6 is the last in-phase task) + every later phase's merge gate consumes this workflow.

## Estimated commit count
**1–3.** All CI config in `.github/` (one area, non-safety, non-contract). Reasonable to land as one commit (the whole workflow), or split: (a) the ci.yml merge gates; (b) the contract-3way + audit + spec-lint jobs; (c) the nightly.yml perf/heavy job. Bundle if it stays grokkable; split if the contract-gate toolchain setup gets large. Your call at Step 2.5.

## Lessons-logged candidates anticipated
- **Convention candidate** — "CI ≡ local `/preflight`: the daemon/ui merge-gate jobs run the exact commands `/preflight` runs, so a green local gate predicts a green CI gate." (Likely a `daemon/LESSONS.md`/runbook note.)
- **Future TODO — operational** — the **Codex schema-regen gate** (P3, OQ-HARN-SPIKE-4) joins this workflow later; leave a commented placeholder. Also the **live-agent smoke** (nightly, real CLIs) is a placeholder until P3 adapters exist.
- **Architecture-doc note candidate** — §14 tiers are now CI-realized; record the mapping (tier → CI job) so future phases know where to add their tier's tests.
- **Runbook candidate** — `docs/runbooks/ci.md`: how to run each gate locally, the corepack workaround, how to accept a `cargo audit` advisory (the `--ignore` + Decisions-tabled record).

## How to invoke
1. **Read this brief end-to-end** — note this is a **non-TDD infra slice** (no RED outline; the green CI run is the coverage).
2. **Run `/tdd ci_merge_gates`** — at Step 2 / 2.5 send the **CI-design review surface** (job graph + per-job commands + toolchain setup + Q1–Q7 answers) instead of a RED test outline.
3. **Step 2.5** — wait for orchestrator sign-off before building; the 3-way-verify network handling (Q2) + the prettier divergence (Q7) + spec-lint scope (Q4) are the ones I most want to align on.
4. **Step 9** — surface which gates you verified actually run (locally or via a test push to a branch), and flag the resolved Carry-forward CI item.
