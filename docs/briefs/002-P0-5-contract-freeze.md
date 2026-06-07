# /tdd brief — shared_contract_freeze (0.5)

> **STATUS: DISPATCHED 2026-06-07.** SoT format **LOCKED = Option A** by the owner (Rust `shared` crate authority → first-class `schemars` JSON-Schema artifact → generated TS Zod + Python Pydantic consumers; see `ARCHITECTURE.md §5.0`). ③ toolchain fix authorized + relayed (run `rustup default stable` before GREEN). All `⏸ PENDING ①` sections below are now finalized against A.

## Feature
Freeze the canonical NexusOps shared contracts — the status-machine enums, the 22 shared IDs + prefixed-ULID format, the actor enum, and the desktop-addendum objects — as versioned constants in `shared/` that the Rust daemon, the TS UI, and the Python Brain all read as a single source of truth. This is **the serial neck**: it is the interface every downstream track (daemon-core, ui, edges) binds against.

## Use case + traceability
- **Task ID:** 0.5 (Phase 0 — contract freeze; `OQ-DATA-SPIKE-5`)
- **Architecture sections it implements:** `ARCHITECTURE.md §5.1` (9 status state machines), `§5.2` (22 shared IDs + ULID format), `§5.3` (desktop-addendum objects), `§7.1` (actor_type enum, R-2), Appendix A (the cross-doc-invariant home for all of these).
- **Related context:** `docs/planning/DATA_MODEL.md` (full DDL + the R-2 actor enum + EM §8 source_type — pull exact value lists from here; do not hand-transcribe from prose), `docs/briefs/001-P0-runnable-spikes.md` (the spikes that gated this), the cat-4 SDK-vs-PTY decision (Decisions tabled) which is **why** ExecutionProfile is excluded.

## Scope — FREEZE NOW (lead-confirmed ②)
**Freeze (driver-agnostic surface):**
1. **9 status state machines** (`§5.1`), value sets exactly as locked, terminal states marked:
   - **Session** (17 — the normalized vocabulary the §9.1 adapter maps INTO; driver-agnostic), **Task** (R-8 superset, 17), **Worktree** (git-axis + overlay), **PullRequest** (11, GitHub-authoritative cache), **WorkflowInstance** (R-7, 12), **ProjectBrain** (10), **Approval** (R-5, 10), **ActionRequest** (R-5, 15), **AgentTeam** (R-6, 9).
2. **22 shared IDs** (`§5.2`) + the **prefixed-ULID format** for platform-minted IDs (`sess_`, `evt_`, `wt_`, `act_`, …); external IDs keep native values. Include the harness→session_id rule as a doc note (Claude settable 1:1; Codex minted `sess_<ULID>` + `harness_session_map` keyed on `(cwd, thread_id)`).
3. **Actor enum** (`§7.1`, R-2 — 10 values incl. `remote_client`). **Pull the exact 10 from `DATA_MODEL.md`.**
4. **4 desktop-addendum objects** (`§5.3`): **LocalRunner** + **EventProjection** (MVP-live), **Device** + **RemoteClient** (dormant `[DEFERRED]` iOS scaffolding — freeze the type/identity, not behavior).

**HOLD — do NOT freeze (guardrail 1 / cat-4):**
- **`ExecutionProfile` state machine** (the 10th `§5.1` machine). Its runtime states (`rate_limited`/`auth_expired` + a possible SDK-credit-exhaustion value) are the one surface the cat-4 SDK-vs-PTY + ≥6/15 credit-pool drain could reshape. **Re-frozen in a follow-up 0.5b** once cat-4 resolves. Leave a clearly-commented placeholder so its absence is intentional, not forgotten.

**OUT OF SCOPE (later phases, do not build here):** transition-legality validation / the degraded-marker logic (R-9 — Phase 1 projections/workflow), the event-type registry payload schemas (`§7.1` `EventTypeRegistry` — Phase 1, task 1.1), the GatewayPort method schema (`§6.1` — task 1.5). 0.5 freezes **value sets + serialization + cross-language agreement only.**

