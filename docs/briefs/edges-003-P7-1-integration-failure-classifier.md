# /tdd brief — integration_failure_classifier

## Feature
The **§17 integration-failure classifier** — a pure, deterministic mapping from an HTTP/transport delivery outcome to a daemon-internal `IntegrationOutcomeClass` (faithful to the §17 line-450 distinctions: transient 429/5xx vs **auth-terminal 401/403** vs **client-terminal 4xx** vs offline/transport), plus a `to_delivery_outcome()` that maps it into the **existing 1.3 `DeliveryOutcome`** the outbox drainer already consumes. Opens Phase 7.1's in-lane foundation. The github/linear `Destination` adapters that *call* this (and the gated §17 auth wiring → `SyncFailed` + `auth_expired`) come later.

## Use case + traceability
- **Task ID:** P7.1 (the §17 integration-failure-classifier portion — the deterministic core; the octocrab/Linear adapters + auth bootstrap + the `SyncFailed`/`auth_expired` wiring are later/gated)
- **Architecture sections it implements:** `ARCHITECTURE.md §17` (the Integration auth-expiry/rate-limit row, line 450 — owner `integrations`: transient 429-Retry-After/5xx → backoff; terminal 401/403 → `*SyncFailed` + profile→auth_expired + re-auth card; dead after bounded retry; **and** the Network-loss row, line 452 — offline writes queue in outbox, not failed), `§9` (integration architecture — the integration-failure contract summary).
- **Related context:** the **existing 1.3 outbox drainer** — `daemon/src/eventstore/outbox.rs` defines `pub enum DeliveryOutcome { Delivered, Retryable(String), Terminal(String) }` + the `Destination` trait (`deliver(&self, payload) -> DeliveryOutcome`) the github/linear adapters will implement; the drainer already owns backoff (30s base, cap 3600s, `MAX_RETRIES=5`) + dead-letter. **This slice maps INTO `DeliveryOutcome` — it adds no new outbox/drainer type and changes no drainer code.**

## Acceptance criteria (what "done" means)
**Classification (`daemon/src/integrations/classifier.rs`):**
- [ ] `classify(...)` maps an HTTP status (+ optional `Retry-After`, + a transport-error signal) to `IntegrationOutcomeClass`.
- [ ] **429** → `RateLimited { retry_after }` (the `Retry-After` hint parsed + carried).
- [ ] **5xx** → `ServerError` (transient).
- [ ] **transport/connection error (offline)** → `TransportError` (transient — §17 line 452: offline queues, not failed).
- [ ] **401 / 403** → `AuthFailed` — **kept DISTINCT from other 4xx** (the load-bearing §17 line-450 distinction: auth → the gated `SyncFailed`+`auth_expired`+re-auth path; ≠ payload-fix).
- [ ] **other 4xx (400/404/422)** → `ClientError { status }` (terminal payload/request error).
- [ ] **2xx** → `Success`.
- [ ] Unknown/unexpected status → a conservative transient class (documented default).

**Mapping into the drainer (`to_delivery_outcome`):**
- [ ] `Success` → `DeliveryOutcome::Delivered`.
- [ ] `RateLimited` / `ServerError` / `TransportError` → `DeliveryOutcome::Retryable(_)` (the drainer backs off).
- [ ] `AuthFailed` / `ClientError` → `DeliveryOutcome::Terminal(_)` (the drainer dead-letters after its budget; the *auth* sub-case additionally drives the gated `SyncFailed`/`auth_expired` wiring later — distinction preserved on `IntegrationOutcomeClass`).

**Retry-After parsing (`parse_retry_after`):**
- [ ] RFC-7231 **delta-seconds** form (`"120"`) → a relative hint.
- [ ] RFC-7231 **HTTP-date** form → carried as an absolute instant (parsed via the existing `time` dep).
- [ ] Absent/malformed → `None`. **Pure — no `Clock`** (the absolute/relative hint is resolved against `now` by the caller later, not here).

**General:**
- [ ] Unit tests pass; `/preflight` clean. **No `shared/` touch, no migration, no `gateway/` touch, no drainer/`eventstore` change** (imports `DeliveryOutcome` read-only). No new Cargo dep (uses existing `time`).

