# P5 arch-drift audit — `/phase-exit 5` (edges track) — 2026-06-15

**Auditor:** arch-drift-auditor · **Branch:** `track/edges` · **Verdict: CLEAR**

Scope note: over-approximated track-diff audit (acceptable for a track's phase-exit). 6 anchors audited — **0 DRIFT / 2 STALE-DOC / 0 ambiguous**.

## Anchor results (all CONFIRMED — no drift)
- **§9 (git/integrations):** git2 reads / git-CLI mutations (forbidden #6 honored); `git.create_worktree`/`create_branch` exact CLI argv; `reject_dash_operands` arg-injection guard; `side_effect_applied=true` for git mutations. CONFIRMED.
- **§7.2 (SoT + re-read):** live git2 re-read before mutate (`compute_worktree_cache`); `git_checked_at` staleness; git-watcher wired (`main.rs:313`); git-axis = live-read cache (NOT event-sourced), distinct from the event watermark. CONFIRMED.
- **§5.1 (Worktree two-axis + ExecutionProfile):** `DerivedWorktreeStatus` covers both axes; precedence rank order exact; wire = frozen snake_case. ExecutionProfile (5.3) = known-deferred (daemon-side/H1-gated). CONFIRMED.
- **§6.3 (catalog):** `project.rescan` risk-0 / `git.*` risk-2; fail-closed lookup; INV-SEC-1 executor-only-via-Gateway. CONFIRMED.
- **§15 (keychain/redaction):** `remote_url` userinfo strip-at-source (`strip_userinfo`, last-`@`-in-authority algorithm; scp-ssh left intact; 6 pinning tests); redactor backstop; `proj_repository` passes the already-stripped committed value through. CONFIRMED.
- **§18 (bench):** `project.rescan` SLO 3s; guard median <50ms; `[[bench]] harness=false`; measures `detect_git`+`detect_workflow`. CONFIRMED.

## Known-deferred (NOT drift)
MVP projects/repositories = event-fed projections (`proj_project`/`proj_repository`) NOT §2.8 durable-registry rows (lead-ruled at R1b, code acknowledges it explicitly) · 5.3 ExecutionProfile (daemon-side/H1) · `proj_project_activity`+graph folds of ProjectRescanned · the IPC read RPC for proj_project · the §15 §7.2-redacted-path over-redaction carry (code-documented, MVP-accept).

## Stale-doc notes (code correct; doc lags → merge-ledger)
1. **§6.3 / Appendix A catalog count** "~21" → as-built **28** (additive per-phase: session.kill/profile_change · git.stage/unstage/discard_hunk · integration.connect). Update at the merge.
2. **§18 bench median** session/brief "0.44 ms" vs bench code "~0.45 ms" — trivial rounding.

**VERDICT: CLEAR** (0 drift; the landed edges P5 surfaces match the spec; deferrals are known-deferred-not-drift).
