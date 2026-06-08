# /tdd brief — event_store (1.1)

> **⚠️ §15 SAFETY-INVARIANT SLICE.** This slice IS the single audited write path. It touches **redaction-before-persist**, **fail-closed-on-audit-write**, **single-mutator/single-writer**, and **secrets-never-in-events** (§15). Per policy: **`security-reviewer` runs** (invariant policy). The **redaction-sequencing safety-design question is RESOLVED (human, 2026-06-07): Option (a+)** — `redaction_status` column + `Redactor` trait + fail-closed gate + the high-recall token-prefix Redactor in 1.1; the **entropy fallback (`OQ-SEC-2`) is a BLOCKING Phase-1-exit task**, not a fast-follow (see Step-2.5 #1). Quote the invariants **by name** from `ARCHITECTURE.md §15` in tests + comments — do not paraphrase.

## Feature
The trust-core spine's first layer: a single-writer WAL SQLite **event store** — the append-only `events` table (the §7.1 envelope), the sole write-actor (append-by-`seq`, read-back-by-`seq`), the redaction gate before every INSERT, `user_version` forward-only migrations with backup/rollback, FTS5 scaffolding, and an injectable `Clock`+`IdGen` so the event log replays deterministically. Everything downstream writes through this.

## Use case + traceability
- **Task ID:** 1.1 (Phase 1 — daemon foundation; deps **0.4** + **0.5**, both landed)
- **Architecture sections it implements:** `§7` (data/state model), `§7.1` (event envelope contract — required/optional fields, enums, EventTypeRegistry), `§7.2` (event-derived SoT row), `§5.2` (the frozen IDs — `evt_`/`ws_`/etc.), `§15` (redaction-before-persist, fail-closed, secrets, single-writer), `§16` (migrations + backup/rollback + version-compat), `§18` (MEASURED event-store budgets), `§5.0` (the contract-SoT mechanism the `shared/` extension follows).
- **Authoritative DDL:** `docs/planning/DATA_MODEL.md §2.1` (the `events` table + indexes — pull the schema from here, not from prose) + `§2 line 36` (pragmas) + `§627` (migrations).
- **Related:** `docs/spikes/OQ-DATA-SPIKE-3.md` (the measured budgets + the background-checkpoint Phase-1 note); LESSONS §2 (the §5.0 contract pattern this `shared/` extension follows); the frozen `shared/` crate (0.5, 06f9576) — reuses `actor_type`, `event_id`/`EventId`, the IdKinds.

## Two layers (note the cross-doc-invariant split)
**(L1) `shared/` contract extension (§5.0 Option A — same mechanism as 0.5):** the **Event envelope** struct + **3 NEW enums** — `source_type` (EM §8), `sensitivity` (`public|internal|confidential|secret|restricted`), `visibility` (`user|project|workspace|system`) — reusing the frozen `actor_type` + `EventId` + IdKinds. Regenerate the published JSON Schema + Zod/Pydantic consumers; bump `CONTRACT_VERSION`. This is a **cross-doc invariant change** → I (orchestrator) write the Appendix A "Event envelope" row + the `daemon/CLAUDE.md` cross-doc row at Step 9.

**(L2) `daemon/src/eventstore/` impl:** schema (DDL + pragmas), the single write-actor (+ redaction gate + fail-closed), migrations (backup/rollback), read-only readers, FTS5 scaffolding, injectable `Clock`+`IdGen`.

## Acceptance criteria (what "done" means)
- [ ] WAL opened with the §2 pragmas: `journal_mode=WAL`, `synchronous=NORMAL`, `foreign_keys=ON`, `busy_timeout=5000`, `fullfsync=OFF` (ADR-003 / §18 caveat).
- [ ] `events` table matches `DATA_MODEL §2.1` (all columns incl. reserved `payload_hash`/`previous_event_hash`; the 6 indexes incl. `ux_events_seq`, `ux_events_idempotency` partial).
- [ ] **Single write-actor:** exactly one writer task owns the write connection; all other access is a **read-only WAL connection** (forbidden-pattern #3). Append assigns a monotonic `seq` (canonical total order); read-back-by-`seq` returns events in `seq` order.
- [ ] **Redaction-before-persist (§15) — Option (a+):** the events table/envelope has a **`redaction_status`** column; every payload routes through the `Redactor` trait (the high-recall token-prefix engine) before INSERT; the writer **fail-closes** — it **refuses to persist any event with `redaction_status='unredacted'`** (or when redaction didn't run). On an unredactable high-confidence secret → quarantine + `SensitiveOutputRedacted` (§15/EM §23). Entropy fallback (`OQ-SEC-2`) is the separate blocking Phase-1 task — out of 1.1's scope but its absence must not weaken the gate.
- [ ] **Fail-closed on audit-write (§15/§17):** an INSERT failure returns a typed error and aborts the operation — never a silent success.
- [ ] **Secrets never in events (§15):** a payload carrying a secret-shaped token is redacted/quarantined before persist (pinned by a test); only `keychain_ref` pointers may appear.
- [ ] `idempotency_key` dedup: a second append with the same key is rejected (the partial unique index).
- [ ] `user_version` migrations: forward-only, ordered, run in a txn **before serving**; **backup** (`nexusops.db` → `nexusops.db.bak-<from>`) before any `user_version` raise; **restore on failure**; `app_version ↔ min/max user_version` floor → refuse-safe on "DB newer than I understand" (§16).
- [ ] Unknown `event_version` on read → **degraded marker, no crash**; corrupt `payload_json` → **quarantine**, no crash (§17).
- [ ] **Deterministic golden-log replay:** with injected `Clock`+`IdGen`, a fixed event sequence appends + reads back byte-identical (the §14 golden-log contract).
- [ ] `shared/` envelope + 3 enums frozen via §5.0 (schema regenerated, Zod/Pydantic + 3-way verify green, `CONTRACT_VERSION` bumped); `cargo fmt --check` + clippy `-D warnings` clean (the preflight gate, now incl. fmt).
- [ ] FTS5: the virtual-table scaffolding exists over a redaction-safe text projection (population wired with the AuditTrail projection in 1.2 — see Step-2.5 #3).

## Files expected to touch
**New (`shared/` — L1, §5.0):** `shared/src/event_envelope.rs` (envelope struct + `source_type`/`sensitivity`/`visibility` enums); regenerated `shared/contracts/schema/*.json` + the TS/Python consumers; `CONTRACT_VERSION` bump in `shared/src/lib.rs`.
**New (`daemon/` — L2):** `daemon/src/eventstore/{mod,schema,writer,migrations,reader}.rs`; `daemon/src/eventstore/redaction.rs` (the `Redactor` seam + the gate; impl per Step-2.5 #1); the injectable `Clock`+`IdGen` (likely `daemon/src/model/` or a `clock`/`idgen` module — confirm placement at Step-2.5).
**Modified:** `daemon/Cargo.toml` (rusqlite bundled+WAL, the migration crate, ulid); depends on the `shared` crate.

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN.

## RED test outline (Step 2)
`daemon/src/eventstore/` tests (+ the `shared/` 3-way for L1):

1. **`test_append_then_read_by_seq`** — append N events, read back in `seq` order. Asserts: monotonic `seq`, round-trip equality. Why: §7.1 / §7 (seq canonical order).
2. **`test_seq_is_canonical_not_occurred_at`** — out-of-order `occurred_at` still orders by `seq`; both timestamps kept. Why: §7.1 (clock skew, EM §23).
3. **`test_single_writer_readers_are_readonly`** — readers open read-only WAL; a write attempt off the read path fails/compiles-out. Why: forbidden-pattern #3 / §15 single-writer.
4. **`test_redaction_gate_fail_closed`** (THE load-bearing §15 assertion) — the writer **refuses to persist** any event whose `redaction_status` would be `'unredacted'` / when redaction didn't run; a redacted event persists with `redaction_status` set accordingly. Why: **§15 redaction-before-persist (by name)** — fail-closed.
5. **`test_secret_token_redacted_or_quarantined`** — a payload with a `ghp_`/`sk-`/PEM token is redacted (or quarantined + `SensitiveOutputRedacted`) before persist. Why: **§15 secrets-never-in-events (by name)**.
6. **`test_audit_write_failure_fails_closed`** — a simulated INSERT failure returns a typed error, aborts, no silent success. Why: **§15/§17 fail-closed-on-audit-write (by name)**.
7. **`test_idempotency_key_dedup`** — duplicate `idempotency_key` rejected. Why: §7.1 / AG §16.1.
8. **`test_migration_forward_only_with_backup_rollback`** — raising `user_version` writes `.bak-<from>` first; a failing migration restores + surfaces the error; downgraded binary refuses on too-new DB. Why: §16 (backup/rollback + version floor).
9. **`test_unknown_event_version_degrades`** — unknown `event_version` on read → degraded marker, no crash. Why: §17.
10. **`test_corrupt_payload_quarantined`** — invalid `payload_json` → quarantine, no crash. Why: §17.
11. **`test_golden_log_deterministic_replay`** (integration) — injected `Clock`+`IdGen` → fixed sequence appends + reads byte-identical. Why: §14 golden-log contract.
12. **(L1, `shared/`)** envelope + 3 enums: presence + snake_case serialize + reject-unknown + the 3-way (Rust/Zod/Pydantic) equality + schema-diff gate. Why: §5.0 / LESSON §2.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** NEW — the Event envelope contract + `source_type`/`sensitivity`/`visibility` enums.
- **Orchestrator doc rows to write hot (Step 9):** Appendix A "Event envelope" row + the `daemon/CLAUDE.md` cross-doc row (envelope + the 3 enums); update the "Status state machines" sibling context if needed. **Implementer does NOT edit Appendix A / `ARCHITECTURE.md` / `daemon/CLAUDE.md` / `MVP_TASKS.md` / `LESSONS.md`** — flag at Step 9.

## Things to flag at Step 2.5
1. **✅ RESOLVED (human, 2026-06-07) — Redactor sequencing = Option (a+).** 1.1 ships: the **`redaction_status` column** (#2) + the **`Redactor` trait** + the writer's **fail-closed gate** (no INSERT unless redaction ran; an event may **NEVER** persist `redaction_status='unredacted'`) + the **high-recall token-PREFIX Redactor** (`ghp_/github_pat_/sk-/xox/AKIA/PEM/JWT`). The **Shannon-entropy fallback (`OQ-SEC-2`) is NOT a fast-follow flag — it is a BLOCKING Phase-1-exit task** (MVP_TASKS Phase 1) gating Phase-1 acceptance, so full secret-detection recall can't drift. **Pin the §15 invariant with the fail-closed test (test 4) — that's the load-bearing assertion.** You may pin tests 4/5 now.
2. **✅ RESOLVED (human, 2026-06-07) — ADD the `redaction_status` column.** The events table/envelope gets a **`redaction_status`** field; the writer sets it and **refuses to persist `unredacted`** (test 4). This is a **cross-doc invariant** (events DDL `DATA_MODEL §2.1` lacked it) → I write the Appendix A / §7.1 / DATA_MODEL §2.1 rows atomic in the round at Step 9. Consider a `redaction_engine_version` provenance field too (your call — flag at Step 9 if added).
3. **FTS5 sequencing.** 1.1 builds the FTS5 virtual table; the AuditTrail projection it indexes lands in **1.2**. Confirm 1.1 = schema/scaffolding only (indexing wired in 1.2), not a populated index.
4. **Migration crate.** `refinery` vs `rusqlite_migration` vs hand-rolled `user_version`. My default vote: **`rusqlite_migration`** (lightweight, embedded ordered SQL, matches the §627 model). Confirm.
5. **`Clock`+`IdGen` placement.** A `daemon/src/clock.rs`+`idgen.rs` (or in `model/`) injected into the writer. My default vote: small dedicated modules injected via constructor (so tests pass fakes). Confirm placement.

## Dependencies + sequencing
- **Depends on:** 0.4 (§18 budgets — landed), 0.5 (frozen `shared/` contracts — landed, 06f9576), the §5.0 mechanism.
- **Blocks:** 1.2 (projections read the event log + the AuditTrail projection FTS5 indexes), 1.3 (outbox), and ultimately the Gateway (Phase 2 writes through this).
- **Parallel-with:** the ui track (mock GatewayPort + fixture projections).

## Estimated commit count
**2–3.** The **safety-critical redaction gate + writer** gets its **own commit** (never bundled). Suggested: (1) `shared/` envelope contract extension (§5.0 cross-doc — own commit for traceability); (2) eventstore schema + single-writer + migrations + readers; (3) the redaction gate + fail-closed (safety pin — own commit). The implementer + Step-2.5 settle the exact split.

## Lessons-logged candidates anticipated
- **Convention candidate** — the single-write-actor pattern (one writer task; everything else read-only WAL) as the enforced shape for §15 single-mutator.
- **Architecture-doc note** — resolve the `redaction_status` column question (§15 vs DATA_MODEL §2.1) once decided.
- **Future TODO** — the OQ-SEC-2 entropy-fallback redactor is a **BLOCKING Phase-1-exit task** (already in MVP_TASKS Phase 1; gates Phase-1 acceptance — not a loose flag); the background-checkpoint thread (0.4 note) to flatten the WAL p99 tail (operational, when the tail matters).

## How to invoke
Session is oriented (continuation). Re-read this brief, then `/tdd event_store`. **Do not write the redaction tests (4/5) to a fixed shape until the human answers Step-2.5 #1** (sequence the store mechanics — events table, envelope, WAL, seq, migrations, golden-log — first; the redaction gate's exact shape lands at Step-2.5). Pull the DDL from `DATA_MODEL §2.1`. Cross-doc rows are mine at Step 9.
