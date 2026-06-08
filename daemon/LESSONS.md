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

## <a id="6"></a>6. Cross-restart locks — a persisted monotonic fencing high-water mark per `(resource_id, lease_kind)`; authority is a LIVE lease; single-instance via a std advisory file lock

**Date:** 2026-06-08.
**Source slice:** 1.4 (lease locks + fencing + pidlock + reaper, L1–L3 — `b3f7612`/`442149a`/`0347ac6`).

Cross-restart resource locks (ADR-008) are a SQLite `leases` row per `(resource_id, lease_kind)` carrying a **monotonic fencing token** that is a **persisted high-water mark** — minted `+1` on each acquire/reclaim and **never lowered**: `release` and the reaper NULL only the holder fields (`owner_id`/`acquired_at`/`heartbeat_at`/`expires_at`) and **keep** `fencing_token`. Because the high-water mark lives on disk in the row (not an in-memory counter, not `MAX()` over deletable rows), monotonicity **survives daemon restart** — the test pins a held+expired lease at token N → drop+reopen → next acquire mints N+1 and the pre-restart token N fails validation. Mint the token under `BEGIN IMMEDIATE` (the read-current-then-write must be one txn — LESSON §3 — or two racing acquires read the same max and collide); `renew`/`release`/`validate` are single atomic statements. `locks/` writes through the **same single writable `Connection`** the event store owns (EventStore exposes thin delegating `acquire_lease`/`renew_lease`/`release_lease`/`validate_lease_held`/`reap_leases` methods) — never a second writer (Forbidden #3).

**Authority is a LIVE lease, not a merely-unsuperseded token (Option B, human-ratified — safety rule #6).** The fencing oracle the Phase-2 gateway calls is `validate_held(resource_id, lease_kind, owner, token, now) = owner-match AND token == fencing_token AND expires_at > now`. "Stale" = **NOT a live lease — expired OR superseded** — §17 (line 387) leaves the word undefined, so the team pinned it: an **expired holder is rejected even if no one has superseded it** (the zombie-holder window), not only when a higher token was minted. The textbook supersession-only check (`token == latest`) was rejected because §17's daemon-crash recovery (line 384) re-derives an old `executing` action via the same "lease check" — authority must track **liveness**, not just supersession. Consequence flagged to Phase 2 (task 2.4): the gateway must **heartbeat/renew (or re-acquire)** a long-running action so it isn't fenced by its **own** lease expiry (a self-fence → false `fencing_conflict`).

Single-instance (the pidlock) is a **std advisory exclusive file lock** (`std::fs::File::try_lock`, stable since Rust 1.89 — toolchain is 1.93, so **no dependency**) on a lock file. The OS holds the lock against the open file descriptor → it is **auto-released on process death (including crash)** and **immune to PID reuse**: the OS-fd lock is the sole oracle. Never use a `kill(pid, 0)` liveness check — a reused PID makes it falsely conclude the old daemon is alive (false-positive block), and it can't detect a crashed holder. The PID written into the lock file is **diagnostic-only** (guarded on `set_len` success so a failed truncate can't leave misleading bytes). Fail-closed: `TryLockError::WouldBlock` → `AlreadyHeld`; `TryLockError::Error(io)` → a typed IO error — **never "acquired."**

The reaper's deterministic unit is `reap_once(now)` — it frees every lease past `expires_at` (`expires_at <= now` is the **exact complement** of `validate_held`'s `> now`, so a live lease can never be reaped) and returns the reclaimed set; the Tokio interval **spawn** is 1.6-bootstrap-wired (joins the outbox drainer spawn, §12). The whole `Lease` model is **daemon-internal** — no `proj_lease`, no `lease_` among the 22 frozen IDs, not one of the 10 §5.1 status machines, and the UI/Brain don't read the `leases` table → **no `shared/` surface, no CONTRACT_VERSION bump** (the 1.3 outbox precedent).

