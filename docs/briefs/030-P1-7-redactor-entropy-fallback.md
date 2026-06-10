# /tdd brief — redactor_entropy_fallback

## Feature
Extend the 1.1 `PrefixRedactor` (§15) with a **Shannon-entropy fallback on `KEY=value` lines** (OQ-SEC-2) so secret-detection recall doesn't drift below the §15 bar, and wire the L3-deferred **quarantine → `SensitiveOutputRedacted`** path (a high-confidence secret that can't be safely redacted diverts the event + records a `SensitiveOutputRedacted` instead) plus the **§15 property/fuzz test** (no secret ever persists `unredacted`). **The LAST Phase-1-acceptance blocker.**

## Use case + traceability
- **Task ID:** P1.7
- **Architecture sections it implements:** `ARCHITECTURE.md §15` (Redaction-before-persist — the MVP engine = "curated high-recall token-prefix set + **Shannon-entropy fallback on `KEY=value` lines (OQ-SEC-2)**; on a high-confidence secret that can't be safely redacted → **quarantine the event + emit `SensitiveOutputRedacted`** (EM §23)"), the `RedactionStatus` contract (`shared/src/event_envelope.rs:49-55` — "a quarantined/unredactable event is **diverted — not persisted** — and a `SensitiveOutputRedacted` recorded instead").
- **Related context:** 1.1 shipped the `PrefixRedactor` (`daemon/src/eventstore/redaction.rs`) + the writer fail-closed gate (`daemon/src/eventstore/mod.rs:212-213` — `redact()` → refuse to persist if `status != Redacted`). This raises *recall* + adds the *quarantine* outcome the 1.1 redactor lacked (it redacts-or-passes, never quarantines). **This task was made BLOCKING (not a loose Step-9 flag) specifically so secret-detection recall can't silently drift below the §15 bar** (`MVP_TASKS.md` 1.7).

## Acceptance criteria (what "done" means)
- [ ] **L1 — entropy fallback (daemon-internal, no contract bump):** the redactor masks the value of a `KEY=value` line whose value is a **high-entropy** token the prefix set misses (Shannon entropy ≥ threshold AND length ≥ min — see Step-2.5 Q1); structure (JSON punctuation, the `KEY=`) stays intact. Deterministic (pure function of the payload — golden-log-safe). `engine_version` bumps (`prefix-v1` → e.g. `prefix-entropy-v2`).
- [ ] **L1 — no false-positive storm:** a **low-entropy** `KEY=value` (e.g. `DEBUG=true`, `PORT=8080`, `LOG_LEVEL=info`, a path/URL) is NOT redacted.
- [ ] **L2 — quarantine → `SensitiveOutputRedacted` (CONTRACT_VERSION 0.13.0→0.14.0):** when a high-confidence secret is detected that can't be safely redacted in-place (Step-2.5 Q2), the original event is **diverted (NOT persisted)** and a `SensitiveOutputRedacted` event is appended in its place (carrying detection metadata only — NO secret content; structural reason, mirroring the 1.6c AIV pattern). The §15 gate invariant holds: only `Unredacted`/`Redacted` ever reach `events`, and no row persists `unredacted`.
- [ ] **§15 property/fuzz test:** across fuzzed payloads with embedded secrets, the redactor **never** lets a secret persist `unredacted` (the fail-closed invariant holds under fuzzing).
- [ ] All three sinks (persist / embed / sync) are gated by the **same** redactor (no second un-gated path) — confirm/pin (§15).
- [ ] All unit + integration tests in `daemon/tests/redaction.rs` (+ `shared/tests/*` pins for L2) pass; `/preflight` clean.
- [ ] Cross-doc invariant updated atomic with the L2 contract change (orchestrator writes at Step 9).

## Files expected to touch
**New:**
- `daemon/tests/redaction.rs` — the redaction unit + integration + property/fuzz tests (if not already a file; else extend the eventstore redaction tests).

**Modified:**
- `daemon/src/eventstore/redaction.rs` — the Shannon-entropy `KEY=value` fallback in the masking path + the quarantine decision (extend `RedactionOutcome`/the `Redactor` return — see Step-2.5 Q3); `engine_version` bump.
- `daemon/src/eventstore/mod.rs` — the writer's **divert path**: on a quarantine outcome, do NOT persist the original; append a `SensitiveOutputRedacted` event instead (L2).
- `shared/src/events.rs` — the `SensitiveOutputRedacted` payload + EventTypeRegistry entry; `shared/src/lib.rs` — `CONTRACT_VERSION` 0.14.0; `shared/src/schema.rs` + `contracts/schema/*.json` regen; `shared/tests/*` wire-pin (L2).
- `daemon/src/projections/*` — fold `SensitiveOutputRedacted` into `proj_audit_trail` (mirror the 1.6c AIV headline) if a read model is warranted (Step-2.5 Q5).

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN.

