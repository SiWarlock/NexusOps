# /tdd brief — codex_launch_auth_profile_umask

## Feature
The deterministic **CodexAdapter launch + auth/profile + perms mechanism**: a pure `CodexLaunchSpec`
(the `codex exec` argv — the no-bypass enforcement surface, the `ClaudeLaunchSpec` analog), Codex **auth
resolution** that **refuses AMBIGUOUS auth** (the no-silent-account-hop §15 #8, the `codex doctor`
analog), **execution-profile binding** (the resolved `(auth,model,provider,approval,sandbox,profile)` →
the daemon-internal profile + the frozen `execution_profile_id` handle recorded at `SessionStarted`,
§15 #8), **umask-0077 perms hardening** (§15 #11 — pre-create `~/.codex/sessions` 0700 + the child umask
so Codex's 0644/0755 become 0600/0700), and the **Codex resume-handle** for the harness-agnostic
`decide_resume` (the `codex exec resume <UUID>` form). **NON-cat-1** — mechanism-built, **NO live spawn,
NO interception** (the reachable live codex + its `PreToolUse`→Gateway interception + the `--sandbox`
defense-in-depth land TOGETHER at **3.3c**, the binding condition). The second slice of the 3.3 Codex
arc (mirrors the Claude `ClaudeLaunchSpec`/042 launch-spec + the 4.0b-1 §15 #8 binding precedents).

## Use case + traceability
- **Task ID:** P3.3b (the 3.3 decomposition — see the §3.3 section of `IMPLEMENTATION_PLAN.md`).
- **Architecture sections it implements:** `ARCHITECTURE.md §9.1` (the `HarnessAdapter` launch/resume +
  the launch-spec-as-enforcement-surface), **§15** (the safety invariants — **#8 execution-profile
  binding** "resolved at approval time + recorded in `SessionStarted`; no silent account-hopping" + **#11
  Codex rollout-files hardened: pre-create `~/.codex/sessions` 0700, files 0600"**), **§8.1** (the
  B2-strict survival ladder — the Codex resume-handle the harness-agnostic `decide_resume` keys off),
  **§5.1** (the `ExecutionProfile` runtime-state machine the binding records, frozen 0.24.0).
  - **Cross-phase anchor declaration (for spec-lint):** this brief **widens phase scope because** the
    §15 (#8/#11) safety invariants + the §8.1 survival ladder are **cross-cutting anchors applied
    per-harness at launch** — sitting outside the Phase-3 Spec-anchors line (§9.1/§5.1/§6/§7.2/§9/§0.1/
    §0.2/§18) but implemented by every harness launch path. This is the exact precedent of **4.0b-1**
    (cited §15 #8 for the Claude session-create binding) and **4.1a/4.1b-1** (cited §8/§8.1 for
    `decide_resume`). _(The other `§`-form tokens the linter sees — §3.2/§4.3/§5/§7 are
    `0.3-codex-schema-research.md` sections, §3.3 is the `IMPLEMENTATION_PLAN.md` decomposition section,
    §2.5 is the contract-seam mechanism — are doc cross-references, not additional ARCHITECTURE anchors.)_
- **THE FOUNDATION (read it first):** `docs/planning/0.3-codex-schema-research.md` — **§3.2** (resume
  mechanism → `ResumeMode` map; the `codex exec resume <UUID>` CLI form CONFIRMED-LOCAL vs the
  app-server `thread/resume`/`thr_` path), **§4.3** (the approval × sandbox enforcement matrix +
  the INV-SEC-1 nuance — sandbox is the OS-enforcement layer, 3.3c's cat-1 concern), **§5** (auth:
  `CodexAuth{ApiKey|Chatgpt|ChatgptAuthTokens|AgentIdentity}`, `auth.json` 0600, the env overrides,
  the `codex doctor` multi-auth warning, the profile-binding handle), **§6** (the mirror/diverge table —
  the "Perms hardening" + "Auth/profile binding" rows), **§7** (Open-Qs #3/#4/#8/#9 — the HITL set).
- **Related context:** `daemon/src/harness/claude/mod.rs` (the `ClaudeLaunchSpec` precedent — pure argv
  `build()` + the fallible fail-closed `write_settings()` + `env_mutations()` env-hygiene + the
  enforcement-surface posture #10; the CodexLaunchSpec mirrors the STRUCTURE, different flags),
  `daemon/src/session/launcher.rs` (the `PtyLauncher` — Option A: the launcher OWNS the single live
  spawn site + the fail-closed settings write; **3.3b does NOT add a spawning `CodexLauncher` here** —
  that's 3.3c, the binding condition), `daemon/src/harness/resume.rs` (`decide_resume`/`ResumeInputs` —
  the harness-agnostic ladder the resume-handle feeds), `daemon/src/session/recovery.rs`
  (`RecoverableSession.{supports_resume,has_resume_handle,execution_profile_id}` — the production
  consumer of the resume-handle), `daemon/src/harness/codex/mod.rs` (the 3.3a observe core — the
  `CodexAdapter` this extends; `capabilities().supports_resume=true` already set), LESSON 25 (the
  launch-spec-as-#10-enforcement-surface, fail-closed), 30 (is the safety INVARIANT a function of this
  I/O? — the umask/dir-precreate IS → fail-closed), 38 (§15 #8 profile preserved from the committed
  `SessionStarted`), 39 (the launcher wraps the UNCHANGED spec; safety = a property of the constructed
  argv, deterministically pinned even though live survival is HITL), 42 (the 3.3a second-adapter pattern).

## Scope ruling (restate at Step 0)
- **BUILD (deterministic launch/auth/profile/umask/resume mechanism):** (a) `CodexLaunchSpec` — the pure
  `codex exec [resume <UUID>] --json --sandbox <s> --ask-for-approval <a> --model <m> --profile <p>`
  argv (no `--yolo`/`--dangerously-bypass-approvals-and-sandbox`/`--full-auto` possible by construction)
  + `env_mutations()` (the auth-pinning env hygiene); (b) **auth resolution** — `resolve_codex_auth` that
  refuses AMBIGUOUS auth (fail-closed) and pins ONE method; (c) **profile binding** — the resolved
  tuple → a daemon-internal profile config + the `execution_profile_id` handle (keychain-ref auth, no
  Debug-leak); (d) **umask-0077 hardening** — `harden_codex_dirs()` (pre-create `~/.codex/sessions`
  0700, fail-closed) + the umask-0077 value on the spec; (e) **resume-handle** — the `codex exec resume
  <UUID>` variant + the `has_resume_handle` predicate feeding `ResumeInputs`.
- **DEFER (the rest of the 3.3 arc):** the CAT-1 `PreToolUse`→Gateway interception + the **`--sandbox`
  defense-in-depth PROOF** + the real spawn-site `CodexLauncher` (`SessionLauncher` impl) → **3.3c** ·
  telemetry emission → **3.3d** · the LIVE drive loop + the umask-doesn't-chmod-back / app-server-resume
  live checks → the **HITL** follow-on. **This slice spawns NOTHING** (NON-cat-1 by construction — the
  binding condition: a reachable live agent lands WITH its interception, the 042→043/4.0b-1→4.0b-2
  precedent).
- **Target the OSS `codex` surfaces** (the `codex exec` flag grammar + the `CodexAuth` enum + the
  `~/.codex/` layout), keying on the research's CONFIRMED-DOCS/LOCAL flags; the live auth/sandbox-flag
  behavior + the OSS-version flag refresh are HITL (research §7 / build-implications).

## Acceptance criteria (what "done" means)
- [ ] **`CodexLaunchSpec` (pure argv).** `build(cwd, profile, …)` → `codex exec --json --sandbox <s>
  --ask-for-approval <a> --model <m> --profile <p>` (exact tokens). A bypass is **impossible by
  construction** — there is no code path that emits `--yolo` / `--dangerously-bypass-approvals-and-sandbox`
  / `--full-auto`; `--sandbox` is ALWAYS present (a safe default, never silently omitted). The
  enforcement-surface posture of `ClaudeLaunchSpec` (the locked `--permission-mode default`), applied to
  Codex (§15 / the 3.3c cat-1 boundary's OS-enforcement half).
- [ ] **Auth resolution refuses ambiguity.** `resolve_codex_auth(env, auth_json_state)` → `Ok(CodexAuth)`
  when exactly ONE auth source is resolvable; **`Err(AmbiguousAuth)`** (→ refuse to launch, fail-closed)
  when ≥2 are present (e.g. an API key in env [`OPENAI_API_KEY`/`CODEX_API_KEY`/`CODEX_ACCESS_TOKEN`]
  AND ChatGPT tokens in `auth.json`). The `codex doctor` multi-auth-warning model → our hard refuse
  (§15 #8 no-silent-account-hop).
- [ ] **Auth-pinning env hygiene.** `env_mutations()` strips the NON-chosen auth env sources so the
  spawned child cannot account-hop off the resolved method (the `ClaudeLaunchSpec` `ANTHROPIC_API_KEY`-
  strip precedent) + carries the `NEXUSOPS_SESSION_ID` correlation key.
- [ ] **Profile binding.** The resolved `(auth, model, provider, approval_policy, sandbox, profile)` →
  a daemon-internal `CodexExecutionProfile` config + the frozen `ExecutionProfileId` handle that §15 #8
  records at `SessionStarted` (the 4.0b-1 binding shape). **The auth field carries a keychain-ref /
  pointer, NEVER the secret**, and the config's `Debug`/any serialization **does not leak it** (§15 #4;
  the carry-forward "OAuth profile-config = keychain-only + no-Debug-leak").
- [ ] **§15 #11 umask hardening (fail-closed).** `harden_codex_dirs()` pre-creates `~/.codex/sessions`
  at **0700** (mode-checked) and **fails closed** (returns `Err` → no launch) if it cannot create/secure
  them; the spec carries **umask 0077** (so the child's Codex-created rollout dirs/files land 0700/0600).
  Is the §15 #11 invariant a function of this I/O succeeding? **Yes** → fail-closed (LESSON 30).
- [ ] **Resume-handle for `decide_resume`.** A `CodexLaunchSpec` resume variant → `codex exec resume
  <UUID> --json …` (keyed off the rollout **UUIDv7**, the CONFIRMED-LOCAL CLI form) + a `has_resume_handle`
  predicate (true iff a resumable rollout UUID exists) so the harness-agnostic `decide_resume` (4.1a)
  picks `Resumed` for Codex vs falls through. The app-server `thread/resume`/`thr_` path + the
  UUID↔`thr_` interconversion are HITL (research §7 Open-Q #3/#4 — folds the carry-forward).
- [ ] **NO spawn (the binding condition).** No production-reachable code path spawns a real `codex` in
  this slice (structurally — the launch module exposes no spawn call; no `SessionLauncher` impl for
  Codex). The reachable live spawn + interception = 3.3c (the 042/4.0a/4.0b-1 "mechanism built, no live
  caller" pattern).
- [ ] All unit tests in `daemon/tests/codex_launch.rs` (+ `daemon/src/harness/codex/` `#[cfg(test)]`)
  pass; `/preflight` clean. **NO CONTRACT bump** (daemon-internal launch spec + auth + profile config;
  the `execution_profile_id` handle + the `ExecutionProfile` enum are already frozen @0.24.0;
  `ResumeInputs` is daemon-internal).

## Wiring / entry point (Step 7.5)
**none — the production spawn caller is 3.3c (+ the HITL live drive).** This slice is the
launch-spec/auth/profile/umask/resume-handle MECHANISM, exercised by unit tests; it spawns nothing (the
binding condition — a reachable live codex lands WITH its `PreToolUse`→Gateway interception at 3.3c, the
042→043/4.0b-1→4.0b-2 precedent). Confirm at Step 7.5: (1) the auth/profile/umask/spec are reachable
from the unit tests; (2) the resume-handle bit feeds `ResumeInputs.{supports_resume,has_resume_handle}`
(the `RecoverableSession`→`recover_sessions_on_restart` consumer chain); (3) **no `codex` spawn call
exists** in the slice's surface (the structural no-spawn pin).

## Files expected to touch
**New:**
- `daemon/src/harness/codex/launch.rs` — `CodexLaunchSpec` (pure argv + the resume variant +
  `env_mutations()`) + the §15 #11 `harden_codex_dirs()` / umask-0077 fail-closed I/O.
- `daemon/src/harness/codex/auth.rs` — `CodexAuth` + `resolve_codex_auth` (refuse-ambiguous) + the
  `CodexExecutionProfile` binding (keychain-ref, no-Debug-leak).
- `daemon/tests/codex_launch.rs` — the launch/auth/profile/umask/resume tests.

**Modified:**
- `daemon/src/harness/codex/mod.rs` — `pub mod launch; pub mod auth;` + the `CodexAdapter` resume-handle
  method (the `has_resume_handle`/`resume_inputs` bit; optionally tighten `resume()` to use it).

If implementation needs files beyond this list (e.g. a small `terminal::EnvMutation` reuse, or splitting
auth/profile into separate files), **flag at Step 2.5** before going GREEN. **Do NOT add a spawning
`CodexLauncher` to `daemon/src/session/launcher.rs`** — that's 3.3c (the binding condition); flag if you
think otherwise.

## RED test outline (Step 2)
Tests in `daemon/tests/codex_launch.rs` (+ module `#[cfg(test)]`):

1. **`test_codex_launch_spec_argv`** — `CodexLaunchSpec::build(...)` → the exact `codex exec --json
   --sandbox <s> --ask-for-approval <a> --model <m> --profile <p>` token sequence.
   - Asserts: the argv equals the expected ordered tokens; program is `codex`.
   - Why: §9.1 launch / the `ClaudeLaunchSpec::build` precedent.
2. **`test_launch_spec_no_bypass_by_construction`** — `--sandbox` is ALWAYS present; no constructor path
   emits `--yolo` / `--dangerously-bypass-approvals-and-sandbox` / `--full-auto`.
   - Asserts: the argv carries `--sandbox` for every spec; a grep/structural check shows no bypass flag
     string in the spec's emitted argv surface.
   - Why: §15 INV-SEC-1 enforcement surface (the spec is the no-bypass half; 3.3c proves the sandbox).
3. **`test_resolve_auth_single_method`** — exactly one auth source → `Ok(that method)`.
   - Asserts: env-only API key → `Ok(ApiKey)`; auth.json-ChatGPT-only → `Ok(Chatgpt)`.
   - Why: §15 #8 profile binding.
4. **`test_resolve_auth_ambiguous_refused`** — ChatGPT tokens in `auth.json` AND `OPENAI_API_KEY` in env
   → `Err(AmbiguousAuth)`.
   - Asserts: the resolver returns the ambiguous error (no silent pick); launch would refuse fail-closed.
   - Why: §15 #8 no-silent-account-hop (the `codex doctor` analog).
5. **`test_auth_env_hygiene_pins_resolved`** — `env_mutations()` removes the non-chosen auth env vars so
   the child can't account-hop; carries `NEXUSOPS_SESSION_ID`.
   - Asserts: for a resolved ChatGPT/OAuth method, the mutations `remove` the API-key env vars (and vice
     versa); the correlation key is `set`.
   - Why: §15 #8 (the `ClaudeLaunchSpec` `ANTHROPIC_API_KEY`-strip precedent).
6. **`test_profile_binding_records_resolved`** — the resolved tuple → the `CodexExecutionProfile` +
   the `ExecutionProfileId` handle (the §15 #8 record-at-start shape).
   - Asserts: the bound profile carries `(auth-method, model, provider, approval, sandbox, profile)`;
     the handle is a valid `ExecutionProfileId`.
   - Why: §15 #8 / §5.1 (the frozen `ExecutionProfile`).
7. **`test_profile_no_secret_leak`** — the profile/auth config never exposes the auth secret (keychain-ref
   only; `Debug`/serialization is secret-free).
   - Asserts: `format!("{:?}", profile)` (and any to-string) contains no secret material — only a
     keychain-ref/pointer.
   - Why: §15 #4/#8 (keychain-refs-only; the OAuth-profile-config no-Debug-leak carry-forward).
8. **`test_umask_dir_precreate_0700`** — `harden_codex_dirs(root)` creates `<root>/sessions` at mode 0700.
   - Asserts: against a `tempdir` root, the dir exists with mode `& 0o777 == 0o700`.
   - Why: §15 #11.
9. **`test_umask_hardening_fail_closed`** — a dir-precreate / chmod failure → `Err` (no launch with
   un-hardened dirs).
   - Asserts: an unwritable/again-as-file root → `Err`; the caller would refuse to launch (the
     `write_settings()?` fail-closed precedent; LESSON 30 — the §15 #11 invariant IS a function of this I/O).
   - Why: §15 #11 + fail-closed posture.
10. **`test_launch_spec_umask_0077`** — the spec/launch surface carries umask `0o077`.
    - Asserts: the spec exposes the umask value `0o077` (so Codex's 0644/0755 → 0600/0700 at the real
      spawn; the live "Codex doesn't chmod back" check is HITL Open-Q #9).
    - Why: §15 #11.
11. **`test_resume_handle_uuid_keyed`** — the resume variant `build_resume(cwd, uuid, profile)` →
    `codex exec resume <UUID> --json …`; `has_resume_handle` true iff a resumable rollout UUID is present.
    - Asserts: the resume argv carries `resume <UUID>`; `has_resume_handle(Some(uuid))==true`,
      `has_resume_handle(None)==false` (so `decide_resume` picks `Resumed` vs falls through).
    - Why: §8.1 / research §3.2 (the UUID CLI form; the `thr_`/app-server path + UUID↔thr_ = HITL Open-Q #4).
12. **`test_no_codex_spawn_in_slice`** — structural: the codex launch surface exposes NO spawn /
    production-launcher call (the binding condition).
    - Asserts: a grep over `daemon/src/harness/codex/` finds no `spawn`/`Command::new("codex")`/
      `SessionLauncher` impl (the 042/4.0a "mechanism built, no live caller" idiom).
    - Why: the cat-1 binding condition (the live spawn + interception = 3.3c, together).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** **none** in `shared/` — daemon-internal launch spec + auth + profile config.
  The `ExecutionProfileId` handle + the `ExecutionProfile` runtime-state enum are already frozen
  (@0.24.0); `ResumeInputs`/`RecoverableSession` are daemon-internal. **NO CONTRACT bump.**
- **Shared-contract seam (§2.5) model touched?** **No** (no `shared/` field added/changed → no
  schema-snapshot test needed; the 3.3a precedent).
- **Orchestrator doc rows to write hot (Step 9):** a **§9.1 + §15 AS-BUILT note** (the CodexAdapter
  launch/auth/profile/umask/resume mechanism LIVE; the spawn + the cat-1 `PreToolUse`→Gateway
  interception + the `--sandbox` defense-in-depth deferred to 3.3c; telemetry → 3.3d) + the LESSON
  candidate + the `daemon/CLAUDE.md` module-org note if `harness/codex/` grew files. Orchestrator-written.

## Things to flag at Step 2.5
1. **The `--sandbox` / `--ask-for-approval` defaults — 3.3b's spec carries them (yes) vs 3.3c owns the
   value.** My default vote: **3.3b's `CodexLaunchSpec` carries `--sandbox` + `--ask-for-approval` as
   REQUIRED fields** (no-bypass-by-construction = the §15 enforcement surface, the `ClaudeLaunchSpec`
   `--permission-mode default` precedent); default sandbox = **`workspace-write` scoped to the cwd /
   approved worktree** (research §4.3), approval = `on-request`. The cat-1 ARGUMENT that sandbox+hook
   together form the INV-SEC-1 boundary + the live containment PROOF = 3.3c. **Escalation check:** if you
   or the security-reviewer judge the sandbox-DEFAULT selection to be itself a cat-1 decision (not just a
   safe default), flag it → I escalate lead→user before sign-off. My read: a safe default here + the
   cat-1 boundary argument at 3.3c is correct (no new fork) — but confirm the default value.
2. **Auth-ambiguity definition — what counts as "ambiguous"?** My default vote: **≥2 distinct resolvable
   auth sources** → refuse fail-closed (an env API key [`OPENAI_API_KEY`/`CODEX_API_KEY`/
   `CODEX_ACCESS_TOKEN`] AND `auth.json` ChatGPT tokens); a single source → resolve + pin. The `codex
   doctor` multi-auth warning is the model. Confirm the exact env-var set + whether a `--profile`'s
   explicit `model_provider` disambiguates (my vote: it selects the provider, not the auth method —
   auth ambiguity is still a refuse).
3. **Profile config shape — daemon-internal vs a `shared/` contract?** My default vote: **daemon-internal
   `CodexExecutionProfile`** in `harness/codex/`; the only `shared/` surface is the already-frozen
   `ExecutionProfileId` handle (@0.24.0). Auth = keychain-ref/pointer, never the secret (no-Debug-leak).
   **NO CONTRACT bump** (the 3.3a + the Claude profile-config precedent). Confirm.
4. **umask mechanism — deterministic dir-hardening now, live-verify HITL.** My default vote: 3.3b builds
   **(a)** `harden_codex_dirs()` (pre-create `~/.codex/sessions` 0700, fail-closed) as the deterministic
   unit-testable I/O, and **(b)** the umask-0077 VALUE on the spec (the spawner applies it pre-exec at the
   real spawn = 3.3c). The live "Codex doesn't chmod the dirs/files back" check = HITL (Open-Q #9).
   Confirm the split (mechanism now, live-verify HITL).
5. **Resume handle — UUID-keyed; the `thr_`/app-server path deferred.** My default vote: 3.3b keys the
   resume handle off the rollout **UUIDv7** (the `codex exec resume <UUID>` CLI form, CONFIRMED-LOCAL);
   `has_resume_handle`=true iff a resumable rollout UUID is present; the app-server `thread/resume`/`thr_`
   path + the UUID↔`thr_` interconversion = HITL (research §7 Open-Q #3/#4 — folds the carry-forward).
   Confirm.

## Dependencies + sequencing
- **Depends on:** 3.3a (✅ the CodexAdapter observe core + `capabilities().supports_resume=true`),
  4.0b-1 (✅ the §15 #8 `execution_profile_id`/`SessionStarted` binding shape + the `ExecutionProfile`
  freeze @0.24.0), 4.1a (✅ `decide_resume`/`ResumeInputs`/`ResumeMode`), the 0.3 research (✅). NOT a
  live Codex (this slice is mechanism-only).
- **Blocks:** 3.3c (the CAT-1 interception + the real spawn-site `CodexLauncher` + the `--sandbox`
  defense-in-depth — consumes the launch spec + the profile binding + the umask hardening) · the HITL
  live drive (consumes the auth resolution + the resume handle).

## Estimated commit count
**2–4** (likely **3**). The **two §15 safety pins each get their OWN commit** (root `CLAUDE.md`
"safety-critical pin → own commit"): (1) the §15 #8 **auth resolution + profile binding** (refuse-ambiguous
+ keychain-ref/no-Debug-leak); (2) the §15 #11 **umask/dir hardening** (fail-closed). The non-safety
**`CodexLaunchSpec` argv + env-hygiene + the resume-handle variant** is a 3rd commit (it can fold the
resume-handle since both are pure argv construction). Do NOT bundle a §15 pin with the others. Confirm
the split at Step 2.5 (the implementer finalizes).

## Reviewer subagents (Step 8 policy)
- **`security-reviewer`: YES** (the `invariant` policy — this slice touches **TWO** §15 invariants: #8
  (execution-profile binding / no-account-hop) + #11 (rollout-files hardened)). The review surface: the
  refuse-ambiguous-auth fail-closed logic, the keychain-ref/no-secret-leak profile, the env-hygiene
  auth-pinning, the umask/dir-precreate fail-closed, and the **no-spawn binding condition** (no reachable
  live codex). **NOT cat-1** (no interception, no live agent, no spawn) → one pass over the §15 #8/#11
  surfaces, not per-layer. _(The CAT-1 per-layer security pass + the lead→user design surface is 3.3c.)_
- **`code-quality-reviewer`: YES** (every-slice).

## Lessons-logged candidates anticipated
- **Convention candidate** — "the second harness's launch path mirrors the first's
  launch-spec-as-enforcement-surface (`ClaudeLaunchSpec`→`CodexLaunchSpec`): a PURE argv built so a
  bypass is impossible by construction (Codex: `--sandbox` always present, never `--yolo`/
  `--dangerously-bypass`; Claude: `--permission-mode default`), the fail-closed I/O separate. §15 #8 =
  resolve-and-PIN exactly one auth (refuse AMBIGUOUS, the `codex doctor` analog, fail-closed) recorded in
  the execution profile (keychain-ref, no-Debug-leak) + env-hygiene strips the non-chosen sources so the
  child can't account-hop. §15 #11 = pre-create the agent's state dirs 0700 + umask-0077 the child so the
  vendor's lax perms (0644/0755) harden — fail-closed (the §15 #11 invariant IS a function of that I/O,
  LESSON 30). The resume handle keys off the vendor's native resume id (the Codex rollout UUID); the
  experimental app-server path is HITL. Built mechanism-first with NO spawn — the binding condition (a
  reachable live agent lands WITH its interception, 3.3c; the 042→043/4.0b-1→4.0b-2 precedent)."
- **Architecture-doc note candidate** — §9.1/§15 AS-BUILT (the Codex launch/auth/profile/umask/resume
  mechanism LIVE; the spawn + the cat-1 `PreToolUse`→Gateway interception + the `--sandbox`
  defense-in-depth deferred to 3.3c; telemetry → 3.3d; live drive → HITL).
- **Future TODO** — the live umask-doesn't-chmod-back verify (Open-Q #9, HITL) · the app-server
  `thread/resume`/`thr_` resume path + the UUID↔`thr_` interconversion (Open-Q #3/#4, HITL) · the
  OSS-version auth/sandbox-flag fixture/grammar refresh (the desktop-build-ahead-of-OSS caveat).

## How to invoke
1. **Read this brief + the 0.3 research doc (`docs/planning/0.3-codex-schema-research.md` — §3.2 resume,
   §4.3 approval×sandbox, §5 auth/profile, §6 mirror table, §7 Open-Qs) end-to-end** + skim
   `daemon/src/harness/claude/mod.rs` (the `ClaudeLaunchSpec` precedent).
2. **Run `/tdd codex_launch_auth_profile_umask`**.
3. **Step 0 (Restate)** — confirm the NON-cat-1, NO-spawn, mechanism-only scope (launch spec + auth +
   profile + umask + resume-handle; spawn + interception + `--sandbox`-proof deferred to 3.3c).
4. **Step 2.5** — send the Asserts/coverage write-up + answers to the 5 design questions (Q1 the
   sandbox-default escalation check is the one to watch). Don't go GREEN until APPROVED.
5. **Step 8** — `security-reviewer` (§15 #8 auth/profile + §15 #11 umask — one pass, not cat-1) +
   `code-quality-reviewer`.
6. **Step 9 (summarize)** — surface flags + the §9.1/§15 AS-BUILT for orchestrator hot-routing.
