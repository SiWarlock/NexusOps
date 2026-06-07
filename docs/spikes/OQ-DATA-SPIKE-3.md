# OQ-DATA-SPIKE-3 — SQLite single-writer event-store load test

| | |
|---|---|
| **MVP task** | 0.4 (Phase 0 — pre-build spikes) |
| **Open question** | `OQ-DATA-SPIKE-3` — SQLite single-writer write-contention load test |
| **Spec anchors** | `ARCHITECTURE.md §18` (budgets — `[OPEN]`), `§7`/`§7.1` (envelope + single writer), `§7.2` (one-commit-many-projections), `DATA_MODEL.md §2.1` (single-writer pragmas), `DECISIONS.md` ADR-003 |
| **Mode** | in-repo, agent-run (real Rust + rusqlite/bundled SQLite harness) |
| **Status** | ✅ RESOLVED — single-writer holds at N=20 with large margin; §18 numbers recommended below |
| **Date** | 2026-06-07 |
| **Gates** | Phase 1 (1.1) event-store freeze — now binds to measured thresholds, not guesses |

---

## 1. Decision (TL;DR)

**The single-writer event-store design holds at the §18 design target (N=20 concurrent
mutating agents) with ~12–19× latency headroom, and the contention ceiling is not reached
even at N=100 (5× the target).**

- **intent→committed p95 @ N=20:** **5.35 ms** on a fresh DB, **8.44 ms** against a
  **1,000,000-event** DB — vs the `< 100 ms` budget. ✅
- **Reader p95 under write load @ N=20:** **0.08 ms** (fresh) / **0.38 ms** (1M events) —
  WAL readers do not contend with the single writer. ✅
- **Ceiling:** commit p95 stays **< 100 ms across the entire N = 1 → 100 sweep**
  (p95 = 28 ms at N=100). The single writer is **not saturated** at 5× the design load.
- **`BEGIN CONCURRENT` is not needed and is not relied upon** (per ADR-003): there is
  exactly one writer, so there is no SQLite write-write lock contention to mitigate. The
  measured cost is serialized-writer throughput + commit, not lock fighting.

ADR-003's `synchronous=NORMAL` is confirmed as the right shipping value: at N=20 the
`synchronous=FULL` durability upgrade costs only ~0.3 ms p95 and ~5 % throughput on this
platform, because macOS `fsync()` does not force a platter flush (see §6 caveat).

---

## 2. Method

### 2.1 What was modelled

A **throwaway** Rust harness (`daemon/spikes/sqlite-loadtest/`, see §7) that mirrors the
load-bearing shape of the real commit path — it is **not** the real event store and is
**not a load-bearing import** (its own `[workspace]` root; safe to delete once §18 is
committed).

Design mirrors `DATA_MODEL.md §2.1`: **one long-lived write connection owns all writes**
(a serialized write-actor fed by an `mpsc` channel); **readers use separate read-only WAL
connections**. The N "agents" are **closed-loop**: each submits one intent and blocks until
the commit is durable before submitting the next — so the measured latency is exactly
**submit → durable-commit (queue wait + commit)**, the metric §18 names.

