# /tdd brief — redactor_recall_hardening

## Feature
Measure the `prefix-entropy-v2` redactor's secret-detection **precision/recall** against a representative (synthetic) corpus, tune the entropy thresholds to the measured optimum, and evaluate — **with data, human-gated** — whether to extend detection beyond `KEY=value` (and/or arm the MVP-unreached quarantine-divert for already-sensitive payloads) with false-positive guards. Turns the qualitatively-accepted §15 recall envelope (Option C) into a **measured, owned, regression-pinned** envelope.

## Use case + traceability
- **Task ID:** 2.0-SEC (Phase 2, early — *before* the Gateway widens the payload-capture surface)
- **Architecture sections it implements:** `ARCHITECTURE.md §15` (Redaction-before-persist; INV-SEC; the recall-envelope human gate — line 360), `EM §23` (quarantine → `SensitiveOutputRedacted`), `OQ-SEC-2` (the redaction-engine open question, MVP answer in §15)
- **Related context:**
  - **Provenance:** this is the **owned condition** of the user's `2026-06-10` Option-C acceptance of the §15 recall envelope (handoff 002 + session 008). The residual was accepted **on the condition that it be actively owned, not just documented** — this slice is that ownership.
  - **Predecessor:** brief `030-P1-7-redactor-entropy-fallback.md`; session `008-2026-06-10-redactor-entropy-fallback-and-quarantine-divert.md`; **LESSON §13**.
  - **The live surface:** `daemon/src/eventstore/redaction.rs` — `PrefixRedactor`, the named consts (`KV_ENTROPY_BITS=4.0`/`KV_MIN_LEN=20`; `BARE_ENTROPY_BITS=4.5`/`BARE_MIN_LEN=40`), `ENGINE_VERSION="prefix-entropy-v2"`, the `quarantine: Option<QuarantineSignal>` divert field (MVP `None`).
  - **The accepted residuals to quantify (§15 line 360):** (a) JSON-`"key":"value"` short non-prefixed secrets; (b) hex secrets ≈ git-SHAs at ~3.8 bits (entropy can't discriminate); (c) adversarial split-into-<20-char + heavy-pad evasion. Primary control remains **keychain-refs-only (rule #5)**; the §15 fail-closed *gate* holds regardless of recall.

## Acceptance criteria (what "done" means)
- [ ] A **labeled, synthetic corpus** of secret / non-secret samples (correctly-shaped fake secrets + realistic non-secrets), embedded in representative payload contexts (`KEY=value`, JSON-`"key":"value"`, bare-run, prefixed), committed as a fixture. **No real credential is committed** (auditable via a synthetic-marker guard test).
- [ ] A **pure, deterministic measurement harness** computes recall, precision, and false-positive-rate of a given `Redactor` against the labeled corpus, **broken down by payload-context category**.
- [ ] A **baseline measurement** of `prefix-entropy-v2` is captured and pinned as a **regression floor** — recall on the catchable set may not silently drift below the measured baseline; FP-rate on the non-secret set may not rise above the measured ceiling.
- [ ] The entropy thresholds are **tuned to the measured optimum and named with measured justification** (or confirmed-as-optimal with the measurement as the justification). `ENGINE_VERSION` is bumped **iff** the recall bar moved (the engine version is a contract of *which bar produced a row* — `redaction.rs:59`).
- [ ] The **extend-detection decision is escalated to the human with the measured report** (precision/recall + per-category FP cost). Whatever the human rules is implemented (extend-with-FP-guards) **or** documented (accept the envelope as-measured) — **never pre-decided agent-only**.
- [ ] **Cross-doc (orchestrator writes hot at Step 9):** the §15 redactor note (`ARCHITECTURE.md:360`) + **LESSON §13** are refined with the *measured* envelope (concrete numbers replace the qualitative residual list); OQ-SEC-2 narrowed.
- [ ] All tests in `daemon/tests/redaction_recall.rs` pass; the existing `daemon/tests/redaction.rs` (1.7 behavior pins) stays green.
- [ ] `/preflight` clean.
- [ ] **security-reviewer** PASS (this is a §15 invariant-touching slice — `invariant` policy applies).

## Files expected to touch
**New:**
- `daemon/tests/redaction_recall.rs` — the measurement harness + the labeled corpus (or a `corpus` submodule) + the baseline-floor / FP-ceiling regression pins + the per-category breakdown assertions.

**Modified (conditional on measurement + the human ruling):**
- `daemon/src/eventstore/redaction.rs` — **L2:** retune the named consts to the measured optimum (+ bump `ENGINE_VERSION` iff the bar moved). **L3 (only if the human approves extension):** add the FP-guarded extended detection (JSON-value scoping with ID-allowlisting, and/or the sensitivity-gated quarantine-bias arming the divert path).

> If L3 needs the `Redactor` trait to receive the event's `sensitivity` (the quarantine-bias is gated to already-sensitive payloads), that is a **daemon-internal trait-signature change** (`redact(payload, sensitivity)`), not a `shared/` contract — flag at Step 2.5 before going GREEN.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2) — `daemon/tests/redaction_recall.rs`

**L1 — corpus + measurement harness + baseline (mandatory; deterministic; own commit):**

1. **`test_measurement_harness_computes_known_metrics`** — given a tiny hand-labeled set and a stub redactor with a known catch pattern, the harness returns the arithmetically-correct recall / precision / FP-rate.
   - Asserts: recall = caught-secrets/total-secrets; precision = caught-secrets/(caught-secrets+false-catches); FP-rate = falsely-masked-non-secrets/total-non-secrets — exact values on a known fixture.
   - Why: pins the measurement math before it's trusted to judge the real redactor.

2. **`test_corpus_contains_no_real_secrets`** — every "secret" sample carries the documented synthetic marker / construction (e.g. a known fake-token sentinel), so the committed corpus is auditably credential-free.
   - Asserts: no corpus sample is a real credential; all are synthetic-by-construction.
   - Why: rule #5 — a corpus of *real* secrets in the repo would itself be a §15 violation.

3. **`test_baseline_recall_meets_measured_floor`** — `prefix-entropy-v2` achieves ≥ the documented baseline recall on the labeled-secret corpus's **catchable** set.
   - Asserts: measured recall ≥ `BASELINE_RECALL_FLOOR` (a named const set to the measured value).
   - Why: §15 — recall must not silently drift below the bar (the OQ-SEC-2 mandate; line 360).

4. **`test_baseline_false_positive_rate_within_ceiling`** — the FP-rate on the non-secret corpus (prefixed-ULIDs, git-SHAs, UUIDs, URLs, paths, config values, log lines) ≤ the documented ceiling.
   - Asserts: measured FP-rate ≤ `BASELINE_FP_CEILING` (a named const set to the measured value).
   - Why: §15/LESSON §13 — IDs/SHAs/URLs must stay clear (precision is the other half of the envelope; a false mask/divert is the cost).

5. **`test_envelope_measured_by_category`** — the report quantifies recall per payload-context category (`KEY=value` / JSON-value / bare-run / prefixed), so the accepted residual categories (a)/(b)/(c) are **measured, not asserted**.
   - Asserts: a per-category recall breakdown exists and matches the documented residual map.
   - Why: §15 line 360 — turns the qualitative residual list into measured numbers (the report the human rules on).

**L2 — threshold tune-or-confirm (mandatory; safety-critical; own commit):**

6. **`test_tuned_thresholds_named_and_measured`** — the consts (`KV_ENTROPY_BITS`/`KV_MIN_LEN`/`BARE_ENTROPY_BITS`/`BARE_MIN_LEN`) are at their measured-optimal values, and the corpus-driven recall-floor + FP-ceiling assertions hold at those values.
   - Asserts: thresholds = measured optimum; the L1 floor/ceiling pins still pass.
   - Why: §15 — the bar is set by data, not by the 1.7 first-cut estimate; if optimal == current, the measurement *is* the justification.

7. **`test_engine_version_reflects_bar`** *(only if the bar moved)* — `ENGINE_VERSION` is bumped (`prefix-entropy-v2` → next) when a retune changes the recall bar.
   - Asserts: `ENGINE_VERSION` != `"prefix-entropy-v2"` iff a threshold that changes the catch set moved.
   - Why: `redaction.rs:59` — the engine version is the provenance contract of which bar produced a persisted row.

**L3 — JSON-value detection + ID-allowlist (HUMAN-RULED 2026-06-11 = Option B; safety-critical; own commit):**

> **Ruling (human, via lead, 2026-06-11):** extend detection with **Option B — JSON-value scoping + ID-allowlist FP guard, mask-in-place.** Rationale: B closes residual (a) — the gap that grows exactly as 2.1 widens the JSON action-payload capture surface. **Option C (sensitivity-gated quarantine-bias) is DEFERRED** (LESSON §13's mask-in-place-over-divert preference holds; fast-follow only if real sensitive-payload traffic later warrants). Residuals **(b) hex≈git-SHA** + **(c) adversarial-split** stay **accepted-and-owned** ((b) is irreducible by tuning). So L3 writes **test 8 only**; test 9 below is **NOT in scope** (deferred with C).

**Design (the GREEN target, `daemon/src/eventstore/redaction.rs`):** a new pass that masks the high-entropy **value** of a JSON `"key":"value"` pair **in-place** at the KV-confidence bar (the `"key":` assignment context raises confidence like `=` does → reuse `KV_ENTROPY_BITS`/`KV_MIN_LEN`, NOT the stricter bare bar), guarded by an **ID-allowlist** so the measured **0% FP-rate is preserved**. Composes with the existing two passes (pure, golden-log-safe). **`ENGINE_VERSION` bumps** (`prefix-entropy-v2` → next) — the recall bar moved, so the provenance string must reflect the new bar (`redaction.rs:59`; this is the brief test-7 case). `engine_version` is a free-string value, **not** a `shared/` enum → **no CONTRACT_VERSION bump.**

8. **`test_json_value_secret_caught_without_masking_ids`** — a JSON `"token":"<≥20char/≥4.0bit secret>"` is masked **in-place** while ID-shaped values are spared.
   - Asserts: the JSON-value secret is masked; ID-shaped values (git-SHA, ULID, UUID) under JSON keys are **untouched**.
   - Why: closes residual (a) without re-introducing the ID-over-mask the 1.7 `KEY=value`-scoping deliberately avoided.

8b. **`test_json_value_pass_preserves_measured_fp_ceiling`** — re-run the harness over the full corpus with the new pass active; the FP-rate on the non-secret set **stays ≤ `BASELINE_FP_CEILING` (0.0)**.
   - Asserts: no corpus non-secret (ID/SHA/UUID/URL/path/config/log) is newly masked by the JSON pass.
   - Why: Option B's whole point is closing (a) **without** spending precision — the measured 0% FP is the guard rail.

8c. **`test_json_value_recall_ratchets_up`** — the JsonValue-category recall + `recall_catchable` improve to the new measured values; the L1 baseline pins (`BASELINE_RECALL_FLOOR`, the per-category map, the `expected_caught` labels on the now-catchable (a) samples) are **updated to the new envelope** (the floor only ever **ratchets up**, never down).
   - Asserts: JsonValue recall > the L1 0.5; `recall_catchable == 1.0` still holds (every newly-catchable sample is caught); ENGINE_VERSION bumped.
   - Why: §15 — extending the bar moves the measured floor up; the regression pin must guard the **new** (higher) envelope, and the now-caught (a) samples flip `expected_caught` false→true.

**Design question for L3's Step 2.5 — the ID-allowlist mechanism (the FP guard):**
- **Value-shape allowlist (my default vote):** spare a value that *matches a known non-secret ID shape* — git-SHA (40-char all-hex), ULID (26-char Crockford base32), UUID (8-4-4-4-12). Robust regardless of key-name; directly preserves the measured 0% FP (those shapes ARE the corpus non-secrets); **avoids the "allowlist is an unbounded maintenance surface" con** I flagged for Option B. Note sparing the git-SHA shape also leaves residual (b) accepted — consistent with the ruling.
- **Key-name allowlist (alternative):** spare values under ID-ish key-names (`id`, `*_id`, `sha`, `uuid`, `hash`, `commit`…). Simpler match but an unbounded key-name maintenance surface, and it misses an ID under an unexpected key.
- Pick the one that holds FP=0.0 on the corpus with the least maintenance surface; flag at Step 2.5.

~~9. `test_quarantine_bias_gated_to_sensitive_payloads`~~ — **DEFERRED with Option C** (not written this slice). _(Was: divert on an unboundable secret in a `secret`/`restricted` payload. Reserved as a fast-follow if real sensitive-payload traffic warrants arming the MVP-unreached divert; the wired path + its `ForcesQuarantine` test stay as the §17-analogous net.)_

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** **none.** No `shared/` contract changes. `ENGINE_VERSION` is a free-string provenance *value* (not an enum/schema field) → bumping it is **not** a `CONTRACT_VERSION` bump. L3's quarantine-bias reuses the already-frozen `SensitiveOutputRedacted` (0.14.0) + the daemon-internal `QuarantineSignal` — no new wire surface. (A `Redactor`-trait signature change for L3 is daemon-internal.)
- **Orchestrator doc rows to write hot (Step 9 routing):**
  - Refine the §15 redactor note (`ARCHITECTURE.md:360`) — replace the qualitative residual list with the **measured** envelope (numbers + per-category recall); narrow/resolve OQ-SEC-2.
  - Refine **LESSON §13** with the measured envelope + the tuned consts.
  - If the human rules to extend: an architecture-doc note on the extended detection + its FP guard.

> **Implementer never edits `daemon/CLAUDE.md`, `ARCHITECTURE.md`, `MVP_TASKS.md`, or `daemon/LESSONS.md`** — flag at Step 9 categorized; the orchestrator writes them hot.

## Things to flag at Step 2.5
1. **Corpus = synthetic-but-representative, NEVER real secrets.** Synthetic correctly-shaped fakes (real *format*, fake *value*) + realistic non-secrets vs. attempting to capture real output. My default vote: **synthetic, committed as a labeled fixture with a synthetic-marker guard (test 2)**. Rationale: rule #5 forbids real secrets in the repo — a "real-secret corpus" would itself be the §15 violation we're defending against. "Representative" = covers the *shapes* (GitHub PAT/AWS/JWT/PEM/base64-blob/hex-of-varying-entropy + ULID/SHA/UUID/URL/path/config/log-line), not real credentials.
2. **Harness + corpus location.** Extend `daemon/tests/redaction.rs` vs a new `daemon/tests/redaction_recall.rs`. My default vote: **new `redaction_recall.rs`** — keeps the measurement/metrics harness separate from the 1.7 behavior pins (different concern: statistical measurement vs. specific-case assertions).
3. **What becomes the regression pin?** The exact floor/ceiling numbers come *from* L1's measurement — the brief can't pre-set them. My default vote: **pin `recall ≥ measured-baseline` + `FP-rate ≤ measured-baseline` as named consts**, so any future regression below today's measured envelope fails CI. (The *measurement* sets the value; the *test* prevents silent drift.)
4. **Tune vs. confirm.** Adjust the consts only if measurement shows the current bar misses real secrets it should catch OR over-masks IDs; else confirm the 1.7 values with measured justification + bump `ENGINE_VERSION` iff the bar moves. My default vote: **measurement-driven tune-or-confirm** — don't churn the bar without data.
5. **THE ESCALATION GATE — extend detection beyond `KEY=value` / arm the quarantine-bias? (§15 safety-design decision.)** ✅ **RESOLVED 2026-06-11 — HUMAN RULED Option B** (JSON-value + ID-allowlist, mask-in-place; C deferred; (b)/(c) accepted-and-owned). See the L3 section above for the baked-in ruling + design. _(Original framing, for the audit trail:)_ Options after L1 lands: **(A)** accept the envelope as-measured (no new code; document it) · **(B)** extend JSON-value detection with an ID-allowlist FP guard · **(C)** additionally arm the sensitivity-gated quarantine-bias for `secret`/`restricted` payloads · **(B+C)**. My default vote was: **do NOT pre-decide — after L1's measurement lands, the orchestrator takes the measured report (recall + per-category FP cost) to the human and the human rules.** Rationale: the Option-C acceptance already accepted the envelope; 2.0-SEC's mandate is to *measure + own* it, and any extension changes the FP/false-divert tradeoff and arms a §15 safety path → human gate (escalate **before** L3's Step-2.5 sign-off, per root `CLAUDE.md` conv #8).
6. **If L3 ships the quarantine-bias: does `Redactor::redact` need the event `sensitivity`?** Gating the bias to already-sensitive payloads requires the redactor to know the payload's sensitivity. My default vote: **only if option C is approved; flag the `redact(payload, sensitivity)` trait change as daemon-internal** (the writer already has the envelope's sensitivity at the call site).

## Dependencies + sequencing
- **Depends on:** 1.7 (the `prefix-entropy-v2` redactor + the wired quarantine-divert path — landed `c795668`/`f807913`). The §15 fail-closed gate (1.1) + the writer divert path (1.7) are the substrate.
- **Blocks:** nothing hard-blocks on it, but it is deliberately slotted **before 2.1** — the Gateway widens the payload-capture surface (every `ActionRequest`/`ActionExecution*` event carries more capturable content), so the recall envelope should be measured + owned *before* that surface grows.
- **Does NOT fold these Carry-forward items** (they are `last-consumer-slice: 2.1` / first-gateway-event-slice, not this slice): the 1.1-L1 `Timestamp` newtype + `seq minimum:1` schema refinements; the `"SessionStarted"`/event-type string-literal dedup. → **fold into the 2.1 brief.**

## Estimated commit count
**2–3, layered, each safety layer its own commit** (the redactor is §15-safety-critical — never bundled, per root `CLAUDE.md` + the template's bundling rule):
- **L1** — corpus + measurement harness + baseline pins (measurement infra; daemon-internal; does not change masking) → **commit 1**.
- **L2** — threshold tune-or-confirm (+ `ENGINE_VERSION` iff the bar moved) → **commit 2** (safety-critical).
- **L3** — extended detection / quarantine-bias → **commit 3, CONDITIONAL on the human ruling** (safety-critical); may be **deferred entirely** (envelope accepted as-measured).

Drive layer→layer (no idle between commits): fold the next layer into the SHIP ask; treat "proceeding" as the re-wake. **Exception:** at the L1→L2/L3 boundary the orchestrator escalates the extend-detection decision to the human — the implementer continues L2 (tune-or-confirm) while that ruling is pending, and L3 waits on the ruling.

## Lessons-logged candidates anticipated
- **Convention candidate** — "Measure a statistical security detector against a *synthetic-but-representative* labeled corpus; pin recall-floor + FP-ceiling as regression consts; never commit a real-secret corpus." (Likely refines/extends LESSON §13 rather than a new lesson.)
- **Architecture-doc note candidate** — the §15 redactor note gains the *measured* envelope (numbers replace the qualitative residual list); OQ-SEC-2 narrowed/resolved.
- **Future TODO — operational** — the corpus is a living artifact; a future re-tune (or a new secret shape) re-runs the harness + re-pins. The harness is the reusable mechanism.

## How to invoke
1. **Read this brief end-to-end** — especially "Things to flag at Step 2.5" item 5 (the escalation gate) and item 1 (synthetic corpus).
2. **Run `/tdd redactor_recall_hardening`** in the implementer session.
3. **Step 0 (Restate)** — confirm the restatement matches the Feature line (measure → tune → human-gated-evaluate).
4. **Step 1 (Identify files)** — confirm against "Files expected to touch."
5. **Step 2.5 (test review pause)** — ping back with answers to the design questions (or take defaults). The escalation gate (item 5) resolves **after L1's measurement**, not at Step 2.5 — surface L1's measured report at L1's Step 9 so the orchestrator can route the §15 decision to the human.
6. **Step 7.5 (reachability)** — the redactor is already on the live persist path (`main.rs` → `bootstrap::cold_start` → `EventStore::open(PrefixRedactor)` → `append()` → `redact()`); L2/L3 masking changes ride that path. The measurement harness is **verification test-infra** (no production entry point — like the 1.7 fuzz pin); state that explicitly rather than claiming a false production reachability.
7. **Step 9 (summarize)** — surface the measured report + the categorized cross-doc flags; the extend-detection decision is the orchestrator's escalation, not an implementer call.
