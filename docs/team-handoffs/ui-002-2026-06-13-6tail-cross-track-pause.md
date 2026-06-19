# ui track — clean cross-track PAUSE handoff (6-tail)

> **Team handoff** (`nexusops-ui`, `track/ui`). The ui track has **exhausted its buildable
> in-lane runway** and clean-pauses (user-ruled 2026-06-13 — the edges-R4 pattern: no
> cross-track packet now; the daemon stays on its P4 critical path). This doc is **the resume
> artifact** — it captures the sealed resume state + the exact cross-track contract packet the
> daemon/edges must freeze before the ui track can resume the high-value tail.
>
> **Predecessor:** `001-2026-06-09-ui-phase6-done-styling-redo.md`. **Session docs:** `ui-008`
> (040–044); 045/046 narrative = the briefs + LESSONS 9-ext/§18 + the Log entry (a `ui-009`
> `/session-end` for 045/046 is the lead's spin-down call).

## 1. Resume state (what's done + sealed)

- **Branch / contract:** `track/ui` @ the final round seal (this `/orchestrate-end`); **CONTRACT_VERSION 0.23.0** (generated Zod layer @ 34 value-sets). Pushed to **origin/track/ui** (main NOT pushed — origin/main is user-gated).
- **Suite:** **271 ui tests green**; `/preflight` clean (oxlint + tsc + test:run). The ui CI job is BLOCKING (promoted at 041).
- **The full in-lane runway is landed (040→046):**
  | Slice | Hash | What |
  |---|---|---|
  | 040 | `e1e2730` | regen ui-Zod → 0.19.0 + bounded provisional reconcile (LESSON 14) |
  | 041 | `63ebb1d` | delta-regen → 0.23.0 + promote the ui CI job to BLOCKING |
  | 042 | `b6b7e7f` | pool-kind-aware credit-pool state (`CreditPool.kind` required — LESSON 15) |
  | 043 | `a198af7` | **cat-1** intent-seam FOUNDATION (Scope-A isolation; LESSON 16) |
  | 044 | `259801c` + `0a05b27` | **cat-1** `GatewayModal`-real — the daemon-driven approval card (LESSON 17) |
  | 045 | `a671e52` | §11.6 accessible-names merge-gate net (`auditAccessibleNames`; LESSON 9 ext) |
  | 046 | `4bdebf3` + `8b44d79` | 6.3d Session Terminal **DISPLAY** half (xterm.js, frozen `terminal_output`, **#9**; LESSON 18) |
- **Round seals:** `730f1bb` (040–044) + this `/orchestrate-end` (045 + 046 + docs). Both on origin/track/ui.
- **The intent-seam vertical is COMPLETE in-lane:** the seam (043) + the live approval card (044) + the §11.5 reject cards + the net-new `precondition_stale` re-approvable card. `security-reviewer` PASS on every cat-1 surface.
- **6.3d Session Terminal:** the inline permission card ✅ (043/044) **and** the PTY terminal-well DISPLAY ✅ (046, xterm.js renders the frozen §6.4 `terminal_output` stream via a fixture, DISPLAY-ONLY #9). The live stream + inbound control = P4 (below).
- **The cat-1 safety rulings** (Q1–Q7, lead away-authority) are durable + logged for user return-review: `docs/planning/intent-seam-cat1-safety-design.md` ("LEAD RULINGS"). **Do NOT re-open them on resume.**
- **The dependency Finding** (why the rest is blocked): `docs/planning/ui-6tail-dependency-finding.md`.

## 2. THE UNBLOCK PACKET — what the daemon/edges must freeze before the ui resumes

The high-value 6-tail (6.3e / 7.2 / 8.2) is **cross-track-blocked**. The ui track resumes once these are frozen + (where noted) landed. **Routing this packet is the USER's later call** — it is NOT routed now (the daemon stays on its P4 critical path).

### (i) 6.3e — Code/Diff review with per-hunk actions
- **Per-hunk git ACTION-catalog extension** — add `git.stage_hunk` / `git.discard_hunk` / `git.apply_hunk` / `git.revert_hunk` (or the agreed set) to the frozen §6.3 action catalog (`shared/src/catalog.rs` MVP_ACTION_TYPES + the per-type policy/preview/executor bindings). Today only read-only `git.diff` exists → any per-hunk MUTATION is policy-denied fail-closed. **(daemon, Phase-2-extended.)**
- **A diff-CONTENT source** — either (a) the Phase-5 git2 diff-read path (task 5.2, edges track) + a frozen `Diff` projection (add `Diff` to `ProjectionName` + a `proj_diff` table + `get_projection("Diff")`), or (b) a frozen diff-read RPC. Today there is NO `Diff` projection. **(daemon/edges, Phase-5/7.)**
- *UI-side ready:* `DiffReview.tsx` already renders a diff fixture with disabled per-hunk buttons; the intent seam (043/044) is the mutation path — so 6.3e is **only** blocked on the two contracts above.

### (ii) 7.2 — Full PR Review Workspace (O-6)
- **Phase-7 PR-data integration** — octocrab GitHub + Linear read+link (PR header/checks/reviews/comments/mergeability; the §7.2 PR SoT projection). Not landed on `track/ui`. **(edges/daemon, Phase-7.)**
- *Boundary:* 7.2 layers PR metadata + the merge workflow ON the 6.3e Code/Diff surface; it additionally consumes the intent seam for merge-at-risk≥3.

### (iii) 8.2 — Project Brain drawer
- **Phase-8 Brain contracts** — the Project Brain stdio-MCP sidecar + the §11.5 Brain-card / multi-step-plan / evidence contracts (Brain proposes via the Gateway, never executes). Not landed. **(daemon, Phase-8.)**

## 3. 046 terminal — P4 / cross-track deferrals (the Session Terminal's remaining work)
The 046 DISPLAY half is built against a fixture. To go LIVE:
- **The live `UdsGatewayPort` terminal demux** — the real transport yields `ServerFrame.terminal_output` (replaces the MockGatewayPort fixture stream). **+ per-frame / total-buffer SIZE BOUNDS** (the security-reviewer forward note — bound untrusted PTY output). **(P4 + the transport slice.)**
- **Inbound `{pause}`/`{resume}` flow-control** (a session MUTATION → the intent seam; the Pause button stays disabled today) + **watermark/backpressure** (the §6.4 pump / 30fps batch). **(P4.)**
- **The `exit_code`/`signal` DETAIL render** — `TerminalProcessExited` is a daemon observation EVENT (→ projection), with no frozen UI-readable source today; needs a session/terminal projection field carrying it. Until then the well shows the honest ended-state from the **Session projection status** (the drift-pinned ENDED/LIVE partition). **(P4 / a projection-wiring slice.)**
- **Headless-VT / scrollback fidelity** — the separate follow-on brief.

## 4. Other parked items (carry into resume)
- **The real `UdsGatewayPort` transport** (the live `ServerFrame.rpc_response` id-correlation/demux + the outbound boundary `.parse()` the 043 security-review deferred — the daemon re-validates regardless, INV-SEC-1). The seam + modal + terminal-well all sit ABOVE the `GatewayPort` (transport-agnostic) — they swap the Mock for the real client here.
- **`gatewayApprovalEnrichment` → real daemon projection** (the 044 security [med]) — swap the daemon-SHAPED fixture side-map for the real daemon projection-enrichment + preview/policy RPC **before any real human approves** against fixture risk values.
- **The "Always allow" `policy_grant` standing-grant** — pre-authorizes a CLASS of future mutations; a distinct trust surface NOT covered by the per-action Q1–Q7 rulings → **its OWN category-1 safety checkpoint** (the orchestrator escalates to the lead/user BEFORE authoring). Stays disabled-pinned today.
- **Q7-(B)/(C) intent cache + auto-retry — PARKED-for-USER** (lead's recorded lean: if ever added, **(C) MANUAL resubmit, NOT (B) auto-replay**).
- **`require_step_approval` bundled-plan step UI** · **actionable `safer_alt`** · the generator `oneOf`-of-`const` extension + the `MetricQuality` provisional→generated reconcile · the deferred perf (memoization, the audit FTS) · the deferred 044/046 quality nits.

## 5. How to resume (for the next ui orchestrator)
1. The user routes the unblock packet (§2) to the daemon/edges tracks; wait for the frozen contracts to land + merge into `track/ui`.
2. `/orchestrate-start`; read this handoff + `IMPLEMENTATION_PLAN.md` "Currently in progress" + the dependency Finding.
3. Regen the ui Zod layer against the new contract version (LESSON 14 discipline); reconcile any new provisional shadows.
4. Author 6.3e (per-hunk actions over the intent seam + the diff-content source) **WITH `security-reviewer`** (it's a mutation path) → then 7.2 → 8.2.
5. The cat-1 rulings (Q1–Q7) + LESSONS 16/17/18 are durable — consume them, do not re-derive.

## 6. Lessons banked this pause
- **LESSON 16** — the UI intent-submission seam (cat-1, pure submitter).
- **LESSON 17** — the intent-seam consumer (the daemon-driven approval card).
- **LESSON 18** — the terminal-well display pattern (pure frozen-frame consumer split from the xterm host; DISPLAY-ONLY #9; a UI subscription maps 1:1 to one daemon channel — an EVENT is not a ServerFrame).
- **LESSON 9 ext** — the a11y merge-gate audit now asserts accessible-NAME coverage.
- **LESSON 15** — a required discriminator for a safety-gating field (`CreditPool.kind`).
- **LESSON 14** — contract-bump regen discipline.
