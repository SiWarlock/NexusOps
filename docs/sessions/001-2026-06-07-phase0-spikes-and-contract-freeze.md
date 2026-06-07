# Session 001 — Phase 0 spikes + 0.5 contract freeze

| | |
|---|---|
| **Date** | 2026-06-07 |
| **Phase** | Phase 0 (pre-build spikes & contract freeze) |
| **Track / role** | `daemon` / daemon-implementer |
| **Predecessor** | — (first implementer session) |
| **Successor** | _(TBD — Phase 1.1, after the out-of-band `/arch-finalize` re-validation)_ |
| **Commits** | `06f9576` (0.5 contract freeze) + this close-out's spike/harness/session-doc commit(s) |

---

## Why this session existed

Fresh project bootstrap. Phase 0 had to (a) resolve the build-gating spikes (so dependent
phases bind to measured reality, not guesses) and (b) freeze the cross-language shared
contracts — **the serial neck** every downstream track (daemon-core, ui, edges) binds against.
Per `docs/briefs/001-P0-runnable-spikes.md` (spikes) then `docs/briefs/002-P0-5-contract-freeze.md`
(the 0.5 freeze, Option A).

---

## What was built

### Phase 0 spikes (research/decision — not `/tdd`)

**Files created — decision docs:**
- `docs/spikes/OQ-DATA-SPIKE-3.md` — SQLite single-writer load test result + recommended §18 budgets.
- `docs/spikes/OQ-HARN-SPIKE-4.md` — Codex app-server schema-pin (codex absent → deferred + CI-gate scaffold).
- `docs/spikes/OQ-INT-SPIKE-6.md` — libgit2 relative-worktree re-check (now reads OK) + octocrab/token spot-check.
- `docs/spikes/OQ-HARN-SPIKE-7.md` — Claude supervision coverage map, #27203 confirmation, SDK-vs-PTY criterion (no pick), drain HITL checklist.
- `docs/spikes/OQ-PLAT-SPIKE-1.md` — macOS sidecar notarization turnkey HITL checklist + loopback-HTTP fallback tree.

**Files created — scaffolding / throwaway harnesses:**
- `daemon/spikes/sqlite-loadtest/` — throwaway rusqlite/WAL load harness (own `[workspace]`; `target/` gitignored).
- `daemon/spikes/git2-worktree-check/` — throwaway git2 relative-worktree read probe.
- `docs/spikes/codex-schema/snapshot.sh` — Codex schema capture + CI diff-gate scaffolding.
- `docs/spikes/claude-supervision/can_use_tool_probe.mjs` — `can_use_tool` coverage probe (run ≥6/15 w/ SDK).
- `docs/spikes/notarization/brain-sidecar.entitlements` + `verify-signing.sh` — notarization turnkey assets.

**Key measured/confirmed results:**
- **0.4:** single-writer holds at N=20 — intent→committed p95 = **5.35 ms** fresh / **8.44 ms** @1M events (budget 100 ms); reader p95 ≤ 0.38 ms; ceiling not reached through N=100.
- **0.3:** **libgit2 1.9.4 CAN read `extensions.relativeWorktrees` repos** (contradicts ADR-007 "fix unreleased"); octocrab merge + `gh auth token` bootstrap adequate; codex CLI absent → schema capture deferred (HITL).
- **0.1:** #27203 **confirmed still present** on Claude Code 2.1.168 (closed as won't-fix) → background subagents stay forbidden; SDK-vs-PTY **criterion + both branches** stated, **no pick** (cat-4 guardrail); drain measurement is HITL ≥2026-06-15.
- **0.2:** turnkey notarization checklist authored; run is HITL (Apple creds).

### Environment repair (toolchain)
- `~/.cargo/bin` `cargo`/`rustc`/clippy/rustfmt proxies were broken dangling symlinks (→ non-existent `/Users/nozzins`). `rustup default stable` alone did NOT fix it (only recreates *missing* proxies). **Fix (user-authorized):** repointed all 13 proxies to the local `rustup` (`ln -sf rustup ~/.cargo/bin/<proxy>`). Verified: plain `cargo`/`rustc` 1.93.0 + clippy + rustfmt + a real build work. This was the Phase-1 build blocker.

### 0.5 — shared contract freeze (`/tdd` slice, committed `06f9576`)
**Files created (all under `shared/`):** `Cargo.toml`, `src/{lib,status,actor,ids,objects,schema}.rs`,
`src/bin/emit_schema.rs`, `tests/contract.rs`, `contracts/schema/nexusops-contract.schema.json`,
`contracts/verify/{verify.py,run.sh}`, `.gitignore`.
- `nexusops-shared` Rust authority crate (Option A, §5.0): **9 status machines** (§5.1), **22 prefixed-ULID ID newtypes** (§5.2), **10-value actor enum** (§7.1/R-2), **4 desktop objects** (§5.3).
- `schemars` → versioned, CI-diff-gated JSON Schema; generated TS-Zod + Python-Pydantic consumers; self-contained **3-way value-set equality harness** (Rust↔schema↔Zod↔Pydantic).
- **ExecutionProfile HELD for 0.5b** (cat-4 SDK-vs-PTY) — deliberate marker + test, not silently absent.
- Reject-unknown / fail-closed end-to-end (§15): closed serde enums; `parse()` rejects wrong-prefix + malformed ULID; prefix map total, unique, non-substring.

