# Session 009 — Phase 2.0-SEC (§15 redactor-recall hardening)

| | |
|---|---|
| **Date** | 2026-06-11 |
| **Phase** | Phase 2 (Action Gateway) — task **2.0-SEC** (the OWNED condition of the §15 Option-C recall-envelope acceptance; slotted EARLY, before 2.1 widens the payload-capture surface) |
| **Track / role** | `daemon` / daemon-implementer |
| **Predecessor** | [008](008-2026-06-10-redactor-entropy-fallback-and-quarantine-divert.md) |
| **Successor** | _(TBD — Phase 2.1 Gateway pipeline; resumes after the scaffolding/workflow upgrade pause)_ |
| **Commits** | **L1:** `950957c` (corpus + measurement harness + baseline pins; test-infra, no contract). **L2:** `2f61a77` (threshold confirm + named `pub` consts; daemon-internal). **L3:** `55f1e7f` (JSON-value detection + value-shape ID-allowlist, Option B; engine `v2`→`v3`). _(This `docs(sessions)` commit seals the session.)_ |
| **Base** | `22cf41f` (Phase 1 DONE / handoff 002; CONTRACT_VERSION 0.14.0) |
| **Contract** | **No `CONTRACT_VERSION` bump** — `ENGINE_VERSION` `prefix-entropy-v2`→`v3` is a free-string provenance value, not a `shared/` enum/schema field. No `shared/` surface touched. |
| **Brief** | `docs/briefs/031-P2-0-SEC-redactor-recall-hardening.md` |

---

## Why this session existed

The 1.7 §15 recall envelope was accepted by the user (Option C, 2026-06-10) **on the condition that the residual be actively owned, not just documented**. 2.0-SEC is that ownership: turn the qualitatively-accepted envelope into a **measured, owned, regression-pinned** one — and, with data + a human gate, decide whether to extend detection beyond the 1.7 surface. Slotted before 2.1 because the Action Gateway widens the JSON action-payload capture surface, so the envelope should be measured + owned before that surface grows.

---

## What was built

**Files created**
- `daemon/tests/redaction_recall.rs` — the §15 measurement infrastructure: a labeled **synthetic** corpus (12 secrets / 11 non-secrets across the `Prefixed`/`KvValue`/`JsonValue`/`BareRun` payload contexts; fake *value*, real *format* — no real credential, guarded by `test_corpus_contains_no_real_secrets`), a pure deterministic recall/precision/FP-rate harness (`Option<f64>` metrics — undefined cells explicit, never NaN/0.0), per-category breakdown, regression floor/ceiling pins, and a `report_measured_envelope` emitter (the regenerable human-gate artifact).

