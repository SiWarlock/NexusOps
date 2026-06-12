# Session 008 — Phase 1.7 (§15 redactor: Shannon-entropy fallback + quarantine-divert)

| | |
|---|---|
| **Date** | 2026-06-10 |
| **Phase** | Phase 1 (daemon foundation) — task 1.7 (OQ-SEC-2) → **Phase 1's last acceptance blocker** |
| **Track / role** | `daemon` / daemon-implementer |
| **Predecessor** | [007](007-2026-06-10-degradable-replay-and-subscribe-serve.md) |
| **Successor** | [009](009-2026-06-11-redactor-recall-hardening.md) — Phase 2.0-SEC §15 recall-envelope measured + owned (the human ruled Option B) |
| **Commits** | **L1:** `c795668` (Shannon-entropy fallback — `KEY=value` + bare-run masking; daemon-internal, no contract bump). **L2:** `f807913` (quarantine→`SensitiveOutputRedacted` divert; CONTRACT 0.13.0→0.14.0). _(This `docs(sessions)` commit seals the session.)_ |
| **Base** | `804fa31` (Phase 1.6 sealed; CONTRACT_VERSION 0.13.0) |
| **Contract** | `CONTRACT_VERSION` **0.13.0→0.14.0** (L2 adds the `SensitiveOutputRedacted` event payload); L1 is daemon-internal (engine `prefix-v1`→`prefix-entropy-v2`, no bump) |
| **Brief** | `docs/briefs/030-P1-7-redactor-entropy-fallback.md` |

## Why this session existed

1.1 shipped the `PrefixRedactor` (token-prefix high-recall set) + the §15 writer fail-closed gate, but secret-detection **recall** could drift below the §15 bar for any secret lacking a recognized prefix. OQ-SEC-2 (the brief made it **BLOCKING + human-owned**, `MVP_TASKS.md` 1.7) called for a **Shannon-entropy fallback on `KEY=value` lines** so recall can't silently drift, plus the L3-deferred §15 items: the **quarantine → `SensitiveOutputRedacted`** divert path (a high-confidence secret that can't be safely redacted diverts the event rather than persisting a partial mask) and a **§15 property/fuzz pin** (no secret ever persists `unredacted`). This is the last Phase-1-acceptance blocker — after it, Phase 1 is functionally complete.

## What was built

Two layers, each its own §15-safety-critical commit (drive layer→layer, no idle).

### Files created
- **`daemon/tests/redaction.rs`** — the 1.7 unit + integration + property/fuzz tests (16 total). L1: prefixless-secret entropy catch, low-entropy-config no-false-positive (URLs/paths spared), determinism, prefix-set regression, bare-run masking + ID-sparing, spaced/quoted `KEY=value`, base64-`+/` in `KEY=value`, entropy-dilution resistance, bare-run length boundary (39 vs 40). L2: quarantine-divert (original not persisted + SOR recorded), content-free reason, routing-preserved/reclassified/namespaced-key, no-recurse fail-closed, real-redactor-never-quarantines, the §15 fuzz pin (`prop_no_secret_persists_unredacted`), three-sink integration.

