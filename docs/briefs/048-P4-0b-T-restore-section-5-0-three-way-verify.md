# /tdd brief — restore_section_5_0_three_way_verify

## Feature
Restore the §5.0 cross-language 3-way verify (`shared/contracts/verify/`) to GREEN by teaching all three extractors (schema / Pydantic / Zod) to treat a **`oneOf`/`const`-string union** — the form schemars emits for a *per-variant-doc'd* enum like `MetricQuality` — identically to a flat enum, **pin the codegen tool versions** so a future auto-update can't silently re-drift, and add a **gate-self-health assertion** so the gate can never sit dark again. Pure CI-tooling fix — **no `shared/` change, no CONTRACT bump.** Non-cat-1, fork-free.

## Use case + traceability
- **Task ID:** P4.0b-T
- **Architecture sections it implements:** `ARCHITECTURE.md §5.0` (Contract source-of-truth & propagation — the Rust→schema→{Zod,Pydantic} generation + the CI-diff-gated 3-way equality this harness enforces). This slice fixes the **harness that enforces §5.0**, not a contract surface — §5.0's content is unchanged.
- **Phase-scope note — this brief WIDENS phase scope because** §5.0 (and the §2.5-seam reference in the cross-doc section) are **Phase-0 cross-cutting contract anchors**, not in P4's nominal Spec-anchor set: this is a CI-tooling fix for the §5.0 verify *harness*, slotted into Phase 4 purely by the lead's timing ruling (the clean non-cat-1 slot at the 4.0b-1 completion boundary, before 4.0b-2). No P4 contract surface is implemented here.
- **Related context:**
  - **Finding origin:** P4.0b-1 L1 Step-9 (session doc `018`, Decisions-tabled, LESSON **29** — already banked in the seal `ac2bb12`). The §5.0 verify has sat silently **RED ~7 slices** (since the 043 seal).
  - **Root cause (reproduced 2026-06-13):** `MetricQuality` (`shared/src/harness.rs:31`) carries **per-variant doc comments** → schemars emits it as `{"oneOf":[{"const":"exact",…},{"const":"estimated",…},{"const":"unavailable",…}]}` instead of a flat `{"enum":[…]}`. `verify.py`'s `from_schema()` only reads defs with an `"enum"` key → **misses it**; `from_zod()` only matches `z.enum([…])` → **misses it** (json-schema-to-zod renders the const-union as `z.any().superRefine(… [z.literal("exact"), z.literal("estimated"), z.literal("unavailable")] …)`); but **datamodel-code-generator 0.63.0** reflects it as `class MetricQuality(Enum)` → **catches it**. Net live result: `schema=35, pydantic=36, zod=35`, `only-pydantic={exact,estimated,unavailable}` → FAIL.
  - **Why undetected:** the authoritative chain (Rust→schema **test-9** byte-diff, inside `cargo test --workspace`) + the Zod path were GREEN throughout, and `run.sh` runs **off the per-slice loop** (nightly/manual). CI-health gap, **not** a contract-correctness defect — the in-loop snapshot tests protected the contract the whole time.

## Acceptance criteria (what "done" means)
- [ ] `from_schema()` extracts the value-set for a `oneOf`/`anyOf` def whose members are **all** `{"const": <string>}` (the const-union enum form, e.g. `MetricQuality`), in addition to the existing flat-`"enum"` form.
- [ ] `from_schema()` does **NOT** extract a value-set for a `oneOf` whose members are objects (a tagged union — `ServerFrame`, `ActionError`): the all-members-are-string-`const` rule excludes them.
- [ ] `from_zod()` extracts the value-set for a `z.any().superRefine(…)` whose `schemas` array is **all** `z.literal("<string>")` (the const-union form), in addition to the existing `z.enum([…])` form.
- [ ] `from_zod()` does **NOT** extract a value-set for a `superRefine` whose `schemas` contain a `z.object(…)` (the `ServerFrame`/`ActionError` tagged-union form).
- [ ] `from_pydantic()` continues to extract `class X(Enum)` value-sets unchanged (regression guard).
- [ ] After the fix, the live run yields `schema == pydantic == zod` (all **36**) → `verify.py` prints `PASS` and `run.sh` exits **0**.
- [ ] **Self-health (the dark-gate detector):** `verify.py main()` asserts the comparison is **non-degenerate** — each extractor surfaced **≥1 const-union enum** AND **≥1 flat enum** from the *live* generated output; a green run therefore proves **both** extraction arms fired. A future generator change that hides const-unions again drops that count to 0 → loud FAIL (not a silent dark pass).
- [ ] **Tool-version pins** in `run.sh`: `datamodel-code-generator==0.63.0` and `json-schema-to-zod@<pinned>` (belt-and-suspenders — the Finding is precisely an upstream auto-update regression).
- [ ] Offline unit tests in `shared/contracts/verify/test_verify.py` pass (the deterministic RED-first surface; no cargo/uvx/npx needed).
- [ ] `run.sh` runs the offline `test_verify.py` self-tests **before** the codegen step (fail-fast; runs wherever the verify runs).
- [ ] **No `shared/` schema change, no `CONTRACT_VERSION` bump** (CI-tooling only).
- [ ] `/preflight` clean.