Each commit is **one transaction** writing what the real event-commit txn writes
(`§7.1` + `§7.2` "a single event may update multiple projections within the one
event-commit transaction"):

| Per-commit transaction writes | Detail |
|---|---|
| 1 × `events` row | full ~25-column envelope (`§7.1`) |
| all **6 indexes** on `events` | incl. the partial `UNIQUE` idempotency index |
| 2 × `object_refs` rows | with the `FK → events(event_id)` (`foreign_keys=ON`) |
| 1 × `fts_events` (FTS5) row | the redaction-safe audit text index |
| 2 × projection upserts | `proj_project_activity`, `proj_session` (`ON CONFLICT … DO UPDATE`) |

Payload is a representative ~400-byte JSON blob, serialized **off** the writer's critical
path (in the agent), matching a design where the request handler serializes and the
write-actor just binds + commits.

### 2.2 Workload regimes

- **Ceiling sweep:** N ∈ {1, 5, 10, 20, 30, 50, 100} at `synchronous=NORMAL` (the locked
  value), 2 concurrent readers, fresh DB.
- **Durability comparison:** N=20 at `synchronous=FULL` vs `NORMAL`.
- **Scale test:** N=20 against a **pre-seeded 1,000,000-event** DB (events + object_refs +
  FTS at production scale), at both `NORMAL` and `FULL`.

Closed-loop with **zero think time** is a deliberate **conservative upper bound**: real
agents have multi-second LLM round-trips between mutations, so true contention is far lower.

---

## 3. Environment

| | |
|---|---|
| Machine | Apple **M4 Max**, 48 GB RAM, `Mac16,5` |
| OS / FS | macOS 26.4.1 (build 25E253), APFS on internal NVMe SSD |
| Toolchain | `rustc`/`cargo` **1.93.0** (stable), `rusqlite` **0.32** (bundled SQLite ~3.46) |
| Pragmas (locked, ADR-003) | `journal_mode=WAL`, `synchronous=NORMAL`, `foreign_keys=ON`, `busy_timeout=5000` |
| `wal_autocheckpoint` | 1000 pages (SQLite default — left at default) |
| Readers | 2 concurrent read-only WAL connections (`query_only=ON`) |

> **Hardware note.** The M4 Max is a high-end target. Because NexusOps is a **local desktop
> app**, the dev machine *is* a representative target (it runs on the user's Mac), but a
> low-end target (M1 base / 8 GB) will be slower. With 12–19× headroom at N=20, even a 5×
> hardware penalty stays within budget. The `§14` CI perf gate should run against the CI
> hardware baseline, not a dev M4 Max — see §6.

---

## 4. Results

### 4.1 Ceiling sweep — fresh DB, `synchronous=NORMAL` (+ N=20 FULL)

Latencies in **ms**; `thrpt` = commits/sec; `c_` = intent→committed, `r_` = reader.

| agents | sync | c_p50 | c_p95 | c_p99 | c_max | c_mean | thrpt/s | r_p50 | r_p95 | r_max |
|---:|:--:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1   | NORM | 0.15 | 0.27 | 0.74 | 2.09 | 0.17 | 5950 | 0.01 | 0.05 | 0.97 |
| 5   | NORM | 0.91 | 1.64 | 2.16 | 5.14 | 0.93 | 5360 | 0.02 | 0.09 | 3.99 |
| 10  | NORM | 1.93 | 3.09 | 3.97 | 5.59 | 1.90 | 5258 | 0.02 | 0.08 | 2.18 |
| **20** | **NORM** | **3.85** | **5.35** | **6.17** | **7.99** | **3.73** | **5350** | **0.02** | **0.08** | **1.60** |
| 30  | NORM | 5.88 | 9.40 | 16.06 | 31.99 | 6.15 | 4869 | 0.02 | 0.08 | 20.22 |
| 50  | NORM | 10.39 | 15.27 | 24.06 | 31.77 | 10.73 | 4649 | 0.02 | 0.08 | 9.38 |
| 100 | NORM | 22.38 | 28.48 | 30.87 | 36.84 | 22.47 | 4437 | 0.02 | 0.09 | 6.03 |
| 20  | FULL | 3.87 | 5.62 | 6.47 | 88.53 | 4.37 | 4560 | 0.02 | 0.07 | 50.11 |

### 4.2 Scale test — N=20 against a 1,000,000-event DB

| agents | sync | c_p50 | c_p95 | c_p99 | c_max | c_mean | thrpt/s | r_p50 | r_p95 | r_max |
|---:|:--:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 20 | NORM | 4.04 | **8.44** | 47.48 | 63.96 | 5.00 | 3997 | 0.09 | 0.38 | 8.68 |
| 20 | FULL | 4.42 | 8.11 | 47.72 | 59.76 | 5.27 | 3792 | 0.09 | 0.38 | 14.54 |

---

## 5. Analysis

1. **Does single-writer hold at N=20? — Yes, decisively.** p95 = 5.35 ms (fresh) /
   8.44 ms (1M events) against a 100 ms budget (**~12–19× headroom**). Throughput
   ~4,000–5,350 commits/s.
2. **Documented ceiling.** p95 stays < 100 ms through **N=100** (the largest tested);
   p95 grows roughly linearly with N (writer serialization) but is only **28 ms at N=100**.
   The single writer is **not the bottleneck at 5× the design target**. Extrapolating the
   near-linear trend, p95 would approach 100 ms only somewhere around **N ≈ 300+** — well
   beyond any realistic local-agent count.
3. **Readers are effectively free under write load.** Reader p95 ≤ 0.38 ms even against a
   1M-event DB with 20 agents hammering the writer — WAL gives readers a consistent
   snapshot without blocking the writer, exactly as the design assumes (`§7.2`).
4. **Durability (FULL vs NORMAL).** At N=20 the p95 delta is within noise (~0.3 ms) and
   throughput ~5 % lower — because on macOS `fsync()` ≠ `F_FULLFSYNC` (caveat §6). NORMAL
   (the ADR-003 locked value) is confirmed as correct; FULL is not worth its (small) cost
   given NORMAL already survives app crash.
5. **The tail is the WAL checkpoint, not lock contention.** p99/max spikes (47–64 ms at
   1M events; the 88 ms FULL max at §4.1) line up with the **1000-page `wal_autocheckpoint`**
   firing on the committing thread (an inline checkpoint blocks that one commit). It is the
   single tail risk and it is still **within the 100 ms budget**. Mitigation if the tail
   ever matters: a **dedicated background-checkpoint thread** (`wal_autocheckpoint=0` +
   periodic manual `wal_checkpoint(PASSIVE)` off the hot path) — flagged as a Phase-1
   implementation note, not a blocker.

---

## 6. Caveats (conditions the numbers assume)

- **macOS fsync semantics.** SQLite uses `fsync()` (not `F_FULLFSYNC`) unless
  `PRAGMA fullfsync=ON` (default OFF, not enabled here — matches what the daemon will
  ship). On macOS `fsync()` does **not** force the drive's cache to platter, so both NORMAL
  and FULL are durable against **app/OS crash** but a **power-loss could lose the last
  WAL frames since the previous checkpoint**. This is standard SQLite-on-macOS behavior; the
  hash-chain / tamper-evidence work is post-MVP (`DATA_MODEL.md`, `[DEFERRED — ADR-003]`).
  Enabling `fullfsync` would raise p95 by ~1–2 orders of magnitude and is **not recommended**
  for the MVP.
- **Hardware.** M4 Max / NVMe SSD — see §3 note. Run the `§14` perf gate on the CI baseline.
- **Workload shape.** Closed-loop, zero think time = conservative upper bound on contention.
- **Run length.** Short bursts (≤ 20k timed commits), not a multi-hour soak. The 1M-event
  scale test covers DB-size growth; a soak is only worth it if we ever approach budget.

---

## 7. Harness location & re-run

Throwaway crate: **`daemon/spikes/sqlite-loadtest/`** (standalone `[workspace]` root; not
imported by anything; delete once §18 is committed).

```bash
# NOTE: the ~/.cargo/bin cargo/rustc shims were broken dangling symlinks at spike
# time (see Flags §9) — now REPAIRED (proxies repointed to the local rustup), so
# plain `cargo` works with no PATH workaround. (If you hit the old breakage on a
# fresh checkout, prepend: export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH")

# full sweep (N=1..100 NORMAL + N=20 FULL):
cargo run --release --manifest-path daemon/spikes/sqlite-loadtest/Cargo.toml

# single config, e.g. the 1M-event scale test:
cargo run --release --manifest-path daemon/spikes/sqlite-loadtest/Cargo.toml -- \
  --agents 20 --commits 1000 --preseed 1000000 --sync normal
# flags: --agents N --commits PER_AGENT --readers R --sync normal|full --preseed COUNT
```

---

## 8. ⭐ Recommended committed §18 budget set (for the orchestrator to write into `ARCHITECTURE.md §18`)

> Per the brief I **flag** these numbers; I do **not** edit §18 (orchestrator territory).
> Keep the user-facing SLO at the PRD number, and add tighter **`§14` CI regression guards**
> set at the measured baseline + margin so a regression surfaces long before the 100 ms ceiling.

| Metric | Recommended committed budget | Basis |
|---|---|---|
| **Event write latency (intent→committed) p95 @ N=20** | **`< 100 ms` (SLO, PRD §19.6) — KEEP** | measured 5.35 ms fresh / 8.44 ms @1M (≈12–19× headroom) |
| → **§14 CI regression guard, p95 @ N=20** | **`< 30 ms`** (new) | catches a ~4× regression; tolerates 1M-event scale + CI-hardware variance |
| → **§14 CI regression guard, p99 @ N=20** | **`< 75 ms`** (new) | measured p99 ~47 ms @1M incl. checkpoint stalls |
| **Reader latency under write load, p95 @ N=20** | **`< 100 ms` SLO**, **§14 guard `< 10 ms`** | measured ≤ 0.38 ms |
| **Sustained single-writer throughput** | **floor `≥ 1,500 commits/s`** | measured ~4,000/s @1M, ~5,350/s fresh |
| **Documented single-writer ceiling** | **p95 < 100 ms holds through ≥ N=100** (5× design target); not saturated | sweep §4.1 |

Implementation note to carry into Phase 1 (1.1): consider a **background-checkpoint thread**
(`wal_autocheckpoint=0` + periodic manual `PASSIVE` checkpoint) to flatten the p99/max
checkpoint tail; keep `synchronous=NORMAL`, `fullfsync=OFF`.

---

## 9. Flags back to the orchestrator

- **§18 budget numbers to write (cross-doc):** the table in §8 above (orchestrator edits §18;
  I do not). These resolve the `[OPEN]` markers on the event-write + reader + throughput rows.
- **Phase-1 implementation note (not a doc change):** background-checkpoint thread to flatten
  the WAL-checkpoint p99/max tail (§5.5).
- **Environment FINDING — RESOLVED 2026-06-07 (was: broken toolchain shims).** All 13
  `~/.cargo/bin` rustup proxies (`cargo`, `rustc`, `cargo-clippy`, `rustfmt`, …) were **broken
  dangling symlinks** pointing at a non-existent `/Users/nozzins/.cargo/bin/rustup` (home-dir
  migration artifact) — blocking every daemon-track `cargo` invocation. **Note `rustup default
  stable` did NOT fix it** (rustup only recreates *missing* proxies, not broken ones). **Fix
  applied (user-authorized):** repointed each broken proxy to the local real `rustup` binary
  (`ln -sf rustup ~/.cargo/bin/<proxy>`). Verified: plain `cargo`/`rustc` = 1.93.0, `cargo
  clippy`/`rustfmt` work, a real build succeeds — **no PATH workaround needed**. Phase-1 build
  blocker cleared.
- **No guardrail breach:** single-writer did **not** fail at N=20 (the brief's escalate-hot
  condition); this is a clean ✅ resolution.
