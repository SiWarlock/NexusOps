# /tdd brief — github_read_client

## Feature
The **GitHub PR read client** — a `GithubReadClient` trait + `FakeGithubReadClient` (test double) + the real `OctocrabGithubReadClient` that fetches a PR + its reviews + its check-runs via octocrab REST, plumbs the fields into edges-008's `signals_from_github_response`, and returns a `PullRequestSignals`; failures map through edges-003's `classify`. The **deterministic extraction** (octocrab models → signals) is fixture-driven test-first; the **live HTTP round-trip** is the non-deterministic edge, covered by the fake (CLAUDE.md "real GitHub calls → recorded fixtures/fakes"). Closes the GitHub-PR read vertical: **live GitHub → signals → §5.1**, all in-lane (the executor/projector wiring stays gated).

## Use case + traceability
- **Task ID:** P7.1 (in-lane, Approach A — the read-client half; the `github` executor arm + `proj_pull_request` projector + `PullRequestSynced` event stay deferred on the R1 daemon seam)
- **Architecture sections it implements:** `ARCHITECTURE.md §9` (GitHub = `octocrab` typed REST+GraphQL; auth bootstrap = reuse `gh auth token` else Device Flow — **the bootstrap is deferred; this client takes an injected `Octocrab` handle**), `§7.2` (PullRequest SoT = GitHub; `proj_pull_request` is a synced cache w/ `pr_checked_at`; re-fetch before merge/checks decision), `§17` (integration-failure contract — transient 429/5xx vs auth-terminal 401/403 vs client-terminal 4xx; via edges-003's `classify`).
- **Widens phase scope because** the fixture pins compose with edges-004/006/008 to produce a **§5.1** `PullRequest` value (the §7.2/§5.1 "PullRequest status" cross-doc invariant Phase-7 task 7.2 extends); this client **consumes** the frozen §5.1 enum read-only and does not modify it. (Same waiver posture as edges-006/008.)
- **Related context:** edges-008 (`0eb60d4`) `signals_from_github_response` + the `parse_*` decoders (the string→enum layer this plumbs octocrab's fields into); edges-006 `PullRequestSignals::from_github`; edges-004 `derive_pull_request_status`; edges-003 (`f5d0d6f`) `classify`/`IntegrationOutcomeClass`/`RetryAfter`. R1 doc Part 3 (`docs/planning/edges-R1-wiring-seam-and-event-specs.md`): the `Github` executor injects this client + the classifier + the (gated) connections registry. octocrab API confirmed via Context7 (`pulls(o,r).get(n)` / `.list_reviews(n)` / a `checks()` handler; models impl `Deserialize` → JSON fixtures work).

## Acceptance criteria (what "done" means)
- [ ] A `GithubReadClient` trait exposes an async `fetch_pr_signals(owner, repo, pr_number) -> Result<PullRequestSignals, GithubReadError>` (exact name/signature confirmable at Step 2.5).
- [ ] `FakeGithubReadClient` implements the trait returning **canned** `Ok(PullRequestSignals)` / `Err(GithubReadError)` — the seam the gated `proj_pull_request` projector + `github` executor will consume in tests.
- [ ] `OctocrabGithubReadClient` implements the trait over an **injected `octocrab::Octocrab`** (auth bootstrap deferred — the client never builds the handle/reads the keychain): fetch PR + reviews + check-runs, plumb into the extraction, return signals.
- [ ] **Extraction (the deterministic core, fixture-driven):** a pure `extract_pr_signals(pr, reviews, check_runs) -> PullRequestSignals` plumbs octocrab's fields into edges-008's `signals_from_github_response` — octocrab exposes `mergeable_state` / review `state` / check `conclusion` as **strings** consumed directly by edges-008's `parse_*`; `state`/`draft`/`merged` map to `PrState`/bools; **`Option`-None fields degrade to the conservative default** (`mergeable_state=None → Unknown`, `draft=None → false`, `merged=None → false`); empty reviews/check-runs lists → `None`/`None` aggregates.
- [ ] **Fixture pins:** recorded GitHub JSON fixtures deserialize through octocrab's models and extract to the expected signals — at minimum: (a) open/not-draft/`mergeable_state="dirty"`/one APPROVED review/one completed-success check → `derive_pull_request_status == Conflict`; (b) open/clean/one APPROVED review/one completed-**failure** check → `ChecksFailing`; (c) open/clean/APPROVED/completed-success → `Mergeable`; (d) a draft PR → `Draft`. (Fixtures are **public PR API shapes — never real secrets/tokens.**)
- [ ] **Failure mapping:** a transport failure and an HTTP error status map through edges-003's `classify(status, retry_after, transport_error)` → `IntegrationOutcomeClass`, surfaced on `GithubReadError` (so the gated `*SyncFailed`/`auth_expired` path can later branch on `AuthFailed` vs `ClientError` — **the §17/security forward-constraint: branch on `IntegrationOutcomeClass`, NOT the collapsed `DeliveryOutcome::Terminal`**).
- [ ] `ReviewRequired` is **NOT** produced here — the REST `reviews[]` can only yield `ChangesRequested`/`Approved`/`None` (edges-006 `aggregate_reviews`); the GraphQL `reviewDecision → ReviewRequired` layering is a **named follow-up** (edges-010). State it in a doc comment.
- [ ] Every extraction path is **total** (no panic / no `unwrap` on octocrab response fields — all are `Option`-guarded) and the extraction is pure (no `Clock`/IO).
- [ ] All tests pass; `/preflight` clean (`cargo fmt --check && clippy -D warnings && check && test`).
- [ ] Cross-doc invariant: **none** (daemon-internal client over the edges-008/006/003 cores; no `shared/` model, no contract surface — confirm "none" at Step 9). Adds the `octocrab` (+ `async-trait` if the trait needs it) dep → `cargo audit` at the P7.1 `/phase-exit`.

## Wiring / entry point (Step 7.5)
**none — wiring lands in the gated `github` executor + `proj_pull_request` projector slices** (R1 seam + the `PullRequestSynced` event type). The client + trait are consumed by (a) the gated `github` executor arm (`ExecutorKind::Github` — injects this client per R1 Part 3) and (b) the gated `proj_pull_request` projector. Tested-but-unwired **by design** (Approach A) — Step 7.5 grep-confirms only the test module + the `FakeGithubReadClient` reference the trait; no production entry point. (`spec-lint brief` requires this section — present.)

## Files expected to touch
**New:**
- `daemon/src/integrations/github.rs` (or `github/mod.rs` + `client.rs` if it grows — Step-2.5 Q) — the `GithubReadClient` trait, `FakeGithubReadClient`, `OctocrabGithubReadClient`, `extract_pr_signals`, `GithubReadError`, and the octocrab-error → `classify` mapping. `use`s `super::pull_request::signals_from_github_response` + `super::classifier::{classify, IntegrationOutcomeClass}`.
- `daemon/tests/github_read_client.rs` — the fixture-driven extraction tests + the fake/trait tests (external integration file, per the edges-004/006/008 convention).
- `daemon/tests/fixtures/github/*.json` (or inline `const` JSON) — recorded public PR/reviews/check-runs response shapes. Step-2.5 Q on inline-const vs files.

**Modified:**
- `daemon/src/integrations/mod.rs` — `pub mod github;`
- `daemon/Cargo.toml` + `Cargo.lock` — add `octocrab` (+ `async-trait` if needed). Pin the current stable octocrab; confirm the version at Step 2.5.

If implementation needs files beyond this list, flag at Step 2.5.

## RED test outline (Step 2)
Tests in `daemon/tests/github_read_client.rs`:

1. **`extract_open_dirty_approved_passing_is_conflict`** — Asserts: fixture (open / not-draft / dirty / [APPROVED] / [completed-success]) → extract → `derive_pull_request_status == Conflict`. Why: §7.2/§5.1 end-to-end through the octocrab-models layer (Conflict > review).
2. **`extract_clean_approved_failing_check_is_checks_failing`** — Asserts: (open / clean / [APPROVED] / [completed-failure]) → `ChecksFailing`. Why: §5.1 HARD-check > review survives the octocrab extraction.
3. **`extract_clean_approved_passing_is_mergeable`** — Asserts: (open / clean / [APPROVED] / [completed-success]) → `Mergeable`. Why: §5.1 fully-ready.
4. **`extract_draft_is_draft`** — Asserts: a draft PR fixture → `Draft`. Why: §5.1 draft precedence.
5. **`extract_option_none_fields_degrade_conservative`** — Asserts: a PR fixture with `mergeable_state` absent + no reviews + no checks → `mergeable=Unknown`, `review=None`, `checks=None` → `Open` (nothing fabricated). Why: totality + the conservative floor (edges-008) survives octocrab `Option`s.
6. **`extract_empty_reviews_and_checks_lists`** — Asserts: present-but-empty `reviews`/`check_runs` arrays → `None`/`None` aggregates. Why: edges-006 aggregate semantics on empty lists.
7. **`fake_client_returns_canned_signals`** — Asserts: `FakeGithubReadClient::new(Ok(signals)).fetch_pr_signals(..).await == Ok(signals)`. Why: the seam the gated projector/executor consume.
8. **`fake_client_returns_canned_error`** — Asserts: a fake configured with `Err(GithubReadError{class: AuthFailed,..})` surfaces it. Why: pins the error carries `IntegrationOutcomeClass` (the §17 forward-constraint — `AuthFailed` distinct from `ClientError`).
9. **`octocrab_error_maps_through_classify`** *(if octocrab errors are constructible — see Step-2.5 Q)* — Asserts: a transport-style failure → `TransportError`; a 403 → `AuthFailed`; a 404 → `ClientError{404}`, via `classify`. Why: §17 line-450/452 mapping holds at the octocrab boundary. (If `octocrab::Error` isn't unit-constructible, cover the `(status,transport)→classify` boundary directly + a `not-tested-because` note on the octocrab-error decode — edges-003 already pins `classify`.)

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none.
- **Orchestrator doc rows to write hot (Step 9 routing):** none for `daemon/CLAUDE.md`/Appendix A. **Anticipated (integration-owned, multi-track — FLAG not edit from `track/edges`; I accumulate into the PLAN-DELTA hand-off):** a `daemon/LESSONS.md` C-list note (the GitHub read client = thin deterministic glue [octocrab fields → edges-008 composer; octocrab::Error → edges-003 classify] around a fake-covered live fetch; injected `Octocrab` handle keeps auth bootstrap deferred; the §17 error carries `IntegrationOutcomeClass`, not the collapsed `DeliveryOutcome`), and the `octocrab`/`async-trait` dep notes for the §18/`cargo audit` row.
- **§2.5-seam (shared-contract) model touched?** **No** — daemon-internal client over already-frozen daemon-internal cores; no Appendix-A model, no `shared/` surface → **no schema-snapshot test.**

## Things to flag at Step 2.5
1. **octocrab version + exact field types.** Confirm against the pinned octocrab version (you have Cargo + docs.rs): `pulls::PullRequest.{state, mergeable_state, draft, merged}` shapes (is `state` an `IssueState` enum or string? `mergeable_state: Option<String>`? `merged`/`merged_at`?), `pulls::Review.state`, and the check-runs API (`checks().list_check_runs_for_git_ref` or similar) + `CheckRun.{status, conclusion}`. **Default lean:** map enums/strings into edges-008's `parse_*`; `merged` via `merged.unwrap_or(false)` or `merged_at.is_some()`. Adjust the extraction to the real types; the BEHAVIOR pins (the fixtures) are version-independent.
2. **octocrab::Error → `classify` extraction + testability.** How to pull `(status, retry_after, transport_error)` from `octocrab::Error` (which variant carries the HTTP status; is `Retry-After` reachable from a typed error or only via the low-level `_get`?), and whether `octocrab::Error` is unit-constructible (drives test #9 vs a `not-tested-because`). **Default lean:** map the GitHub-status variant → `status`; a connection/hyper variant → `transport_error=true`; `retry_after=None` from a typed error is acceptable-degraded for the MVP (a follow-up reads it via `_get` if needed). If errors aren't constructible, test the `(status,transport)→classify` boundary directly (edges-003 already covers `classify`) + note it.
3. **Slice size / split.** This bundles the trait + fake + real client + extraction + error-map. If it balloons past a clean single sitting, the natural split is **009a** (`extract_pr_signals` fixtures + `GithubReadClient` trait + `FakeGithubReadClient`) / **009b** (`OctocrabGithubReadClient` live fetch + the error-map). **Default vote: one slice** (it's one coherent "read client"); split only if the live-fetch + octocrab-error work makes it too big — your call with the version in hand.
4. **Module layout.** `integrations/github.rs` single-file vs `integrations/github/{mod,client}.rs`. **Default vote: single `github.rs`** now; promote to a dir when the gated executor lands there. (R1 Part 3 names `integrations::github` as the module home.)
5. **Fixtures: inline `const` JSON vs `tests/fixtures/github/*.json`.** **Default vote: inline `const &str`** for a handful of small hand-trimmed public PR shapes (hermetic, no fixture-file IO, matches the "no committed git fixtures" hygiene from edges-001) — unless you want a couple of fuller recorded responses as files.

## Dependencies + sequencing
- **Depends on:** edges-008 (`0eb60d4`) `signals_from_github_response` + `parse_*`; edges-006 `from_github`/the enums; edges-004 `derive_pull_request_status`; edges-003 (`f5d0d6f`) `classify`/`IntegrationOutcomeClass`.
- **Blocks:** the gated `github` executor arm (`ExecutorKind::Github`, R1 Part 1 seam) + the `proj_pull_request` projector + the `PullRequestSynced`/`GithubSyncFailed` events (R1 Part 2). **Next in-lane after this:** edges-010 = the GraphQL `reviewDecision → ReviewRequired` layering (+ the richer required-approvals review rule) — overlays `ReviewRequired` onto this REST client's signals.

## Estimated commit count
**1** (default) — one coherent GitHub read client; deterministic extraction is the test-first core, the live fetch is the fake-covered edge, no safety invariant, no cross-doc change. **Split to 2** (009a extraction+trait+fake / 009b live-fetch+error) only if Step-2.5 Q3 fires. The dep add (`octocrab`) rides this commit.

## Lessons-logged candidates anticipated
- **Convention candidate** — "A GitHub read client is **thin deterministic glue** (octocrab fields → edges-008's strings→signals composer; `octocrab::Error` → edges-003's `classify`) around a **fake-covered live fetch**: the extraction is fixture-driven test-first (octocrab models deserialize from recorded public JSON), the HTTP round-trip is the non-deterministic edge behind the `GithubReadClient` trait + `FakeGithubReadClient`; the client takes an **injected `Octocrab`** so auth bootstrap stays deferred; the §17 error carries `IntegrationOutcomeClass` (NOT the collapsed `DeliveryOutcome::Terminal`) so the gated `auth_expired` path can branch on `AuthFailed`." (extends the edges-003/006/008 §9/§17 line of lessons).
- **Future TODO — next-brief working set** — edges-010 the GraphQL `reviewDecision → ReviewRequired` layering + required-approvals review rule.
- **Architecture-doc note candidate** — the `Octocrab`-handle-injection boundary (auth bootstrap deferred) + the REST-can't-see-`ReviewRequired` constraint, under §9.

## How to invoke
1. **Read this brief end-to-end** — esp. the 5 Step-2.5 questions (octocrab version/types, error mapping, split, module, fixtures).
2. **Run `/tdd github_read_client`** in the implementer session (already oriented — no `/session-start`).
3. **Step 0 (Restate)** — confirm the restatement matches the Feature line.
4. **Step 1 (files)** — confirm the NEW `integrations/github.rs` + the test file + the `octocrab` dep add.
5. **Step 2.5** — send the test-design write-up + answers to the 5 questions (esp. the confirmed octocrab field/error types) before GREEN; wait for `APPROVED.`/`TWEAK:`/`ADD:`.
6. **Step 9** — surface cross-doc "none", the `octocrab` dep (→ `cargo audit` at phase-exit), and the anticipated C-list lesson (integration-owned — flag, I route).
