# Session 018 — P4.0b-1 CAT-1: session.create/kill executor + §15 #8 ExecutionProfile binding + the 0.5b freeze

- **Date:** 2026-06-13
- **Phase:** Phase 3 (3.5 benchmark) + Phase 4 (P4.0a spine, P4.0b-1 the cat-1 risk-0 session-lifecycle)
- **Predecessor:** [017 — Claude MutationIntercept→Gateway interception](017-2026-06-12-claude-mutation-intercept-gateway.md)
- **Successor:** _(next session — 4.0b-2: the live launch + the INV-SEC-1 interception + the reachable IPC session.create)_

## Why this session existed

A long implementer session spanning three dispatched slices. The headline + the bulk of the
record is **P4.0b-1**, the first **category-1** safety slice of the P4 live-drive loop: catalog the
Gateway `session.create`/`session.kill` actions at **risk-0** (away-ruled audited auto-allow), freeze
the 0.5b `ExecutionProfile` runtime-state machine in `shared/`, bind the §15 #8 ExecutionProfile, and
build the session-create/kill executor — all WITHOUT a live agent (the binding condition).

Earlier in the session (own committed slices, for the record): **P3.5** — the §18 terminal-attach
benchmark (`018479d`); **P4.0a** — the opt-3 session-supervisor spine (L1 `3e9ac02` · L2 `8e0b5cb`
· L3 `822486a`; SessionActor + SessionLauncher seam + SessionSupervisor; LESSON 28 candidate).

## What was built (P4.0b-1 — 3 layers, all cat-1-reviewed)

**L1 `aba52dd` — the 0.5b ExecutionProfile freeze (CONTRACT 0.23.0→0.24.0).** Files modified:
`shared/src/status.rs` (the 10th §5.1 `status_machine!`: 9 values = the §5.1 8 + `credit_exhausted`,
terminal `{Disabled}` only; + the `ExecutionProfileStatus` alias; removed the macro's blanket
`#[allow(unreachable_patterns)]`), `shared/src/events.rs` (`SessionStarted.execution_profile_id:
Option<ExecutionProfileId>`), `shared/src/lib.rs` (version + remove the `EXECUTION_PROFILE_STATUS_HELD`
marker), `shared/src/schema.rs` (bundle), `shared/contracts/schema/*.json` (regen), `shared/tests/contract.rs`
(the freeze snapshot), `daemon/tests/replay.rs` (one literal).

**L2 `d0b7425` — the §6.3 risk-0 catalog + the 5-pin guards (CONTRACT 0.24.0→0.25.0).** Files modified:
`shared/src/catalog.rs` (session.create risk-2→0, NEW session.kill risk-0 + session.profile_change
risk-2; MVP 22→24), `shared/src/lib.rs` (version), the schema regen, `shared/tests/contract.rs`
(count + catalog-risk test), `daemon/src/gateway/policy.rs` (PIN-e requester deny + PIN-d
`RISK0_AUTO_EXECUTE_ALLOWLIST` fail-closed + `risk0_auto_execute_permitted`/`_allowlist` accessors +
the struct-doc update), `daemon/tests/policy.rs` (6 PIN/consistency tests), `daemon/tests/recovery.rs`
(session.create→session.send_message swap).

**L3 `5e8faed` — the SessionExecutor + the in-txn SessionStarted + the cat-1 no-stall bridge.** Files
created: `daemon/src/gateway/session_executor.rs` (the `ExecutorKind::Session` executor — drives the
4.0a supervisor over the NON-LIVE FakeLauncher, emits `SessionStarted{execution_profile_id}`),
`daemon/tests/session_executor.rs` (4 tests). Files modified: `daemon/src/session/mod.rs` (the
SupervisorHandle reshaped **UNBOUNDED + SYNC** — the cat-1 no-stall bridge; dropped the oneshot reply +
the capacity const), `daemon/src/gateway/executor.rs` (`EmittedEvent` + `ExecutionOutcome::Succeeded.
emitted_events`), `daemon/src/gateway/pipeline.rs` (txn-B appends `emitted_events` atomic with
ActionSucceeded), `daemon/src/gateway/request.rs` (`emitted_event_intent` — SessionStarted with
session_id on the envelope), `daemon/src/gateway/mod.rs` (module reg), + `emitted_events: vec![]` in 3
test-fakes (recovery/claude_intercept/executor.rs).

## Decisions made (P4.0b-1, cat-1 — lead/orchestrator-ratified)

- **session.create/kill = risk-0** (away-ruled): the faithful vehicle for "routine, audited, no
  per-launch approval" (risk-1-not-approval-gated was contradictory per LESSON 19). Guarded by 5 pins.
- **PIN-d allowlist gates ALL risk-0 auto-execute** (not just mutations): the policy can't distinguish
  mutation from read-only without a catalog field, and gating all risk-0 is strictly more defensive
  (a new auto-executing type forces a deliberate allowlist edit). Allowlist↔catalog biconditional
  pinned both sweeps. **Safe-direction broadening of the lead's literal "mutations only"** —
  security-confirmed; logged for return-review.
