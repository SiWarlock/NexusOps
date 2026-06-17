# /tdd brief — codex_inv_sec_1_interception (🔴 CAT-1)

## Feature
The **CAT-1 Codex INV-SEC-1 interception**: a `nexusops-codex-gate` **`PreToolUse` hook adjudicator** that
routes every Codex tool-call through the **EXISTING 4.0b-2 Gateway adjudication loop** (reuse the daemon
`intercept` RPC + the `decision_sink` fail-closed wait — swap ONLY the I/O envelope) and **blocks** on the
daemon's verdict, fail-closed. **🔴 THE LOAD-BEARING CAT-1 PIN — DEFENSE-IN-DEPTH (genuinely-new vs
Claude):** Codex's own docs call `PreToolUse` *"a guardrail, not a complete enforcement boundary"* — so the
hook ALONE is **NOT** a sufficient INV-SEC-1 single-mutator guarantee. The hook is the **adjudication+audit**
channel; **`--sandbox workspace-write`** (the OS-enforcement boundary, scoped to {the approved worktree +
per-profile user-approved extra read/write paths}, network off by default) is the **containment** layer.
Both together = INV-SEC-1 for Codex. **Mechanism-built,
NO live agent** (the reachable live-codex spawn-site + the LIVE interception proof + the `--sandbox`
containment proof = the **0.1/0.3-HITL follow-on**, the 4.0b-2-smoke analog). **OWN security-reviewer EVERY
layer; the cat-1 design surfaces lead→user before Step-2.5 sign-off.**

## Use case + traceability
- **Task ID:** P3.3c (the 3.3 decomposition's CAT-1 slice — the Codex analog of 043 + 4.0b-2; see the §3.3
  decomposition in `IMPLEMENTATION_PLAN.md`).
- **Architecture sections it implements:** `ARCHITECTURE.md §15` (**INV-SEC-1 / the single audited mutator** —
  no FS/git/external/session mutation except via a typed, policy+approval-gated, audited Action), **§9.1**
  (the `HarnessAdapter::intercept_mutation` / `MutationIntercept`→Gateway routing), **§6** (intercept→Gateway
  intent), **§6.3** (the agent-mutation `ActionTypeCatalog` family — `agent.bash`/`agent.file_edit`/
  `agent.mcp_tool` + `ExecutorKind::Adjudication`), **§5.1** (Session).
  - **Widens phase scope because** §15 (the cat-1 INV-SEC-1 invariant) is the load-bearing cross-cutting
    safety anchor this slice implements per-harness — the exact 043/4.0b-2 precedent (both cited §15 for the
    Claude interception). The 3.3-phase Spec-anchors line stays §9.1/§5.1/§6/§7.2/§9/§0.1/§0.2/§18.
- **THE FOUNDATION (read it first):** `docs/planning/0.3-codex-schema-research.md` **§4** — **§4.1** (the
  Hooks `PreToolUse`: stdin `{turn_id,tool_name,tool_use_id,tool_input{command,…}}`; stdout
  `hookSpecificOutput.permissionDecision:"allow"|"deny"` — **identical to Claude's output** — or exit-2;
  the `[hooks.state].trusted_hash` tamper-check), **§4.2** (the app-server approval protocol — the alt
  transport, HITL), **§4.3** (the approval×sandbox matrix + **the 🔴 INV-SEC-1 nuance: the hook is a guardrail
  not a boundary → MUST layer `--sandbox`**).
