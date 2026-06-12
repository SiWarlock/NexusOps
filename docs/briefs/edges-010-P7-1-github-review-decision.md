# /tdd brief — github_review_decision_layering

## Feature
The **GraphQL `reviewDecision → ReviewRequired` layering** that completes the GitHub-PR read vertical. edges-009's REST client can only aggregate `ChangesRequested`/`Approved`/`None` from `reviews[]` — it can never produce **`ReviewRequired`** (a branch-protection state exposed only by GitHub's GraphQL `reviewDecision`), so a PR awaiting required review reads as `Open` instead of `NeedsReview`. This slice adds a GraphQL fetch of `reviewDecision` and **overlays** it onto the REST signals (GraphQL authoritative when present; REST aggregate the fallback). The deterministic decode + overlay are test-first; the GraphQL fetch is the fake-covered edge, and a GraphQL failure **degrades to the REST-only signals** (the enrichment is best-effort, never fails the whole read).

## Use case + traceability
- **Task ID:** P7.1 (in-lane, Approach A — completes the read vertical; the `github` executor/`proj_pull_request` projector/`PullRequestSynced` wiring stays gated)
- **Architecture sections it implements:** `ARCHITECTURE.md §9` (GitHub = octocrab typed REST **+ GraphQL**), `§7.2` (PullRequest SoT = GitHub synced cache), `§17` (integration-failure contract — applies to the GraphQL fetch too).
- **Widens phase scope because** the pins compose with edges-004/006/008/009 to produce a **§5.1** `PullRequest` value (`NeedsReview` specifically — the gap this closes); this client consumes the frozen §5.1 enum read-only. (Same waiver posture as edges-006/008/009.)
- **Related context:** edges-009 (`2eec8f2`) the REST read client + `extract_pr_signals` + the `GithubReadClient` trait/fake/`OctocrabGithubReadClient`; edges-006 `ReviewDecision`(incl. `ReviewRequired`)/`aggregate_reviews`; edges-004 `derive_pull_request_status` (`ReviewRequired → NeedsReview`). edges-006/009 design notes explicitly defer the `reviewDecision` layering to here. octocrab GraphQL confirmed via Context7: `octocrab.graphql(&body) -> Result<GraphqlResponse<T>>` where `GraphqlResponse = Ok(data) | Err(errors)` (a GraphQL logical error is HTTP-200 `Err(errors)`, distinct from transport `Err(octocrab::Error)`).

## Acceptance criteria (what "done" means)
- [ ] `parse_review_decision(Option<&str>) -> ReviewDecision` maps GitHub's GraphQL values: `"REVIEW_REQUIRED"→ReviewRequired`, `"APPROVED"→Approved`, `"CHANGES_REQUESTED"→ChangesRequested`, `None`/`null`/unrecognized → `None` (least-salient floor, edges-008 convention; case-insensitive per edges-008/009).
- [ ] `layer_review_decision(signals, graphql_decision) -> PullRequestSignals` (or equivalent) overrides `signals.review` with `graphql_decision` **when it is non-`None`**, else keeps the REST aggregate — because GraphQL `reviewDecision` is GitHub's authoritative branch-protection-aware aggregate, and `None` means "no decision / not reachable" → the REST `reviews[]` aggregate stands.
- [ ] `OctocrabGithubReadClient::fetch_pr_signals` now ALSO fetches `reviewDecision` (GraphQL) and layers it onto the REST signals → fully-layered `PullRequestSignals`.
- [ ] **Best-effort degrade:** a GraphQL failure — **either** transport `Err(octocrab::Error)` **or** logical `Ok(GraphqlResponse::Err(errors))` — leaves the REST-only signals intact (reviewDecision treated as `None`), never failing the whole `fetch_pr_signals`. (The REST half already succeeded; an enrichment failure must not lose the PR status.)
- [ ] **End-to-end pin (the gap closed):** a PR with `reviewDecision="REVIEW_REQUIRED"`, no `reviews[]`, clean mergeable, checks-success → `derive_pull_request_status == NeedsReview` (was `Open` in edges-009).
- [ ] **Override pins:** GraphQL `APPROVED` + REST aggregate `ChangesRequested` (stale REST review) → final `Approved` (GraphQL wins); GraphQL `None` + REST `Approved` → `Approved` (fallback holds).
- [ ] `ReviewRequired` is produced **only** via the GraphQL path (documented); the REST `aggregate_reviews` is unchanged.
- [ ] All tests pass; `/preflight` clean. Cross-doc invariant: **none** (daemon-internal over edges-006/009; no `shared/` model). No new dep if the raw-query approach is taken (Step-2.5 Q1).

