# edges P5 / P7.1 — PHASE-EXIT READINESS ASSESSMENT (DRAFT)

> **Status:** DRAFT (R4, 2026-06-12), lead-queued after edges-017 (`open_diff`) landed. **Not a seal/cycle.**
> A readiness snapshot + a pre-staged phase-exit/merge checklist for when the gate is actually run.
> **The verdict gates on the daemon track delivering R1 — see Verdict.**

---

## VERDICT — phase closure BLOCKED on R1; in-lane work COMPLETE

edges' P5 + P7.1 **in-lane** surfaces (all read/derivation/parse logic) are **COMPLETE** (387/0, CONTRACT
held 0.20.0, `shared/` untouched). But **`/phase-exit 5` and `/phase-exit 7` cannot tick the phases yet** —
the load-bearing **wiring** tasks (real executors + the new event types + their projectors) are **gated on
the daemon R1 deliverable** (the executor-registration seam + the Phase-5/7 `EventTypeRegistry` types),
which is **not landed/merged**. A `/phase-exit` run today would CONFIRM "phase incomplete — blocked on R1 +
deferred migrations," not close it.

**The phase-exit is also a MERGE event.** Per the tracker's multi-track integration order (`IMPLEMENTATION_PLAN.md:165`):
*"edges → main at each P5/P7.1 phase-exit (daemon-area code, same crate — rebase-merge, run `/phase-exit`)."*
So the gate = rebase `track/edges` onto `main` + land the (R1-unblocked) wiring + run `/phase-exit`. It needs
the **daemon track's participation** (it owns `main`, the seam, the event types, `CONTRACT_VERSION`, and the
eventstore migration numbers).

**Bottom line:** edges has **exhausted clean in-lane runway** (R4 = edges-016 + edges-017; richer-fields +
test-support held for the user; copy-detection deferred as a finding-doc). The real unblock is **routing R1
to the daemon track**. Until then, edges parks.

---

## What's IN-LANE COMPLETE (R1–R4)

| Vertical / work | Status | Anchors |
|---|---|---|
| Project detection engine (P5.1) | ✅ in-lane (executor gated) | §9 |
| Worktree-status reads + precedence (P5.2) | ✅ in-lane | §7.2, §5.1 |
| git diff/log backend — file + per-hunk + rename (P5.2) | ✅ COMPLETE in-lane | §9, §7.2 |
| §17 integration-failure classifier (P7.1) | ✅ COMPLETE in-lane (+ R4 NotFound/epoch-ms) | §17 |
| PR status-derivation + GitHub-PR read vertical (P7.1) | ✅ COMPLETE in-lane | §9, §7.2, §5.1 |
| Linear read vertical — derivation + client + GraphQL adapter (P7.1) | ✅ COMPLETE in-lane | §9, §17, §5.1 |
| **R4: §17 error-taxonomy refinements** (edges-016 `70a7196`) | ✅ | §17 |
| **R4: `open_diff` DRY refactor** (edges-017 `3b6c20b`) | ✅ | §9, §7.2 |

Suite **387/0**; every slice TDD-clean + Step-2.5-reviewed; security-reviewer CLEAN/SKIP per policy each layer.

---

## Per-anchor readiness (what `/phase-exit` would find)

