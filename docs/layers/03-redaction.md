# Redaction engine — §15 secret masking

## Executive summary

Every event NexusOps records is immutable — once a payload lands in the event log it can never be unwritten. The redaction engine is the safety filter that scrubs secrets (API keys, tokens, private keys) out of every payload *before* it is persisted, so an accidentally-leaked credential never becomes a permanent record. It is explicitly **defense-in-depth**, not the primary control: the primary rule is that secrets live only in the OS keychain and rows carry `keychain_ref` pointers (safety rule #4 / ARCHITECTURE.md §15); the redactor catches the accidental leaks that slip past that rule. The engine (`prefix-entropy-v3`) runs three ordered in-place masking passes — env-style `KEY=value` entropy scoring, JSON `"key":"value"` entropy scoring with an ID-shape allowlist, and known token prefixes + bare high-entropy runs — plus a whole-payload PEM mask. Its detection quality is not asserted, it is *measured*: a labeled synthetic corpus pins recall 1.0 on the catchable set / false-positive rate 0.0 as CI regression floors, and the known misses are named, accepted residuals.

## Responsibilities

- **Accountable for:** masking detectable secrets in every event payload before it reaches any of the three §15 sinks — persist (`events`), sync (`outbox`), embed (`fts_events`) — verified end-to-end by `test_all_three_sinks_same_redactor` (`daemon/tests/redaction.rs:577-613`).
- **Accountable for:** recording engine provenance — `ENGINE_VERSION = "prefix-entropy-v3"` is stamped onto every event (`daemon/src/eventstore/redaction.rs:67`, persisted at `daemon/src/eventstore/mod.rs:272`) so each row says which recall bar produced it.
- **Accountable for:** signaling quarantine — `RedactionOutcome.quarantine` (`daemon/src/eventstore/redaction.rs:37`) lets a redactor tell the writer "I cannot safely bound this secret; divert the event."
- **NOT** the fail-closed writer gate itself — the refusal to persist `redaction_status='unredacted'` and the quarantine-divert execution live in the event-store writer (`daemon/src/eventstore/mod.rs:210-226`); see [02-event-store.md](02-event-store.md).
- **NOT** the primary secret control — that is keychain-refs-only (ARCHITECTURE.md §15, "primary control remains keychain-refs-only"; `daemon/CLAUDE.md` forbidden #5). The redactor is the net under it.
- **NOT** a guarantee of perfect recall — it is a statistical detector with a *measured, accepted* envelope (see Gotchas).

## Key components

| Component | What it does | Where |
|-----------|--------------|-------|
| `Redactor` trait | `redact(&str) -> RedactionOutcome`; the seam the writer calls | `daemon/src/eventstore/redaction.rs:51-53` |
| `RedactionOutcome` | status + masked payload + engine version + optional quarantine signal | `daemon/src/eventstore/redaction.rs:28-38` |
| `QuarantineSignal` | content-free divert signal: structural `reason` + `detector`, never a payload byte | `daemon/src/eventstore/redaction.rs:43-47` |
| `PrefixRedactor` | the production engine; runs the three passes + PEM mask | `daemon/src/eventstore/redaction.rs:59`, `:89-112` |
| `SECRET_PREFIXES` | `ghp_`, `github_pat_`, `sk-`, `xox`, `AKIA`, `eyJ` | `daemon/src/eventstore/redaction.rs:62` |
| `ENGINE_VERSION` | `"prefix-entropy-v3"` provenance const (v1→v2→v3 as the bar moved) | `daemon/src/eventstore/redaction.rs:67` |
| `KV_ENTROPY_BITS` / `KV_MIN_LEN` | KV-context bar: 4.0 bits/char, ≥20 chars | `daemon/src/eventstore/redaction.rs:76-77` |
| `BARE_ENTROPY_BITS` / `BARE_MIN_LEN` | bare-run bar: 4.5 bits/char, ≥40 chars | `daemon/src/eventstore/redaction.rs:86-87` |
| `mask_kv_values` + `kv_value_should_mask` | pass 1: env-style `KEY=value` values, base64-aware span, sub-run scored | `daemon/src/eventstore/redaction.rs:133-166`, `:173-179` |
| `mask_json_values` + `json_value_should_mask` | pass 2 (2.0-SEC L3): JSON `"key":"value"` values at the KV bar, allowlist-guarded | `daemon/src/eventstore/redaction.rs:187-227`, `:233-240` |
| `is_id_shape` (+ hex/UUID/ULID helpers) | value-shape FP guard: git-SHA / UUID / prefixed-ULID spared | `daemon/src/eventstore/redaction.rs:248-288` |
| `mask_tokens` + `should_mask_token` | pass 3: known prefixes + bare ≥40-char/≥4.5-bit runs | `daemon/src/eventstore/redaction.rs:292-305`, `:320-328` |
| `shannon_bits_per_char` | deterministic byte-histogram Shannon entropy | `daemon/src/eventstore/redaction.rs:333-350` |
| Writer gate + divert (consumer) | gate + `divert_quarantined` → `SensitiveOutputRedacted` | `daemon/src/eventstore/mod.rs:210-226`, `:305-347` |
| Fuzz pin | `prop_no_secret_persists_unredacted` — 256 seeded payloads, 5 secret shapes | `daemon/tests/redaction.rs:500-570` |
| Measurement harness + corpus | recall/precision/FP math + labeled synthetic corpus + regression pins | `daemon/tests/redaction_recall.rs:499-656`, `:663-810`, `:37-48` |

## Interfaces & contracts

- **Input:** the raw `payload_json` string of an `AppendIntent`, handed over by `EventStore::append` (`daemon/src/eventstore/mod.rs:212`). The redactor sees only the payload — not the envelope.
- **Output:** `RedactionOutcome` — `status: RedactionStatus` (the frozen two-value contract enum `Unredacted | Redacted`, `shared/src/event_envelope.rs:54`), the possibly-masked `payload_json`, `engine_version`, and `quarantine: Option<QuarantineSignal>`.
- **Contract with the writer:** the writer persists `outcome.payload_json` (`daemon/src/eventstore/mod.rs:269`) and refuses any outcome whose status is not `Redacted` (`mod.rs:224-226`, error `EventStoreError::RedactionRequired`, `mod.rs:63`). A `Some(quarantine)` outcome makes the writer drop the original and append a content-free `SensitiveOutputRedacted` instead (`mod.rs:218-223`; payload struct `shared/src/events.rs:81-90`).
- **Purity contract:** `redact` is a pure function of the payload — byte-identical on repeat (golden-log-safe, §14; pinned by `test_entropy_redaction_is_deterministic`, `daemon/tests/redaction.rs:196-205`).
- **Threshold contract:** the four bars are `pub` consts so the recall bar is a named, regression-guarded contract — `test_tuned_thresholds_named_and_measured` pins their exact values (`daemon/tests/redaction_recall.rs:340-357`); `test_engine_version_reflects_bar` pins the version-bump-iff-bar-moves rule (`:364-367`).

## Data & state

The engine is **stateless** — no tables, no caches, no config files. All "state" is compile-time consts (prefix set, entropy bars, `ENGINE_VERSION`). Its outputs persist as two columns on every event row: `redaction_status` and `redaction_engine_version` (DATA_MODEL §2.1 columns; written at `daemon/src/eventstore/mod.rs:271-272`). The mask replacement strings are `[REDACTED]` (`redaction.rs:114`) and, for PEM, the whole payload becomes `"[REDACTED-PEM]"` (`redaction.rs:100-102`).

The measured-quality state lives in tests as named constants (`daemon/tests/redaction_recall.rs:37-48`):

| Metric | Measured (v3) | Pin |
|---|---|---|
| recall on catchable set | **1.0** | `BASELINE_RECALL_FLOOR`, `test_baseline_recall_meets_measured_floor` (`:207-215`) |
| FP-rate on non-secrets | **0.0** | `BASELINE_FP_CEILING`, `test_baseline_false_positive_rate_within_ceiling` (`:223-231`) |
| precision | **1.0** | implied by FP 0.0; printed by `report_measured_envelope` (`:289-322`) |
| Prefixed recall | 1.0 | `RECALL_PREFIXED` (`:45`) |
| KvValue recall | 0.75 | `RECALL_KV_VALUE` (`:46`) — adversarial-split missed |
| JsonValue recall | 0.667 (2/3) | `RECALL_JSON_VALUE` (`:48`) — <20-char sub-residual retained |
| BareRun recall | 0.5 | `RECALL_BARE_RUN` (`:47`) — hex≈git-SHA missed |

The corpus is **synthetic-by-construction**: every secret sample carries a documented sentinel (`FAKE`, `EXAMPLE`, `deadbeef`, …; `SYNTHETIC_MARKERS`, `:668`), audited by `test_corpus_contains_no_real_secrets` (`:187-199`) so the repo never commits a real credential (rule #5).

## Dependencies

- **Depends on:** `nexusops_shared::event_envelope::RedactionStatus` (`daemon/src/eventstore/redaction.rs:24`) — the frozen wire enum; nothing else. No DB, no clock, no IO.
- **Used by:** the event-store single-writer — `EventStore::open` takes a `Box<dyn Redactor>` (`daemon/src/eventstore/mod.rs:149-172`) and `append` routes every payload through it (`mod.rs:212`) before the INSERT, the in-txn projection fold (`mod.rs:283-284`), and the outbox write (`mod.rs:288`). Because *every* event write in the daemon goes through this one `append`, the redactor implicitly gates all three §15 sinks.

## How it works (flow)

```
append(intent)                                 daemon/src/eventstore/mod.rs:197
  └─ redactor.redact(payload)                  mod.rs:212
       PrefixRedactor::redact                  redaction.rs:90-111
         1. mask_kv_values      KEY=value, KV bar (4.0/20), sub-run scored   :92
         2. mask_json_values    "key":"value", KV bar, is_id_shape spared    :96
         3. mask_tokens         prefixes + bare runs (4.5/40)                :98
         4. PEM check           whole payload -> "[REDACTED-PEM]"            :100-102
       -> RedactionOutcome { Redacted, masked, "prefix-entropy-v3", None }
  ├─ quarantine? -> divert_quarantined         mod.rs:218-223 -> :305-347
  ├─ status != Redacted? -> Err(RedactionRequired)   mod.rs:224-226  (fail-closed)
  └─ INSERT masked payload + status + engine_version  mod.rs:269-272
       -> projections fold + outbox write on the ALREADY-REDACTED event  mod.rs:283-288
```

Pass details, in execution order:

1. **`mask_kv_values`** (`redaction.rs:133-166`): on every `=`, skip whitespace/quotes/JSON-escape backslashes, capture a base64-aware value span (`is_value_char` includes `+ / =`, `:119-121`, so an AWS secret key scores as one span). Mask if the whole span clears 4.0 bits/≥20 chars **or any maximal alnum sub-run does** — the sub-run scoring defeats padding-dilution evasion (`kv_value_should_mask`, `:173-179`; pinned by `test_kv_entropy_dilution_resisted`, `daemon/tests/redaction.rs:149-161`). The key itself (`API_SECRET=`) survives.
2. **`mask_json_values`** (`redaction.rs:187-227`, 2.0-SEC L3): on every `:` followed by a quoted string, capture to the next *unescaped* quote (`\"` is skip-2, `:212-214`; leak pinned by `test_json_value_escaped_quote_does_not_leak_secret`, `daemon/tests/redaction_recall.rs:419-433`). Mask at the same KV bar, sub-run scored so prose/paths/URLs are spared — **unless** the value is an ID shape: all-hex (git-SHA), canonical UUID, or 26-char Crockford-base32 ULID with optional single `lowercase_` prefix (`is_id_shape` + helpers, `redaction.rs:248-288`). Runs *after* pass 1 so a `KEY=value` inside a JSON value keeps its env-key (`:93-95`).
3. **`mask_tokens`** (`redaction.rs:292-305`): tokenize on `[A-Za-z0-9_-]` (`is_token_char`, `:126-128` — `:`/`/`/`.` excluded so URLs and paths fragment into short, unmaskable segments); mask any token starting with a `SECRET_PREFIXES` entry, or any bare run ≥40 chars *and* ≥4.5 bits/char (`should_mask_token`, `:320-328`) — the stricter bar deliberately spares 31-char prefixed-ULID IDs and ~4.0-bit hex hashes.
4. **PEM**: if the (already-masked) payload contains `BEGIN` and `PRIVATE KEY`, the entire payload is replaced (`:100-102`).

**Quarantine-divert (wired, MVP-unreached):** `PrefixRedactor` always masks in place and returns `quarantine: None` (`redaction.rs:107-109`; pinned by `test_real_redactor_masks_never_quarantines`, `daemon/tests/redaction.rs:482-491`). The divert path exists for a future redactor that finds a secret it cannot bound: the writer drops the original payload entirely and appends a `SensitiveOutputRedacted` carrying only `{original_event_type, reason, detector}` — routing/identity envelope fields preserved for forensics, sensitivity reclassified to `Internal`, idempotency key namespaced `divert-<key>` (`daemon/src/eventstore/mod.rs:305-347`). A divert-of-a-divert fails closed via the recursion guard (`mod.rs:219-221`; `test_divert_recursion_guard_fails_closed`, `daemon/tests/redaction.rs:463-476`). The whole path is exercised by test-only redactors (`QuarantineOnMarker`/`AlwaysQuarantine`, `daemon/tests/redaction.rs:274-314`).

## Design decisions & rationale

- **Mask-in-place over divert** (ARCHITECTURE.md §15; `daemon/LESSONS.md` §13): a false divert loses the whole event; a false mask loses only the blob. So every boundable span is masked and the divert stays a safety net for the unboundable case (Q2 ruling, `redaction.rs:55-58`).
- **Context-scaled confidence bars** (§15 / OQ-SEC-2): an `=` or `"key":` context raises confidence a value is a credential, so those passes use the lower 4.0/20 bar; a bare run has no context, so it needs 4.5/40 — explicitly to spare git-SHAs (~4.0 bits) and ≤31-char prefixed-ULID IDs (`redaction.rs:79-87`).
- **Extend recall via mechanism, not looser thresholds** (LESSON §13, banked from 2.0-SEC): residual (a) (JSON-value secrets) was closed by *adding the JSON pass* (human-ruled Option B, 2026-06-11; ARCHITECTURE.md §15), not by lowering a bar — lowering the bare bar to catch hex secrets would mask real git-SHAs, an irreducible trade (`redaction.rs:83-85`).
- **Measured, not asserted** (§15 "MEASURED + ACCEPTED-AND-OWNED"): detection quality is a number from a labeled corpus, with `catchable` fixed at corpus-definition (never derived from redactor output — so the floor pin is non-circular, `daemon/tests/redaction_recall.rs:529-535`), and ratchets only upward (`test_json_value_recall_ratchets_up`, `:454-493`).
- **Engine version as provenance contract:** `ENGINE_VERSION` bumps iff the recall bar moves (v2 → v3 for the JSON pass; L2 threshold-confirm did *not* bump it), so every persisted row's `redaction_engine_version` states which bar scrubbed it (`redaction.rs:64-67`).
- **Pure function** so redacted payloads are reproducible parts of the immutable log (§14; `redaction.rs:22`).

## Gotchas & sharp edges

- **The redactor is the NET, not the WALL.** The primary §15 control is keychain-refs-only (safety rule #4; ARCHITECTURE.md §15 line 361). Code must never *rely* on the redactor to launder a secret it knowingly handles.
- **Accepted residuals — these secrets WILL leak past the engine** (measured, human-accepted, ARCHITECTURE.md §15): **(b)** a hex-encoded secret indistinguishable from a git-SHA (~4.0 bits — irreducible by tuning, and the JSON-pass `is_id_shape` hex allowlist deliberately spares all-hex values, `redaction.rs:252-256`); **(c)** an adversarially-split secret in <20-char `.`-joined pieces (`daemon/tests/redaction_recall.rs:716-721`); **(<20ch-a)** sub-20-char JSON values (`:750-759`). The corpus retains all three so the envelope stays honest.
- **A 39-char bare high-entropy string survives; a 39-char JSON *value* does not** — the bare floor is 40 but the JSON pass catches ≥20 under a `"key":` (the 1.7 boundary test had to be re-isolated to a bare string for exactly this, `daemon/tests/redaction.rs:163-190`).
- **Over-masking is the accepted FP cost:** a non-ID-shaped high-entropy non-secret under a JSON key gets masked — fail-closed, loses only the blob (`redaction.rs:246-247`). FP-rate on the *realistic* corpus is 0.0, but the corpus is synthetic and finite (`redaction.rs:73-75`: "sufficient on the corpus, not a proven unique global optimum").
- **PEM detection is crude:** any payload containing `BEGIN` + `PRIVATE KEY` nukes the *entire* payload to `"[REDACTED-PEM]"` (`redaction.rs:100-102`) — structure is not preserved on this path.
- **Drift (minor, doc-vs-code):** ARCHITECTURE.md §15 says the `Redactor` is "owned by `policy`"; the code homes it in `eventstore/` and acknowledges the move as future (`redaction.rs:49-50`). The corpus doc-comment says "11 secrets / 11 non-secrets" (`daemon/tests/redaction_recall.rs:670`) but the corpus actually holds **12** secrets (matching ARCHITECTURE.md's "12 secrets / 11 non-secrets") — stale comment from before the L3 <20-char sample. `should_mask_token`'s doc still says "pass-2" (`redaction.rs:318`) — stale v2 numbering; it is pass 3.
- **Quarantine-divert is wired but MVP-unreached** — no production redactor ever fires it; it is kept alive purely by the test redactors. Don't mistake test coverage for production traffic.
- **`RedactionStatus` is frozen at two values** — quarantine is a writer-path decision, not a third status (`redaction.rs:34-36`).

## Connects to

- **[02-event-store.md](02-event-store.md)** — the consumer: the §15 fail-closed gate (`daemon/src/eventstore/mod.rs:210-226`), the `divert_quarantined` append (`mod.rs:305-347`), and the replay-side defense that quarantines any row somehow carrying `unredacted` (`mod.rs:722-734`).
- **[01-shared-contracts.md](01-shared-contracts.md)** — `RedactionStatus` (`shared/src/event_envelope.rs:54`), the `redaction_status`/`redaction_engine_version` envelope columns, and the `SensitiveOutputRedacted` event type (`shared/src/events.rs:81-90`, CONTRACT_VERSION 0.14.0).
- **[04-projections.md](04-projections.md)** — projections fold the *already-redacted* event in the same txn (`daemon/src/eventstore/mod.rs:283-284`); the `fts_events` embed sink is downstream of this gate.
- **[07-daemon-runtime.md](07-daemon-runtime.md)** — the single write-actor is the only thread that ever calls `append`, so the redactor is structurally un-bypassable for DB writes.