## Acceptance criteria (what "done" means)
- [ ] Every value of all 9 in-scope state machines is present as a constant, with terminal states distinguished, and **serializes to the exact `TEXT status` string** the architecture lists (snake_case as written).
- [ ] All 22 shared IDs are represented as distinct ID-kind constants; platform-minted kinds carry their ULID prefix; the prefix→kind mapping is total and unique.
- [ ] The actor enum has exactly the R-2 value set from `DATA_MODEL.md` (incl. `remote_client`).
- [ ] The 4 desktop objects' identities/kinds are defined; Device/RemoteClient marked deferred.
- [ ] **`ExecutionProfile` is explicitly held** (commented placeholder; not silently missing).
- [ ] **Cross-language agreement test passes: Rust, TS, and Python read the identical value sets** (the §0.5 integration test) — mechanics per ① below.
- [ ] Unknown / unlisted value is **rejected** (not silently accepted) at the parse boundary in each language.
- [ ] `/preflight` clean (Rust side; TS side per `ui/` once it exists — at minimum the Rust + shared crates).
- [ ] Cross-doc invariant: Appendix A rows authored by the orchestrator atomic with this round (see below).

## Files expected to touch (Option A)
**New — Rust authority (`shared/` crate):**
- `shared/Cargo.toml` — `nexusops-shared` crate (`serde`, `schemars`, `ulid`).
- `shared/src/lib.rs` — module re-exports + **`CONTRACT_VERSION`** const.
- `shared/src/status.rs` — the 9 in-scope status-machine enums; `#[derive(Serialize, Deserialize, JsonSchema)]` + `#[serde(rename_all = "snake_case")]`; terminal-state metadata.
- `shared/src/ids.rs` — the 22 ID **newtypes** + the ULID-prefix→kind map (one newtype per kind; `ulid` crate per Step-2.5 #3).
- `shared/src/actor.rs` — actor enum (R-2, 10 values incl. `remote_client`; pull exact set from `DATA_MODEL.md`).
- `shared/src/objects.rs` — the 4 desktop objects (LocalRunner + EventProjection live; Device + RemoteClient deferred markers).
- `shared/src/bin/emit_schema.rs` (or an xtask) — emits JSON Schema from the Rust authority.
- `shared/tests/contract.rs` — the Rust-side contract tests (RED outline below).

**New — published artifact + generated consumers (checked-in):**
- `shared/contracts/schema/*.json` — **first-class, versioned, generated** JSON Schema (the neutral interchange artifact; the Python Brain + any external consumer bind to THIS).
- `shared/contracts/ts/` — generated **Zod** consumer (`json-schema-to-zod`) — the UI's parse-don't-trust validators.
- `shared/contracts/python/` — generated **Pydantic** consumer (`datamodel-code-generator`).
- `shared/contracts/verify/` — the **self-contained 3-way equality harness** (Rust value set ↔ published schema ↔ generated Zod ↔ generated Pydantic) — does NOT depend on `ui/` or `brain/` being built.

**New — CI wiring:**
- the **schema-diff gate** (regenerate from Rust, fail on diff vs the checked-in `schema/*.json` — same pattern as `OQ-HARN-SPIKE-4`'s Codex gate) + the verify harness.

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN.

## RED test outline (Step 2)
Tests 1–7 in `shared/tests/contract.rs` (Rust authority); test 8 in `shared/contracts/verify/`:

1. **`test_every_state_machine_value_present_and_serializes`** — each of the 9 machines' values exists + round-trips to its exact snake_case `TEXT` string. Why: `§5.1` (binding enums; stored as `TEXT status`).
2. **`test_terminal_states_marked`** — terminal states (bold in `§5.1`) are flagged terminal. Why: `§5.1` ("each declares terminal states").
3. **`test_all_22_ids_present_with_prefixes`** — 22 ID kinds present; platform-minted kinds carry the right ULID prefix; prefix→kind is total + unique. Why: `§5.2`.
4. **`test_actor_enum_matches_R2`** — actor enum == the 10 R-2 values from `DATA_MODEL.md` (incl. `remote_client`). Why: `§7.1` / R-2.
5. **`test_desktop_objects_defined_and_deferred_marked`** — LocalRunner/EventProjection live; Device/RemoteClient deferred. Why: `§5.3`.
6. **`test_execution_profile_held_not_frozen`** — ExecutionProfile is intentionally absent/placeholdered (asserts the hold is deliberate). Why: guardrail 1 / cat-4.
7. **`test_unknown_value_rejected`** — an unlisted status/ID-kind/actor value fails to parse in each language. Why: §0.5 edge ("unknown value rejected").
8. **`test_cross_language_equality`** (integration, `shared/contracts/verify/`) — the Rust authority's value sets, the published `schema/*.json`, the generated Zod, and the generated Pydantic all expose the **identical** value sets (+ `CONTRACT_VERSION` agrees). Why: §0.5 integration ("Rust + TS + Python read the same constants"); self-contained (no `ui/`/`brain/` dependency).
9. **`test_schema_artifact_matches_rust`** (CI diff-gate) — regenerating the schema from the Rust authority yields **no diff** vs the checked-in `schema/*.json`. Why: §5.0 (drift impossible-by-CI; same pattern as the Codex schema gate).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** NEW frozen contract set — this is the canonical cross-doc-invariant landing.
- **Orchestrator doc rows to write hot (Step 9 routing):** Appendix A rows for **(i)** the 9 state machines, **(ii)** the 22 shared IDs + ULID format, **(iii)** the actor enum, **(iv)** the desktop objects — plus the matching rows in the `daemon/CLAUDE.md` cross-doc-invariants table. Note ExecutionProfile as "deferred to 0.5b" in Appendix A so the gap is visible. **Implementer does NOT edit Appendix A / `daemon/CLAUDE.md` / `ARCHITECTURE.md`** — flag at Step 9; orchestrator writes hot.

## Things to flag at Step 2.5
1. **SoT format — LOCKED = Option A** (no longer open; `ARCHITECTURE.md §5.0`). Confirm your file plan matches `## Files` above. The non-negotiable A invariants: Rust owns the types (newtypes for IDs, serde-closed enums), the JSON Schema is checked-in + versioned + CI-diff-gated, TS/Python are *generated* from the schema (not hand-written).
2. **TS/Python toolchain availability.** A's 3-way verify needs Node/pnpm + Python in-repo (the Rust toolchain was found broken — see ③). **If either is genuinely unavailable**, do NOT silently skip the consumer half: flag it. Fallback = land the **non-negotiable core** (Rust authority + published `schema/*.json` + CI diff-gate + Rust tests) in 0.5, and stage the generated TS/Python consumers + the 3-way `test_cross_language_equality` as **0.5c** once the toolchain is set up. My default vote: **attempt the full 3-way in 0.5**; only stage to 0.5c on a real toolchain blocker (a finding, not a convenience cut).
3. **ULID minting library (Rust side).** Options: `ulid` crate vs a hand-rolled prefixed wrapper. My default vote: **`ulid` crate + a thin newtype-per-kind wrapper** (newtypes enforce ID-kind at the type level per the daemon typing posture). Confirm.
4. **Enum naming across languages.** The wire/`TEXT` value is snake_case (locked); the in-language identifier case differs (Rust `PascalCase` variants, TS union of string literals, Python `Enum`). My default vote: **wire value is the contract (snake_case); each language uses its idiomatic identifier but serializes to the exact snake_case string** — test #1 pins this. Confirm.
5. **`Worktree` machine's two-axis shape** (git-axis + overlay via precedence fn). 0.5 freezes the **value sets of both axes**; the precedence fn is Phase-1 derived-projection logic (out of scope). Confirm you're freezing values only, not the precedence fn.

## Dependencies + sequencing
- **Depends on:** ① SoT format = **Option A (LOCKED)** `§5.0`; ③ Rust toolchain fix (`rustup default stable`) for GREEN — authorized + relayed; 0.1 + 0.3 (LANDED — gate satisfied).
- **Blocks:** **everything downstream** — Phase 1 (1.1 event envelope consumes the actor/source enums + IDs), and the entire **ui** track (binds to the frozen enums/IDs + a mock GatewayPort). This is the fan-out trigger.
- **Spawns:** **0.5b** — re-freeze the `ExecutionProfile` runtime-state enum once cat-4 SDK-vs-PTY resolves (≥6/15).

## Estimated commit count
**1 (possibly 2).** This is a cross-doc-invariant contract change → it gets clean traceability and is **not bundled** with unrelated work. If ① = A/C (codegen), a sensible split is: (1) the SoT definitions + generation wiring, (2) the cross-language consumers + equality test — but a single coherent commit is acceptable if small. The orchestrator's round commit lands the Appendix A + `daemon/CLAUDE.md` rows separately (commit cadence).

## Lessons-logged candidates anticipated
- **Convention candidate** — "the wire/`TEXT` value (snake_case) is the contract; in-language identifiers are idiomatic but must serialize to the exact string — pinned by the round-trip test."
- **Architecture-doc note candidate** — the cross-language SoT mechanism chosen (① A/B/C) becomes the pattern every future contract addition follows; record it in Appendix A's preamble.
- **Future TODO** — 0.5b (ExecutionProfile) once cat-4 resolves.

## How to invoke
The session is already oriented (spikes ran in it). When dispatched: re-read this brief (the `⏸ PENDING ①` sections will be filled), confirm the Step-2.5 items, then `/tdd shared_contract_freeze`. Pull exact enum/actor/ID value lists from `DATA_MODEL.md` — do not transcribe from `ARCHITECTURE.md` prose. Apply the toolchain fix (③) before GREEN.