## Wiring / entry point (Step 7.5)
**none — wiring lands in the gated `github` executor + `proj_pull_request` projector slices** (R1 seam + `PullRequestSynced`). The fully-layered `fetch_pr_signals` is consumed by the gated `github` executor / projector. Tested-but-unwired **by design** (Approach A) — Step 7.5 grep-confirms only the module + test reference the new fns. (`spec-lint brief` requires this section — present.)

## Files expected to touch
**Modified:**
- `daemon/src/integrations/github.rs` — `parse_review_decision` + `layer_review_decision` (the deterministic core) + the GraphQL fetch + degrade-on-failure in `OctocrabGithubReadClient::fetch_pr_signals` (the live edge). `use`s edges-006's `ReviewDecision`.
- `daemon/tests/github_read_client.rs` — the parse + overlay + end-to-end NeedsReview pins (extend the edges-009 file).
- *(maybe)* `daemon/Cargo.toml` + `Cargo.lock` — only if the typed `graphql_client` approach is chosen (Step-2.5 Q1); the raw-query approach needs nothing new (`serde_json` is already in).

If implementation needs files beyond this list, flag at Step 2.5.

## RED test outline (Step 2)
Tests in `daemon/tests/github_read_client.rs`:

1. **`parse_review_decision_known_values`** — Asserts: `REVIEW_REQUIRED→ReviewRequired`, `APPROVED→Approved`, `CHANGES_REQUESTED→ChangesRequested`. Why: §9 GraphQL reviewDecision decode.
2. **`parse_review_decision_none_and_unknown_floor`** — Asserts: `None`/`null`/`"FOO"` → `None`. Why: edges-008 least-salient floor (unknown can't fabricate a decision).
3. **`parse_review_decision_case_insensitive`** — Asserts: `review_required`/`Approved` decode case-folded. Why: edges-008/009 case convention.
4. **`layer_review_required_overlays_onto_rest_none`** — Asserts: REST review `None` + GraphQL `ReviewRequired` → final `ReviewRequired`. Why: the core gap — REST can't see it.
5. **`layer_graphql_overrides_stale_rest`** — Asserts: REST `ChangesRequested` + GraphQL `Approved` → `Approved`. Why: GraphQL reviewDecision is authoritative when present.
6. **`layer_graphql_none_falls_back_to_rest`** — Asserts: REST `Approved` + GraphQL `None` → `Approved`. Why: `None` = no decision → the REST aggregate stands.
7. **`layered_review_required_derives_needs_review`** — Asserts: a PR (open / clean / no reviews / checks-success) layered with GraphQL `ReviewRequired` → `derive_pull_request_status == NeedsReview`. Why: §5.1 end-to-end — the gap edges-009 left as `Open`.
8. **`graphql_failure_degrades_to_rest_signals`** *(via the fake or a thin seam — see Step-2.5 Q2)* — Asserts: when the reviewDecision fetch fails, the returned signals equal the REST-only signals (review unchanged). Why: best-effort enrichment never loses the PR status. (If the live GraphQL failure isn't unit-reachable, pin `layer_review_decision(signals, None) == signals` + a `not-tested-because` on the live degrade path — the live fetch is fake-covered.)

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none.
- **Orchestrator doc rows to write hot (Step 9 routing):** none for `daemon/CLAUDE.md`/Appendix A. **Anticipated (integration-owned — FLAG, I route into the PLAN-DELTA hand-off):** a §9 arch-note (the GraphQL `reviewDecision` layering completes the §5.1 PullRequest read — `ReviewRequired`/`NeedsReview` is GraphQL-only; the best-effort-degrade semantics) + a C-list lesson extension (the REST/GraphQL two-source read with GraphQL authoritative + degrade-to-REST).
- **§2.5-seam (shared-contract) model touched?** **No** — daemon-internal over already-frozen daemon-internal enums → **no schema-snapshot test.**

## Things to flag at Step 2.5
1. **GraphQL query approach.** Lightweight **raw query** (`octocrab.graphql(&serde_json::json!({"query": "..."}))` → a small custom `Deserialize` struct, **no new dep**) vs. the typed **`graphql_client`** path (needs a committed GitHub GraphQL **schema file** + codegen dep — heavy for one tiny query). **Default vote: raw query** — one field (`repository(owner,name){pullRequest(number){reviewDecision}}`), the schema-file + graphql_client weight isn't justified. Confirm `octocrab.graphql`'s exact signature against 0.53.1 (you have Cargo/docs.rs).
2. **GraphQL-failure handling + testability.** Both `Err(octocrab::Error)` and `Ok(GraphqlResponse::Err)` → **degrade to REST-only** (reviewDecision = `None`). Is the degrade path unit-reachable, or only via the fake? **Default vote: degrade-to-REST** (best-effort enrichment); test the deterministic `layer_review_decision(.., None) == REST` + a `not-tested-because` on the live GraphQL round-trip (fake-covered, like edges-009's REST fetch). A `GraphqlResponse::Err` is HTTP-200 → it is NOT a `classify`-able status; treat it as `None`, optionally log the structural error count.
3. **`parse_review_decision` unknown → `None`.** **Default vote: `None`** (least-salient floor — an unknown decision must not fabricate `ReviewRequired`/`Approved`).
4. **Overlay precedence.** GraphQL `reviewDecision` authoritative when non-`None`, else the REST aggregate. **Default vote: as stated** — `reviewDecision` is GitHub's branch-protection-aware aggregate; `None` (no protection / unreachable) → the actual `reviews[]` aggregate is the better signal.
5. **Fold into `fetch_pr_signals` vs a separate method.** **Default vote: fold** — `fetch_pr_signals` returns fully-layered signals (one consumer call); the GraphQL fetch + layer happen after the REST extract. (`FakeGithubReadClient` is unaffected — it already returns canned full signals, so it can carry `ReviewRequired` directly.)

> Considered-and-rejected: a REST-only proxy (`requested_reviewers` non-empty → `ReviewRequired`). Rejected — `requested_reviewers` ≠ branch-protection-required-review (a PR can require review via protection with no explicit requested reviewer, and vice-versa); `reviewDecision` is the accurate signal and matches the edges-006/009 design intent.

## Dependencies + sequencing
- **Depends on:** edges-009 (`2eec8f2`) the REST client + `GithubReadClient`/`OctocrabGithubReadClient`/`extract_pr_signals`; edges-006 `ReviewDecision`/`aggregate_reviews`; edges-004 `derive_pull_request_status` (`ReviewRequired→NeedsReview`).
- **Blocks:** nothing new in-lane — **this completes the GitHub-PR read vertical.** The gated `github` executor + `proj_pull_request` projector consume the fully-layered `fetch_pr_signals`. After this, the next in-lane work is a different vertical (Linear read client, the git2 diff/rename refinement, or registry migrations — orchestrator's sequencing call).

## Estimated commit count
**1** — a focused layering on edges-009's client; deterministic decode + overlay are the test-first core, the GraphQL fetch is the fake-covered edge, no safety invariant, no cross-doc change. No new dep on the raw-query default.

## Lessons-logged candidates anticipated
- **Convention candidate** — "GitHub PR review state is a **two-source read**: REST `reviews[]` (`aggregate_reviews` → Approved/ChangesRequested/None) + GraphQL `reviewDecision` (the branch-protection-aware aggregate, the **only** source of `ReviewRequired`). The GraphQL value is **authoritative when present**, the REST aggregate the fallback; the GraphQL enrichment is **best-effort** — a failure (transport `Err` OR `GraphqlResponse::Err`) degrades to REST-only, never failing the read. A `GraphqlResponse::Err` is HTTP-200 → not `classify`-able by status." (extends the edges-006/009 §9/§7.2 line).
- **Architecture-doc note candidate** — `ReviewRequired`/`NeedsReview` is GraphQL-only under §9; the best-effort-degrade contract for the enrichment fetch.

## How to invoke
1. **Read this brief end-to-end** — esp. the 5 Step-2.5 questions (raw-query vs graphql_client, the degrade semantics, the overlay precedence).
2. **Run `/tdd github_review_decision_layering`** (already oriented — no `/session-start`).
3. **Step 0 (Restate)** — confirm the restatement matches the Feature line.
4. **Step 1 (files)** — confirm `github.rs` (extend) + the test file; flag if Q1 pulls in `graphql_client`.
5. **Step 2.5** — send the test-design write-up + answers (esp. the confirmed octocrab GraphQL signature + the raw-query-vs-typed call) before GREEN; wait for `APPROVED.`/`TWEAK:`/`ADD:`.
6. **Step 9** — surface cross-doc "none", any new dep (if `graphql_client`), and the anticipated §9 arch-note + C-list lesson (integration-owned — flag, I route).
