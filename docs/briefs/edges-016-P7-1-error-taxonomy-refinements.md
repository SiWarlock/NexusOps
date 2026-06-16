# /tdd brief — linear_error_taxonomy_refinements

## Feature
Two cohesive §17 error-taxonomy refinements on the (in-lane) Linear read path: (1) a dedicated
`IntegrationOutcomeClass::NotFound` terminal variant replacing the synthetic `ClientError{404}` the
Linear adapter mints for an absent issue, and (2) an epoch-ms-aware `parse_rate_limit_reset` so a
Linear rate-limit carries a real `RetryAfter::Until` hint from `X-RateLimit-Requests-Reset` (Linear's
non-RFC-7231 reset header) instead of dropping it.

## Use case + traceability
- **Task ID:** P7.1 (the §17 integration error-taxonomy portion — the deterministic Linear read core)
- **Architecture sections it implements:** `ARCHITECTURE.md §17` (the integration auth-expiry / rate-limit
  / retry-classification row, line-450/452). Daemon-internal granularity refinement — the architecture
  names the terminal-client and transient-rate-limit *behaviors*; the daemon owns the enum granularity.
- **Related context:** resolves two flagged carries from `docs/sessions/edges-006-…-r3-orchestrator-round-seal.md`
  §D — the "§17 epoch-ms refinement" and the "§17 not-found taxonomy variant." Builds directly on
  edges-014 (`LinearReadError` / `IntegrationOutcomeClass`) + edges-015 (`map_linear_response`,
  `classify_graphql_error_code`, `LinearGraphqlReadClient::fetch_issue`, and the explicit epoch-ms TODO
  at `daemon/src/integrations/linear.rs:349-353`). Both LANDED.

## Acceptance criteria (what "done" means)
- [ ] `IntegrationOutcomeClass::NotFound` variant exists (fieldless terminal); `to_delivery_outcome()`
      maps it → `DeliveryOutcome::Terminal(_)` — **behavior-preserving** vs the prior synthetic
      `ClientError{404}` (the outbox drainer still dead-letters it; only the *named* class changes).
- [ ] `map_linear_response` issue-absent path (issue:null / absent `data` / malformed-on-2xx) returns
      `LinearReadError{ class: NotFound }` (was `ClientError{ status: 404 }`).
- [ ] New pure `parse_rate_limit_reset(Option<&str>) -> Option<RetryAfter>` (no `Clock`): an all-ASCII-digit
      **epoch-milliseconds** value → `RetryAfter::Until(<that instant, RFC3339-Z>)`; absent / empty /
      non-digit / pathological-overflow → `None`. (Mirrors `parse_retry_after`'s robustness, but the
      digit form means **epoch-ms → absolute `Until`**, never `Delta`.)
- [ ] The Linear live path (`LinearGraphqlReadClient::fetch_issue`) reads `X-RateLimit-Requests-Reset`
      and threads it so a Linear rate-limit (`RATELIMITED` GraphQL code, which Linear returns as HTTP 400)
      carries `RateLimited{ retry_after: Some(Until(...)) }` when the reset header is present; the standard
      `Retry-After` path is unchanged when reset is absent. Precedence per Step-2.5 Q2.
- [ ] All existing tests still pass: `daemon/tests/integration_classifier.rs` (21) +
      `daemon/tests/linear_graphql_client.rs` (12) — no regression.
- [ ] `/preflight` clean (`cargo fmt --check && clippy -D warnings && check && test`).
- [ ] **No `CONTRACT_VERSION` bump** — `IntegrationOutcomeClass` is daemon-internal (`daemon/src/integrations/`,
      not `shared/`); `shared/` untouched. Confirmed: no Appendix-A model, no cross-doc table row.

## Wiring / entry point (Step 7.5)
Both refinements land behind the **in-lane production read surface**: `map_linear_response`
(`daemon/src/integrations/linear.rs`) → reachable from `LinearGraphqlReadClient::fetch_issue` (the live
reqwest read path, edges-015) and from the `LinearReadClient` trait the gated `tasks`(external_task)
projector will consume. The downstream **live Gateway-action consumer stays gated on the daemon R1 seam
(D1, Approach A) — unchanged this slice**; consistent with every prior edges read slice (read core +
fake-covered consumer; live mutator gated). `/wired map_linear_response` will show reachable-to-the-read-
client; the gated consumer boundary is the standing edges posture, not a new dead-wiring gap.

