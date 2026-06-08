# LESSONS.md — NexusOps (the Rust daemon (trust core))

> Full prose for every lesson logged during work in `daemon/`. The compact index lives in `daemon/CLAUDE.md` "Lessons logged" table.
>
> **Lesson numbers are stable IDs.** New lessons get the next sequential number. Numbers may be referenced from code comments, commit messages, and cross-references between lessons. **Don't reorder; don't reuse a deleted number's slot.**
>
> **Lessons start at §1.** Each code area has its own lesson sequence — lessons don't carry across code areas.

---

## Lesson format

```markdown
## <a id="N"></a>N. <Short topic> — <one-line rule>

**Date:** YYYY-MM-DD.
**Source slice:** <slice-id or commit hash>.

<2-5 paragraphs explaining: what was discovered, why it matters, how to
apply the rule, what edge cases are still open. Cite file:line references
where applicable.>

**Rule:** <one-sentence summary, same as the heading subtitle>.
```

---

## <a id="1"></a>1. Broken cargo/rustc proxies — `rustup default stable` won't fix them; repoint the shims

**Date:** 2026-06-07.
**Source slice:** 0.4 (`OQ-DATA-SPIKE-3` env-finding; cleared under ③).

During Phase-0 the daemon-track build was blocked: `~/.cargo/bin/{cargo,rustc}` (and 11 other proxies, 13 total) were **broken dangling symlinks** pointing at a non-existent `/Users/nozzins` path — the artifact of a moved home directory. The real stable toolchain (1.93.0) was intact at `~/.rustup/toolchains/stable-aarch64-apple-darwin/bin`.

The non-obvious part: **`rustup default stable` did NOT fix it.** `rustup` only (re)creates *missing* proxies; it leaves *existing-but-broken* symlinks untouched. The obvious first move silently no-ops. The working fix was to repoint every broken proxy at the local `rustup` binary directly — `ln -sf rustup ~/.cargo/bin/<proxy>` for each of the 13 — after which the plain shims resolve (no PATH workaround needed). Verify with `cargo --version && rustc --version` (expect 1.93.0), `cargo clippy`, and a real `cargo build`.

Edge cases still open: if the home dir moves again the same breakage recurs; a `rustup self uninstall` + reinstall would also fix it but is heavier. Prefer the targeted shim-repoint.

**Rule:** When `~/.cargo/bin` shims are *broken* (dangling symlinks, e.g. after a home-dir move), `rustup default stable` does **not** repair them (rustup only recreates *missing* proxies) — repoint each broken proxy to the local `rustup` binary (`ln -sf rustup ~/.cargo/bin/<proxy>`), then verify with the plain shims.

## <a id="2"></a>2. The wire value is the contract; the SoT propagation pattern is §5.0 — follow it for every contract addition

**Date:** 2026-06-07.
**Source slice:** 0.5 (shared contract freeze, `OQ-DATA-SPIKE-5`).

The 0.5 freeze settled two conventions every future contract surface (event-type registry, GatewayPort schema, action-type catalog, any new enum/ID) must follow.

**(a) The serialized wire/`TEXT` value is the contract — not the in-language identifier.** Enums serialize to exact snake_case strings via `#[serde(rename_all = "snake_case")]`; each language uses its idiomatic identifier (Rust `PascalCase` variants, TS string-literal unions, Python `Enum`) but the *string on the wire / in the `TEXT status` column* is what's frozen. A round-trip test (`test_every_state_machine_value_present_and_serializes`) pins this — never assume the identifier and the wire value match. Closed-enum / reject-unknown holds end-to-end (serde closed enums → JSON-Schema `enum` → `z.enum` → Pydantic): unknown values are rejected at every boundary, which is the fail-closed posture (§15), not optional.

