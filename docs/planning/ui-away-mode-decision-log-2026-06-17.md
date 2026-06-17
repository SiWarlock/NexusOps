# UI Track — Away-Mode Decision Log (2026-06-17)

Lead (`ui-team-lead`, team `nexusops-ui`) operating under **delegated user authority** while the
user is away.

**User directive (2026-06-17):** *"Stepping away. The lead is responsible for any surfaced items
needing my call. Prefer the architecturally-correct approach for a production-grade application.
Log all decisions while away. Defer any HITL steps until I'm back to keep the build going."*

---

## Standing posture (applies to every decision below)

- **DEFAULT = architecturally-correct, production-grade > expedient.** Heavy ≠ correct — *correct*
  is the bar (not gold-plating, not MVP shortcut).
- The lead now **ADJUDICATES escalation category #4** (load-bearing Option A/B/C calls) itself —
  picking the architecturally-correct option + logging it here — instead of routing to the user.
  The user reviews all entries on return.
- **Cross-track coordination** (migration numbers, additive integration edits, D6/D7/D8 routing) —
  DECIDED by the lead + logged (per the established away-mode bar).
- **DEFERRED, not decided:** category #1 (safety / INV-SEC design Q), any category #2 Finding with a
  safety dimension, and any genuine **HITL** step (something only the human can physically do or has
  explicitly reserved). These are logged + parked; the build continues on the next in-lane work,
  **never blocked**.

## Deferred-until-return (HITL / user-gated — parked; build continues around them)

- **D-1 — PR-review MUTATIONS go-live** (Merge / Approve-PR / per-hunk accept-reject). A cat-1 arc
  needing user sign-off like the L2 go-live. Phase-7-UI is built **READ-ONLY** (mutations render
  DISABLED), so this is already deferred *by design* — no block.
- **D-2 — "Always allow" / `policy_grant` standing-grant.** Its own cat-1; stays DISABLED.
- **D-3 — ui → main boundary merge.** USER-GATED on "main idle"; deferred. The in-lane arcs (ui-063,
  Phase-7 read-only shell) need no merge. `track/ui` pushes at round close-outs are fine; **never
  push main.**
- **D-4 — Daemon asks D6 / D7 / D8** (PR diff-stats producer-capture · `get_pr_diff` RPC ·
  recovery-status signal). Need the daemon track (not currently up) + are partly HITL. The UI builds
  **placeholders** for all three; routing to the daemon team is deferred until the user coordinates.
  **D7 TIGHTENED (2026-06-17, per ui-064 verify-before-build):** narrower than first recorded — the
  reviews-list is **built & live now** (daemon already serves the Review projection); ONLY the PR
  *code-diff* panel needs `get_pr_diff(repo_id, pr_number, file?) → DiffResult`. So D7 = "PR code-diff
  RPC" specifically, not the whole Review tab.
- **D-6 — ui-064 manual VISUAL gate (HITL).** The new PR Review Workspace panel can't be
  headless-rendered (production `UdsGatewayPort` mount, no Mock-injection — LESSON §22/§23). Deferred
  to the user's return. **Comparison spec (run on return):** new PR Review Workspace panel vs the
  prototype `kit-views2.jsx` DiffReview PR-detail → route **Code/Diff Review → "Pull requests" tab →
  click a PR card** → verify: header / mergeability / checks badges · reviews-list verdict badges ·
  the honest D6 (diff-stats) + D7 (code-diff) "unavailable" placeholders · all mutation + Brain
  controls render DISABLED. ui-064 is committed + 392/392 green; this is a verification sign-off, not
  a build blocker.
- **D-5 — Brain controls** (Ask Brain / Ask why). Deferred sibling product; render DISABLED.
- **D-7 — `/preflight` prettier-honesty fix (PERMISSION-BLOCKED → HITL).** The ui-mode `/preflight`
  Step 3 runs `pnpm prettier --check .`, but prettier isn't a ui dependency → a vacuous no-op masked
  as "format ✓" (the real gates oxlint/tsc/tests run honestly). Fix = drop the vacuous step (ui is
  prettier-free by design per `ui/CLAUDE.md`) OR add prettier+config. **BLOCKED:** the fix edits
  `.claude/commands/preflight.md`, which the auto-mode classifier blocks as **agent
  self-modification** — needs the *literal user's* authorization (a teammate's or the lead's
  delegated authority does NOT clear it; forcing it = permission-laundering). The orch verified the
  issue + drafted the fix, but edit AND revert were denied; `preflight.md` is restored to its
  committed original (clean tree). **On return:** the user applies the fix directly, or adds a
  settings permission rule to let an agent do it. Low-impact.

## Decisions log (append-only — newest last)

- **2026-06-17 ~05:46 — Away-mode authority accepted; posture above set.** Team `nexusops-ui`
  resuming from handoff ui-003: ui-063 whole-cockpit-live → Phase-7-UI L2 read-only PR Workspace
  shell. Cursor `179dfeb`, CONTRACT 0.38, clean, 376/376 green. Orchestrator + implementer spawned;
  implementer read-back verified (`/session-start`, registry written). Awaiting orchestrator
  read-back to authorize the slice sequence.