## Files expected to touch
**Modified:**
- `daemon/src/integrations/classifier.rs` — add `NotFound` variant to `IntegrationOutcomeClass` + its
  `to_delivery_outcome` arm (→ `Terminal`); add the pure `parse_rate_limit_reset` fn (epoch-ms → `Until`).
- `daemon/src/integrations/linear.rs` — map the issue-absent site (currently `ClientError{ status: 404 }`
  at `linear.rs:271-274`) → `NotFound`; thread `X-RateLimit-Requests-Reset` (the header read at
  `linear.rs:349-358`) into the rate-limit hint (signature/precedence per Step-2.5 Q1/Q2); delete the
  now-resolved epoch-ms TODO comment.
- `daemon/tests/integration_classifier.rs` — RED tests for `NotFound` + `parse_rate_limit_reset`.
- `daemon/tests/linear_graphql_client.rs` — RED tests for issue-null→NotFound + epoch-ms reset→`Until`.

If implementation needs files beyond this list (e.g. another exhaustive `match` on `IntegrationOutcomeClass`
in `github.rs`/`pull_request.rs` that the new variant forces an arm into), **flag at Step 2.5** before GREEN.

## RED test outline (Step 2)

In `daemon/tests/integration_classifier.rs`:
1. **`not_found_to_terminal`** — `IntegrationOutcomeClass::NotFound.to_delivery_outcome()` matches
   `DeliveryOutcome::Terminal(_)`.
   - Asserts: NotFound → Terminal.
   - Why: §17 terminal-client semantics; behavior-preserving vs the synthetic 404 (`client_to_terminal`
     already pins ClientError→Terminal — NotFound joins it).
2. **`parse_rate_limit_reset_epoch_ms`** — `parse_rate_limit_reset(Some("<epoch_ms>"))` → `Some(Until(ts))`
   where `ts` equals the instant `OffsetDateTime::from_unix_timestamp_nanos(epoch_ms * 1_000_000)` (compared
   **as an instant**, RFC3339 `Z` vs `+00:00` agnostic — mirror `retry_after_http_date`).
   - Asserts: all-digit epoch-ms → absolute `Until`, NOT `Delta`.
   - Why: §D — Linear's reset is epoch-ms; `parse_retry_after` would misread it as `Delta(~1.7e12 s)` ≈
     infinite backoff. This is the load-bearing fix.
3. **`parse_rate_limit_reset_absent_or_malformed`** — `None` / `Some("")` / `Some("not-digits")` /
   `Some("<20+-digit overflow>")` all → `None`.
   - Asserts: robustness (no panic; pathological → None, like `parse_retry_after`).
   - Why: §D robustness parity.

In `daemon/tests/linear_graphql_client.rs`:
4. **`map_linear_response_issue_null_is_not_found`** — a 2xx body `{"data":{"issue":null}}` →
   `Err(e)` with `e.class == IntegrationOutcomeClass::NotFound`.
   - Asserts: the issue-absent terminal is `NotFound` (was `ClientError{404}`).
   - Why: §D not-found taxonomy variant.
5. **`map_linear_response_ratelimited_carries_epoch_ms_reset`** — a `RATELIMITED` GraphQL error body
   (HTTP 400) WITH the epoch-ms reset threaded → `Err(e)` with
   `e.class == RateLimited{ retry_after: Some(Until(_)) }`.
   - Asserts: the reset hint reaches the class as an absolute `Until`.
   - Why: §D thread-the-real-header.
6. **`map_linear_response_ratelimited_no_reset_is_none`** — the same `RATELIMITED` body with NO reset and
   NO `Retry-After` → `RateLimited{ retry_after: None }` (preserves edges-015 behavior).
   - Asserts: regression guard — absent hint stays `None`, never a bogus value.
   - Why: don't regress the current Linear-omits-both path.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** NONE crossing `shared/`. `IntegrationOutcomeClass` is daemon-internal — no
  Appendix-A model, no `shared/` surface, **no `CONTRACT_VERSION` bump**.
- **Shared-contract (cross-track) seam model touched?** NO → **no schema-snapshot test required.**
- **Orchestrator doc rows to write hot (Step 9):** none to the cross-doc table. ONE **Architecture-doc
  note candidate** for the §B arch-notes accumulation (PLAN-DELTA, applied at the P5/P7.1 phase-exit, NOT
  a contract change): "§17 daemon-internal taxonomy gains a `NotFound` terminal sub-class; the Linear
  adapter honors the epoch-ms `X-RateLimit-Requests-Reset` reset as `RetryAfter::Until`." Flag it at Step 9;
  the orchestrator accumulates it (edges does not edit `ARCHITECTURE.md` from the worktree).

