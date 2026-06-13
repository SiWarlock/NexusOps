# /tdd brief — executor_registration_seam_and_test_support_feature

## Feature
Turn the daemon's `CatalogExecutor` from a unit-struct uniform-stub into a **per-namespace registration registry** (`register(kind, handler)` → dispatch to the registered handler else today's stub, incremental), and introduce the **`test-support` cargo-feature mechanism**. Behavior-preserving (R1a registers NO handler in production). The cross-track **edges-R1** unblock — and the production dispatch home the daemon's own 4.0b-1 `SessionExecutor` (and 4.0b-2) needs. **Daemon-internal: no `shared/` change, no CONTRACT bump.** Non-cat-1.

## Use case + traceability
- **Task ID:** P4.0b-R1a
- **Architecture sections it implements:** `ARCHITECTURE.md §6.2`/`§6.3` (the Action Gateway executor dispatch — both in Phase 4's Spec-anchor set).
- **Related context:**
  - **edges-R1 routing packet + spec** (read-only, cross-worktree): `../NexusOps-edges/docs/planning/edges-R1-routing-packet.md` Part 1 + Part 4, and `edges-R1-wiring-seam-and-event-specs.md` Part 1. **The daemon owns the final shape** (consumer-driven proposal). The 3 daemon-owned design choices are RULED (below).
  - **Seam-delta confirmed (this orchestrator, 2026-06-13):** the edges spec's base (`a40ac00`) predates the daemon Phase-3/4 seals. On current main: `CatalogExecutor` is STILL a unit struct returning a uniform stub for every `ExecutorKind` (no per-kind dispatch) — `gateway/executor.rs:159`, wired at `main.rs:72`. The daemon's own 4.0b-1 `SessionExecutor` is a STANDALONE `ActionExecutor` dispatched nowhere (tests-only). **So Part 1 is NOT delivered; this seam serves both edges AND the daemon's SessionExecutor** (4.0b-2 registers it here).
  - **Lead rulings (away-authority, 2026-06-12/13):** (1) **async = keep the frozen SYNC trait + `block_on`** on the dedicated std::thread (no `async fn execute`; github/linear hold a `Handle` at P7.1) — **no trait change**. (3) **registration = incremental `register()` mutation**. (Granularity choice #2 is an R1b concern.)
  - **LESSONS:** §20 (the executor seam = framework, not side effect), §21 (the `fault-injection` cargo-feature idiom mirrored by `test-support`), §26 (the `Adjudication` INV-SEC-1 guard, pinned by `claude_intercept.rs` #13).

## Acceptance criteria (what "done" means)
- [ ] `CatalogExecutor::new()` returns an empty registry; `CatalogExecutor::register(&mut self, kind: ExecutorKind, handler: Arc<dyn ActionExecutor>)` inserts a per-namespace handler.
- [ ] `execute(req)` dispatches to the registered handler for the resolved `entry.executor`; an **unregistered** kind falls back to **today's structured stub** (byte-for-byte the same `detail` string + `side_effect_applied:false` + empty `emitted_events`) — behavior-preserving.
- [ ] **INV-SEC-1 preserved:** an `Adjudication` action is fail-closed-refused **BEFORE** the handler dispatch — a handler registered for `ExecutorKind::Adjudication` is **never** called; `execute` returns `Failed("adjudication-only … refused")` (the existing `claude_intercept.rs` #13 guarantee survives the refactor).
- [ ] The §6.3 `requires_resource_refs` precondition still fails-closed (`ExecError::MissingResourceRef` → `Failed`) for all kinds, registered or not.
- [ ] `main.rs` builds `CatalogExecutor::new()` with **NO handlers registered** → production behavior is identical to today (every action stubs); the cat-1 binding condition (no live executor wired) holds. The first real registration is `SessionExecutor` at 4.0b-2 (+ edges' Project/Git/Github/Linear at P5/P7).
- [ ] `rollback(req)` delegates to the registered handler if present, else the existing fail-closed default `Failed`.
- [ ] `[features] test-support = []` added to `daemon/Cargo.toml` + enabled on the self-dev-dependency (`features = ["fault-injection", "test-support"]`, the LESSON 21 idiom). `cargo build --release` succeeds and `cargo test` passes. **Gating `FakeHarness` is OUT of scope here** (see Step-2.5 #4 — deferred to 4.0b-2).
- [ ] **No `shared/` change, no `CONTRACT_VERSION` bump** (the `ActionExecutor`/`CatalogExecutor` seam is daemon-internal — LESSON 20).
- [ ] `/preflight` clean (incl. clippy `-D warnings` — add `Default` for `CatalogExecutor` to satisfy `new_without_default`).

## Wiring / entry point (Step 7.5)
`daemon/src/main.rs:72` builds the gateway's executor — becomes `Box::new(CatalogExecutor::new())` (no handlers in R1a). The dispatch path (`execute` → `handlers.get(kind)` → handler-or-stub) is reachable in production; its **handler branch** is exercised only by tests until the first `register(...)` lands (`SessionExecutor` at 4.0b-2). This is the **"mechanism built, first real caller next slice"** pattern (the 4.0b-1 precedent) — state it; it is not unreachable code, it is the seam 4.0b-2 + edges wire into.

## Files expected to touch
**Modified:**
- `daemon/src/gateway/executor.rs` — `CatalogExecutor` unit-struct → `{ handlers: HashMap<ExecutorKind, Arc<dyn ActionExecutor>> }` + `new()`/`register()`/`Default`; `execute`/`rollback` delegate (handler-else-stub/default); the `resolve` precondition + the `Adjudication` guard preserved BEFORE dispatch.
- `daemon/src/main.rs` — `CatalogExecutor` → `CatalogExecutor::new()` (no registrations).
- `daemon/tests/executor.rs` — the dispatch + Adjudication-guard + precondition tests (RED outline below).
- `daemon/Cargo.toml` — the `test-support` feature stanza + self-dev-dep enable.

Grep every `CatalogExecutor` construction site (`main.rs`, any test) and update to `::new()`. If implementation needs files beyond this list — **flag at Step 2.5**.

## RED test outline (Step 2)
Tests in `daemon/tests/executor.rs` (a recording/spy `ActionExecutor` test double — records whether its `execute`/`rollback` was called):

1. **`test_register_then_execute_dispatches_to_handler`** — register a recording handler for a non-Adjudication kind; `execute(req of that kind)` → the handler's `execute` ran (and the stub did NOT).
   - Asserts: registered-kind dispatch. Why: §6.3 — the seam's core behavior.
2. **`test_unregistered_kind_falls_back_to_stub`** — empty registry; `execute(req)` → the structured stub (`"would execute … via the {ns} adapter …"`, `side_effect_applied:false`).
   - Asserts: behavior-preserving fallback. Why: §6.3 — today's behavior unchanged for unregistered kinds.
3. **`test_incremental_registration_only_registered_kind_live`** — register a handler for kind A; `execute(req of kind B)` → stub (B not live).
   - Asserts: incremental registration. Why: §6.3 — a namespace goes live exactly when its handler registers.
4. **`test_adjudication_refused_before_dispatch_even_if_registered`** — register a recording handler for `ExecutorKind::Adjudication`; `execute(an adjudication-type req)` → `Failed` AND the handler was **never** called.
   - Asserts: the INV-SEC-1 fail-closed guard fires before dispatch. Why: §15 / LESSON 26 — the load-bearing safety pin (the agent runs the tool, never the daemon).
5. **`test_requires_resource_refs_precondition_survives`** — `execute(req that requires resource_refs, carrying none)` → `Failed(MissingResourceRef)`, with and without a handler registered.
   - Asserts: the catalog precondition is uniform + survives the refactor. Why: §6.3.
6. **`test_rollback_delegates_to_handler_else_default`** — a handler whose `rollback` returns `Succeeded` → `CatalogExecutor::rollback` returns it; unregistered → the fail-closed default `Failed`.
   - Asserts: rollback delegation. Why: §6.2 rollback seam.

**Acceptance-by-build (not a unit test):** `cargo build --release` succeeds (the `test-support` feature compiles; nothing it gates breaks release — it gates nothing yet) AND `cargo test` passes (the self-dev-dep enables it).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none. The `ActionExecutor`/`CatalogExecutor` seam is daemon-internal (LESSON 20) — **no `shared/` surface, no `CONTRACT_VERSION` bump, no schema snapshot.**
- **Orchestrator doc rows to write hot (Step 9 routing):** a §6.3 **executor-dispatch AS-BUILT** prose note (the registry shape + the incremental register-or-stub model + the Adjudication-guard-before-dispatch) — orch writes at the round. Possibly a convention lesson (below).
- **Shared-contract (cross-area dependency-seam) model touched?** No.
- **Reviewer policy:** **`security-reviewer` = YES** (the `invariant` policy — the slice relocates the INV-SEC-1 `Adjudication` guard into the new dispatch structure; the pass confirms no agent-mutation action can reach a handler and the seam introduces no bypass). `code-quality-reviewer` = every-slice.

## Things to flag at Step 2.5
1. **Delegate `validate`/`preview` to registered handlers, or only `execute` + `rollback`?** My default vote: **only `execute` + `rollback` delegate; `validate` stays the catalog `requires_resource_refs` precondition and `preview` stays the catalog-driven `generate_preview` render.** Rationale: the precondition is catalog-authoritative + applies uniformly; `preview` already renders from the catalog; no registered handler exists yet to carry a custom `validate`/`preview`; P5/P7 refine handler-delegation when a real handler needs it. Minimal, behavior-preserving. Flag if you'd rather delegate all four now.
2. **Handler type — `Arc<dyn ActionExecutor>` vs `Box`?** My default vote: **`Arc`** (the edges spec's shape; a handler may be shared; the trait is `Send + Sync`). The gateway still holds the `CatalogExecutor` as `Box<dyn ActionExecutor>` — Arc-handlers inside are fine.
3. **Adjudication guard — execute-time only, or also refuse in `register()`?** My default vote: **keep the explicit execute-time `if entry.executor == Adjudication { Failed }` BEFORE `handlers.get` (defense-in-depth — matches the #13 pin and protects even a mis-registration); optionally ALSO `debug_assert!`/refuse in `register()`** as belt-and-suspenders. Either way test #4 stays the load-bearing assertion.
4. **`test-support` scope — confirm FakeHarness gating defers.** My default vote: **introduce the feature mechanism only; do NOT gate `FakeHarness` in R1a.** `PtyLauncher` (production, `session/launcher.rs:137`) constructs a `FakeHarness` placeholder until the real ClaudeAdapter lands at 4.0b, so `FakeHarness` is not yet test-only and gating it would break the release compile. The feature exists now so edges gates its two `integrations/` fakes on resume; FakeHarness/FakePty/FakeLauncher gating rides 4.0b-2 (a Carry-forward note). Flag if you want a different split.

## Dependencies + sequencing
- **Depends on:** 4.0b-T (✅, `981de9d` — the clean non-cat-1 boundary).
- **Blocks:** 4.0b-2 (registers `SessionExecutor` via this seam → leaner cat-1 wiring) · R1b (the event types get emitters) · the edges-track resume (R1-on-main gate — with R1b).

## Estimated commit count
**2.** (1) the registration seam (`executor.rs` + `main.rs` + `daemon/tests/executor.rs`); (2) the `test-support` feature (`daemon/Cargo.toml`). Two independent surfaces (gateway refactor vs build-config), each bisectable; neither is a safety pin (the seam PRESERVES the INV-SEC-1 guard, doesn't change it). Keep them as two commits.

## Lessons-logged candidates anticipated
- **Convention candidate** — "the executor registry: incremental `register()`-or-stub dispatch; the `Adjudication` INV-SEC-1 guard stays BEFORE dispatch (a registered handler is never reached for an adjudication action); daemon-internal seam (no contract)." (May refine LESSON 20.)
- **Future TODO — operational** — gate `FakeHarness`/`FakePty`/`FakeLauncher` behind `test-support` at 4.0b-2 (once `PtyLauncher`'s placeholder → the real ClaudeAdapter makes them test-only). Carry-forward, `last-consumer-slice: 4.0b-2`.
- **Architecture-doc note candidate** — the §6.3 executor-dispatch AS-BUILT (the registry shape).

## How to invoke
1. **Read this brief end-to-end** — the seam-delta context + the lead rulings + the 4 Step-2.5 questions (esp. #4, the FakeHarness deferral).
2. **Run `/tdd executor_registration_seam_and_test_support_feature`**.
3. **Step 0 (Restate)** — confirm against the Feature line.
4. **Step 1 (Identify files)** — confirm + grep all `CatalogExecutor` construction sites.
5. **Step 2.5** — answer the 4 questions (or take defaults). Test #4 (the Adjudication guard) is the load-bearing safety assertion — don't drop it. Dispatch `security-reviewer` at Step 8 (invariant policy).
6. **Step 9** — surface the §6.3 AS-BUILT note + confirm the FakeHarness-gating Carry-forward marker.