- **Related context (the REUSE surface — swap the envelope, keep the loop):** `daemon/src/hook.rs` (the
  `nexusopsd hook PreToolUse` ingress — reads stdin → tags `NEXUSOPS_SESSION_ID` → the `intercept` RPC →
  translates the verdict, **FAIL-CLOSED on any error → DENY**; the `HOOK_READ_TIMEOUT=360s` > the daemon's
  5-min `APPROVAL_WAIT`); `daemon/src/harness/claude/intercept.rs` (the Claude `MutationIntercept`→Gateway
  routing — the daemon-side adjudication 3.3c reuses); `daemon/src/decisions.rs` (the per-session
  `decision_sink` — the verdict channel + the fail-closed wait, 4.0b-2/F2); `daemon/src/runtime/listener.rs`
  (the `intercept` handler + the `InterceptWaitClass` permit-class, LESSONS §34); the §6.3 agent-mutation
  catalog (`shared/src/catalog.rs` — `agent.bash`/`agent.file_edit`/`agent.mcp_tool`/`agent.todo_write` +
  `ExecutorKind::Adjudication`); `daemon/src/harness/codex/status.rs` (**`CodexToolKind{ShellExec,FilePatch,
  McpTool,Other}`** — the semantics classification 3.3c keys on, LESSONS §42); `daemon/src/harness/codex/
  launch.rs` (3.3b's `CodexLaunchSpec` — already carries `--sandbox` required-no-bypass + the
  `NEXUSOPS_SESSION_ID` env + the hook-config wiring point); LESSONS **§26** (the Claude INV-SEC-1
  interception — adjudication-only, audit-before-verdict, coverage-gap→deny; the pattern 3.3c mirrors), **LESSONS §30**
  (fail-closed: is the invariant a function of this I/O?), **LESSONS §34** (the intercept-wait permit-class), **LESSONS §42**
  (the Codex tool-kind classification).

## 🔴 The CAT-1 safety design (surfaces lead→user BEFORE Step-2.5 sign-off)
The genuinely-new-vs-Claude safety design, stated explicitly for the lead→user surface:
1. **DEFENSE-IN-DEPTH is mandatory (not the hook alone).** Codex's `PreToolUse` is a *guardrail*, so a tool
   path the hook doesn't match could still reach FS/shell. INV-SEC-1 for Codex = **(a)** the hook routes
   every *intended* mutation into the Gateway (adjudication + audit-before-verdict) **AND (b)** `--sandbox`
   (the OS-enforcement boundary) contains anything that slips the hook. Layer (b) is the new load-bearing
   pin Claude didn't need.
