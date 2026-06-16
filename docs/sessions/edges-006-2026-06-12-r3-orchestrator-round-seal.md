# Session edges-006 — R3 orchestrator round seal: Linear read vertical COMPLETE (client + network adapter) + cross-track migration FINDING

> **Orchestrator-side Round-3 round doc** (companion to the implementer's `edges-005-2026-06-12-r3-linear-network-adapter.md`). Predecessor: `edges-004` (R2 orch) ← `edges-003` (R2 impl) ← `edges-002` (R1 orch) ← `edges-001` (R1 impl). Successor: `edges-007-2026-06-12-r4-section-d-refinements.md` (R4 impl close-out) + `edges-008-2026-06-12-r4-orchestrator-round-seal.md` (R4 orch round seal — the cumulative PLAN-DELTA). **Multi-track:** this doc carries the **PLAN-DELTA HAND-OFF** for the integration owner — the shared `IMPLEMENTATION_PLAN.md` / `ARCHITECTURE.md` / `daemon/LESSONS.md` / `daemon/CLAUDE.md` are integration-owned and **NOT edited from `track/edges`**; apply this at the P5/P7.1 phase-exit merge. Lead decisions live in `docs/team-handoffs/edges-lead-decision-log.md` (referenced, not duplicated).

## Why this round existed
Round 3 (fresh orch+impl after the 2nd context cycle) opened the **client + network side** of the Linear read vertical that R2's edges-013 issue-state derivation foundation set up — under **Approach A** (all read/derivation logic in-lane; ALL wiring deferred; never touch `gateway/`/`shared/`). Two in-lane slices drove the vertical to completion: **edges-014** (the read-client core + seam) then **edges-015** (the big real GraphQL network adapter over reqwest — R2 was sealed right before it to give it full runway). The round closed on a **wholesale closeout** (user exiting iTerm + restarting each track, lead-directed) at edges-015, the clean boundary — and surfaced a **cross-track migration FINDING** that gates the next planned work.

## What landed (2 in-lane slices · daemon suite 361→381/0 · all LOCAL on `track/edges`, unpushed/unmerged)
| Slice | Commit | What |
|---|---|---|
| edges-014 P7.1 | `6ebdc4e` | Linear read-client core+seam (`extract_issue(&str)->Option<LinearIssue>` + `LinearReadClient`/`FakeLinearReadClient`/`LinearReadError`/`LinearIssue`; 8 tests) |
| edges-015 P7.1 L1 | `7445ae7` | the pure core — `build_issue_query` (typed-var, injection-safe) + `map_linear_response` (the GraphQL `errors[].code` override over edges-003 `classify`); 9 tests, no dep |
| edges-015 P7.1 L2 | `581fa61` | the real `LinearGraphqlReadClient` over reqwest (injected key/auth-deferred; §15 no-key-leak; rustls-tls); 3 tests; **+reqwest 0.12** |

**The Linear read vertical is now COMPLETE in-lane:** live Linear GraphQL → `LinearIssue` → §5.1 `Task` (edges-013 derivation + edges-014 client/seam + edges-015 network adapter). Impl session-doc commit: `bdd84c0` (`edges-005`).

**Quality:** 381/0; TDD clean (every slice RED-first, every Step-2.5 orch-reviewed); **zero cross-doc invariant change** (`shared/` untouched — daemon-internal over edges-014/013/003 + the frozen `Task`; no CONTRACT bump); **security-reviewer CLEAN on the credential slice (edges-015 L2)** + correctly policy-SKIPPED on the pure-derivation edges-014; code-quality 2-fixed/3-low-deferred per slice. **Dep added: `reqwest 0.12` (default-features off, `json`+`rustls-tls`)** — reuses the in-tree rustls (no native-tls/OpenSSL; no `danger_accept_invalid_certs`; `cargo tree` verified) → `cargo audit` at the P7.1 phase-exit.

---

## ⚠️ FINDING (cross-track — UNRESOLVED; carried to the restart + the lead's `/team-end` handoff)

**Cross-track eventstore migration-number contention gates the edges "registry + integration_connections migrations" line.**

- **Context.** edges has been strictly pure read/derivation in-lane (Approach A): the GitHub-PR read chain, the Linear read vertical, and the git diff backend all completed WITHOUT touching the eventstore schema. The next-planned line ("registry + integration_connections migrations") would be the **FIRST edges slice to add a DB migration.**
- **The contention.** A migration = a new `MIGRATION_9` + bump `SUPPORTED_USER_VERSION` 8→9 in `daemon/src/eventstore/{schema,migrations}.rs`. `user_version` is a **single global linear sequence** (one daemon, one DB schema) shared by all tracks. Both `track/edges` AND the daemon track (HEAD `f1c0ca8`, Phase 3) currently sit at user_version 8 → no immediate collision, BUT the daemon's P4 work (`harness_session_map` + other durable tables) will independently claim `MIGRATION_9`/user_version 9 → at the P5/P7.1 phase-exit merge, two independent "MIGRATION_9 / user_version 9" definitions = a hard merge conflict + a runtime schema-version conflict.
- **Why cross-track (not unilateral orch scoping).** The migration sequence is a de-facto shared contract across tracks (like `shared/`, but for the DB schema). Claiming a number from one track without coordinating the other risks the collision — lead-coordination territory (an R1-style daemon-track referral).
- **The table itself.** `integration_connections` is daemon-internal (`conn_` is NOT in the frozen-22 IDs — confirmed absent from `shared/`; no frozen §5.1 status machine — outbox/leases-analogous), §15 keychain_ref-only (tokens NEVER stored; keychain_ref pointer only — DATA_MODEL §2.8 / ADR-007/011), and has **NO in-lane consumer** (population = a gated Gateway action + the gated `IntegrationConnectionRegistered` event + the gated projector). So the migration is consumer-less forward-laying.
- **Sub-note (contract reconcile).** `integration_connections.connection_id` uses a `conn_` prefix not in the frozen-22. Likely resolution = daemon-internal; the determination (+ whether the UI consumes `connection_id` → a possible frozen-22 escalation) should be made when the gated population/UI-consumption slice lands.
- **Options.** **A (orch rec)** — defer ALL edges registry/migration work to the P5/P7.1 phase-exit merge, where the daemon track owns the eventstore schema sequence + assigns coordinated migration numbers (consumer-less → ~free; keeps edges' pure-read posture; coordinates once). **B** — coordinate a number now via a daemon-track referral (edges claims 9; daemon takes 10+). (A reserved-range-per-track convention doesn't work — user_version is strictly sequential.)
- **Status: UNRESOLVED.** No edges slice should add an eventstore migration until the cross-track number allocation is coordinated. **Recommendation: A.**

---

## PLAN-DELTA HAND-OFF (integration owner — apply at the P5/P7.1 phase-exit merge)

### A. Task-tick deltas (partial — in-lane logic; wiring deferred)
- **7.1 (still partial `[ ]`):** the **Linear read vertical is now COMPLETE in-lane** — edges-013 issue-state derivation (`d7a9458`, R2) + edges-014 read-client core+seam (`6ebdc4e`) + edges-015 real `LinearGraphqlReadClient` (`7445ae7`+`581fa61`): live Linear GraphQL → `LinearIssue` → §5.1 `Task`. **Deferred wiring (unchanged):** the Linear `Destination` adapter + auth bootstrap (keychain/OAuth) + the `tasks`(external_task) projector + the `IntegrationConnectionRegistered`/`*SyncFailed` events + the `integration_connections` table (see FINDING).
- **5.2 (still partial `[ ]`):** unchanged from R2 (git diff backend complete in-lane; deferred wiring unchanged).
- **5.3 / 5.4 / `auth_expired`:** untouched — DEFERRED (H1 0.5b ExecutionProfile gate · phase-exit bench cadence, baseline 1.029 ms · H1-linked).

### B. Arch notes (→ `ARCHITECTURE.md` §9 — daemon-defined; the architecture leaves these unpinned, like R1/R2's notes)
1. **§9 Linear read-client boundary** (edges-014): the client takes an injected key/handle (auth bootstrap deferred); `extract_issue(&str) -> Option<LinearIssue>` folds deserialize (malformed-JSON / GraphQL `errors[]` / `data:null` / `issue:null` all → None by design) and derives the §5.1 `Task` via edges-013; **status derives from `state.type` (closed 6-value set), NEVER the free-form `state.name`** (§B-#5, fixture-pinned); private wire structs (the GraphQL `{data:{issue}}` shape stays internal); the §17 error carries `IntegrationOutcomeClass` (not the collapsed `DeliveryOutcome`). The real fetch is edges-015.
2. **§9 Linear network-adapter boundary** (edges-015): the real `LinearGraphqlReadClient` over reqwest; `map_linear_response` layers the GraphQL `errors[].extensions.code` **OVER** edges-003's HTTP-status `classify` — **Linear signals rate-limits as HTTP 400 + `RATELIMITED` (NOT 429)**, so the code override flips the 400-would-be-terminal-`ClientError` into a retryable `RateLimited` (mirrors edges-010's GraphQL-over-REST layering); transport error → `TransportError`; reqwest uses `rustls-tls` to reuse the in-tree stack (no native-tls); the injected `api_key` reaches ONLY the Authorization header (§15 rule #5, structurally + transport-test pinned).

### C. Lessons (→ `daemon/LESSONS.md` C-list — coordinate §-numbers with the daemon track; **note: the daemon track took §26 (043 interception) during R3, so edges' lessons re-number — propose §28/§29 (or next free) at the merge, NOT §26/§27**)
1. **Linear thin-glue mirror** (edges-014): the Linear read client mirrors the GitHub thin-glue pattern — `extract_issue` (GraphQL response → `LinearIssue`, deriving §5.1 `Task`) fixture-driven test-first; the live fetch is a fake-covered separate slice (Linear has no octocrab-equivalent); injected key (auth deferred); the §17 error carries `IntegrationOutcomeClass`. (Extends the edges-009/013 line.)
2. **errors-as-400/200 code-override** (edges-015): the real Linear client layers the GraphQL `errors[].extensions.code` over edges-003's HTTP-status `classify` — Linear rate-limit = HTTP 400 + `RATELIMITED` (not 429), so the code override is load-bearing (mirrors edges-010's GraphQL-`reviewDecision`-over-REST). The pure `map_linear_response` (status+body→Result) + `build_issue_query` (typed-variable) are test-first; the reqwest POST is the thin fake-covered shell; the injected api_key is structurally key-free (§15 rule #5); reqwest rustls-tls reuses the in-tree stack.

> **Lesson-number coordination:** edges proposed §26/§27 during the round, but the daemon track landed its own §26 (043 INV-SEC-1 interception) in parallel. At the merge, renumber edges' two lessons to the next free daemon slots (likely §28/§29) — never reuse/collide a daemon slot.

### D. Carry-forward (gated-wiring + deferred — for next briefs / the phase-exit)
- **The cross-track migration FINDING** (above) — gates the registry/`integration_connections` migration line. Recommendation A (defer to the coordinated phase-exit merge). UNRESOLVED.
- **§17 epoch-ms refinement** — Linear's rate-limit hint is `X-RateLimit-Requests-Reset` (epoch-ms), NOT `Retry-After`; edges-003 `parse_retry_after` would misread epoch-ms (all-digits) as `Delta(~1.7e12 s)` ≈ infinite backoff, so edges-015 passes `retry_after=None` for now. Deferred: an epoch-ms-aware parse + threading the real header. RateLimited *class* is correct meanwhile.
- **§17 not-found taxonomy variant** — `issue:null` uses a synthetic `ClientError{404}`; a dedicated `NotFound` terminal class in `IntegrationOutcomeClass` is the refinement.
- **Auth bootstrap** (keychain/OAuth, 24h refresh — §9) as the injected key's real source + the auth-code (`AUTHENTICATION_ERROR`/`FORBIDDEN`) confirm-against-live.
- **Richer `LinearIssue` fields** (description/team/priority/timestamps — secondary signals).
- **`test-support` cargo feature** to gate `FakeLinearReadClient`/`FakeGithubReadClient` out of the release binary (they ship ungated because integration tests link the lib as an external crate; cf. LESSON §21) — cross-cutting hardening.
- **reqwest `cargo audit`** at the P7.1 `/phase-exit` (new dep this round) — alongside the octocrab/async-trait audit carried from R2.
- **Engine refinements (carried from R2, unchanged):** the `open_diff` DRY refactor; copy detection (git2 0.21 can't); huge-diff perf; the §17 terminal-non-auth taxonomy variant (folds with the not-found refinement above); octocrab `ReviewState` strict-deserialize hardening.
- **The wiring slices (5.1/5.2/7.1)** — gated on the daemon R1 executor-registration seam + the new shared event types (`docs/planning/edges-R1-wiring-seam-and-event-specs.md`). **Security-load-bearing carries (unchanged from R2):** the §17 auth wiring branches on `IntegrationOutcomeClass::AuthFailed` (not the collapsed `DeliveryOutcome::Terminal`); the gated `*SyncFailed` persist path runs the read-error message through the §15 Redactor before any sink.

### E. Decisions this round (lead-logged in `docs/team-handoffs/edges-lead-decision-log.md` — referenced, not duplicated)
- **edges-014 signature = Form A** (`extract_issue(&str) -> Option<LinearIssue>` + private wire structs) over the brief body's `node->LinearIssue` — orch-ratified at Step-2.5 (Linear has no octocrab-equivalent public model; edges-015's HTTP body IS a `&str`; the lead's task-desc specified `&str`).
- **§B-#5 status-from-type-not-name** pinned via a Step-2.5 TWEAK (the load-bearing Linear-derivation invariant).
- **edges-015 errors-over-classify layering** + the **L2 epoch-ms retry-after trap** (orch caught at Step-2.5: do NOT thread the all-digit epoch-ms header into `parse_retry_after`) + **Q1 no-mock-server** + **Q3 synthetic ClientError{404}** + **Q4 rustls-tls** — all orch-reviewed at Step-2.5, routed as the §B arch-notes + §D carries.
- **WHOLESALE CLOSEOUT** (lead-directed): R3 sealed at edges-015 (the clean boundary, Linear vertical complete); the held migration FINDING dropped-as-dispatch + documented for the restart; no push, no merge.

---

## Open follow-ups / next round (Round 4 — post-restart)
- **The cross-track migration FINDING is the gating decision** for the registry/`integration_connections` line — the restarted team / user resolves it (orch rec A: defer to the coordinated phase-exit merge). Until resolved, no edges slice adds an eventstore migration.
- **If migrations are deferred (rec A):** the next in-lane edges work is thin (the major read verticals — GitHub-PR, Linear, git diff — are all COMPLETE; remaining = the §D refinements). A natural assessment point for whether the edges track approaches its P5/P7.1 phase-exit (gated on the daemon R1 seam + event types, still not landed/merged).
- **The daemon track delivers the R1 executor-registration seam + the Phase-5/7 event types** → unblocks ALL edges wiring slices (still the cross-track gate; not yet landed/merged).
- **No merge-to-main this round** — `track/edges` stays on its branch (based `a40ac00`; main since advanced to the daemon Phase-3 seals `f1c0ca8` — reconcile at the P5/P7.1 phase-exit). Rebase cadence = the user's call.

## Round seal
- Round artifacts committed on `track/edges` (this `/orchestrate-end`): the `edges-015` brief + this orch round doc (`edges-006`) + the lead's `edges-lead-decision-log.md` R3 update (committed on the lead's behalf — their shell git is sandbox-blocked) + the `edges-004` successor-link update. Round commit hash recorded in the close-out ack to the lead. **NOT pushed** (user-gated). **NO merge/rebase to main** (phase-exit only — P5/P7.1 incomplete). The impl session doc (`edges-005`) rode its own commit `bdd84c0`.