### Files modified — NOT by me (orchestrator-owned; ride the orchestrator round commit)
`ARCHITECTURE.md`, `MVP_TASKS.md`, `daemon/CLAUDE.md`, `daemon/LESSONS.md`,
`docs/planning/OPEN_QUESTIONS.md`, `docs/briefs/` — the orchestrator wrote the Appendix A /
cross-doc-invariant rows + LESSONS hot this round. Left unstaged by me.

---

## Decisions made
- **0.5 status-machine value sets pulled from ARCHITECTURE §5.1 (LOCKED, R-4..R-9), not DATA_MODEL §4** (ROUGH DRAFT, stale: `ready_for_team_mode`, pre-R-5 Approval, no ActionRequest/AgentTeam). Rationale: §5.1 is the reconciliation; §4 predates it.
- **10 new ID prefixes defined + ratified** (orchestrator TWEAK de-collided `art_`→`artf_`, `prj_`→`eprj_`). Rationale: §5.2's "single id_kind const" is created here; readability (log/audit/Brain-citation) demanded de-homographing.
- **Full 3-way cross-language verify done in 0.5** (not staged to 0.5c). Rationale: node/npx + python/uv available; `pnpm` broken but unneeded.
- **Toolchain repaired by repointing proxies** (not reinstall). Rationale: minimal, reversible, existing-breakage repair; user-authorized.

## Decisions explicitly NOT made (deferred)
- **SDK-vs-PTY primary** (cat-4) — criterion + both branches recorded; decided ≥6/15 with drain data, carried by orchestrator to lead/user.
- **ExecutionProfile runtime-state enum** — held for 0.5b.
- **Codex schema bundle capture** — deferred (codex CLI absent; HITL).
- **macOS notarization run** — deferred (Apple creds; HITL).

## TDD compliance
- **Clean.** The spikes (0.1–0.4) are research/decision + throwaway measurement code — exempt from test-first per the brief + `CLAUDE.md` TDD posture.
- **0.5 followed `/tdd` strictly:** tests written first → confirmed RED (missing types, right reason) → Step-2.5 review (APPROVED w/ TWEAK) → GREEN → refactor (none needed) → full suite → Step-8 clippy + dual reviewers → Step-9 routing → Step-10 commit. Newtype-coverage tests added during GREEN were RED-first (referenced not-yet-existing newtypes). **No violations.**

## Reachability
- **0.5 contract:** serial neck. Reachable now via the `emit_schema` bin (build step → checked-in schema artifact) + the CI gates (test 8 cross-language harness, test 9 schema-diff). Type consumers = Phase 1 (1.1 event envelope: actor/IdKind/event_id) + ui track (generated Zod) — **belongs-to-a-phase**, not silently unreachable.
- **Spike harnesses:** run-on-demand / HITL by design (not production-wired) — `daemon/spikes/*` via `cargo run`, `verify.py`/`snapshot.sh` via their runners.

## Open follow-ups
- **0.5b** — re-freeze `ExecutionProfile` runtime-state enum once cat-4 SDK-vs-PTY resolves (≥2026-06-15). Orchestrator may also need to reconcile any contracts `/arch-finalize` moves back into `shared/`.
- **Cross-doc reconciles → Phase-0-exit `/arch-finalize`:** DATA_MODEL §4 (stale machines) vs §5.1; EM §7 `remote_device`→`remote_client`; DATA_MODEL §6.4 `prj_`→`eprj_`; §5.0 `[PROVISIONAL]` → ratify Option A.
- **CI wiring** — wire test 8 (verify harness) + test 9 (schema diff) + the Codex schema gate into `.github/workflows/` (none exist yet).
- **HITL (user) ≥6/15 + creds:** Claude credit-pool drain + coverage probe; Codex install + schema capture; macOS notarization run.
- **Deferred low (code-quality):** narrow the `#[allow(unreachable_patterns)]` in `shared/src/status.rs` (orchestrator tracking as op-TODO).
- **Env:** if a teammate hits the old broken-shim error on a fresh checkout, the workaround is in `OQ-DATA-SPIKE-3.md §7`/§9.

## How to use what was built
- Regenerate the contract schema: `cargo run --manifest-path shared/Cargo.toml --bin emit_schema`.
- Run the 3-way cross-language verify: `bash shared/contracts/verify/run.sh`.
- Re-run the SQLite load sweep: see `OQ-DATA-SPIKE-3.md §7`.