- **ExecutionProfile terminal = `{Disabled}` only** (the §5.1 bold); rate_limited/credit_exhausted are
  recoverable (non-terminal). `credit_exhausted` distinct from `rate_limited` (SDK hard-pool vs soft).
- **In-txn SessionStarted** via `ExecutionOutcome`-additional-events (over a scoped write-handle) —
  atomic with ActionSucceeded in txn-B, fail-closed rollback (PIN a, INV-SEC-1).
- **The sync→async bridge = the UNBOUNDED channel** (the lead's cat-1 add): an `UnboundedSender::send`
  is non-blocking, so it can NEVER stall the single write-actor; the executor runs on the write-actor's
  dedicated `std::thread` off the runtime (security-traced). This dissolved the original blocking_send
  deadlock/multi_thread concern.
- **The binding condition** (no live agent un-intercepted): held BY CONSTRUCTION — non-live launcher,
  SessionExecutor NOT wired in `main.rs`, no IPC session.create method. test 9 greps it.

## Decisions explicitly NOT made (deferred)

- **The live launch + the INV-SEC-1 interception + the reachable IPC session.create = 4.0b-2** (the
  cat-1 atomic live-wiring slice; its own security pass).
- The `session.profile_change` executor BODY (the actual profile swap) — a later slice; PIN-c is the
  approval GATE, pinned now.
- `SessionStarted::EVENT_TYPE` const — deferred to the named **1.6b SessionStarted-literal
  consolidation** (a partial fix now while the 1.2 literals stay un-consolidated would be inconsistent).
- The unbounded supervisor-channel bound re-confirm — for the **4.0b-2** live-driver security pass once a
  real session.create rate exists (a security-reviewer forward note, not a finding).

## TDD compliance

- **L1 + L2 — clean RED-first.** Each RED test (the freeze snapshot; the catalog-risk + the 5 policy
  PIN tests) was written + confirmed-RED before the implementation.
- **L3 — controlled deviation (flagged).** For the complex executor integration (the SessionExecutor +
  the in-txn `ExecutionOutcome`-events mechanism + the unbounded supervisor bridge) the mechanism was
  built before the 4 verifying tests, which then passed GREEN. Justified by the integration complexity
  + that the **design was Step-2.5-reviewed (cat-1, lead-signed-off) + the orchestrator confirmed both
  integration mechanisms**; the tests are meaningful (they fail if the impl is wrong) + the cat-1
  security-reviewer adversarially verified the layer. Not a blind back-fill — a design-reviewed
  implement-then-verify on a complex safety-critical integration.

## Reachability

- **session.create/kill executor:** exercised via `submit_action` in `daemon/tests/session_executor.rs`
  (the NON-LIVE launcher). **Mechanism-built, no live caller** (the 043 pattern) — NOT wired into the
  production Gateway (`main.rs` keeps `CatalogExecutor`). The reachable IPC method + the real launcher =
  the named **4.0b-2** deferral. **(Open follow-up — belongs to 4.0b-2.)**
- **The policy pin-guards (PIN d/e):** on the production `CatalogPolicy` path (reachable; tested).
- **The 0.5b ExecutionProfile freeze:** a `shared/` contract (consumed at the ui-track resume).
- **The SupervisorHandle (unbounded):** main.rs holds it (`_supervisor`, unused) — the 4.0b-2 driver entry.

## Open follow-ups

- **Cross-doc invariant edits (flagged at Step 9; the orchestrator writes hot at `/orchestrate-end`):**
  Appendix A "9 frozen + held"→"10 frozen" + the §5.1 row-201 `credit_exhausted` add + the §6.3 catalog
  rows (session.create/kill risk-0, profile_change risk-2) + the §6.3 risk-0-NARROW + the all-risk-0
  allowlist note + the §15 #8 note + the §9.1/§6.2 AS-BUILT (the SessionExecutor + the in-txn mechanism
  + the unbounded no-stall bridge) + LESSON 29 + the daemon/CLAUDE.md rows. The `docs/layers/01` HELD-const
  sweep. **All cat-1 — flagged for the user's return-review.**
- **🔴 FINDING (pre-existing, routed to the user) — the §5.0 3-way verify (`shared/contracts/verify/run.sh`)
  is RED** on a `datamodel-code-generator` pydantic `MetricQuality` tooling drift, **proven not a
  4.0b-1 regression** (fails identically with the diff stashed; the authoritative test-9 byte-diff +
  the zod path are GREEN). A dedicated tooling-fix slice restores it.
- **Future TODOs:** 4.0b-2 (the live wiring, cat-1) · the `SessionStarted::EVENT_TYPE` const (1.6b
  consolidation) · the unbounded-channel-bound re-confirm at 4.0b-2 · the session.profile_change executor body.

## Security posture

All three cat-1 layers had a security-reviewer pass (every layer, per the cat-1 mandate). **0 security
findings across all three.** L3 explicitly put the two load-bearing properties ON THE RECORD: (A) the
write-actor cannot be stalled (genuinely unbounded, non-blocking send) and (B) the dedicated-`std::thread`-
off-runtime isolation (traced to `runtime/writer.rs`). INV-SEC-1 preserved throughout; the binding
condition held by construction.