**P5 anchors** (`§9 §7.2 §5.1 §6.3 §15 §18`):
- ✅ **§9 / §7.2 / §5.1** — in-lane read/derivation surfaces present + tested → arch-drift + spec-coverage PASS.
- ⏸️ **§6.3 (git.* ACTIONS)** — the executor wiring; **GATED on R1**. Known-deferred-not-drift (like Phase-2's gated rows).
- ⏸️ **§15 (profile binding / keychain)** — gated (H1 ExecutionProfile + the `IntegrationConnectionRegistered` wiring; keychain_ref-pointer-only spec is authored, population gated).
- ⏸️ **§18 (P5.4 project.rescan bench < 3 s)** — deferred to phase-exit cadence; **baseline already measured: median 1.029 ms ≪ 3 s** (re-author trivially with the known number).

**P7.1 anchors** (`§9 §11.2 §7.2 §6.3 §17 §8`):
- ✅ **§9 / §7.2 / §17** — read clients + the §17 classifier (incl. R4 NotFound/epoch-ms) present + tested.
- ⏸️ **§11.2 (PR Review Workspace)** — the per-hunk read backend is in-lane (edges-012); the UI surface is ui-track/gated.
- ⏸️ **§6.3 (github/linear ACTIONS)** — executor wiring; **GATED on R1**.
- ⏸️ **§8 (intake/PR flows)** — partially gated (the `*SyncFailed` events + flows need the wiring).

---

## Per-checklist-row readiness (the `/phase-exit` template, lines 92–106)

| Row | Readiness | Note |
|---|---|---|
| **Reachability audit** (`reachability-auditor`) | ⏸️ PARTIAL | Read clients reachable-to-the-trait + fake-covered; the gated executor/projector consumers are **intentionally-gated** (Phase-2 `fault.rs` precedent). Full reachability needs the R1 wiring. |
| **Arch-drift audit** over the anchors | ✅ (in-lane) | The in-lane §9/§7.2/§5.1/§17 surfaces match the spec; the gated §6.3/§15/§8 rows are known-deferred-not-drift (declare the list). |
| **Spec coverage** (`spec-lint tests 5` / `tests 7`) | ⚠️ needs gated-waivers | In-lane anchors are tagged/covered; the **gated wiring anchors (§6.3, parts of §15/§8) need explicit gated-waivers** on the phase Spec-anchor line, or they false-FAIL. The §18 bench already carries its waiver class. |
| **Dependency audit** (`cargo audit`) | ⬜ run-at-merge | **New deps this track: reqwest 0.12, octocrab 0.53.1, async-trait 0.1.89** → run `cargo audit` vs the Phase-2 baseline (`docs/audits/P2-cargo-audit.txt`); record new-vs-baseline. (§D carry.) |
| **Cross-doc invariants** | ✅ / reconcile-at-merge | edges added **NO `shared/` contract** (CONTRACT held 0.20.0). The merge reconciles edges' 0.20.0 against **main's 0.23.0** (regen, not an edges bump). |
| **Commits pushed** | ⏸️ verify-only, user-gated | Consistent with the project posture (Phase-2 row 12). `track/edges` is **pushed to origin** (backup, R4) but **unmerged to main**. |

---

## MERGE-RECONCILIATION CHECKLIST (the real phase-exit work, once R1 lands)

1. **Daemon track delivers R1** — the `ActionExecutor` registration seam (`gateway/`) + the Phase-5/7 `EventTypeRegistry` types + `CONTRACT_VERSION` bump (specs: `docs/planning/edges-R1-routing-packet.md`). **THE unblock.**
2. **Rebase** `track/edges` (`a40ac00`, CONTRACT 0.20.0) onto **`main` (`018479d`, CONTRACT 0.23.0)** — absorb the 0.20→0.21→0.22→0.23 daemon bumps (Phase 3.1/3.2/3.4/3.5 — Terminal/Claude-adapter/telemetry; **disjoint from edges' `git/`+`integrations/`** → low conflict; regen the consumed schema).
3. **Land the gated wiring** (5.1/5.2/7.1 executors + projectors + the new events) against the delivered R1 seam — each its own TDD slice + the security-load-bearing carries (§17 `AuthFailed`-branch, `*SyncFailed` redaction-before-sink).
4. **Eventstore migrations** (D5) — the `integration_connections` + registry tables; **daemon (schema owner) assigns coordinated `user_version` numbers** at the merge (NO edges migration before this — D5 Option A).
5. **H1 ExecutionProfile (5.3)** — expected resolved by the daemon's Phase-3.2 enum freeze (per cross-track outlook); regen the frozen enum, then 5.3 + the `auth_expired` sync variant unblock.
6. **Apply the PLAN-DELTA** (held in `docs/sessions/edges-006-…` §A–§E + this round's additions): task-ticks (5.2/7.1 partial→complete as wiring lands) · arch-notes (§9 read-client boundaries + R4 NotFound/epoch-ms) · **lessons → `daemon/LESSONS.md` renumbered to the next free daemon slot (daemon took §26/§27 in its Phase 3 — propose §28+; NEVER reuse a daemon slot)** · the R4 §B/§C accumulations (NotFound/epoch-ms arch-note + the epoch-ms-reset lesson).
7. **P5.4 bench** — run `project.rescan` < 3 s (baseline 1.029 ms PASS); re-author the harness with the known number.
8. **`cargo audit`** — reqwest/octocrab/async-trait vs the Phase-2 baseline; record/escalate any new finding.
9. **Held R4 carries** — richer `LinearIssue` fields (YAGNI-deferred) + `test-support` cargo-feature (cross-track Finding — gate all 3 fakes incl. daemon's `FakeHarness` once) — per the user's pending call.
10. **Run `/phase-exit 5` + `/phase-exit 7`** row-by-row; tick the phases only on CLEAR (push row verify-only/user-gated).

---

## RECOMMENDATION
edges is **in-lane COMPLETE / phase-closure BLOCKED on R1**. The single highest-leverage action is **routing
the R1 packet to the daemon track** (it serves the daemon's own Phase-3 `session.*` arms too). Until R1 lands +
merges, edges has no clean in-lane path to phase closure — **park** (or run only a lead/user-unblocked held
carry). This assessment pre-stages the gate so the eventual coordinated phase-exit/merge is fast.