## Wiring / entry point (Step 7.5)
The verify is invoked by **`shared/contracts/verify/run.sh`** → `python3 verify.py`; `run.sh` is wired into CI as the §5.0 test-8 3-way gate (`.github/workflows/nightly.yml`, per the §5.0-CI carry-forward). The new extractor branches are reachable via `verify.py main()` ← `run.sh` ← the CI verify job; the new `test_verify.py` is reachable via the `run.sh` preamble (`python3 …/test_verify.py`). **Confirm** `run.sh` still calls `verify.py` and now also calls `test_verify.py`, and that the CI job still calls `run.sh` (no rename). Restoring this gate to GREEN is what makes it *meaningfully* reachable again — a perpetually-RED gate is effectively dead.

## Files expected to touch
**New:**
- `shared/contracts/verify/test_verify.py` — offline, fixture-based, plain-`assert` unit tests (no pytest dep; runnable via `python3 test_verify.py`) for `from_schema` / `from_zod` / `from_pydantic` (both the flat form and the const-union form, plus the object-union exclusion) and a self-health fixture.

**Modified:**
- `shared/contracts/verify/verify.py` — `from_schema()` + `from_zod()` recognize const-unions (string-`const`/`z.literal` members only; object members excluded); add the self-health assertion in `main()`; keep the extractor functions top-level + importable (they already are) so `test_verify.py` can target them directly.
- `shared/contracts/verify/run.sh` — pin the two codegen tool versions; run the offline `test_verify.py` self-tests before the (network-dependent) codegen + live comparison.

