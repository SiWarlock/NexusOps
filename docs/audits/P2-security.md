# Phase 2 — Whole-System Security Review (security-reviewer)

**Dispatch:** phase-boundary (Phase 2 exit gate / `/phase-exit`), reviewer policy `invariant`.
**Review surface:** the phase's accumulated branch diff `git diff 0578d60..HEAD` (origin/main `0578d60` @ 2.1b → HEAD `259a094` the 2.4 seal). Covers 2.1c + 2.2 + 2.3 + 2.4 + their session/brief docs. This is the **whole-system trust-boundary security pass** for Phase 2 (the Action Gateway, the single audited mutator).
**Files in scope reviewed:** `daemon/src/gateway/{pipeline,executor,policy,mod,request,idempotency,recovery,precondition,plan,preview}.rs`, `daemon/src/eventstore/{mod,schema,migrations}.rs`, `daemon/src/fault.rs`, `daemon/src/bootstrap.rs`, `daemon/src/ipc/{methods,server,peer}.rs`, `daemon/src/main.rs`, `daemon/src/lib.rs`, `daemon/Cargo.toml`, `shared/src/{actions,events,ipc,catalog}.rs`.
**Note (over-approximation):** at a phase boundary the surface over-approximates to the accumulated track diff for the daemon track. Accepted per the phase-boundary dispatch policy.

**Verdict: CLEAR.** No new security findings. Every touched §15/§17 safety invariant holds across the cross-slice surface; the per-slice CLEAN passes (2.1c ×3, 2.2 ×3, 2.3 ×3, 2.4 ×5) are corroborated end-to-end and no emergent cross-slice issue surfaced.

---

## Invariant pass (cross-slice)

### INV-SEC-1 (no-bypass) — PASS
No state mutation path bypasses the typed-Action → policy → approval → audit-event pipeline. Grep for raw `INSERT/UPDATE/DELETE`/`execute(` outside `gateway/`+`eventstore/`+`projections/`+`locks/`+`runtime/` returns nothing. Every new SQL write (`request::insert/update_*`, `plan::insert`, `approval::*`, `clear_idempotency_key`, `bind_fencing_token`, `update_preview`) is inside a `gateway_txn` on the single write-actor and parameterized. The executor is reachable ONLY via the 3 gated pipeline seams (risk-0 auto-execute, `approve_single`, `approve_plan_cascade`), each routing through `Gateway::execute`. The risk-0 auto-execute path carries a **defense-in-depth re-gate** (`pipeline.rs:183` — refuses to auto-queue any non-Level0 action even if a buggy policy returned `allow`). `submit_action_plan` rejects any uncatalogued step → whole-plan fail-closed `PolicyDenied` (no claim-0-into-approve-all door via the unknown-type path).