## Things to flag at Step 2.5
1. **How does the epoch-ms reset header reach the RateLimited hint?** (A) add a `rate_limit_reset:
   Option<&str>` 4th param to `map_linear_response` (threaded into `classify_graphql_error_code`), parsed
   via `parse_rate_limit_reset`; the 12 existing `map_linear_response` call-sites get a mechanical `None`
   4th-arg. (B) replace `retry_after: Option<&str>` with a pre-resolved `retry_hint: Option<RetryAfter>`
   resolved in `fetch_issue`. My default vote: **A** — keeps the rate-limit-hint precedence logic in the
   pure, fully-tested core (`map_linear_response`/`classify_graphql_error_code`); the call-site ripple is
   mechanical (`None`). B moves resolution into the IO shell where it's only fake-covered.
2. **Precedence when BOTH `X-RateLimit-Requests-Reset` (epoch-ms) AND `Retry-After` are present.** Default
   vote: **epoch-ms reset wins; fall back to `Retry-After` only when reset is absent** — the reset is
   Linear's authoritative, Linear-specific signal (this is the Linear adapter path). Realistically Linear
   sends only the former, but the precedence must be deterministic + tested.
3. **`NotFound` shape.** A fieldless `NotFound` terminal variant vs overloading `ClientError` with a flag.
   Default vote: **fieldless `NotFound`** — cleaner; the gated `SyncFailed` path can branch not-found vs
   a payload-fix `ClientError` distinctly. Confirm at Step 1 that no other exhaustive `match` on
   `IntegrationOutcomeClass` (scan `github.rs`/`pull_request.rs`) silently breaks — add an arm or flag.
4. **GitHub not-found scope.** GitHub's read client has **no** current synthetic-not-found site (verified —
   its only `404` mention is a doc comment about empty refs). Default vote: **out of scope this slice** —
   add `NotFound` to the enum + apply it at the Linear site only; GitHub adoption is a follow-up if/when
   its read path surfaces a not-found. Keeps the slice scoped to the two flagged §D carries.

## Dependencies + sequencing
- **Depends on:** edges-014 (`LinearReadError`/`IntegrationOutcomeClass`/`extract_issue`), edges-015
  (`map_linear_response`/`classify_graphql_error_code`/`LinearGraphqlReadClient`/the epoch-ms TODO). Both LANDED.
- **Blocks:** nothing in-lane. Refines the §17 taxonomy the **gated** `SyncFailed`/`auth_expired` wiring
  (R1, deferred) will branch on — a forward-looking, behavior-preserving hardening.

## Estimated commit count
**1.** One logical unit — "§17 Linear error-taxonomy refinements." Both refinements touch the same two
production files (`classifier.rs` + `linear.rs`) + the same two test files, share the §17 `IntegrationOutcomeClass`
context, and neither touches a safety invariant (root `CLAUDE.md` Key safety rules — §17 is resilience
classification, not a safety invariant). Bundle is
correct per the template criteria. **If the implementer judges the epoch-ms signature ripple (Q1) heavy
enough to warrant a split, flag it at Step 2.5** — NotFound (tiny) then epoch-ms (the bigger half).

## Reviewer posture (Step 8)
- **security-reviewer:** policy `invariant` → **SKIP** (no safety invariant touched — root `CLAUDE.md` Key
  safety rules; §17 is resilience classification — the api_key never enters this code; the new parse is
  total/no-panic). Note the skip rationale.
- **code-quality-reviewer:** policy `every-slice` → runs on the slice diff.

## Lessons-logged candidates anticipated
- **Architecture-doc note candidate** — §9/§17: the daemon-internal `IntegrationOutcomeClass` gains a
  `NotFound` terminal sub-class; the Linear adapter honors epoch-ms `X-RateLimit-Requests-Reset` as
  `RetryAfter::Until`. (Routes to the §B arch-notes accumulation at `/orchestrate-end`.)
- **Convention candidate (low confidence)** — "a provider's rate-limit *reset* may be epoch-ms (Linear's
  `X-RateLimit-Requests-Reset`), NOT RFC-7231 `Retry-After`; parse it to an absolute `RetryAfter::Until`,
  never `Delta` — an all-digit epoch value misread as delta-seconds is ~infinite backoff." Flag at Step 9
  only if it reads as generalizable.
- **Future TODO — carry** — the §17 not-found taxonomy could extend to GitHub's read path if/when it
  surfaces a not-found (Q4 deferral).