If implementation needs files beyond this list (e.g. a tweak to `nightly.yml` to surface the gate's PASS/FAIL more visibly, or a `LESSONS.md#29` `pin:` ref refresh) — **flag at Step 2.5**.

## RED test outline (Step 2)
Tests in `shared/contracts/verify/test_verify.py` (plain `assert`, fixture inputs — deterministic + offline):

1. **`test_from_schema_extracts_flat_enum`** — a `{"enum":["a","b"]}` def → `{a,b}`.
   - Asserts: existing flat-enum extraction still works.
   - Why: regression guard for the 35 enums already paired.
2. **`test_from_schema_extracts_const_union`** — a `{"oneOf":[{"const":"exact"},{"const":"estimated"},{"const":"unavailable"}]}` def → `{exact,estimated,unavailable}`.
   - Asserts: const-union (per-variant-doc'd enum) is now extracted schema-side.
   - Why: §5.0 — the root-cause gap; currently **RED** (`from_schema` returns nothing for this def).
3. **`test_from_schema_excludes_object_union`** — a `{"oneOf":[{"type":"object","properties":{…}}, {"type":"object",…}]}` (ServerFrame-shaped) → **NOT** extracted.
   - Asserts: tagged object-unions are not mistaken for enums.
   - Why: §5.0 — keep the equality comparing *enums*, not discriminated unions (don't over-match).
4. **`test_from_zod_extracts_flat_enum`** — `z.enum(["a","b"])` → `{a,b}`.
   - Asserts: existing `z.enum` extraction still works. Why: regression guard.
5. **`test_from_zod_extracts_const_union_superrefine`** — the `z.any().superRefine((x,ctx)=>{ const schemas = [z.literal("exact").describe(…), z.literal("estimated")…, z.literal("unavailable")…]; })` form → `{exact,estimated,unavailable}`.
   - Asserts: const-union rendered as a literal-only `superRefine` is extracted Zod-side.
   - Why: §5.0 — the Zod half of the root-cause gap; currently **RED**.
6. **`test_from_zod_excludes_object_union_superrefine`** — a `superRefine` whose `schemas` array contains a `z.object({…})` (ServerFrame/ActionError shape, including inner `z.literal("rpc_response")` discriminants) → **NOT** extracted, and the inner discriminant literals are **not** harvested.
   - Asserts: object-union `superRefine`s are excluded; inner `frame_type`/`kind` literals don't leak into the enum sets.
   - Why: §5.0 — the dangerous over-match to guard against (ServerFrame carries `z.literal("rpc_response"/"subscription_push"/"terminal_output")`).
7. **`test_from_pydantic_extracts_enum_class`** — `class X(Enum):\n  a = 'a'\n  b = 'b'` → `{a,b}`; a `class Y(BaseModel)` → not an enum.
   - Asserts: Pydantic extraction unchanged. Why: regression guard (Pydantic was already the correct 36).
8. **`test_self_health_detects_degenerate_run`** — feed the self-health checker a synthetic set with **0 const-union enums** → it FAILs; feed it a set containing both forms → it passes.
   - Asserts: the dark-gate detector actually fires when an extraction arm goes silent.
   - Why: §5.0 / LESSON 29 — "a green run must prove it exercised every arm."

**Integration (acceptance-by-run, not a per-slice-gated unit test — depends on uvx/npx + network):** `bash shared/contracts/verify/run.sh` exits 0 and prints `PASS: … 36 enums agree`.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none. No `shared/` model, no schema, no `CONTRACT_VERSION` bump.
- **Orchestrator doc rows to write hot (Step 9 routing):** none required. **LESSON 29 is already banked** (seal `ac2bb12`; `daemon/CLAUDE.md` index + `LESSONS.md#29`) — if `test_verify.py` lands, the orchestrator may refine LESSON 29's `pin:` ref from `shared/contracts/verify/ (4.0b-T tooling-fix)` to point at `test_verify.py` (a hot doc-prose touch-up, not a new lesson). Implementer flags it; orchestrator writes.
- **§2.5-seam (shared-contract) model touched?** No — this slice touches no Appendix-A model. **No schema-snapshot test required.**
- **Reviewer policy:** `security-reviewer` = `invariant` → **correctly SKIPPED** (non-cat-1, no §15 surface). `code-quality-reviewer` = `every-slice` → runs on the slice diff.

## Things to flag at Step 2.5
1. **Self-health shape.** (a) Assert each extractor surfaced **≥1 const-union enum AND ≥1 flat enum** from the live output (form-coverage); (b) assert a *specific* known value-set (`MetricQuality {exact,estimated,unavailable}`) is present in all three. My default vote: **(a) form-coverage** — robust to contract churn (doesn't hard-code an enum that could be renamed/removed) and directly detects *this* recurrence (a generator change that hides const-unions drops the count to 0). Add (b) as a cheap extra positive assertion only if it reads cleanly.
2. **Const-union vs object-union discriminator.** My default vote: **a `oneOf`/`anyOf` (schema) or `superRefine.schemas` (zod) is an enum-value-set IFF every member is a string `const`/`z.literal` with no `type:object`/`properties`/`z.object`.** Any object member ⇒ not an enum (tagged union). Pin both directions (tests 2/3 + 5/6). Anything ambiguous (e.g. a `oneOf` mixing const + object) → exclude + flag.
3. **Pin granularity.** Exact (`datamodel-code-generator==0.63.0`, `json-schema-to-zod@<exact>`) vs compatible (`~=0.63`). My default vote: **exact pins** — the Finding is precisely an unpinned auto-update regression; freeze hard, revisit on a deliberate bump. (Verify the current `json-schema-to-zod` version during impl — `--version` returned empty in the orchestrator's probe; resolve the concrete pinned version, e.g. via `npm view json-schema-to-zod version`.)
4. **Per-slice loop vs nightly.** Should the verify move INTO `cargo test` / the per-slice loop? My default vote: **no — keep it the nightly/CI gate; rely on the self-health assertion + the pins + the offline `test_verify.py` (which DOES run cheaply)**. Moving the full 3-way into every commit needs uvx/npx + network per-commit (heavy, flaky); the authoritative Rust→schema **test-9** byte-diff already guards contract-correctness per-slice. Flag if you disagree.

## Dependencies + sequencing
- **Depends on:** P4.0b-1 complete (✅, `5e8faed`). The `MetricQuality` const-union has existed since 3.1 (CONTRACT 0.20.0); nothing else gates this.
- **Blocks:** nothing hard (4.0b-2 doesn't require it) — but **restores the §5.0 gate GREEN before** 4.0b-2 / 4.0c / 4.1 land their next contract bumps, so those bumps are verified by a live gate rather than landing on a dark one.

## Estimated commit count
**1.** A single focused tooling fix — `verify.py` (two extractor branches + self-health) + `test_verify.py` (the RED-first unit surface) + `run.sh` (pins + self-test preamble). One logical unit, same directory, no safety invariant, non-cat-1. Bundles cleanly.

## Lessons-logged candidates anticipated
- **Convention candidate** — "the §5.0 3-way verify treats a per-variant-doc'd enum (`oneOf`/`const` schema → `superRefine` of `z.literal`s → Pydantic `Enum`) identically to a flat enum across all three languages; tagged object-unions are excluded by the *all-members-are-string-const* rule." (Refines/realizes LESSON 29.)
- **Future TODO — operational** — if the per-slice loop should ever ingest the 3-way (network permitting), that's a CI-hardening follow-up; the self-health + pins are the MVP closure.
- **Architecture-doc note candidate** — none (CI-tooling; §5.0 prose unchanged).

## How to invoke
1. **Read this brief end-to-end** — the root cause + the const-union-vs-object-union discriminator are the crux; don't skip the Step-2.5 questions.
2. **Run `/tdd restore_section_5_0_three_way_verify`** in the implementer session.
3. **Step 0 (Restate)** — confirm against the Feature line.
4. **Step 1 (Identify files)** — confirm against Files expected to touch.
5. **Step 2.5 (test review pause)** — answer the 4 design questions (or take defaults). The extractor exclusion tests (3/6) are the load-bearing correctness guard — don't drop them.
6. **Step 9 (summarize)** — surface the concrete `json-schema-to-zod` pin version resolved, and whether LESSON 29's `pin:` ref should be refreshed to `test_verify.py`.