## RED test outline (Step 2)
`daemon/tests/redaction.rs` (+ `shared/tests/*` for L2):

**L1 — entropy fallback**
1. **`test_entropy_fallback_catches_prefixless_secret`** — Asserts: a high-entropy `KEY=value` secret with no known prefix (e.g. `API_SECRET=<random 40-char base64>`) → the value is masked `[REDACTED]`. Why: §15 OQ-SEC-2 recall.
2. **`test_low_entropy_config_not_redacted`** — Asserts: `DEBUG=true` / `PORT=8080` / a path / a URL → NOT redacted (no false-positive storm). Why: §15 recall vs precision balance.
3. **`test_entropy_redaction_is_deterministic`** — Asserts: the same payload redacts byte-identically on repeat (pure function — golden-log-safe). Why: §14 determinism / LESSON §3.
4. **`test_prefix_set_still_redacts`** — Asserts: the 1.1 prefix secrets (`ghp_`, `sk-`, PEM, …) still mask (no regression). Why: §15 — recall only rises.

**L2 — quarantine + `SensitiveOutputRedacted`**
5. **`test_unredactable_secret_quarantines_and_records`** — Asserts: a high-confidence secret that can't be safely redacted → the original event is NOT persisted; a `SensitiveOutputRedacted` event IS persisted (status `Redacted`, structural reason, no secret content). Why: §15 quarantine + `event_envelope.rs:49-55`.
6. **`test_quarantine_reason_is_content_free`** — Asserts: the `SensitiveOutputRedacted` payload carries no bytes of the original secret/payload (mirrors the 1.6c AIV content-free reason). Why: §15 — the audit record must not leak what it redacted.
7. **`shared`: `test_sensitive_output_redacted_wire_pin` + `test_contract_version_bumped`** — Asserts: payload round-trips snake_case; `deny_unknown_fields`; `CONTRACT_VERSION` 0.14.0. Why: §5.0 / LESSON §2.

**§15 property/fuzz**
8. **`prop_no_secret_persists_unredacted`** — Asserts: over fuzzed payloads (random structure + embedded prefix/entropy secrets), the writer never persists a row with `redaction_status='unredacted'` AND no known-secret substring survives in a persisted payload. Why: §15 fail-closed invariant under fuzzing (the load-bearing pin — the reason this task is BLOCKING).

**Integration**
9. **`test_all_three_sinks_same_redactor`** — Asserts: persist + embed + sync all route through the one redactor (no un-gated second path). Why: §15 "the SAME redactor gates all three sinks."

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** NEW `SensitiveOutputRedacted` event type → EventTypeRegistry accretion → `CONTRACT_VERSION` 0.13.0→0.14.0 (L2). The entropy fallback + `engine_version` bump are daemon-internal (L1, no contract surface).
- **Orchestrator doc rows to write hot (Step 9 routing):** EventTypeRegistry row (`SensitiveOutputRedacted`) → `daemon/CLAUDE.md` cross-doc + `ARCHITECTURE.md` §7.1/Appendix-A; §15 "Redaction-before-persist" marked **[IMPLEMENTED 1.7 — entropy fallback + quarantine path]** (OQ-SEC-2 resolved); the §15 redactor cross-doc note (engine `prefix-entropy-v2`).

> Implementer never edits `daemon/CLAUDE.md`, `ARCHITECTURE.md`, `MVP_TASKS.md`, `daemon/LESSONS.md` — flag categorized at Step 9; orchestrator writes hot.