2. **The sandbox containment policy — ✅ USER-CONFIRMED 2026-06-15 (via the lead).** The launched Codex runs
   under **`--sandbox workspace-write`** scoped to **{the approved git worktree + the per-profile
   user-approved extra read/write paths}**, **`network_access=false` (network-off default)** — the agent can
   write WITHIN the Gateway-authorized boundary PLUS the explicit per-profile paths the user pre-approved
   (honoring the user's "need to read/write outside the workspace sometimes"), nothing outside that set, no
   network. The sandbox bounds writes to {worktree + approved paths}, **never arbitrary** — the hook+sandbox
   defense-in-depth holds. (read-only is maximally safe but breaks a coding agent that must edit;
   workspace-write-scoped-with-approved-extras is the production-correct containment.) **No longer a held
   decision — the sandbox half is DESIGN-COMPLETE.** **At authoring (after D5b): verify the exact Codex
   `writable_roots` / read-scope config grammar live** (the per-profile extra-path mechanism — part of the
   OSS-version flag-grammar refresh carry-forward; the desktop build may diverge).
3. **`codex_home` is daemon-resolved, not agent-controlled** (the 3.3b security NIT) — the hook-config dir +
   the sandbox scoping resolve from the daemon's `$HOME`/config, never an agent-supplied path.
4. **Fail-closed everywhere** (the 043 posture): no daemon / unreachable / parse-fail / hook-miss / wait
   timeout → **DENY** (block the tool). The daemon-side receiver deny is the PRIMARY control; the hook conduit
   defaults to deny; the sandbox is the backstop.
5. **NO live agent in this slice** — the mechanism is built + tested vs a FakeGateway; the LIVE interception
   proof (deny actually blocks the `exec_command`/`apply_patch`; the fail-closed wait vs the hook `timeout`)
   + the `--sandbox` containment proof = the 0.1/0.3-HITL follow-on. No reachable un-intercepted live codex
   ships (the binding condition — the live spawn-site lands WITH the HITL validation).

## Acceptance criteria (what "done" means)
- [ ] **The `nexusops-codex-gate` adjudicator** (a `nexusopsd hook --harness codex PreToolUse` variant, OR a
  sibling subcommand — Step-2.5 Q): reads Codex's `PreToolUse` stdin `{turn_id,tool_name,tool_use_id,
  tool_input}`, tags it with `NEXUSOPS_SESSION_ID` (the daemon correlation key, set by 3.3b's `CodexLaunchSpec`
  env), **normalizes** it to the SAME `intercept` RPC params the Claude hook sends (classify `tool_name` →
  `CodexToolKind` semantics → the §6.3 `agent.*` action type: ShellExec→`agent.bash` · FilePatch→
  `agent.file_edit` · McpTool→`agent.mcp_tool`), sends `intercept` over the UDS (**reuse the daemon loop
  UNCHANGED**), blocks on the verdict, and emits Codex's `PreToolUse` output (`hookSpecificOutput.
  permissionDecision:"allow"|"deny"` — same shape as Claude's `hook.rs::emit_allow/emit_deny`).
- [ ] **FAIL-CLOSED (the 043 posture, pinned):** any error — no daemon / unreachable socket / parse failure /
  unexpected frame / a non-`allow` verdict / a read past the timeout → **DENY** (exit/stdout block). The hook
  read-timeout > the daemon `APPROVAL_WAIT` so a legitimately-pending human approval isn't cut short.
- [ ] **The daemon adjudication is REUSED, not forked** — the `intercept` handler + the Gateway pipeline
  (adjudication-only `ActionRequest`, audit-BEFORE-verdict, `ExecutorKind::Adjudication` no-execute) +
  the `decision_sink` + the `InterceptWaitClass` permit-class are UNCHANGED (3.3c adds NO new daemon
  mutation/adjudication path — the Codex envelope normalizes INTO the existing one). _(If the handler needs a
  harness discriminator, that's a Step-2.5 flag — default is the hook normalizes, handler untouched.)_
- [ ] **The `--sandbox` defense-in-depth (the cat-1 pin):** assert 3.3b's `CodexLaunchSpec` carries
  `--sandbox workspace-write` scoped to {the approved worktree + the per-profile user-approved extra
  read/write paths} + `network_access=false` (network-off default — the USER-CONFIRMED 2026-06-15 containment
  boundary) — and that there is NO path to `--dangerously-bypass-approvals-and-sandbox`/`--yolo` (already
  pinned in 3.3b; re-assert here as the INV-SEC-1 enforcement layer). The exact Codex `writable_roots`/
  read-scope config is verified live at authoring. The LIVE containment proof = HITL.
- [ ] **The hook-config + trust-hash discipline:** the generated Codex `hooks.json`/`[hooks]` config wires
  `PreToolUse` (matcher on the mutating tool set — `^(Bash|apply_patch|shell|local_shell|…)$` or `*`,
  Step-2.5 Q) to the `nexusops-codex-gate` command, under a `codex_home` the daemon resolves; the
  `[hooks.state].trusted_hash` is registered once + kept stable (a changed hash → Codex re-prompts; the
  adapter must satisfy the tamper-check). **The coverage-gap compensation:** any un-hooked tool path is an
  INV-SEC-1 bypass → the `--sandbox` containment is the backstop (the hook-miss can't escape the sandbox).
- [ ] **Test-first vs `FakeGateway`** (the 043/4.0b-2 pattern): deny-blocks (a Deny verdict → the hook emits
  deny) · allow-passes (an Allow → emit allow) · fail-closed-timeout (a verdict that never arrives within the
  read-timeout → deny) · fail-closed-error (no daemon/parse-fail → deny) · the CodexToolKind→`agent.*`
  classification (ShellExec/FilePatch/McpTool map correctly; an un-classified tool → the conservative
  `CoverageGap`→deny). All unit-level (NO live agent).
- [ ] All tests in `daemon/tests/codex_intercept.rs` pass; `/preflight` clean. **CONTRACT:** likely **NO
  bump** (the Codex hook + the normalization are daemon-internal; the §6.3 `agent.*` catalog + `intercept`
  RPC are already frozen) — confirm at Step-2.5 (a `harness_session_map` freeze only if the session-id↔
  thread-id mapping needs a shared shape; likely daemon-internal).

## Wiring / entry point (Step 7.5)
**The mechanism is reachable from the `nexusopsd hook` CLI entry** (`main.rs` dispatches the `hook`
subcommand) — the `nexusops-codex-gate` adjudicator is invoked by Codex per tool-call (when a live codex is
launched). **NO production live-codex spawn-site in this slice** (the binding condition — the reachable
`CodexLauncher` `SessionLauncher` impl + the LIVE drive land WITH the HITL validation; the 042→043/4.0b-1→
4.0b-2 precedent). Confirm: the adjudicator is reachable from `main.rs` + routes to the live `intercept`
handler (`/wired` the hook→intercept→Gateway path); the FakeGateway tests exercise the full
normalize→adjudicate→verdict→emit path; NO `Command::new("codex")`/spawn exists (the no-live-agent pin).

## Files expected to touch
**New:**
- `daemon/src/harness/codex/intercept.rs` — the Codex `PreToolUse` envelope parse + normalize (CodexToolKind
  → `agent.*`) + the verdict→Codex-output translation. (OR extend `daemon/src/hook.rs` with a `--harness`
  branch — Step-2.5 Q.)
- `daemon/tests/codex_intercept.rs` — the FakeGateway tests (deny/allow/fail-closed/classification).

**Modified:**
- `daemon/src/hook.rs` — the `--harness codex` dispatch (or the shared normalize seam) — the Codex envelope
  variant; the Claude path UNCHANGED.
- `daemon/src/harness/codex/launch.rs` — wire the generated `hooks.json`/`[hooks]` config (the
  `nexusops-codex-gate` command + the trust-hash) into the `CodexLaunchSpec` (the 042 `ClaudeSettings`
  precedent); assert the `--sandbox` containment is the INV-SEC-1 layer.
- `daemon/src/main.rs` — the hook subcommand dispatch (if a `--harness` flag is added).
- `daemon/src/harness/codex/mod.rs` — `intercept_mutation()` returns the real `MutationIntercept` (was the
  3.3a `None` stub) IF the adapter surfaces it (Step-2.5 — likely the hook is the ingress, the adapter stub
  stays; confirm).

If implementation needs files beyond this list, **flag at Step 2.5** before GREEN. **Do NOT add a live-codex
spawn-site** (`CodexLauncher`) — that's the HITL follow-on; flag if you think otherwise.

## RED test outline (Step 2) — `daemon/tests/codex_intercept.rs`, vs a FakeGateway
1. **`test_codex_hook_deny_blocks`** — a Deny verdict from the (fake) daemon → the hook emits Codex
   `permissionDecision:"deny"`. Why: §15 INV-SEC-1 (deny blocks the tool).
2. **`test_codex_hook_allow_passes`** — an Allow verdict → emits `"allow"`. Why: the adjudication-allow path.
3. **`test_codex_hook_fail_closed_no_daemon`** — no daemon/unreachable/parse-fail → DENY. Why: §15/LESSONS §26
   fail-closed (the conduit defaults to deny).
4. **`test_codex_hook_fail_closed_timeout`** — a verdict that never arrives within the read-timeout → DENY.
   Why: LESSONS §30/§26 (a stalled daemon can't open the tool; the wait never silently allows).
5. **`test_codex_envelope_normalizes_to_intercept`** — Codex stdin `{turn_id,tool_name,tool_use_id,
   tool_input}` → the SAME `intercept` params shape the Claude hook sends (+ `NEXUSOPS_SESSION_ID` tag). Why:
   reuse-not-fork (the daemon loop is harness-agnostic).
6. **`test_codex_tool_classification`** — `tool_name`→`CodexToolKind`→`agent.*`: shell/local_shell/
   exec_command→`agent.bash` · apply_patch→`agent.file_edit` · mcp→`agent.mcp_tool`; an un-classified tool →
   `CoverageGap`→deny (conservative). Why: §6.3 / LESSONS §42 (classify by semantics; deny-unknown).
7. **`test_codex_no_live_spawn`** — structural grep over `harness/codex/`: no `Command::new("codex")`/
   `.spawn(`/`SessionLauncher` impl (the binding condition). Why: no reachable un-intercepted live codex.
8. **`test_sandbox_is_inv_sec_layer`** — the `CodexLaunchSpec` carries `--sandbox workspace-write` scoped to
   {worktree + per-profile approved extra paths}, `network_access=false` (never a bypass flag); the
   hook-config wires `PreToolUse`→`nexusops-codex-gate` under a daemon-resolved `codex_home`. Why: the
   defense-in-depth cat-1 pin (the spec + the hook config are the two layers; the USER-CONFIRMED policy).

## Cross-doc invariant impact (implementer flags at Step 9; orchestrator writes the docs)
- **Model field changes:** likely **NONE** in `shared/` — the Codex hook + normalization are daemon-internal;
  the §6.3 `agent.*` catalog + the `intercept` RPC are frozen. **Confirm NO CONTRACT bump** (a
  `harness_session_map` shared shape only if needed — Step-2.5). §2.5-seam: only if a shared shape is added.
- **Orchestrator doc rows to write hot (Step 9):** the §9.1/§15 AS-BUILT note (the Codex INV-SEC-1
  interception LIVE — mechanism-built, defense-in-depth [hook + sandbox]; the LIVE drive = HITL) + the
  `daemon/CLAUDE.md` §9.1/§6.3 row (the Codex hook reuses the 4.0b-2 loop) + the LESSON candidate. **Safety:**
  this is the cat-1 INV-SEC-1 slice → **security-reviewer EVERY layer**; the cat-1 design escalated to the
  human BEFORE Step-2.5 sign-off.

## Things to flag at Step 2.5 (the cat-1 design — surfaces lead→user)
1. **The `--sandbox` containment policy — ✅ USER-CONFIRMED 2026-06-15 (no longer open).** The policy is
   **`workspace-write` scoped to {the approved worktree + per-profile user-approved extra read/write paths} +
   `network_access=false` (network-off default)** — production-correct (read-only breaks a coding agent;
   workspace-write-scoped-with-approved-extras contains it to {Gateway-approved boundary + the explicit
   user-approved paths}, honoring the out-of-workspace need, never arbitrary). Implement the confirmed policy;
   **verify the exact Codex `writable_roots`/read-scope config grammar live at authoring** (don't re-ask the
   user — the decision is made; the only open item is the live config-grammar check).
2. **The PreToolUse matcher coverage.** `*` (every tool) vs `^(Bash|apply_patch|shell|local_shell|exec_command|
   mcp__*)$` (the mutating set). Default vote: **match `*`** (intercept every tool — the conservative
   coverage; the daemon classifies + auto-allows the benign read-only set per the §6.3 catalog, like Claude's
   `agent.todo_write`), so no mutating tool slips the matcher. The sandbox backs up any miss. Confirm.
3. **The hook variant shape — `nexusopsd hook --harness codex` vs a sibling `nexusops-codex-gate`.** Default
   vote: **a `--harness codex` branch in `hook.rs`** (one binary, the envelope swap behind a flag — minimal
   surface; the Claude path UNCHANGED) vs a separate subcommand. Confirm.
4. **The CodexToolKind→`agent.*` mapping + the un-classified→deny.** Default vote: ShellExec→`agent.bash` ·
   FilePatch→`agent.file_edit` · McpTool→`agent.mcp_tool` · Other/un-classified → `CoverageGap`→**deny**
   (conservative, the 043 deny-unknown). Confirm (esp. that an un-hooked/unknown tool is a deny, backed by
   the sandbox).
5. **CONTRACT — NO bump expected.** The hook + normalize are daemon-internal; the catalog/`intercept` are
   frozen. Confirm no `harness_session_map`/shared shape is needed (the session-id↔thread-id is daemon-internal).

## Dependencies + sequencing
- **Depends on:** 3.3a (✅ `CodexToolKind` + the observe core), 3.3b (✅ `CodexLaunchSpec` + `--sandbox` +
  `NEXUSOPS_SESSION_ID` + the hook-config wiring point), 4.0b-2/043 (✅ the daemon `intercept`→Gateway loop +
  `decision_sink` + the permit-class — the REUSE surface). NOT a live Codex (mechanism-only; the live drive
  = HITL).
- **Blocks:** 3.3d (telemetry — independent) · the LIVE Codex drive loop (the HITL follow-on — the live
  interception proof + the sandbox containment proof + the app-server `requestApproval` handshake).

## Estimated commit count
**1–2 (CAT-1 → the safety-critical pin gets its OWN commit).** Likely: (1) the Codex hook adjudicator +
normalize + the FakeGateway tests (the interception mechanism — the cat-1 core, its own commit + own security
pass); optionally (2) the hook-config wiring into `CodexLaunchSpec` (the `hooks.json` + trust-hash) if it
grows. The `--sandbox` assertion rides the hook commit (it's the same INV-SEC-1 surface). **Never bundle the
cat-1 interception with anything non-safety.** Confirm the split at Step-2.5.

## Reviewer subagents (Step 8 policy)
- **`security-reviewer`: YES — EVERY layer (cat-1).** This is THE Codex INV-SEC-1 slice. Review surface: the
  fail-closed conduit (every error→deny), the audit-before-verdict reuse, the coverage-gap→deny, the
  CodexToolKind→`agent.*` classification (no mutating tool mis-classified as benign), the `--sandbox`
  defense-in-depth (the hook is NOT the sole boundary), the no-live-spawn binding, the hook-trust-hash, the
  `codex_home` daemon-resolved. Critical findings → Step-9 `Finding` (→ human via lead).
- **`code-quality-reviewer`: YES** (every-slice).

## Lessons-logged candidates anticipated
- **Convention candidate** — the second harness's INV-SEC-1 interception REUSES the first's Gateway loop by
  swapping ONLY the I/O envelope (Codex `PreToolUse` stdin/stdout ↔ the daemon `intercept` RPC; normalize the
  vendor tool-name → `CodexToolKind` → the frozen §6.3 `agent.*` family) — the daemon adjudication stays
  harness-agnostic (LESSONS §26 generalized). The genuinely-new layer for a vendor whose hook is "a guardrail
  not a boundary" is **`--sandbox` OS-containment as a mandatory defense-in-depth second layer** (hook =
  adjudication+audit; sandbox = containment) — built mechanism-first, NO live agent (the live proof = HITL).
- **Architecture-doc note candidate** — §9.1/§15 AS-BUILT (the Codex INV-SEC-1 interception; defense-in-depth;
  the LIVE drive = HITL).
- **Future TODO** — the LIVE Codex drive loop (the HITL interception + sandbox-containment proof) · the
  app-server `requestApproval` in-band approval transport (the alt to the hook, §4.2) · the OSS-version
  tool-name/hook-coverage validation (#2/#5).

## How to invoke
1. Read this brief + the 0.3 research **§4** (§4.1 hooks / §4.3 the sandbox matrix + the INV-SEC-1 nuance) +
   `daemon/src/hook.rs` (the reuse surface) end-to-end.
2. **Run `/tdd codex_inv_sec_1_interception`**.
3. **Step 0 (Restate)** — confirm the CAT-1, defense-in-depth (hook + sandbox), NO-live-agent, reuse-the-loop
   scope.
4. **Step 2.5** — send the Asserts/coverage write-up + the 5 design-Q answers. **The cat-1 design (Q1 the
   sandbox containment policy especially) surfaces lead→user — do NOT go GREEN until the human signs off via
   the lead** (the 4.0b-2 discipline; I route it).
5. **Step 8** — `security-reviewer` EVERY layer (cat-1) + `code-quality-reviewer`.
6. **Step 9** — categorized flags + the §9.1/§15 AS-BUILT; critical findings escalate as a `Finding`.