**(b) The source-of-truth propagation is fixed by `ARCHITECTURE.md §5.0` (Option A):** Rust `shared` crate = native authority (newtypes for IDs, serde-closed enums) → `schemars` emits a **first-class, versioned, diff-gated** JSON Schema artifact (`shared/contracts/schema/`) → TS Zod + Python Pydantic are **generated** from that artifact → a self-contained 3-way value-set equality harness proves they agree. Do **not** hand-author a consumer's types or invert the authority (an external IDL/codegen-into-the-trust-core was rejected — it generates bare types in the safety-critical module and fights the newtype posture). Every new contract addition extends the Rust authority and regenerates the artifact; the diff-gate catches drift.

**Rule:** Freeze the *wire value* (snake_case `TEXT`), not the identifier — pin it with a round-trip test; and author every contract in Rust (`shared/`) per §5.0, regenerating the published schema + consumers (never hand-write a consumer or invert the authority).

## <a id="3"></a>3. Single-write-actor — one writable `Connection`; everything else read-only WAL; canonical `seq` in one `BEGIN IMMEDIATE` txn

**Date:** 2026-06-07.
**Source slice:** 1.1 L2 (event store).

The §15 single-mutator invariant has a concrete enforced shape in the daemon: **exactly one writable rusqlite `Connection`** (owned by the write-actor); **every other access opens a read-only WAL connection** (`Connection::open_with_flags(..., SQLITE_OPEN_READ_ONLY)` / `open_read_only`). A read-path that can't even *construct* a writable handle can't violate single-mutator by construction — enforce it at the type/connection level, not by convention.

The canonical `seq` (the total event order, §7.1) must be assigned **atomically**: the `SELECT max(seq)+1` and the `INSERT` go in **one `BEGIN IMMEDIATE` transaction**, not two statements. `BEGIN IMMEDIATE` takes the write lock up front, so two concurrent appends can't read the same max and produce a gap/duplicate. A borrow-checker-only "one writer" guarantee is **not** enough — the *transaction boundary* is what makes the ordering atomic (1.1 L2 security-reviewer high finding).

Pair this with an **injectable `Clock` + `IdGen`** (constructor-injected; fakes in tests) so the event log — `seq`, `event_id` (ULID), `recorded_at` — replays **byte-identically** in the golden-log test. Determinism-for-testability is a daemon-wide seam (root `CLAUDE.md` "Determinism-for-testability"), not a per-slice afterthought.

**Rule:** One writable `Connection` (the write-actor); all other access is read-only WAL (`open_read_only`). Assign the canonical `seq` via `SELECT max+1` + `INSERT` inside one `BEGIN IMMEDIATE` txn (atomic order, not borrow-checker-only). Inject `Clock`+`IdGen` so the log replays deterministically.

## <a id="4"></a>4. Projections fold in-band in the event-commit txn; savepoint-isolated per-projector; offsets never ahead of rows; recovery = catch-up + rebuild

**Date:** 2026-06-07.
**Source slice:** 1.2 (projection engine, L1–L3 — `c8c6c72`/`a1dc482`/`e66c659`).

Projections are applied **in-band, inside the same `BEGIN IMMEDIATE` event-commit txn** as the event INSERT (`§7` "within the one event-commit transaction"; DATA_MODEL `§2.4`), *after* the §15 redaction gate — so a projector only ever sees the **already-redacted** payload, and a reader never sees an event whose (healthy) projections haven't applied. Each projector runs under a **uniquely-named SAVEPOINT**: a projector **logic error** rolls back *that projector's* rows + offset and marks `projection_offsets.state='degraded'` (skip-and-continue), while a **Db error** (or the gate) aborts the whole append closed (§15). `mark_degraded` is written *outside* the savepoint but *inside* the outer txn — so it shares the event's commit/rollback fate (atomic either way). Offsets advance in the same txn — **never ahead of the rows they represent** (crash-safe).