## Things to flag at Step 2.5
1. **Entropy threshold + min length.** The Shannon-entropy cutoff (bits/char) over the value + a min length to avoid flagging short tokens. Default vote: **entropy ≥ ~4.0 bits/char AND length ≥ ~20 chars**, tuned against the test fixtures (a real 40-char API key vs `LOG_LEVEL=info`); pick the numbers that pass tests 1+2 with margin and name them as consts. Flag the final numbers.
2. **Quarantine trigger — when divert vs mask-in-place?** Default vote: **mask in-place whenever the secret span is cleanly isolable (prefix or `KEY=value`); quarantine (divert + `SensitiveOutputRedacted`) only when a high-confidence secret is detected but its span can't be safely bounded** (e.g. a high-entropy blob that IS the whole payload, or an ambiguous boundary) — so quarantine is the rare fallback, not the common path. Flag the exact rule.
3. **How does `RedactionOutcome` signal quarantine?** Default vote: a small enum — `RedactionDecision::Redacted { payload_json, engine_version }` vs `::Quarantine { reason, engine_version }` (or add a `quarantine: Option<reason>` to the outcome) — the writer matches on it. Keep `RedactionStatus` (the persisted column) at its frozen 2 values; quarantine is a writer-path decision, not a third persisted status. Flag the shape.
4. **`SensitiveOutputRedacted` payload shape.** Default vote: `{ original_event_type, reason, detector }` — metadata only (NO secret/payload bytes), structural reason (mirror the 1.6c AIV). Preserve the diverted event's `event_type` for forensics; do NOT carry its payload. Flag fields.
5. **Projection fold for `SensitiveOutputRedacted`?** Default vote: **fold into `proj_audit_trail`** (an audit headline, mirror the 1.6c AIV) — it's a security-audit event the UI/§17 surface will want; no new DDL. Flag if a different/no read model.
6. **Redactor placement.** It lives in `daemon/src/eventstore/redaction.rs` today (§15 says "owned by `policy` long-term"). Default vote: **keep it in `eventstore/redaction.rs` for 1.7** (don't move it this slice — the move to `policy/` is a separate refactor); the trait already lets `policy` own it later. Flag if you'd move it now.

## Dependencies + sequencing
- **Depends on:** 1.1 (the `PrefixRedactor` + the writer fail-closed gate + `redaction_status` + `engine_version` columns — all LANDED).
- **Independent of:** 1.6 (the runtime/replay/subscribe surfaces — orthogonal).
- **Blocks:** **Phase-1 acceptance** (the §15 entropy-recall bar — acceptance criterion 4 in the Phase-1 checklist). After this lands, **Phase 1 is DONE** → Phase 2 (Action Gateway).

## Estimated commit count
**2** (layer→layer — drive each layer RED→GREEN→commit, no idle between):
- **L1** — Shannon-entropy `KEY=value` fallback + false-positive guard + `engine_version` bump. **Daemon-internal, no contract bump.** §15 safety-critical → its own commit; security-reviewer.
- **L2** — quarantine → `SensitiveOutputRedacted` divert path + the new event type (CONTRACT_VERSION 0.14.0) + projector fold + the §15 property/fuzz pin. **Contract bump + safety-critical** → its own commit; security-reviewer.

Both layers are **§15 safety-critical** — never bundled with non-safety work; each gets its own commit. Not split further (L1+L2 share the redactor surface + the property test spans both).

## Lessons-logged candidates anticipated
- **Convention candidate** — "Secret detection = high-recall prefix set + Shannon-entropy fallback on `KEY=value`; when a high-confidence secret can't be safely bounded, **divert the event + record a `SensitiveOutputRedacted`** rather than persist a partially-masked payload (fail-closed > best-effort-mask)."
- **Architecture-doc note candidate** — §15 OQ-SEC-2 resolved; the MVP redactor engine is `prefix-entropy-v2`; the quarantine path is the §15 "can't safely redact → divert" branch realized.
- **Future TODO — operational** — entropy-threshold tuning against a real corpus (recall/precision) post-MVP; the redactor's eventual move to `policy/`.

## How to invoke
1. Read this brief end-to-end (don't skip Step-2.5 — **Q1 entropy threshold + Q2 quarantine trigger are the load-bearing §15 design calls**).
2. Run `/tdd redactor_entropy_fallback`.
3. Step 0 (Restate) — confirm against the Feature line.
4. Step 1 — confirm the file list.
5. Step 2.5 — send the L1+L2 test-design write-up (the §15 fail-closed + entropy-recall + quarantine-divert invariants are the review surface). **security-reviewer applies to BOTH layers.**
6. Drive L1 (entropy, daemon-internal) → commit → straight into L2 RED (quarantine + `SensitiveOutputRedacted`, CONTRACT 0.14.0). No idle between.
7. Step 9 — categorized flags; the CONTRACT bump + EventTypeRegistry row + the §15-[IMPLEMENTED] note are the orchestrator's hot-write. After this lands → **Phase 1 DONE**.