- **2026-06-17 — Both teammates verified (Step-4 complete); slice queue AUTHORIZED.** Orchestrator
  read-back clean (`/orchestrate-start`, registry `4de3e271…`, cursor matches handoff). DECISION:
  authorized the resume queue **without holding for an explicit user-go** (user is away +
  pre-authorized "keep the build going"); **endorsed the orch's sequencing** — ui-063 first (lower-risk
  mechanical), then the Phase-7 read-only PR Workspace shell. Relayed the away-mode posture to the orch.
- **2026-06-17 — Standing ruling on the ui-063 verify-before-build gate.** ENDORSED the orch's gate:
  before writing nudge-listeners, confirm the merged daemon actually emits `row:None` nudges for
  ProjectActivity / PullRequest / UsageLedger (D4 was to add the other-4; pre-merge only Session +
  ApprovalQueue were verified-emitting). RULING: if any of the 3 is **not** emitted, that projection
  **drops to a daemon-ask** (D-series, USER-routed — I log it) and ui-063 ships the rest. Rationale:
  don't build listeners that never fire (architecturally-correct; matches the verify-before-build
  discipline that's paid off all track).
- **2026-06-17 ~07:15 — Both authorized arcs LANDED clean (track/ui, not pushed).** ui-063
  whole-cockpit-live `7dd11fa` (ProjectActivity/PullRequest/UsageLedger refetch-on-nudge; AuditTrail
  excluded-by-design = refetch-storm) · ui-064 read-only PR Review Workspace shell `723f90e` (L1
  reviews vertical) + `a28cb06` (L2 assembly); **392/392 green, tsc/oxlint/preflight clean, every
  slice security-clean / read-only.** `PrWorkspace` takes no gateway prop → no mutation/`get_diff`
  reach by construction. Orch hot-routing (ui/CLAUDE.md consumer row · §29 Review-live · LESSON §30)
  staged for the round commit.
- **2026-06-17 — RULING: ui-064 visual gate DEFERRED to return (HITL).** See D-6 above for the
  comparison spec. Not a blocker; the build continues.
- **2026-06-17 — RULING: CONTINUE building, do NOT close out at queue-empty.** Queue-empty is a
  routine boundary; close-out fires only on user-on-demand or an auto-cycle trigger. APPROVED the
  in-lane `gen-contracts.mjs` oneOf-const generator extension (retire the
  ResumeMode/RecoveryState/MetricQuality drift-pinned provisional shadows) + the `/preflight`
  prettier-honesty pickup. **GUARDRAIL given to the orch:** must stay CONTRACT-neutral (never bump
  CONTRACT_VERSION — daemon-only) + keep the §5.0 drift gate GREEN; if the generated oneOf-const
  output is NOT equivalent to the retired shadows (consumers need real reconciliation), STOP and
  surface as a Finding. Rationale: architecturally-correct, daemon-independent, removes a real drift
  hazard.
- **2026-06-17 ~07:17 — ui-065 dispatched (task #3, brief `@a8f280a9`); guardrail CLEARED.** The
  gen-contracts oneOf-const cleanup's verify-before-build confirmed all 3 enum value-sets
  (ResumeMode / RecoveryState / MetricQuality) are IDENTICAL schema-vs-shadow → a clean representation
  swap, NOT the equivalence-Finding I guarded for. CONTRACT-neutral (no version bump). Orch added task
  6.9 to IMPLEMENTATION_PLAN.md (worktree copy, per the established ui-track reconcile-at-merge
  pattern) + ticked the now-complete live-delta-spread checkbox (Session / the-3 / Review all live;
  AuditTrail excluded-by-design). Impl on RED.
- **2026-06-17 — `/preflight` prettier-honesty fix DEFERRED (permission-blocked HITL).** See D-7. The
  lead did NOT apply it on the user's behalf: command-config self-modification requires the literal
  user, and the away-mode delegation doesn't extend to clearing system permission gates. Parked for
  return.
- **2026-06-17 ~07:42 — ui-065 LANDED `c97d652` (task #3 done); 389/389 green, CONTRACT 0.38.0 held,
  guardrails verified clean.** Three substantive arcs now landed this run (ui-063 / ui-064 / ui-065).
- **2026-06-17 ~07:45 — Queue-exhausted checkpoint #2. RULING: SEAL + PUSH the 3-arc round, then KEEP
  BUILDING (6.6 benchmark).** Ran `/context-check nexusops-ui` (canonical): **impl 56% / orch 53% /
  lead 17% — all OK.** This REFUTES the orch's cycle-timing rationale for closing out (plenty of
  runway) → **no teammate cycle.** Decision: (a) direct a round close-out NOW (impl /session-end →
  orch /orchestrate-end: commit staged hot-routing + reconcile tracker + **push track/ui**) to bank
  the coherent, complete 3-arc feature round as a safe, reviewable, pushed checkpoint for the away
  user — teammates PERSIST (no respawn); (b) then continue per "keep the build going" with the **6.6
  graph-render benchmark** (§18 <500ms budget — legit production-grade, runnable now), then 6.7
  diff-open if clean; (c) tidy: confirm-then-DELETE the obsolete DiffReview bare-# chip carry-forward
  if ui-064 superseded the baseline PRsTab. Rationale: honors BOTH safety (pushed checkpoint while
  the user is away) AND momentum (build continues into 6.6); architecturally clean (feature round
  sealed; benchmarks are a separate own-cadence round). NEVER push main; ui→main merge stays deferred
  (D-3).