## Wiring / entry point (Step 7.5)
**`none — wiring lands in the gated 7.1 adapter slice.`** The classifier is pure logic; its only consumers are the github/linear `Destination::deliver()` adapters (which call `classify(...).to_delivery_outcome()`), and the §17 auth path (`AuthFailed` → `SyncFailed` event + `ExecutionProfile→auth_expired`) — both **gated** (octocrab/Linear + the `SyncFailed` shared event type + the 0.5b-held `ExecutionProfile` unfreeze). Reachability intentionally deferred (named).

## Files expected to touch
**New:**
- `daemon/src/integrations/mod.rs` — `integrations` module decl (NEW module; `pub mod classifier;`)
- `daemon/src/integrations/classifier.rs` — `IntegrationOutcomeClass` + `classify(...)` + `to_delivery_outcome(&self) -> DeliveryOutcome` + `parse_retry_after(...)`
- Test file: `daemon/tests/integration_classifier.rs` (or inline `#[cfg(test)]` — Step-1 choice)

**Modified:**
- `daemon/src/lib.rs` — `pub mod integrations;`

No `Cargo.toml` change (no octocrab/keyring/Linear yet — those land with the adapters; `time` already a dep). **Do NOT touch `gateway/`, `shared/`, `eventstore/` (import `DeliveryOutcome` read-only — don't modify the drainer), or any migration.**

## RED test outline (Step 2)
**`classify` (status/transport → class):**
1. **`classify_429_rate_limited`** — 429 + `Retry-After: 120` → `RateLimited{retry_after: Some(Delta(120))}`. Why: §17 transient.
2. **`classify_5xx_server_error`** — 503 → `ServerError`. Why: §17 transient.
3. **`classify_transport_error`** — a transport/connection failure → `TransportError`. Why: §17 line 452 (offline queues).
4. **`classify_401_auth`** — 401 → `AuthFailed`. Why: §17 line-450 auth-terminal.
5. **`classify_403_auth`** — 403 → `AuthFailed`. Why: §17 line-450 auth-terminal.
6. **`classify_400_client`** — 400 → `ClientError{400}`. Why: §17 terminal payload.
7. **`classify_404_client`** — 404 → `ClientError{404}`. Why: terminal (not auth).
8. **`classify_422_client`** — 422 → `ClientError{422}`. Why: terminal payload.
9. **`classify_2xx_success`** — 200/204 → `Success`. Why: happy.
10. **`classify_unknown_conservative`** — an unexpected status (e.g. 308) → the documented conservative transient. Why: fail-safe default.
11. **`classify_auth_distinct_from_client`** — 401/403 → `AuthFailed` AND 400/404/422 → `ClientError` (NOT collapsed). Why: **the load-bearing §17 line-450 distinction** (auth → gated SyncFailed/auth_expired; client → payload-fix).

**`to_delivery_outcome` (class → drainer outcome):**
12. **`rate_limited_to_retryable`** / **`server_error_to_retryable`** / **`transport_to_retryable`** → `Retryable`. Why: §17 backoff.
13. **`auth_to_terminal`** / **`client_to_terminal`** → `Terminal`. Why: §17 dead-after-budget.
14. **`success_to_delivered`** → `Delivered`. Why: happy.

**`parse_retry_after`:**
15. **`retry_after_delta_seconds`** — `"120"` → `Delta(120)`. Why: RFC-7231.
16. **`retry_after_http_date`** — an HTTP-date string → `Until(<instant>)`. Why: RFC-7231 (the `time` parse).
17. **`retry_after_absent_or_malformed`** — `None`/garbage → `None`. Why: robustness.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** **none.** `IntegrationOutcomeClass` is **daemon-internal** (integrations module); it maps INTO the existing `DeliveryOutcome` (no new outbox/shared type).
- **Shared-contract seam model touched?** **NO** — no envelope/ID/status-machine/catalog/`EventTypeRegistry` change, no `DeliveryOutcome` change → **no schema-snapshot, no CONTRACT_VERSION**. (The `SyncFailed` event type + `ExecutionProfile.auth_expired` that the auth path will need are **out of scope** here and are daemon-track/0.5b-gated — flagged at Step 9.)
- **Orchestrator doc rows to write hot:** none new this slice.

## Things to flag at Step 2.5
1. **The `IntegrationOutcomeClass` variant set + the §17 auth/client/transient split.** My default vote: `{ Success, RateLimited { retry_after: Option<RetryAfter> }, ServerError, TransportError, AuthFailed, ClientError { status: u16 } }`. **Load-bearing pin:** 401/403 → `AuthFailed` distinct from other-4xx → `ClientError` (the §17 line-450 auth-vs-payload distinction the gated wiring depends on). Confirm the variant set (esp. whether `TransportError` should fold into `ServerError` — I keep them distinct so the later "stale (offline)" badge §17 line-452 can tell offline from 5xx).
2. **`to_delivery_outcome` mapping.** My default vote: Success→Delivered; RateLimited/ServerError/TransportError→Retryable; AuthFailed/ClientError→Terminal. Confirm (esp. ClientError→Terminal — a 4xx delivery is a terminal payload/request error, not a retry).
3. **Retry-After: parse+carry, or ignore?** My default vote: **parse + carry** the hint on `RateLimited` (the §17 contract names "429 Retry-After"). The drainer currently uses its own exponential backoff and does NOT yet consume the hint — honoring it is a later **drainer change (daemon-core, out of this lane)**; this slice preserves the hint for that. Confirm parse+carry.
4. **Retry-After value representation (keep the classifier PURE).** My default vote: `enum RetryAfter { Delta(u64 /*secs*/), Until(Timestamp) }` carried as-is — **no `Clock`** in the classifier (the absolute HTTP-date vs relative delta is resolved against `now` by the caller later). Confirm (vs. forcing a `now` param to normalize to a single Duration — which I'd avoid, to keep the fn pure).

## Dependencies + sequencing
- **Depends on:** the 1.3 outbox drainer (`DeliveryOutcome` + `Destination`, landed) — imported read-only. No Gateway / `shared/` dependency.
- **Blocks:** the gated 7.1 adapter slice (the github/linear `Destination` impls that call `classify().to_delivery_outcome()`; the auth-bootstrap; the `SyncFailed`/`auth_expired` §17 auth wiring) — gated on octocrab/Linear/keyring + the `SyncFailed` shared event type + the 0.5b `ExecutionProfile` unfreeze.

## Estimated commit count
**1–2.** Bundle the classifier + the `to_delivery_outcome` mapping + the Retry-After parse (one cohesive concern, integrations module, **no safety-invariant pin**). Split the Retry-After parse into its own commit only if it grows.

## Lessons-logged candidates anticipated
- **Convention candidate** — "the §17 integration-failure classifier is a PURE fn (no `Clock`) producing a daemon-internal `IntegrationOutcomeClass` that preserves the auth-vs-client-vs-transient distinctions §17 needs, then maps INTO the existing `DeliveryOutcome` — adding no outbox/drainer type; Retry-After carried as `Delta|Until` for the caller to resolve."
- **Future TODO — belongs-to-a-phase (gated 7.1 adapter slice)** — the github/linear `Destination` adapters + auth bootstrap + the `AuthFailed → SyncFailed + ExecutionProfile.auth_expired` §17 wiring (needs the `SyncFailed` shared event type [daemon-track] + the 0.5b `ExecutionProfile` unfreeze [cat-4 HITL]).
- **Future TODO — operational** — the drainer honoring the parsed `Retry-After` hint (vs its own exponential backoff) is a later daemon-core drainer change.

## How to invoke
1. **Read this brief end-to-end** — Step-2.5 Q1 (the §17 auth/client/transient split) is the load-bearing one.
2. **Run `/tdd integration_failure_classifier`.**
3. **Step 0 (Restate)** — confirm: the pure classifier + mapping only; adapters/auth/SyncFailed deferred.
4. **Step 1 (files)** — confirm against the list; do NOT touch `gateway/`, `shared/`, `eventstore/`, migrations.
5. **Step 2.5** — send the test-design write-up + the 4 design answers; wait for `APPROVED.` before GREEN.
6. **Step 9** — surface anything beyond the anticipated candidates (esp. confirm the gated-wiring deps for my carry-forward).