### Files modified
- **`daemon/src/eventstore/redaction.rs`** — the entropy fallback. Two pure masking passes (golden-log-safe): `mask_kv_values` (env-style `KEY=value`, whitespace/quote/JSON-escape tolerant around `=`, base64-aware value span incl. `+`/`/`/`=`, **sub-run-scored** so padding-dilution can't drag the whole-span average below the bar; ≥20 char ∧ ≥4.0 bits) and `mask_tokens` (known prefixes + PEM + bare high-entropy runs ≥40 char ∧ ≥4.5 bits, alnum tokenizer that keeps URLs/paths/IDs clear). `engine_version` `prefix-v1`→`prefix-entropy-v2`. **L2:** additive `quarantine: Option<QuarantineSignal{reason,detector}>` field on `RedactionOutcome` (the MVP `PrefixRedactor` always sets `None` — masks in place, never diverts).
- **`daemon/src/eventstore/mod.rs`** — the L2 writer **divert** path in `append()`: a `quarantine: Some` outcome diverts (original NOT persisted) → `divert_quarantined` builds a content-free `SensitiveOutputRedacted` intent preserving the diverted event's routing/identity envelope fields (workspace/actor/source/correlation) but dropping its payload, reclassifying `sensitivity → Internal`, namespacing the dedup key `divert-{k}`, and re-`append`s it (the §15 gate + projector fold run on the content-free payload). No-recurse guard: a quarantine signal on a `SensitiveOutputRedacted` event itself returns `RedactionRequired` (fail closed).
- **`daemon/src/projections/audit.rs`** — folds the `SensitiveOutputRedacted` headline ("Sensitive output redacted") into `proj_audit_trail` (mirrors the 1.6c AIV).
- **`daemon/tests/{eventstore,outbox,projections,runtime}.rs`** — the 4 `NeverRedacts` test redactors gain `quarantine: None` (additive field).
- **`shared/src/events.rs`** — `SensitiveOutputRedacted{original_event_type, reason, detector}` payload (`deny_unknown_fields`, content-free) + `EVENT_TYPE` const.
- **`shared/src/schema.rs`** — registered `sensitive_output_redacted` in the `ContractBundle`.
- **`shared/src/lib.rs`** — `CONTRACT_VERSION` 0.13.0→0.14.0.
- **`shared/contracts/schema/nexusops-contract.schema.json`** — regenerated (SOR `$def` + version).
- **`shared/tests/contract.rs`** — `test_sensitive_output_redacted_wire_contract` + the 0.14.0 version assertion.
- **`shared/tests/envelope.rs`** — the canonical `CONTRACT_VERSION` pin updated to 0.14.0.

## Decisions made

- **Q1 — entropy thresholds (orch-approved).** `KEY=value` masks at ≥4.0 bits/char ∧ ≥20 char; bare runs at the stricter ≥4.5 bits/char ∧ ≥40 char. Scoped the entropy fallback to **env-style `KEY=value`** (the spec's literal "KEY=value lines"), NOT JSON `"key":"value"` — deliberately, so prefixed-ULID IDs (`sess_`/`evt_`…), URLs, and paths are spared. Bare-run ≥40 char spares ≤31-char IDs; ≥4.5 bits spares 4.0-bit hex/git-SHAs.
- **Q2 — mask in-place, never divert for a bare blob (orch TWEAK).** All three forms mask **in place** (the real redactor never quarantines). A bare contiguous run *can* be safely bounded (its span is the run), so it's masked, not diverted — masking strictly dominates divert here (asymmetric harm: lose the blob vs. lose the whole event). The quarantine/divert path stays wired + `ForcesQuarantine`-tested as a **§17-quarantine-analogous, MVP-unreached safety net**.
- **Q3 — additive `quarantine` field** on `RedactionOutcome` (not an enum rename), to avoid `RedactionDecision::Persist(..)` churn across the 4 `NeverRedacts` redactors; `RedactionStatus` stays frozen at 2 values (quarantine is a writer-path decision).
- **Q4–Q6 (brief defaults):** SOR payload `{original_event_type, reason, detector}` content-free; fold into `proj_audit_trail`; keep the redactor in `eventstore/redaction.rs` (the move to `policy/` is a later refactor).
- **Security-review-driven hardening (in-slice).** L1: closed spaced/quoted `KEY = value`, AWS-style standard-base64 `+/` in `KEY=value`, and an **entropy-dilution evasion** (padding glued to a secret to drop the whole-span average below the bar → sub-run scoring resists it). L2: namespaced the SOR dedup key (`divert-{k}`, mirrors AIV — retry-dedups, no cross-type collision), reclassified the content-free SOR to `Sensitivity::Internal`, added the recursion-guard fail-closed + identity-preservation tests, `QuarantineSignal` derives `Debug, Clone`.
- **Fuzz generator uses a dependency-free seeded LCG** (no `rand`/`proptest` dep on a safety slice; reproducible). Distinct-char (coprime-stride) tokens guarantee each generated secret is genuinely **catchable** (a sub-threshold token is in the recall envelope by design, not the catchable set).

## Decisions explicitly NOT made (deferred)

- **The §15 recall ENVELOPE — escalated to the human** (the orchestrator is routing it via the lead). The brief made 1.7 human-owned so recall can't *silently* drift; the orchestrator must not silently declare the envelope acceptable. The redactor deliberately/inherently does NOT catch: (a) JSON `"key":"value"` <40-char non-prefixed secrets (ID-sparing tradeoff, matches the spec's "KEY=value" scope); (b) hex secrets indistinguishable from git-SHAs at ~3.8 bits (entropy can't discriminate); (c) bare `+/` base64 in non-KV position; (d) a deeply-adversarial split-into-<20-char-pieces + heavy-pad evasion (inherent limit of any statistical detector — real defense is rule #5 don't-put-secrets-in-payloads + keychain-refs-only). **This gates the Phase-1-DONE declaration, not the L2 commit.**
- **Moving the redactor to `policy/`** — a separate refactor (Q6).
- **Entropy-threshold tuning against a real corpus** — post-MVP operational item (brief's anticipated future TODO).

## TDD compliance

**Clean.** Every code change this session landed test-first: each layer (and each reviewer-driven hardening fix — spaced/quoted KV, `+/` base64, dilution, the recursion guard, identity preservation) was written as a failing test, confirmed RED for the right reason, then driven GREEN. No implementation preceded its test. Both layers are §15-safety-critical and each got its own commit (never bundled).

## Reachability

- **L1 entropy masking** — reachable from `main.rs:51` (production daemon) → `bootstrap::cold_start` → `EventStore::open(redactor: PrefixRedactor)` → `append()` → `redact()` on every persisted event. On the live persist path.
- **L2 quarantine-divert** — `divert_quarantined` is wired inside the production `append()` (`main.rs` → cold_start → write-actor → `append` → divert → re-`append` the SOR → `proj_audit_trail` fold in-txn). **MVP-unreached by design:** the `PrefixRedactor` masks all forms in place and never sets a quarantine signal, so the divert is a wired-but-unreached §15 safety net (analogous to the §17 quarantine, which only fires on real corruption). Exercised by the `ForcesQuarantine`/`QuarantineOnMarker`/`AlwaysQuarantine` test redactors. Not a silent gap — intentional, documented in the commit + the redactor module doc.

## Open follow-ups

- **§15 recall-envelope ruling (Finding → human, BLOCKS Phase-1-DONE declaration).** The orchestrator is escalating it via the lead. Until the human accepts the envelope (or directs further hardening, e.g. JSON-value scoping with explicit ID-allowlisting), Phase 1 is functionally complete but not *declared* DONE.
- **Orchestrator hot-write (pending at `/orchestrate-end`):** the `SensitiveOutputRedacted` EventTypeRegistry row + CONTRACT 0.14.0 in `ARCHITECTURE.md` §7.1/Appendix-A + `daemon/CLAUDE.md`; §15 "Redaction-before-persist" → **[IMPLEMENTED 1.7 — entropy fallback + quarantine-divert]**, OQ-SEC-2 resolved (worded pending the human's recall-envelope acceptance); the redactor engine note `prefix-entropy-v2`; **LESSON §13** (secret detection convention). _Implementer does not edit these — orchestrator territory._
- **Optional doc-polish (non-blocking):** `shared/src/event_envelope.rs:105` `redaction_engine_version` field doc-example still says "e.g. `prefix-v1`" — could mention `prefix-entropy-v2` too (regenerates into the schema). Flagged to the orchestrator.
- **Deferred reviewer lows (non-blocking):** a quarantine-specific error variant for forensic ergonomics (the divert surfaces an idempotency clash as a generic `DuplicateIdempotencyKey`) — only matters once an alternate Redactor arms the MVP-unreached divert; the `shared/tests/contract.rs` `is_some()` assertions could assert values (the round-trip equality already pins them).

## How to use what was built

The redactor is invisible to callers — every `EventStore::append` routes its payload through `PrefixRedactor.redact()` automatically (the §15 gate). To exercise the divert path in tests, inject a `Redactor` that returns `quarantine: Some(QuarantineSignal{..})` (see `QuarantineOnMarker` in `daemon/tests/redaction.rs`); the writer then records a content-free `SensitiveOutputRedacted` in place of the diverted event.