**Rule:** Cross-restart locks = a `leases` row per `(resource_id, lease_kind)` with a **persisted monotonic fencing high-water mark** (minted +1 on acquire/reclaim under `BEGIN IMMEDIATE`; release/reap keep it; survives restart); **authority is a LIVE lease** — `validate_held` = owner + token-match + `expires_at > now`, so "stale" = expired OR superseded (safety rule #6; the gateway must heartbeat long actions); single-instance via a **std advisory file lock** (OS-fd-held, auto-released on death, immune to PID reuse — never `kill(pid,0)`); the whole model is daemon-internal (no `shared/` surface, no CONTRACT_VERSION bump).

## <a id="7"></a>7. The UDS GatewayPort transport — length-prefix framing + getpeereid-first peer-auth + two version axes + `ServerFrame` frame-mux; ship the mechanism, wire the runtime at the bootstrap

**Date:** 2026-06-08.
**Source slice:** 1.5 (UDS GatewayPort transport, L1–L4 — `ae27f1d`/`fb941e8`/`c1c54e5`/L4).

The out-of-daemon GatewayPort transport (§6.4) is a UDS server with **4-byte big-endian length-prefix + JSON-body** framing, bounded by `MAX_FRAME_SIZE` (8 MiB) — an oversized frame is rejected **from the length prefix alone, before any body allocation** (anti-DoS). Peer-auth (safety rule #7 / ADR-004) is **`getpeereid()` called FIRST in the per-connection handler** — a peer whose uid ≠ the daemon-uid is dropped + disconnected before any frame is read; **`getpeereid`, NOT `SO_PEERCRED`** (Linux-only). Isolate the FFI in one `peer_uid(fd)` fn + **fail-closed** (a getpeereid error is never "authorized"; no uid-width cast that could silently truncate). Test the peer-auth **enforcement** path, not just the predicate — give the per-connection handler `peer_uid` as a **parameter** so a foreign-uid → disconnect is pinned deterministically (you can't spawn a foreign-uid peer in a unit test).

There are **two independent version axes** — don't conflate them: **`CONTRACT_VERSION`** is the §5.0 schema/codegen artifact version (the IPC method/error/frame schemas authored in `shared/` → schemars → the ui's generated validators; bumped per additive surface, gated by the byte-diff + 3-way verify); **`protocol_version`** is the §6.4 wire-handshake compatibility check (`HelloFrame`→`HelloAck`|`VersionSkewError`; `SUPPORTED_PROTOCOL_RANGE = {min:1,max:1}`, **daemon-authored** — the ui pins provisionally + reconciles to the daemon's range). Handshake-FIRST (after peer-auth): no method is served before a successful handshake; a version-skew or malformed/non-Hello first frame fails closed → structured error + disconnect.

When the architecture's LOCKED error-code set is found incomplete at implementation time, that's a **Finding → escalate the contract correction** (here: §6.4's 6-code set had no code for a bad-first-frame / handshake-required / malformed / bad-params violation; lead-ratified Option B added **`protocol_error`** — `unknown_method` is reserved for a genuine unknown method NAME). Frame-type multiplexing (§6.4, so the Terminal Channel can share the socket) is an **internally-tagged `ServerFrame`** (`{frame_type: rpc_response | subscription_push}`, Terminal reserved) — a JSON discriminant over the **unchanged** codec, NOT a binary type-prefix (which would retrofit the committed codec; the raw-byte Terminal encoding is a **measured Phase-3 call**). The dispatch **error split**: an **infra** error (a failed frame read) disconnects; a **client** `WireError` (`protocol_error`/`unknown_method`/bad-params) is returned and the connection continues. Read methods (`get_projection`/`subscribe`) read **read-only WAL** (`open_read_only`; single-writer preserved — LESSON §3); the projection-name→`proj_*` map is a **closed-enum const** (never client input → no SQL injection); an unfed projection returns its **empty table**, not an error.

The whole server's production wiring lives at the **1.6 bootstrap** (the runtime spawn): `bind()` + the accept-loop, the live `subscribe` delta-source (`EventStore::append`→a broadcast→the subscriber push, via a `try_clone` read/write split), the **platform cfg-guard** (getpeereid is macOS/BSD-only — a Linux build won't link), and an accept-loop **concurrency cap**. 1.5 ships the **mechanisms + the cross-language contract** (testable over a temp socket + with fed deltas); the runtime sources wire at 1.6 — the same "ship the deterministic unit, wire the production source at the runtime" pattern as the 1.3 outbox drainer + the 1.4 reaper. Reachability is stated honestly per layer (pub primitive, consumer-wired later) — never claimed live.

**Rule:** The UDS GatewayPort transport (§6.4): 4-byte-BE-length-prefix + JSON framing (oversized rejected pre-alloc; `MAX_FRAME_SIZE` 8 MiB); **`getpeereid()` peer-auth FIRST** (reject uid≠daemon-uid; NOT `SO_PEERCRED`; isolated unsafe + fail-closed; test the *enforcement* path via an injectable `peer_uid`); handshake-first; **two version axes** (`CONTRACT_VERSION` schema/codegen vs `protocol_version` wire-skew, `{1,1}` daemon-authored); a LOCKED-error-set gap → escalate the contract correction (`protocol_error`); frame-mux via an internally-tagged `ServerFrame` over the unchanged codec (Terminal encoding = a Phase-3 measured call); read-only-WAL reads + a closed-enum table map (no injection); ship the mechanism + contract now, wire the runtime source (bind/accept/subscribe-broadcast) at the 1.6 bootstrap.