Recovery has two paths, both built on the strict `seq > last_seq` boundary: **catch-up replay** (wired into `open`, runs every start) folds pending events per projector — and is a **strict no-op on an already-current log** (the `>` boundary, not `>=`, guards non-idempotent counters against double-counting on reopen; pin it with a "catch-up-noop-when-current" test). **Full rebuild** truncates the derived tables (a compile-time-`const` list that **excludes `events`**) + replays all — **byte-equivalent to the incremental fold**, and raw `events` are never mutated (§7.2 "projection corruption must not corrupt raw events"). A degraded projector advances past the bad event on later successes (sticky `last_seq`), never stranded. In-band apply and replay share one `apply_one`, so the degrade semantics agree by construction. Status columns **bind the frozen §5.1 `shared/` enums** (reject-unknown), never bare strings. Guard `advance_offset`/`mark_degraded` with a rows-affected check (`expect_one_row`) — a missing offset row must not silently no-op (it would re-fold every reopen). **Open caveat:** `catch_up_replay` currently reconstructs via the *strict* read path → one corrupt event row aborts `open` (daemon-won't-start); the §17-degradable-replay fix is task 1.6 (decide skip-vs-refuse semantics + audit-integrity).

**Rule:** Fold projections in-band in the event-commit txn (after the redaction gate), each under its own SAVEPOINT (logic error → degrade+skip that projector; Db error → fail the append closed); advance offsets in the same txn (never ahead of rows); recover via catch-up replay (strict `seq>last_seq`, no-op on current) + full rebuild (truncate a const derived-table list, replay-all, byte-equivalent, raw events untouched); bind status columns to the frozen §5.1 enums.

## <a id="5"></a>5. External side-effects go through the transactional outbox: written in the event-commit txn; §15 sync-sink; at-least-once with reset-on-open

**Date:** 2026-06-08.
**Source slice:** 1.3 (transactional outbox, L1–L2 — `343cc09`/`707845a`).

Every external side-effect (Brain/GitHub/Linear/notifier/JSONL mirror) goes through the **transactional outbox**, never a direct call. Outbox rows are written **in the same event-commit txn** as the event + projections (the transactional-outbox pattern: a fact is **recorded-iff its delivery-intents are**, and **delivered-iff recorded**) — atomic with everything else; a txn abort (e.g. the redaction gate) leaves 0 events **and** 0 outbox rows. The outbox is the **§15 *sync* sink** (root rule #3 "same redactor gates persist+embed+**sync**"): the per-destination payload derives **only from the already-redacted stored event**, never re-fetched raw, and per-destination filtering only *removes* fields (e.g. Brain = envelope minus restricted/secret) — it can never re-introduce a secret. Rebuild/replay must **never re-emit outbox rows** (outbox ∉ the rebuild table list) — a read-model reconstruction must not re-deliver historical events to the outside world.

Delivery is **at-least-once** with a deterministic drainer (`drain_once(clock, dest)` — the Tokio-loop unit; the loop spawn itself is 1.6-bootstrap-wired): claim `in_flight` **committed before** `deliver` (the no-loss window), transition `delivered` / `failed`+`backoff(retry_count)` / `dead` (terminal, or retryable past a bounded `MAX_RETRIES` — bounded dead-letter, never an unbounded loop). Crash recovery = **`reset_in_flight` (in_flight→pending) wired into `open`** (single-instance-safe via the pidlock); consumers must be **idempotent** (dedup). The due-comparison `next_attempt_at <= now` is a **lexical string compare** — only sound because the `Clock` RFC3339 contract is **UTC `Z`-suffixed and format-consistent** (verify the time source emits `Z`; treat it as a Clock contract). Use a separate `IdGen` counter for `out_` ids so event-id/golden-log determinism is unchanged.

**Rule:** Route all external side-effects through the outbox: write rows in the event-commit txn (recorded-iff-intended); treat it as the §15 sync sink (payload from the already-redacted event, filter-only; rebuild never re-emits); deliver at-least-once via a deterministic `drain_once` (in_flight claim-before-deliver, reset-on-open recovery, backoff + retryable/terminal + bounded dead-letter, idempotent consumers); keep timestamps UTC-`Z` for the lexical due-compare.
