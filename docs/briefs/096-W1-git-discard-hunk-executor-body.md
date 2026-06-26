# /tdd brief — git_discard_hunk_executor_body (SAFETY-PINNED, risk-3 DESTRUCTIVE)

## Feature
Implement the real `git.discard_hunk` Gateway-executor body in `GitExecutor` — re-derive the targeted hunk from the live working-tree diff, **verify a UI-supplied `displayed_hunk_sha256` content-hash against the re-derived hunk's canonical content BEFORE any mutation**, then reverse-apply the one-hunk patch to the **WORKING TREE** (`git apply -R`, NO `--cached`) to discard the change. Today `git.discard_hunk` is an R1-A registry stub (side-effect-free) → the UI Discard control is held disabled behind `DISCARD_AVAILABLE=false` (ui-088). **This is the DESTRUCTIVE / irreversible sibling of W1-git-stage (095) — its OWN safety-pinned slice, its OWN commit, MANDATORY security-reviewer.** The position-only race posture that stage/unstage ACCEPT (LESSONS §72) does **NOT** carry here — discard adds the content-hash guard.

## Use case + traceability
- **Task ID:** W1-git-discard
- **Architecture sections it implements:** `ARCHITECTURE.md §6.2`/`§6.3` (the Gateway executor body realizing the frozen `git.discard_hunk` catalog entry — **risk-3 · `standing_grant_eligible=false` · `preview_class=Diff`**, all frozen at 4.0b-ui1, NO catalog change this slice), `§17` (read↔mutate consistency for a DESTRUCTIVE op — fail-closed on ANY drift), `§15` (git-CLI mutation guards + structural-reason redaction + 0600 temp), `§6.1` (the `get_diff` / `daemon/src/git` read the hunk re-derives from).
- **LEAD RULING (2026-06-26) — (A) content-hash verify-before-destroy:** the UI sends a `displayed_hunk_sha256` in the action **INPUTS** (additive — rides the open `inputs` JSON map, NOT a resource-ref change, NOT a `shared/` contract type → **contract-neutral, no `CONTRACT_VERSION` bump**); the daemon re-derives the live hunk + computes the SAME canonical sha256 + verifies it equals `displayed_hunk_sha256` BEFORE discarding; mismatch → `Failed` "hunk changed since you reviewed it, re-examine" with NO mutation. This CLOSES the same-position content-drift race (a multi-agent worktree where the workdir hunk at the audited position now holds DIFFERENT content than the human approved) — irreversible content loss must never apply to un-reviewed bytes.
- **Related context — the sibling body to mirror:** `daemon/src/git/executor.rs::execute_apply_hunk` (W1-git-stage/095) — `validate` → `decode_hunk_ref` (the frozen `\x1f` position-only ref) → `reject_dash_operands(file)` (LESSONS §45) → `resolver.resolve(worktree_id)` (read-only WAL) → `git diff -- <file>` → `find_hunk_patch(positions)` (the §17 GUARD #1 no-match→Failed) → `TempPatch::write` (0600 O_EXCL, Drop-cleanup) → `git apply --check` (GUARD #2) → `git apply`. The decode/resolve/reject/temp helpers + `find_hunk_patch` are ALREADY THERE — reuse verbatim. Discard differs by: (1) the NEW content-hash guard between GUARD #1 and the temp write; (2) `git apply -R` against the WORKING TREE (NO `--cached`); (3) the patch source is the workdir-vs-index `git diff` (NO `--cached`, same as stage's forward read). LESSONS §47 (worktree git-axis = live-read cache → NO domain event), §63 (resolve from the AUDITED ref, never `inputs` — EXCEPT the hash, which is a verification token not a target), forbidden #6 (git CLI for mutations, never git2 mutating).
- **Scope (MVP):** discard an **unstaged working-tree** hunk (the workdir-vs-index change the UI displays by default). A staged-hunk discard is OUT (the user unstages then discards) — flag if the UI surface implies otherwise.

## Acceptance criteria (what "done" means)
- [ ] **`git.discard_hunk`** validates `requires_resource_refs` FIRST (`inner.validate`), decodes the `\x1f` position-only resource_ref (malformed → `Failed` before any git call), `reject_dash_operands` the `file` (LESSONS §45), resolves `worktree_id → path` over read-only WAL (unresolvable → `Failed`/NotFound-class).
- [ ] **§17 GUARD #1 (position):** reads the LIVE workdir diff (`git diff -- <file>`, NO `--cached`) and re-derives the matching hunk via `find_hunk_patch`; no position match → `Failed` (the displayed hunk is gone), NO mutation.
- [ ] **GUARD #2 (content-hash, the (A) ruling — the load-bearing destructive-op guard):** reads `displayed_hunk_sha256` from `inputs` (absent/empty/non-string → `Failed` "discard requires the displayed-hunk hash", fail-closed — a discard with NO hash is REFUSED, never falls through to position-only); computes the canonical sha256 over the re-derived hunk's content; `computed != displayed` → `Failed` "hunk changed since you reviewed it, re-examine", NO mutation. (The canonicalization is FROZEN at Step-2.5 — see "Things to flag".)
- [ ] **GUARD #3 (apply --check):** `git apply -R --check <patch>` runs FIRST against the working tree; non-clean → `Failed` structural "the hunk no longer applies (re-examine)", NO apply.
- [ ] **The destructive apply:** `git apply -R <patch>` (reverse-apply to the WORKING TREE — NO `--cached`) discards exactly that one hunk; other hunks / other files untouched; `side_effect_applied=true` on a clean apply.
- [ ] **§15:** raw git stderr NEVER reaches the persisted `ActionFailed` (structural class-names only — a path/diff carries content); the temp patch is 0600 + Drop-cleaned on EVERY return path (the `TempPatch` precedent).
- [ ] **NO domain event** — the worktree git-axis is a live-read cache (LESSONS §47); the UI re-reads `get_diff`. `emitted_events` empty (the Gateway `ActionSucceeded` IS the audit trail). **Contract-neutral** (catalog + resource-ref froze at 4.0b-ui1; the hash rides `inputs`).
- [ ] `/preflight` clean.

## Wiring / entry point (Step 7.5)
`daemon/src/git/executor.rs::GitExecutor::execute` — add a `GIT_DISCARD_HUNK` const + a match arm → `execute_discard_hunk` (a NEW body, NOT a `reverse` flag on `execute_apply_hunk` — discard is working-tree not index, adds the hash guard, and is DESTRUCTIVE; conflating it with the recoverable stage path is a safety smell). Reachable from the production gateway execute via the registered `ExecutorKind::Git` (main.rs registers `GitExecutor`; the create_worktree/`execute_apply_hunk` `/wired` precedent). Reuses `decode_hunk_ref`/`reject_dash_operands`/`self.resolver`/`find_hunk_patch`/`TempPatch`.

## Files expected to touch
**Modified:**
- `daemon/src/git/executor.rs` — the `execute_discard_hunk` body + `GIT_DISCARD_HUNK` const + match arm + a small `canonical_hunk_sha256(&Hunk)` (or `(&[DiffLine])`) helper (the FROZEN canonicalization — see Step-2.5). Possibly a `sha2` dep if not already present (`cargo tree | grep sha2` — the idempotency-key path uses SHA-256 already, LESSONS §20; reuse that dep, don't add a second).
- `daemon/tests/git_executor.rs` — extend (decode/resolve/re-derive-match / **hash-match-discards** / **hash-mismatch-fails-no-mutation** / **missing-hash-fails** / position-no-match-fails / --check-race-fails / reject-dash / structural-reason / no-event / other-hunks-untouched).

**New:** none expected (extends GitExecutor; reuses the git read module + the existing SHA dep).

If implementation needs a file beyond this list (e.g. the canonicalization wants to live in `daemon/src/git/mod.rs` next to `find_hunk_patch` so a shared conformance test can reach it), **flag at Step 2.5**.

## RED test outline (Step 2)
Tests in `daemon/tests/git_executor.rs` (real git-repo fixtures — the `execute_apply_hunk` precedent):

1. **`git_discard_hunk_removes_workdir_change`** — a worktree with an unstaged hunk + the CORRECT `displayed_hunk_sha256` → discard reverts exactly that hunk in the working tree (the file no longer shows it; the index untouched; OTHER hunks untouched). Why: §6.3 body + the destructive apply.
2. **`git_discard_hunk_hash_mismatch_fails_closed`** *(THE load-bearing safety pin)* — the live hunk content drifted (same position, different bytes) so the re-derived canonical hash ≠ `displayed_hunk_sha256` → `Failed`, the working tree UNCHANGED (assert the bytes survive). Why: the (A) ruling — irreversible loss never touches un-reviewed content.
3. **`git_discard_hunk_missing_hash_fails_closed`** — `inputs` has no `displayed_hunk_sha256` (or empty/non-string) → `Failed` before any apply, working tree unchanged. Why: GUARD #2 fail-closed (no hash ⇒ no destructive fall-through to position-only).
4. **`git_discard_hunk_no_matching_position_fails`** — the resource-ref positions match no live hunk → `Failed`, no apply. Why: §17 GUARD #1.
5. **`git_discard_hunk_check_race_fails_closed`** — `git apply -R --check` non-clean (forced via a per-command fake, the LESSONS §72 technique — the re-derived patch ~always check-passes otherwise) → `Failed`, no apply. Why: §17 GUARD #3.
6. **`git_discard_hunk_malformed_ref_or_missing_target_fails`** — malformed `\x1f` id / no resource_ref → `Failed` before any git call. Why: §6.3 precondition + LESSONS §63.
7. **`git_discard_hunk_rejects_dash_file_operand`** — a leading-`-` file → rejected fail-closed. Why: LESSONS §45.
8. **`git_discard_hunk_structural_reason_no_stderr`** — a forced git failure → the `ActionFailed` reason is a structural class, NOT raw git stderr. Why: §15.
9. **`git_discard_hunk_emits_no_event`** — a successful discard emits NO domain event (`emitted_events` empty); `side_effect_applied=true`. Why: LESSONS §47 live-read cache.
10. **`canonical_hunk_sha256_is_stable`** — the canonicalization helper produces the frozen, byte-stable hash for a fixture hunk (the conformance anchor the UI mirrors). Why: the cross-cutting daemon↔UI contract.

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** none (`displayed_hunk_sha256` rides the open `inputs` map; catalog + resource-ref froze at 4.0b-ui1). **NO `CONTRACT_VERSION` bump → NO paired UI regen** (contrast LESSONS §69 — that's for `shared/` type changes; an `inputs` key is not one).
- **NEW FROZEN CONVENTION (orchestrator writes hot at Step-9):** the `displayed_hunk_sha256` canonicalization (the exact bytes hashed) — a documented cross-doc convention like the `\x1f` hunk-encoding (daemon/CLAUDE.md §6.3 catalog row + the UI `hunk-resource-ref` sibling). The orchestrator relays the LOCKED canonicalization to ui-orchestrator so the UI send + the daemon verify are byte-identical (a 1-byte disagreement fails EVERY discard closed — total feature break).
- **Orchestrator doc note (Step-9):** `git.discard_hunk` executor body now LIVE (the last R1-A git stub realized); the position-only-accepted-limitation does NOT carry (LESSONS §72 contrast).
- **§2.5-seam:** NO (no shared model changes).

## Things to flag at Step 2.5 (the canonicalization is THE load-bearing decision)
1. **The sha256 canonicalization — what EXACT bytes are hashed (cross-cutting; freezes the daemon↔UI contract).** My default vote: hash the **hunk BODY content lines only** — for each `DiffLine` in the re-derived `Hunk.lines`, emit `{origin_char}{content}{LF}` where `origin_char` = `' '` (Context) / `'+'` (Added) / `'-'` (Removed); sha256 → **lowercase hex**. Rationale: this is exactly what the UI renders + unambiguously has from `get_diff` (the `DiffLine.content` is already origin-stripped — ui-048 fixup `74d26e6`), and the daemon reconstructs it from the SAME `parse_unified_diff` that served `get_diff`. EXCLUDE the `@@` header (positions are already verified by GUARD #1; the header's optional trailing section-heading is a fidelity hazard). **Sub-decisions to lock:** (a) trailing-LF after the last line — always append, vs match git's "\ No newline at end of file" handling (vote: always append a LF per line; treat the no-eol marker as a Context-less terminal — confirm both sides agree); (b) confirm `DiffLine.content` carries NO trailing `\n` (so we add exactly one). Flag if any of (a)/(b) is ambiguous — this is the ONE thing that must be perfect.
2. **`canonical_hunk_sha256` location** — `executor.rs` (private) vs `git/mod.rs` next to `find_hunk_patch` (so a shared/conformance test can reach it). Vote: `git/mod.rs`, `pub(crate)`, with the stability test (Test 10) as the frozen anchor.
3. **SHA dep** — reuse the existing SHA-256 dep (the idempotency-key path, LESSONS §20). Vote: reuse `sha2`; do NOT add a second hashing crate. Confirm the import.
4. **Working-tree vs index target** — discard reverse-applies to the WORKING TREE (`git apply -R`, NO `--cached`); the read is workdir-vs-index (`git diff`, NO `--cached`). Vote: confirmed working-tree. Staged-hunk discard is OUT of MVP (flag if the UI implies otherwise).
5. **Separate body, not a `reverse` flag** — `execute_discard_hunk` is its OWN fn (working-tree + hash-guard + DESTRUCTIVE), NOT a branch on `execute_apply_hunk`. Vote: separate body (safety clarity; the destructive path must not share the recoverable index path's structure).

## Dependencies + sequencing
- **Depends on:** `GitExecutor` + `execute_apply_hunk` (095, landed `b3728c6`) + `find_hunk_patch`/`decode_hunk_ref`/`TempPatch`/the resolver (landed) + the frozen `git.discard_hunk` catalog entry + position-only resource-ref (4.0b-ui1).
- **Blocks:** the UI Discard activation (ui-088's `DISCARD_AVAILABLE` flip to `true`) — which ALSO needs the UI to send `displayed_hunk_sha256` per the LOCKED canonicalization. The orchestrator coordinates that with ui-orchestrator AFTER Step-2.5 locks the canonicalization (so the UI never builds against a guess).
- **Then:** W2-audit (`event_type` on `proj_audit_trail`) → the rest of WAVE-2.

## Estimated commit count
**1.** The destructive discard body is one cohesive safety-critical unit. **SAFETY-PINNED — its OWN commit** (never bundled — the `daemon/CLAUDE.md` "never bundle a safety-critical slice with anything else" rule; the W1-git-discard slice is the canonical case). **`security-reviewer` is MANDATORY** (risk-3 DESTRUCTIVE / irreversible + INV-SEC-1 + §15/§17 + the new verify-before-destroy guard — this is the `invariant` policy trigger AND a cat-1-weight destructive op). Load-bearing pins: the hash-mismatch-no-mutation (Test 2), the missing-hash-fail-closed (Test 3), the structural-reason (Test 8).

## Lessons-logged candidates anticipated
- **Convention candidate** — "a DESTRUCTIVE / irreversible per-hunk git mutation adds a content-hash verify-before-destroy guard (the UI sends `displayed_hunk_sha256` over the open `inputs` map; the daemon re-derives + compares against a FROZEN canonicalization; mismatch/absent → fail-closed, NO mutation) ON TOP OF the §72 position-only re-derivation + the two apply-guards — the position-only accepted limitation explicitly does NOT carry to an irreversible op (LESSONS §72 contrast)."
- **Architecture-doc note candidate** — `git.discard_hunk` body LIVE (the last git R1-A stub realized) + the `displayed_hunk_sha256` canonicalization frozen convention.

## How to invoke
1. Read this brief end-to-end (esp. Step-2.5 Q1 — the canonicalization is the cross-cutting freeze; and the §17 GUARD ordering: position → hash → check → apply).
2. Run `/tdd git_discard_hunk_executor_body`.
3. Step 2.5 — ping back with answers (esp. the LOCKED canonicalization bytes); reply gates on it.
4. Step 8 — `security-reviewer` MANDATORY (destructive / irreversible).
5. Step 9 — surface the frozen canonicalization for the orchestrator to relay to ui-orchestrator + the §6.3 doc note.