**Files modified**
- `daemon/src/eventstore/redaction.rs` — **L2:** the 4 entropy thresholds (`KV_ENTROPY_BITS`/`KV_MIN_LEN`/`BARE_ENTROPY_BITS`/`BARE_MIN_LEN`) made `pub` with measured-justification doc comments (a named, regression-guarded contract; zero behavioral delta). **L3:** new `mask_json_values` pass (pass 2 of `kv → json → tokens → PEM`) masking high-entropy JSON `"key":"value"` values in-place at the KV bar (sub-run-scored so prose/paths/URLs are spared), guarded by a value-shape ID-allowlist (`is_id_shape`/`is_hex_id`/`is_uuid`/`is_ulid`/`is_crockford_base32`); `ENGINE_VERSION` `v2`→`v3`; module doc updated to "three passes."
- `daemon/tests/redaction.rs` — `test_bare_run_length_boundary` re-isolated to a **bare-string** payload (the L3 JSON pass now correctly masks a ≥20-char JSON value, so the bare-run floor `BARE_MIN_LEN=40` is pinned via a context the JSON pass doesn't reach). Composition adjustment, not a regression.
- `daemon/tests/eventstore.rs` — the persisted-engine golden pin `redaction_engine_version` `prefix-entropy-v2`→`v3`.

**Measured envelope** (`prefix-entropy-v3`; L1 v2 → L3 v3):

| Metric | L1 (v2) | L3 (v3) |
|---|---|---|
| overall recall | 0.727 | 0.75 (9/12) |
| recall (catchable) | 1.0 | **1.0** |
| precision | 1.0 | **1.0** |
| FP-rate | 0.0 | **0.0** |
| Prefixed | 1.0 | 1.0 |
| KvValue | 0.75 | 0.75 — residual (c) adversarial-split-<20 |
| JsonValue | 0.5 | **0.667** — (a) closed for ≥20ch; <20ch retained as accepted sub-residual |
| BareRun | 0.5 | 0.5 — residual (b) hex≈git-SHA ~4.0 bits |

---

## Decisions made

- **L2 = confirm, not tune.** Measurement showed the 1.7 thresholds are precision-optimal for the catchable set (recall_catchable 1.0 / FP 0.0). Confirmed-as-sufficient with the measurement as justification; thresholds named `pub` as a regression-guarded contract; `ENGINE_VERSION` kept `v2` (bar didn't move).
- **L3 = Option B (human-ruled 2026-06-11):** JSON-value detection + value-shape ID-allowlist, mask-in-place. Closes residual (a) for ≥20-char secrets with **zero precision cost** (FP held 0.0).
- **FP guard = value-shape allowlist** (git-SHA all-hex / ULID 26-char Crockford incl. `<lowercase>_` prefix / UUID 8-4-4-4-12) over key-name allowlist — robust regardless of key-name, no unbounded maintenance surface. Sparing the hex shape keeps residual (b) accepted (consistent with the ruling).
- **Detector = sub-run scoring at the KV bar**, not whole-value entropy — so prose/paths/URLs (short sub-runs) are spared (whole-value entropy on a log line would FP).
- **Honesty pin (lead TWEAK):** keep BOTH a ≥20ch (catchable) AND a <20ch (residual) JsonValue sample so the measured JsonValue recall is 2/3 — "(a) closed for ≥20ch, <20ch residual remains" is encoded in the measurement, not only prose. `test_json_value_recall_ratchets_up` pins `0.5 < JsonValue < 1.0` against a future silent over-claim.
- **Metric accessors return `Option<f64>`** (lead ADD) — a zero-denominator metric is an explicit `None`, never a silently-wrong 0.0 (this is safety-decision-gating infra).

## Decisions explicitly NOT made (deferred)

- **Option C (sensitivity-gated quarantine-bias) — DEFERRED.** LESSON §13's mask-in-place-over-divert preference holds; a fast-follow only if real `secret`/`restricted`-sensitivity payload traffic later warrants arming the (1.7-wired, MVP-unreached) divert path. No `Redactor::redact(payload, sensitivity)` trait change made.
- **Residuals (b) hex≈git-SHA + (c) adversarial-split + (a-deep) <20ch JSON** — accepted-and-owned, now measured. (b) is irreducible by threshold tuning (lowering the bare bar would mask real git-SHAs); (a-deep) <20ch can't be distinguished from a short ID/token without FP. Documented as the §15 envelope edge.
- **2.1 carry-forward items NOT folded here** (per the brief): the 1.1 `Timestamp` newtype + `seq minimum:1` schema refinements; the `"SessionStarted"`/event-type string-literal dedup → fold into the 2.1 brief.

---

## TDD compliance

**Clean.** Each layer was test-first with RED confirmed for the right reason before GREEN:
- L1 — tests referenced not-yet-existing `harness`/`corpus` modules (RED: unresolved-module), then implemented.
- L2 — tests referenced private threshold consts (RED: E0603 private), then made `pub`.
- L3 — tests asserted JSON-value masking + v3 (RED: 6 failing — secret survives, recall_catchable 8/9, JsonValue stuck, v2≠v3), then implemented the pass + bumped the engine.
- Added-during-session, both covering real surfaces (not back-fill): `report_measured_envelope` (instrumentation + a corpus-completeness guard) and `test_json_value_escaped_quote_does_not_leak_secret` (covers the load-bearing escape-capture branch the reviewer flagged as uncovered).

---

## Reachability

- **L1 (harness + corpus)** — verification test-infra; no production entry point (like the 1.7 fuzz pin). The redactor it *measures* is live on the persist path.
- **L2 (thresholds `pub` + docs)** — the thresholds are read by the live mask functions on the persist path; L2 added no production path (visibility + docs only).
- **L3 (`mask_json_values`)** — **LIVE on the persist path:** `main.rs` → `bootstrap::cold_start` → `EventStore::open(PrefixRedactor)` → `append()` → `redact()` → `mask_json_values`.
- No tested-but-unwired gaps introduced. The 1.7 quarantine→`SensitiveOutputRedacted` divert path remains wired-but-MVP-unreached (unchanged; the real redactor masks in place — Option C deferred).

---

## Open follow-ups

- **Orchestrator hot-routing (in flight at `/orchestrate-end`):** §15 line-360 arch-note (extended measured envelope + (a)-closed-for-≥20ch + the composition/test-isolation note) · LESSON §13 refinement · `MVP_TASKS.md` 2.0-SEC checkbox ticks · brief `031` + Decisions-tabled Option-B record. These ride the orchestrator's round commit.
- **Operational (living artifact):** the corpus is a living artifact — re-run `cargo test --test redaction_recall report_measured_envelope -- --nocapture` after any future re-tune or new secret shape to regenerate the human-facing report and re-pin the floor/ceiling.
- **Reviewer findings** — all addressed in-slice (security-reviewer PASS 0 findings; code-quality 1 med + 3 low fixed: stale cross-ref → symbol ref, stale corpus comment, `is_ulid` doc clarification, escaped-quote coverage test).
- **Next slice:** Phase 2.1 (Gateway pipeline + `ActionRequest`/`ActionPlan` model + mutation methods) — held by the lead-directed scaffolding/workflow upgrade pause.

---

## How to use what was built

- **Regenerate the §15 envelope report:** `cargo test --test redaction_recall report_measured_envelope -- --nocapture` (via `rtk proxy` to see stdout).
- **The regression pins** (`BASELINE_RECALL_FLOOR`, `BASELINE_FP_CEILING`, the per-category `RECALL_*` consts) fail CI if recall on the catchable set drifts below the measured floor or FP rises above the ceiling — the floor only ever ratchets up.
- **The thresholds** are now a named `pub` contract in `redaction.rs`; a deliberate re-tune updates the const + the measured-justification doc + bumps `ENGINE_VERSION` iff the recall bar moves.
