---
description: Implementer-only close-out — TDD audit, cross-doc audit, wiring + Step-9 verification, session doc, /preflight.
allowed-tools: Read, Edit, Write, Bash
argument-hint: ""
---

> **Role guard — IMPLEMENTER sessions only.** If you are the **orchestrator**, stop: run `/orchestrate-end`, not this. `/session-end` writes the implementer's session doc; the orchestrator's round close-out is `/orchestrate-end`.

The user has indicated the implementer session is wrapping. Capture state.

`/session-end` is **technical close-out only** — TDD audit, cross-doc invariant audit, Step-9 routing verification, session doc creation, `/preflight`. **Do NOT touch `MVP_TASKS.md`** — the orchestrator owns that file via `/orchestrate-end`. The handoff: this command produces the session doc + recap; the user pastes the recap to the orchestrator session, which runs `/orchestrate-end`.

Procedure:

1. **Recap to the orchestrator** what landed this session in 3–5 bullets — work-completed only (features, tests, decisions, follow-ups). **No context % / self-reported ctx** (root `CLAUDE.md` "Canonical context source"). In team mode this recap (via `SendMessage`) wakes the orch for `/orchestrate-end`; single-operator: the user relays it.

2. **Self-audit TDD compliance** for code changes this session:
   - For each non-trivial code change, did a corresponding test land *first*?
   - Non-deterministic behavior is exempt from unit testing — but the project's non-deterministic-coverage path should still have been followed.
   - If a test was written *after* the implementation: flag it. Note in the session doc: *"TDD violation: <file> implemented before tests existed."*
   - If TDD was skipped on something safety-critical: surface it as a blocker, not just a note.

2.5. **Cross-doc invariant audit.** Read the "Cross-doc invariants" table in `daemon/CLAUDE.md`. For each model listed, check whether its field list (added/removed/renamed fields) changed this session. If yes, verify the corresponding `ARCHITECTURE.md` section was updated in the same set of commits. If a model field changed without a doc edit:
   - Flag it as a discipline violation.
   - List the affected model + section + the specific fields that changed.
   - Surface to the user with a recommendation: update the doc now, or accept the drift with an explicit ADR-style note.
   - The session doc must annotate this as an open follow-up.

2.6. **Step-9 items — already routed hot; don't re-route or re-enumerate.** The orchestrator routed each during the session; its `/orchestrate-end` is the single verify pass. Just ensure any *still-open* follow-up is captured in the session doc's "Open follow-ups." **Do NOT modify `MVP_TASKS.md` or `daemon/LESSONS.md` here.**

2.7. **Wiring / reachability — confirm, don't re-trace.** Each feature already stated *"Reachable from `<entry>` via `<path>`"* at `/tdd` Step 7.5; carry those into the session doc's Reachability section. Re-trace (`/wired`) only a feature whose wiring a *later* slice might have removed. Any still tested-but-unwired feature → an open follow-up "Future TODO — belongs to a phase." A green suite over an unreachable feature is a silent gap.

3. **ALWAYS create a session doc** at `docs/sessions/<NNN>-<YYYY-MM-DD>-<topic>.md`. This is required, not optional. Compute `<NNN>` as the next sequential number — `ls docs/sessions/`, find the max NNN prefix, increment, zero-pad to 3 digits.

   The doc must include these sections:
   - **Header** — date, phase, predecessor + successor session links
   - **Why this session existed** — what problem prompted it
   - **What was built** — **Files created** (list with one-line purposes) + **Files modified** (list with what changed)
   - **Decisions made** — with rationale for each
   - **Decisions explicitly NOT made** — deferred to a future session, with rationale
   - **TDD compliance** — clean or violations (matches §2 above)
   - **Reachability** — for each feature, the entry point it's reachable from (§2.7); list any tested-but-unwired gaps
   - **Open follow-ups** — what's left for future-you, including the Step-9 categorized list from §2.6 + any wiring tasks from §2.7
   - **How to use what was built** — only when relevant

   After writing the new session doc, **update the predecessor session doc's "Successor session" link** to point at the new file. The link convention is bidirectional.

4. **Run `git status --short`** to confirm: every file the session doc lists as created/modified shows up in the diff. If something's missing, the doc is incomplete — fix before the user sees it.

5. **Invoke `/preflight`** as the final gate. If it fails, surface the failure and **do not finalize the session doc as "ready"** — annotate it as `incomplete: preflight failures` with the specific failures listed.

6. **Hand off to the orchestrator** (directly — single-operator fallback: paste it across):
   > *"`/session-end` complete. Session doc at `docs/sessions/<NNN>-<date>-<topic>.md`. Recap + Step-9 categorized list + any wiring follow-ups are in the doc's Open follow-ups section. Over to you for `/orchestrate-end`."*

**Commit posture for this command:**

Slice commits already happened during the session — `/tdd` Step 10 commits each slice immediately after the user approves Step 9. By the time `/session-end` runs, the only implementer-side artifact left to commit is the session doc itself (and any test-fix from the §2 audit).

```bash
git add docs/sessions/<NNN>-<date>-<topic>.md
# + any test-fix files from the audit
git status --short  # Verify only session doc + audit fixes staged
```

Commit with Conventional Commits + `Assisted-by: Claude Code` trailer. Topic prefix `docs(sessions)` or `chore(sessions)` typically.

**Do NOT push.** Push happens at end of round (the orchestrator's `/orchestrate-end`).

**Do NOT modify `MVP_TASKS.md`, `daemon/LESSONS.md`, or `ARCHITECTURE.md` from this command.** Those are orchestrator-owned. Step 2.6 *surfaces* what should be in those files; the orchestrator *writes* via `/orchestrate-end` (or hot during the session).