### Single mutator — PASS
Production wiring (`main.rs`) uses `CatalogPolicy` + `CatalogExecutor`; all DB writes flow through the write-actor `gateway_txn`. `EventStore::read_conn()` (new, recovery scan) is a documented READ borrow, not a second writer (forbidden #3 governs writes). Plan/approval/recovery all write only via `gtx.tx()`.

### Redaction-before-persist — PASS
Row payloads stay row-redacted: `request::insert` runs the §15 row-redaction gate before INSERT; `record_stale_precondition` regenerates the preview and persists it through `gtx.redact_row(...)` (`pipeline.rs:748`) before `update_preview`. New terminal events carry no raw content — `ActionPartiallySucceeded.reason` is a fixed STRUCTURAL string; `ActionError` variants carry only structural data (`ExecutorError{message}` originates from executor-internal strings, not row/PTY content; the 2.4 stubs produce fixed provenance strings). `gtx.append` enforces the event-side §15 gate uniformly (no terminal-event path skips it).

### Secrets only in the OS keychain — PASS
No secret-shaped literals or secret-to-non-keychain writes introduced. The idempotency key (next item) is the one raw-input-derived value and is a one-way fingerprint, not a secret.

### Idempotency key = one-way SHA-256 of RAW inputs (lead-ruled §15 Option A) — PASS
`idempotency::derive_key` hashes the RAW preimage (`SHA-256`, truncated to 128 bits, `idem_` prefix) — a fingerprint, not the secret (rule #4: one-way, nothing recoverable). It is **catalog-authoritative, never requester-supplied**: `pipeline.rs:105` overwrites `req.idempotency_key` from the catalog `idempotency_formula` BEFORE persist, so a proposer cannot force a collision to suppress a victim's action or evade dedup. `FromInputs` uses canonical (BTreeMap-sorted) JSON for construction-order independence; `NaturalResourceRef` sorts prefixed-ULID ids (non-secret). NUL separator prevents concatenation collisions across distinct tuples. `serde_json::to_string(...).expect(...)` fails LOUD rather than collapsing two distinct actions to one key (a false dedup that would suppress a mutation). Anti-collision-suppression and keychain-refs-only assumptions documented; C/HMAC upgrade trigger (off-machine egress) recorded. Backstopped by the `ux_action_idem` partial UNIQUE index (a racing insert → fail-closed AuditWriteFailed).

### Fail-closed on audit-write — PASS
`execute()` is split into txn-A (queued→executing + ActionStarted, COMMITS) → executor off-actor → txn-B (terminal). A txn-B terminal-event write failure rolls txn-B back → the action STAYS `executing` (never acked succeeded) → L5 reconciles. `side_effect_applied()` is centralized on `ExecutionOutcome` (one place) so a future variant can't silently skip the partial-success path: `false` (every 2.4 stub, and every `Failed`) → clean rollback → Err; `true` (a real applied side effect whose success event is lost) → best-effort txn-C `ActionPartiallySucceeded`; txn-C also failing → return the original `AuditWriteFailed` and stay `executing`. UNIFORM/risk-agnostic. `WHERE status=queued` on txn-A makes a stale slot a typed `NotFound`, never a silent double-execute.

### Fencing tokens mandatory — PASS
The L3 execute-path fence (`pipeline.rs:810-857`) acquires the `resource_mutation` lease, binds the minted token to the row (so L5 re-derives authority), and re-checks the REAL `gw_validate_lease`/`validate_held` (owner + token-match + `expires_at > now`) AFTER lock + BEFORE execute. A live lease held by ANOTHER owner, OR a self-Held with no valid recorded token, OR `validate_held==false` → **record-then-throw**: `record_fencing_conflict` COMMITS the terminal `ActionFailed(FencingConflict)` in its own txn BEFORE the typed `Err(FencingConflict)` propagates (the audit record survives the Err). Mapped to the distinct `IpcErrorCode::FencingConflict` → the NEVER-auto-resolved hard-conflict card (rule #6), kept separate from the re-approvable `precondition_stale`. The self-Held-with-NULL-token branch fails CLOSED (fences) rather than proceeding with a guessed/zero token.

### UDS peer-auth = getpeereid() — PASS
Unchanged and remains FIRST. `serve_connection` calls `authorize_peer(peer_uid, daemon_uid)?` at `server.rs:40` BEFORE any frame is read; the new `submit_action_plan` method routes through the same post-auth `dispatch` (`methods.rs:59`) as every other mutation method. No new entry point bypasses the gate. `peer_uid`/`authorize_peer` fail-closed (a `getpeereid` failure yields no uid → no authorization). Rule #7 holds for the added method.

### Stale-precondition (§6.2 AG-16.4 / §7.2) — PASS
L4 re-check (`pipeline.rs:868`) runs AFTER lock+fencing, BEFORE execute. A `Changed` answer → `record_stale_precondition` (regenerate preview through the §15 gate, COMMIT terminal `ActionFailed(StalePrecondition)`) then `Err(StalePrecondition)` → `IpcErrorCode::PreconditionStale` (re-approvable). The production `NullPreconditionOracle` reports `Unchanged` (no-op until real per-action_type re-reads land Phase 5/7); the seam itself fails closed on a real `Changed`. Never executes a mutation different from what was approved.

### Crash-reconcile (§17 L5) — PASS
`reconcile_orphans` (wired into `cold_start` step 7, AFTER replay/outbox-recovery/quarantine) scans `status IN ('executing','queued')` and drives each terminal in ITS OWN atomic txn (one orphan's write failure leaves it for the next restart — fail-closed). The Q6 dedup-key contract is correct: **`queued`** (never executed → no side effect) → `failed` + `ExecutorError(honest crash msg)` + **CLEAR** idempotency_key (safe to re-submit, avoids the dedup lockout); **`executing`** (side effect MAY have landed, un-reconcilable with stub executors) → `failed` + `UnknownOutcome` (loud audit-integrity record) + **KEEP** idempotency_key (protects against a double-run of a maybe-applied mutation). The `(Queued, Failed)` state-machine edge is the only new driver of that transition; the rollback edges (`Succeeded|PartiallySucceeded → RolledBack|RollbackFailed`) are sinks with no path back to executing/queued (no re-execute backdoor). Reconcile drives the action terminal but does not release the lease (TTL/reaper handles it) — noted, not a finding.

### §14 fault-injection compiled-out + un-armable in release — PASS
`mod fault` (`lib.rs:14`) and all 3 consult sites (`eventstore/mod.rs:595`, `pipeline.rs:784`, `pipeline.rs:875`) are behind `#[cfg(feature = "fault-injection")]`. The feature is enabled ONLY via the self-dev-dependency (`nexusopsd = { path = ".", features = ["fault-injection"] }` in `[dev-dependencies]`); `cargo build`/`cargo run`/`--release` do not pull dev-dependencies, so the module and every hook disappear from production binaries. There is no production fault surface; it cannot be armed. The injected error is a synthetic `SQLITE_IOERR` wrapped identically to a real write error (the fail-closed path it exercises is the real one).

### Not-touched invariants (no diff surface) — N/A
Execution-profile binding (#8), never-scrape-PTY (#9), Brain-proposes-only (#10), Codex rollout hardening (#11): no Phase-2 diff touches these surfaces (profiles, PTY, brainclient, Codex file creation). No regression introduced.

---

## General security pass

- **Input validation (GatewayPort boundary):** `submit_action_plan` parses `ActionPlan` via `serde_json::from_value` with `deny_unknown_fields` end-to-end; a parse failure → `ProtocolError` (no partial trust). Empty plan and `Blocked` mode rejected up-front fail-closed. `PlanAck`/`PlanStepAck`/`ActionError`/`ActionPartiallySucceeded` all `deny_unknown_fields`. PASS.
- **Injection (SQL):** every new query is parameterized (`find_by_idempotency_key`, `scan_orphans`, `clear_idempotency_key`, `update_preview`, `bind_fencing_token`, `plan::insert`, `request::insert`). The `scan_orphans` literal `status IN ('executing','queued')` is a constant, not interpolated. No string-concat-to-SQL. PASS.
- **Reentrancy / TOCTOU:** the approve→execute boundary is two txns by design; the L3 fence + L4 precondition re-check between lock and execute close the approval-to-execution TOCTOU window. Risk reconciled and idempotency key derived from the catalog at submit (not re-read mutably between approval and execution). No lock held across an executor call (executor runs OFF the write-actor). PASS.
- **Unbounded loops / DoS:** the new loops (`approve_plan_cascade`/`deny` over covered steps, `reconcile_orphans` over orphans) iterate bounded DB result sets (plan steps; crash orphans) — not agent/network-controlled unbounded streams. Frame size still bounded by `MAX_FRAME_SIZE` (unchanged). PASS.
- **Integer overflow:** `i64::try_from(lease.fencing_token.0)` and `u64::try_from(...)` are checked; overflow → typed Serialize error / fail-closed fence, no wraparound. SHA truncation is a fixed slice. PASS.
- **Allowance / approval races:** a plan-level approve-all approval cascades only NON-critical steps (critical-4 keeps its own per-step approval — §11.5 safety pin, never cascaded). One approval grant maps only to its tied action(s); no approval reused across distinct unrelated actions. PASS.
- **Information disclosure:** new error strings (`UnsupportedPolicyDecision`, `FencingConflict`, `StalePrecondition`, recovery messages, `ActionPartiallySucceeded.reason`) are structural — they name action_type/risk/phase, never row inputs/secrets/PTY content/paths-outside-project. PASS.
- **Resource exhaustion:** no new unbounded PTY/child/FD/connection allocation; lease TTL bounded (`FENCE_TTL_SECS = 300`). PASS.

## Carry-forward / non-blocking observations (NOT findings)
- Multi-resource fence: L3 fences only the PRIMARY `resource_refs[0]`; `resource_refs[1..]` unfenced. Documented Future-TODO; the MVP catalog is ~single-resource. Revisit when multi-resource mutating actions land (Phase 5/7).
- Crash orphan lease release: reconcile drives the action terminal but relies on the TTL/reaper to free the lease (most crash leases are TTL-expired at restart). Documented; prompt-release is a Future-TODO.
- `sha2` dependency added beyond the brief's list (RustCrypto, audited; `cargo audit` runs at /phase-exit). Flagged at the originating slice's Step-9 per process; recorded here for completeness.

These are explicitly owned/deferred in-code with phase markers; none is a Phase-2 security gap.
