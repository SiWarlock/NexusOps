# /tdd brief — discard_activation (the W1-git-discard UI half)

## Feature
Activate the destructive per-hunk **Discard** control: send a `displayed_hunk_sha256`
content-hash in the `git.discard_hunk` action inputs (the lead-ruled **(A) verify-before-destroy**
content-hash, canonicalization LOCKED by the daemon at 096 Step-2.5), then flip the
`DISCARD_AVAILABLE` daemon-readiness hold to `true` once the daemon `git.discard_hunk` executor lands.

## Use case + traceability
- **Task ID:** W1-git-discard (the UI half — the plan's `- [ ] **W1-git-discard**` line says
  "Coordinate the UI `displayed_hunk_sha256` send with ui-orchestrator"; this brief IS that UI half).
- **Architecture sections it implements:** `ARCHITECTURE.md §6.3` (per-hunk git action catalog +
  resource targeting), `§6.1` (action `inputs` map + the `get_diff` `DiffLine` shape), `§17`
  (failure-mode contract — verify-before-destroy closes the same-position content-drift race),
  `§15` (destructive-action safety gates).
- **Related context:** ui-088 (`6a399fb`) held Discard behind `DISCARD_AVAILABLE=false` + the
  `it.skip`'d discard-submit integration signpost; LESSON [[42]] (daemon-readiness-vs-go-live hold);
  LESSON [[19]] (resource_ref verbatim from the displayed hunk — submitted==displayed); LESSON [[16]]
  (the pure-submitter intent seam). Daemon side = brief 096 / plan `W1-git-discard` (in flight, task #22).

### 🔒 LOCKED canonicalization (daemon-frozen at 096 Step-2.5; relayed + independently re-verified)
`sha256` over the hunk **BODY only** (the `@@` header EXCLUDED). For each `DiffLine` in `Hunk.lines`,
in order, emit ONE **origin byte** (`context`→`" "` / `added`→`"+"` / `removed`→`"-"`) immediately
followed by `DiffLine.content` **VERBATIM** — the RAW `content` field exactly as `get_diff` serves it
(the daemon already strips the origin marker INTO content; the trailing `\n` is retained). Concatenate
all lines with **NO separator**. SHA-256 → **lowercase hex** (64 chars).

**UI mirror:** `sha256(lines.map(l => PREFIX[l.kind] + l.content).join("")).toLowerCase()` —
`.join("")` is correct because `content` carries its own `\n`. Do **NOT** re-render / strip / re-add
newlines, and do **NOT** collapse leading whitespace in `content` (an indented context line's content
may legitimately start with a space → prefix `" "` + content `" x\n"` = `"  x\n"`, verbatim).

**FROZEN conformance vector (pin BOTH the canonical string AND the hash — this is the cross-language
contract lock, daemon pins the identical vector):**
- Input hunk lines: `[{kind:"context",content:"a\n"}, {kind:"removed",content:"b\n"}, {kind:"added",content:"c\n"}]`
- Canonical string: `" a\n-b\n+c\n"`
- `sha256` → `2980a502c1e0d3a04db1ff7021ede674af4f53fa3b352e485ac67e0b15c39ee6`
  (independently verified: `printf ' a\n-b\n+c\n' | shasum -a 256`).

**Daemon behavior:** re-derives the live hunk, re-computes this hash, compares; **absent / empty /
mismatch → `Failed`** ("hunk changed, re-examine") — no mutation. So a wrong/stale hash never destroys
the wrong content; the §17 content-drift race that stage's position-only posture accepts is CLOSED.

## Acceptance criteria (what "done" means)

**Commit 1 — the content-hash send (dispatch NOW; daemon executor NOT required):**
- [ ] A pure `displayedHunkSha256(hunk: Hunk): string` (co-located in `src/intent/hunk-resource-ref.ts`,
      the security-critical unit module) computes the LOCKED canonicalization above.
- [ ] The frozen conformance vector pins it EXACTLY: `displayedHunkSha256(VECTOR_HUNK) === "2980a502…ee6"`
      (the 64-char lowercase hex), AND the intermediate canonical string equals `" a\n-b\n+c\n"`.
- [ ] `buildHunkActionRequest("git.discard_hunk", …)` puts `{ displayed_hunk_sha256: <hex> }` in `inputs`.
- [ ] `buildHunkActionRequest("git.stage_hunk"/"git.unstage_hunk", …)` leaves `inputs` `null` (discard-only).
- [ ] The `PREFIX` map is `Record<DiffLineKind, string>` — tsc-complete over the generated 3-value enum;
      a drift test iterates `bundle.shape.DiffLineKind.options` and asserts every kind has a prefix
      (a daemon-added 4th kind fails until covered — a canonicalization-divergence net, [[5]]/[[30]]).
- [ ] Verbatim-content edges pinned: (a) a context line whose content starts with a space hashes with
      prefix+content un-collapsed; (b) a final line WITHOUT a trailing `\n` hashes verbatim (no fabricated `\n`).
- [ ] `DISCARD_AVAILABLE` UNCHANGED (`false`) this commit; Discard stays held (the send is inert until commit 2).
- [ ] All unit tests in `src/intent/hunk-resource-ref.test.ts` pass; `/preflight` clean.

**Commit 2 — the readiness flip (dispatch when the daemon `git.discard_hunk` executor SEALS, 096):**
- [ ] `DISCARD_AVAILABLE` flips to `true` (the lead-ruled flip = send-wired [commit 1] AND executor-landed).
- [ ] The `it.skip`'d `discard_hunk_submits_discard_action_type` integration test (`DiffReview.test.tsx:167`)
      is RE-ENABLED and EXTENDED to assert the discard submit carries `inputs.displayed_hunk_sha256` (the
      64-char hex), proving the end-to-end click→build→submit path now ships the content hash.
- [ ] The ui-088 `discard_held_until_daemon_executor` / `discard_disabled_tooltip_is_honest` tests are
      reconciled to the now-active state (Discard ENABLED when `canSubmit`; the readiness tooltip removed).
- [ ] `/preflight` clean. HITL visual-gate note recorded (live daemon + a worktree-with-changes →
      Discard a hunk → GatewayModal renders the daemon `preview_class=diff` → approve → daemon verifies
      the hash + discards; jsdom can't exercise this — flag for the user/lead visual sign-off).

## Wiring / entry point (Step 7.5)
**Commit 1:** `buildHunkActionRequest` is the production entry — already called by
`DiffReview.tsx` `ReviewTab.onAction` (`:199`) on every per-hunk button click. The new
`displayedHunkSha256` is reached the moment a `git.discard_hunk` request is built (commit 2 makes that
click possible; until then it is reached by the unit tests + by stage/unstage requests confirming the
`null`-inputs branch). **Commit 2:** the `DISCARD_AVAILABLE` const + the `HunkGitActions` Discard button
(`DiffReview.tsx:75`/`:125-136`) — the production Review tab renders `HunkGitActions` with the default
const, so the flip lights the live path. No new entry point; this activates an existing wired-but-held control.

## Files expected to touch
**Modified (commit 1):**
- `src/intent/hunk-resource-ref.ts` — add `displayedHunkSha256(hunk)` + the `PREFIX` Record; branch
  `buildHunkActionRequest` to set `inputs` for `git.discard_hunk` only.
- `src/intent/hunk-resource-ref.test.ts` — the conformance vector + discard-includes/stage-excludes +
  prefix-completeness + verbatim-content edge tests.
- `package.json` / `pnpm-lock.yaml` — IF the Step-2.5 hash-primitive decision adds a dep (see Q1).

**Modified (commit 2):**
- `src/views/code/DiffReview.tsx` — `DISCARD_AVAILABLE = false → true`; remove/repoint the readiness tooltip.
- `src/views/code/DiffReview.test.tsx` — un-skip + extend the discard-submit integration test; reconcile
  the two ui-088 held-state tests to the active state.

If implementation needs files beyond this list, **flag at Step 2.5** before going GREEN.

## RED test outline (Step 2)
Tests in `src/intent/hunk-resource-ref.test.ts` (commit 1):
1. **`displayed_hunk_sha256_matches_frozen_conformance_vector`** — `displayedHunkSha256(VECTOR_HUNK)`
   === `2980a502…ee6` AND the canonical string === `" a\n-b\n+c\n"`.
   - Why: the LOCKED daemon canonicalization (096 Step-2.5); the cross-language contract lock.
2. **`discard_request_carries_displayed_hunk_sha256_input`** — `buildHunkActionRequest("git.discard_hunk",…)`
   `.inputs.displayed_hunk_sha256` === `displayedHunkSha256(hunk)`.
   - Why: §6.3/§17 verify-before-destroy — the UI sends the content hash in INPUTS (additive).
3. **`stage_unstage_requests_have_null_inputs`** — both non-discard actions keep `inputs === null`.
   - Why: discard-specific; the daemon relay scoped the hash to `git.discard_hunk`.
4. **`diff_line_prefix_map_is_complete_over_generated_kinds`** — every `bundle.shape.DiffLineKind.options`
   value has a `PREFIX` entry.
   - Why: [[5]]/[[30]] — a daemon-added 4th kind must fail until the canonicalization covers it.
5. **`canonicalization_is_byte_verbatim`** — (a) leading-space context content not collapsed;
   (b) a no-trailing-`\n` final line hashed without a fabricated newline.
   - Why: the daemon's "do NOT re-render/strip/re-add newlines / collapse whitespace" invariant.

Tests in `src/views/code/DiffReview.test.tsx` (commit 2):
6. **`discard_hunk_submits_discard_action_type`** (un-skip) — extended: the submitted request's
   `action_type === "git.discard_hunk"` AND `inputs.displayed_hunk_sha256` is the 64-char hex.
   - Why: the end-to-end click→build→submit path ships the content hash once Discard is live.
7. Reconcile `discard_held_until_daemon_executor` → Discard ENABLED when `canSubmit` (active state).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** NONE. `inputs` is `z.unknown()` opaque passthrough (`intent-contracts.ts:114`)
  → adding an object key is NOT a contract/Zod change (the daemon relay confirmed "open inputs map,
  NO contract/Zod change"). **No `shared/` change, no regen, no CONTRACT bump.**
- **Orchestrator doc rows to write hot:** none (no frozen-shadow / value-set touched). A likely
  **convention candidate** (a UI-mirrored cross-language content-hash canonicalization, conformance-vector
  pinned both sides) → a `ui/LESSONS.md` entry at Step 9.
- **§2.5-seam model touched?** No — `inputs` opacity means no schema-snapshot test is required.

## Things to flag at Step 2.5
1. **Hash primitive — sync audited dep vs async Web Crypto.** Options: **(A)** a sync, audited sha256
   (`@noble/hashes` or `js-sha256`) — `buildHunkActionRequest` stays SYNC, the conformance test is a
   trivial sync assertion, zero env-availability risk; cost = +1 well-known dep (audited, zero-transitive).
   **(B)** `crypto.subtle.digest("SHA-256", new TextEncoder().encode(canonical))` — zero new dep,
   platform crypto, but ASYNC (compute in the already-async `onAction`, thread via an additive optional
   `inputs?` param on the builder) AND jsdom/vitest may not expose `crypto.subtle` (Node-global-dependent
   — verify first). **My default vote: (A)** — a security-relevant verify-before-destroy hash wants the
   dead-simple deterministic sync conformance pin with no test-env risk; the dep is justified. If you
   prefer to avoid the dep, (B) is acceptable ONLY after confirming `crypto.subtle` is available under
   `pnpm test:run`.
2. **Where `displayedHunkSha256` lives.** Default: co-located in `hunk-resource-ref.ts` (the existing
   security-critical unit, alongside `hunkResourceRef`/`buildHunkActionRequest`). Default vote: **there**
   — one module owns the "submitted == displayed" security surface ([[19]]).
3. **Always-send vs guard empty-hunk.** The UI always computes + sends a 64-char hex (even an empty/odd
   hunk → its hash); the daemon's absent/empty/mismatch→`Failed` is its own defense. Default vote:
   **always-send** — no UI-side empty special-casing; the daemon is the verify authority.

## Dependencies + sequencing
- **Commit 1 depends on:** nothing new — buildable NOW against the LOCKED spec (the daemon executor is
  NOT a prerequisite for the send-wiring; the conformance vector is the contract anchor).
- **Commit 2 depends on:** the daemon `git.discard_hunk` executor landing (plan `W1-git-discard`, task #22,
  brief 096). The daemon-orchestrator pings "096 sealed" → dispatch commit 2 (its task is `addBlockedBy`
  the daemon task → auto-unblocks on seal). **Do NOT flip `DISCARD_AVAILABLE` before that ping.**
- **Blocks:** nothing downstream.

## Estimated commit count
**2 — each is safety-relevant, so each gets its OWN commit** (per the bundle/atomize rule: a
safety-critical pin is never bundled). Commit 1 = the verify-before-destroy content-hash primitive
(security-reviewer: the canonicalization is the destroy-gating hash). Commit 2 = the destructive
go-live flip (security-reviewer: a flag-only flip that re-enabled destructive submit WITHOUT the
commit-1 hash would be the LESSON [[42]] footgun — the two are sequenced, never collapsed). Commit 2
is gated on the external daemon-seal signal, so this is a staged two-commit slice, not a back-to-back drive.

## Lessons-logged candidates anticipated
- **Convention candidate** — a UI-mirrored cross-language content-hash: canonicalize byte-verbatim to a
  daemon-frozen spec, pin BOTH the canonical string and the digest against ONE frozen conformance vector
  shared with the daemon; the prefix/format map is `Record`-complete over the generated enum.
- **Convention candidate** — verify-before-destroy: a destructive mutation's UI half sends a content hash
  of the displayed read so the daemon can refuse on drift (extends [[19]] submitted==displayed from
  position-identity to content-identity).
- **Architecture-doc note candidate** — §6.3/§17: `git.discard_hunk` inputs carry `displayed_hunk_sha256`;
  the daemon re-derives + compares (mismatch → `Failed`), closing the same-position content-drift race
  stage's position-only posture accepts.

## How to invoke
1. **Read this brief end-to-end** — especially the LOCKED canonicalization block + the Step-2.5 hash-primitive question.
2. **Run `/tdd discard_activation`** (commit 1 first; commit 2 only after the "096 sealed" wake).
3. **Step 2.5** — ping back the test-design write-up + the Q1 hash-primitive decision before GREEN.
4. **Step 9** — surface the content-hash convention candidate(s).
