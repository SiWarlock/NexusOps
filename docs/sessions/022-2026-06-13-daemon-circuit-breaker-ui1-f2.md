# Session 022 — 2026-06-13 — daemon: audit-backbone breaker · ui-① freeze · F2 permit-class

**Implementer (daemon track).** Three slices, RED-first TDD, each its own commit; round sealed by the
orchestrator at `dc497b0`. Preflight at close: fmt+clippy(-D)+check clean, **398 tests pass / 0 fail**,
tree clean. Push USER-GATED (never pushed). Closed at the HARD-STOP cycle (canonical ctx 89%) — clean
sealed boundary, nothing in flight.

## Slices

### 4.0b-2c — audit-backbone circuit-breaker (`bf0ad74`; brief 054; CAT-1-adjacent §15/§17)
The §15/§17 fail-secure layer: the single audited mutator does not operate when it cannot audit. A
daemon-wide circuit-breaker over the 2.4 `AuditWriteFailed` pipeline — per-action faults already deny +
raise the C2 durable alarm; this declares SYSTEMIC (N=5 consecutive OR a clearly-unrecoverable class:
disk-full/DB-corruption/IO) → **RULED-B quiesce-and-refuse**: a latched gate denies every mutation
WITHOUT an audit-write while reads/subscribe/terminals stay live; never auto-resets (restart-only).
- **Cat-1 fork Q1 → (B) quiesce-and-refuse** (lead-ruled, a realization of the user's fail-stop posture).
- Seam = the **write-actor** (`run_actor`, the single Gateway-driver chokepoint) — gate+feed there;
  the feed resets ONLY on a proven `Ok` audit commit (other errors are no-ops, so denial-spam can't
  mask a concurrent systemic fault); every mutation path (submit/plan/approve/deny + intercept) gated.
- Files: NEW `gateway/circuit_breaker.rs` + `tests/circuit_breaker.rs`; MOD `integrity.rs` (systemic
  `IntegrityKind`), `gateway/mod.rs` (`AuditBackboneDown`), `harness/claude/intercept.rs`,
  `runtime/writer.rs`, `main.rs`, `ipc/methods.rs`, tests `claude_intercept.rs`/`session_executor.rs`.
- +12 tests. security-reviewer CLEAR. NO CONTRACT bump (daemon-internal `IntegrityKind`).

### ui-① / 4.0b-ui1 — per-hunk git actions + get_diff RPC (`9848513`; brief 052; CONTRACT 0.28.0)
The cross-track ui-6.3e contract freeze (USER-ruled). 3 `git.*` catalog types (`stage_hunk`/
`unstage_hunk` risk-2; `discard_hunk` risk-3 + NON-standing-grantable + `preview_class=Diff`) + the
`standing_grant_eligible` catalog field (the §6.2 non-standing-grant floor, generalizing the risk-4
approve-all exclusion to `risk==Level4 OR !standing_grant_eligible`, both disjuncts kept + a drift
invariant) + the hunk-structured `get_diff` git2-live read RPC + `Hunk`/`DiffResult`/`DiffLine`/
`DiffLineKind`/`GetDiffParams` + `IpcErrorCode::NotFound`. git executor bodies = stubs (Phase 5).
- Two pre-GREEN findings flagged + ruled YES: the FIRST **git2 daemon dep** (read-only `read_diff`) +
  the `NotFound` §6.4 code (9→10). Q-b ruled **id-based** `get_diff(worktree_id,file)` → resolve via
  `proj_worktree.path` (NotFound until P5). resource_type=File; the hunk resource_ref-id encoding
  (`{wt}\x1f{file}\x1f{old_start},{old_lines},{new_start},{new_lines}`) handed to the orch for the
  durable §6.3/Appendix-A freeze.
- Files: NEW `daemon/src/git/mod.rs` + `tests/get_diff.rs`; MOD `shared/src/{catalog,ipc,lib,schema}.rs`
  + `shared/contracts/schema/*` (regen) + `shared/tests/contract.rs` + `gateway/pipeline.rs` (cascade
  gen) + `ipc/{methods,mod}.rs` + `daemon/Cargo.toml`+`Cargo.lock` (git2).
- +8 tests; 3-way verify 38==38==38 @ 0.28.0. security-reviewer CLEAR. **ui-track-sealed `26c87a3`** (with 4.0b-2c).

### 4.0b-2-F2 — intercept-wait permit-class split (`4a7572a`; brief 055; §6.4/§10 fail-safe)
Availability hardening for real concurrent multi-agent supervision. The live intercept handler held its
accept-pool permit for the full 5-min approval-wait → enough concurrent waits starved the UI approve/
deny (circular starvation). Fix (**Q1 = A reserved sub-bound**): a 2nd `Semaphore(wait_cap = MAX −
RESERVED = 48 of 64)`; a parked wait holds BOTH permits, so RESERVED=16 general slots always stay free;
wait-class exhaustion → fail-closed Deny WITHOUT entering the wait (never a bypass).
- Factored into a PURE `intercept_verdict_with_wait_class` seam + `InterceptWaitClass` (unit-testable
  without a concurrent harness); the live handler delegates with the real wait class (/wired-confirmed:
  wait_cap=48 in production).
- Files: NEW `runtime/wait_class.rs` + `tests/intercept_wait.rs`; MOD `runtime/{mod,listener}.rs`,
  `ipc/{methods,server,mod}.rs`, `main.rs`, `tests/ipc.rs`.
- +6 tests. security-reviewer CLEAR. NO CONTRACT bump (daemon-internal).

## Audit
- **TDD:** every slice RED-first (compile-fail for the right reason), GREEN minimal, full suite green at
  each Step-7 + close. Step-2.5 surfaced to the orch each slice (cat-1 forks routed to lead→user).
- **Cross-doc:** all model/contract/§-note changes flagged at Step-9; the orchestrator hot-wrote them
  (§6.3/§6.1/§6.4/§15/§17 + Appendix A + daemon/CLAUDE.md + LESSONS + CONTRACT 0.28.0). No implementer
  edits to orchestrator-territory docs.
- **Reachability (7.5):** each slice confirmed reachable from a production entry (the write-actor gate;
  the IPC dispatch for get_diff; the live intercept handler for the wait class).

## Carry-forward (orchestrator-owned, noted at Step-9)
- record-then-deny on the timeout + capacity-Deny paths (the orphan `awaiting_approval` audit-completeness
  gap — shared with the existing timeout path; NOT a safety violation) · F2 test #4 path-parametrize.
- ui-① Future-TODOs: the git executor BODIES (Phase 5) · a binary-diff signal (`DiffResult.is_binary`) ·
  the "\ No newline at eof" marker · git2 `cargo audit` @ /phase-exit 4.
- The resource_ref-id encoding → the orch freezes durably in §6.3/Appendix-A (+ a Phase-5 conformance test).

**Next (suggested): 4.0c** (the live telemetry pump + sink-bind — the 044 P4 deferral).
