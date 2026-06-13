# Phase 4 deep-dive — the live drive loop, session survival & the failure-mode contract

> **Status:** PLANNING ARTIFACT for the user (orchestrator-authored 2026-06-12). **Not slices.** No P4 code
> is authored until the 4 forks below are ruled. Grounded against `ARCHITECTURE.md §8 / §17 / §5.1 / §9.1 /
> §10 / §18` + the live code seams (cited inline). P4 kickoff is a **category-1 safety checkpoint** (the
> first time real agent tool-calls hit the live Gateway).
>
> **Decision surface:** the user weighs in on the **4 forks** (§4 below). Everything else is settled
> architecture or orchestrator-default. **3.5 (the terminal-attach benchmark) is fork-free and is being
> built in parallel** while these forks are with the user — so P4's gating doesn't idle the track.

> **▶ RULINGS RECEIVED 2026-06-12 (via lead, from the user) — see §7 for detail + the B2 Finding:**
> **(b) = B2 full live-reattach** (the user wants complete O-2: the agent process outlives the daemon +
> reconnect to the live in-flight turn) — **BUT raised back as a FINDING** (§7): B2-strict needs a
> detachable-PTY-broker subsystem the 3.4 host doesn't have, and exceeds §8-as-written; awaiting the user's
> confirm of B2-strict (accept the broker) vs B2-achievable (auto-resume-from-transcript, no broker).
> **(d.1)** approval-wait = configurable §6.2 policy knob, default ~5 min; fail-closed-on-timeout/cancel/death
> LOCKED. **(d.2)** split tool-policy: `TodoWrite`+benign-internal auto-allow; `WebFetch`/`WebSearch`
> require-approval (two policies). **(d.3)** MCP/Task/bg-subagent denied = confirmed MVP. **(c)** locked the
> same beat as the B2-flavor (§7 enum proposal). **(a) STILL OPEN** — user weighing opt-1 (task/session) vs
> opt-3 (actor/session); the 1-vs-3 read is in §7. **HOLD: no P4 slice authored until (a) + the B2 Finding
> resolve** (every P4 slice is downstream of one or both).

---

## 1. What Phase 4 actually is (in one paragraph)

Phases 1–3 built **mechanisms with no production caller, by design**: the Action Gateway (the single audited
mutator), the event store + projections, the Terminal Channel data plane (3.4), and the Claude adapter —
its observe path (042), its `MutationIntercept`→Gateway INV-SEC-1 interception (043, adversarially verified),
and its telemetry emission (044). Every one of these is unit/integration-tested against fakes and synthetic
payloads, but **nothing in the running daemon drives them yet**. `main.rs` still builds the Gateway with
`CatalogPolicy` and never launches an agent. **Phase 4 is the lifecycle that drives the Phase-3 mechanisms
in production** — a per-session supervisor that launches an agent, ingests its status/telemetry, routes its
real tool-calls through the live interception, keeps it alive across daemon restarts (survival), and emits
the §17 failure-mode events the UI renders. It is the security-critical capstone: **P4 is where the
INV-SEC-1 interception goes from "tested" to "live."**

---

## 2. Grounding — the seams that exist vs. what P4 wires (cited)

| Mechanism (built, Phase 3) | The seam, in code | What P4 must wire |
|---|---|---|
| **Live interception** | `route_intercept(gateway, store, payload) → InterceptOutcome` (`harness/claude/intercept.rs:196`). On a mutating tool it returns `AwaitingApproval{action_request_id}` — the doc-comment says verbatim "*the live wall-clock wait + the per-session `decision_sink` binding are P4*". | The hook transport (real `PreToolUse` → daemon receiver), the per-session `decision_sink`, the wall-clock wait on the action's terminal status, `timeout → Deny`. |
| **The production policy** | `AgentMutationPolicy` (`gateway/policy.rs:107`) — **wraps** `CatalogPolicy`: delegates every non-agent + non-dangerous action unchanged, only *raises* `agent.*` to `Deny` on an O-13 deny-rule. The doc-comment calls it "the Gateway's production policy when supervising agents." | Swap `main.rs:71` `Gateway::new(Box::new(CatalogPolicy), …)` → `Box::new(AgentMutationPolicy)`. Mechanically clean (wrapper, not a parallel policy). |
| **Telemetry emission** | `ClaudeAdapter` emits per-heartbeat DELTAS via an injected `TelemetryEventSink` (044). Today the sink is test-injected; no production bind, no periodic pump. | Bind `TelemetryEventSink → WriteHandle::append`; a periodic pump (statusLine `refreshInterval`); the `HarnessAdapter` trait **async-ify**; live transcript/statusLine ingestion I/O. |
| **The adapter trait** | `HarnessAdapter` (`harness/mod.rs:182`) is **sync, `Send`, `Box<dyn>`**, `launch(&mut self)`. The doc-comment: "*3.2/3.3 reshape the drive loop freely … the async drive I/O lives in the runtime/tasks.*" | The drive loop is where the async boundary finally bites (the wall-clock wait + the telemetry pump are inherently timed). Fork (a). |
| **Terminal host** | `TerminalSession` (`terminal/mod.rs:163`) — `read_step()`/`flush()` + emits `TerminalProcessExited` once on child exit via the injected sink. No production caller. | The per-session read-pump task + the §17 PTY-death cascade that *consumes* `TerminalProcessExited`. |
| **Resume** | `ResumeResult{resumed_live: bool, replayed_event_count}` (`harness/mod.rs:162`) is daemon-internal/unfrozen; `FakeHarness::resume()` returns a stub. The ui pins provisional `ResumeMode("resumed"\|"replayed")` + `RecoveryStatus` (`ui/src/contracts/provisional.ts:24-37`) waiting on the §2.5-seam freeze. | The real resume-or-replay logic + the `shared/` freeze of `ResumeResult`. Forks (b)+(c). |

**Key takeaway:** the runtime wiring is small and mechanical (a policy swap + a supervisor module). The *risk*
and the *open decisions* are all in the four forks — not in the plumbing.

---

## 3. Task decomposition + proposed slice order

The tracker already has **4.1 / 4.2 / 4.3**. P4 needs one new task ahead of them — **4.0, the live
drive-loop spine** — because the carry-forward "P4 pins" (the `AgentMutationPolicy` swap, `route_intercept`
→ live-hook, the `decision_sink`, the telemetry pump) describe a *supervisor* that 4.1's survival builds on.

| # | Slice | What it builds | Anchors | Fork it bakes in | Security pass |
|---|---|---|---|---|---|
| **4.0a** | Drive-loop spine + trait async-ify | The per-session supervisor task: launch → status-stream ingestion → read-pump wiring → telemetry pump scaffolding. `HarnessAdapter` async-ify. **No live interception yet.** | §9.1, §10, §5.1 | **(a)** concurrency model | invariant-only |
| **4.0b** | **LIVE interception wiring — CAT-1 SAFETY SLICE** | `AgentMutationPolicy` runtime swap · `route_intercept`→live-hook transport · per-session `decision_sink` · the wall-clock approval wait · `timeout→Deny`. The split tool-policy ruling lands here. | §9.1, §6.3, §15 (INV-SEC-1) | **(d)** live security pass | **OWN security-reviewer pass (every layer)** |
| **4.0c** | Live telemetry pump + sink-bind | Production `TelemetryEventSink → WriteHandle::append` · the periodic pump (statusLine `refreshInterval`) · non-monotonic-cost clamp→≥0 · `metric_quality` degrade-on-guard. | §9.1, §18 | — (rides 4.0a's async-ify) | non-safety |
| **4.1** | Survival: resume-or-replay | The §2.5-seam **`ResumeResult` freeze** · the resume-vs-replay decision · projection rebuild + lease reclaim on restart · the "restart session" affordance · fault-injection recovery tests. | §8, §17, §5.1 | **(b)** survival granularity + **(c)** ResumeResult shape | invariant (lease/audit) |
| **4.2** | Supervised-child-death recovery | Process-group reaper → `SessionFailed`/`TerminalProcessExited`/`TerminalPTYFailed` → fail in-flight `ActionRequest` + release lease; Codex pipe-drop vs crash. | §17, §8 | — (rides 4.0a) | invariant |
| **4.3** | Background jobs + §17 surfaces | Heartbeat/status pollers (derive `stale` by age) · WAL checkpointer · sidecar supervisor (ping/restart/backoff) · the §17 failure-table events the UI renders. | §10, §17 | — | invariant |

**The load-bearing conclusion:** *every P4 slice is downstream of at least one fork.* 4.0a→fork (a); 4.0b→
fork (d); 4.1→forks (b)+(c); 4.0c/4.2/4.3 chain off 4.0a/4.1. **So no P4 code can start before the forks are
ruled** — which is exactly why 3.5 (fork-free) is the parallel work in flight now.

---

## 4. The 4 forks (the user's decision surface)

Each fork: plain-language framing → options with a scored trade-off → my recommendation → what it gates.
Scores are 1–5 (5 = best on that axis). **I recommend, I do not pick** — these shape contract/UX/safety surface.

---

### Fork (a) — drive-loop concurrency model  *(category-4: load-bearing, shapes the trait's async surface)*

**Plain language:** the daemon will supervise several agent sessions at once. Each session has a few
always-running jobs — read its terminal output, ingest its status events, pump its telemetry, and wait on
human approvals. How do we structure those concurrent jobs? (Note: *mutations* always serialize through the
single write-actor regardless — forbidden #2/#3 — so this is purely about the per-session *supervision*
tasks, not about DB safety.)

| Option | Description | Isolation | Matches existing runtime | Shutdown simplicity | MVP fit | Score |
|---|---|---|---|---|---|---|
| **A — one Tokio task per session (RECOMMENDED)** | Each session gets its own async supervisor task; the blocking PTY read-pump on a `spawn_blocking` thread (the terminal-host precedent); status/telemetry/approval-wait as async sub-tasks. Async-ify the `HarnessAdapter` trait here. | 5 — a crashed session can't head-of-line others | 5 — mirrors the drainer/reaper/accept-loop async-task pattern + the one-blocking-write-actor | 4 — per-task `JoinHandle` + the shutdown watch | 5 | **4.6** |
| **B — single multiplexed supervisor loop** | One task polls all sessions round-robin. | 2 — a slow/blocked session stalls the rest | 2 | 4 | 3 | 2.8 |
| **C — actor-per-session w/ its own mini-runtime** | Each session owns a runtime + mailbox. | 5 | 3 | 2 — N runtimes to tear down | 2 — over-built for MVP | 3.0 |

**Recommendation: A.** It is the established daemon pattern (async edge tasks + one blocking write-actor),
gives per-session fault isolation, and is the natural place to finally async-ify the trait (the carry-forward
already names the async-ify as "rides the P4 drive loop"). **Why it's a fork and not a default:** it sets the
`HarnessAdapter` trait's **async signature** — a daemon-internal-but-load-bearing surface that 3.3 (Codex)
and every future adapter implement. I want your sign-off before reshaping it.

**Gates:** slice 4.0a (and transitively 4.0c, 4.2).

---

### Fork (b) — survival granularity: replay-only vs. live-reattach  *(THE PIVOTAL FORK — shapes scope + UX + §14 testability)*

**Plain language:** when the daemon restarts (a crash, an app update, a reboot), the agent processes it was
supervising are gone (they were its children). What do we bring back? Two honest poles:

- **(i) Rebuild-and-offer (replay-only):** on restart the daemon rebuilds the cockpit's view (projections),
  reclaims leases, redraws the terminal from serialized scrollback, and surfaces a **"restart session"**
  button. The agent's *conversation* is preserved in its own transcript either way — the human clicks to
  relaunch it. Fully deterministic, testable now with `FakeHarness` + fault-injection.
- **(ii) Auto-resume (live-reattach):** on restart the daemon *automatically* resumes each agent's
  conversation via the harness's native resume (`claude --resume <id>` / `codex thread/resume`), no human
  click. More ambitious; its correctness depends on each harness's resume actually working, which needs a
  **live** Claude/Codex to verify (the 0.1/0.3 HITL gates) — so the *live* path can't be fully pinned by a
  deterministic test.

The comprehensive-MVP posture (O-2 = "full survival") points at (ii). But the §14 determinism posture wants
the recovery *logic* test-first against fakes. These aren't actually in tension if we **stage** it:

| Option | Description | Deterministic-testable now | UX ambition (O-2) | Live-HITL dependency | Scope in P4.1 | Score |
|---|---|---|---|---|---|---|
| **B1 — staged: deterministic spine now, live-resume thin follow-on (RECOMMENDED)** | P4.1 builds the full resume-or-replay *decision logic* + projection rebuild + lease reclaim + the `ResumeResult` shape + the "restart session" affordance, with `FakeHarness::resume()` returning a deterministic outcome + fault-injection recovery tests. The **live** `--resume`/`thread/resume` call is a thin slice that lands once 4.0a's live drive loop exists + the 0.1/0.3 harnesses are wired. | 5 | 4 — full survival, delivered in two honest steps | isolated to the thin follow-on | bounded, test-first | **4.5** |
| **B2 — full auto-resume in P4.1 now** | P4.1 includes the live `--resume`/`thread/resume` integration. | 2 — the live path needs a real agent to verify | 5 | blocks P4.1 on 0.1/0.3 HITL | large, partly non-deterministic | 3.0 |
| **B3 — replay-only for MVP, auto-resume deferred to P-later** | Ship only (i); the human always re-launches. | 5 | 2 — under-delivers O-2 | none | smallest | 3.2 |

**Recommendation: B1 (staged).** It honors the comprehensive-MVP O-2 goal while keeping P4.1's *logic*
test-first and deterministic (the project's non-negotiable TDD posture for the recovery state machine), and
it quarantines the unavoidable live-agent dependency into one small, clearly-labelled follow-on instead of
letting it block the whole survival slice. **This is your call** because it sets the MVP's survival ambition
and directly shapes fork (c) below (what `ResumeResult` must express) and the 4.1 acceptance criteria.

**Gates:** slice 4.1 (and the §2.5-seam freeze in fork c). **Also resolves:** whether the headless-VT /
scrollback-serialize follow-on brief is a P4.1 *dependency* (needed for (i)'s "redraw the terminal") or stays
deferred (if display-redraw-on-resume is itself staged).

---

### Fork (c) — the `ResumeResult` §2.5-seam freeze (shape + timing)  *(category-4: a `shared/` contract surface the ui consumes)*

**Plain language:** the UI already has placeholder types for "how did this session come back?" — it's been
waiting since Phase 6 for the daemon to freeze the real shape. P4 is where that freeze happens. The only
question is the exact fields, and the daemon's current internal shape doesn't match the ui's placeholder, so
we reconcile them.

- **Daemon internal today:** `ResumeResult{ resumed_live: bool, replayed_event_count: u64 }`.
- **ui provisional today:** `ResumeMode = "resumed" | "replayed"`; `RecoveryState = "recovering" |
  "recovered" | "recovery_failed"`; `RecoveryStatus{ state, … }`; `SessionRow.resume_mode: ResumeMode?`.

The 3.1 freeze *deliberately deferred* this exact reconcile (a `bool` freeze then would've forced a breaking
reshape into the ui's enum). So the lift `resumed_live: bool → resume_mode: ResumeMode` is the planned move.

| Option | Frozen shape | ui reconcile | Fits fork-(b) | Risk | Score |
|---|---|---|---|---|---|
| **C1 — minimal reconciled enum (RECOMMENDED)** | `ResumeResult{ resume_mode: ResumeMode(resumed\|replayed\|**relaunched**), replayed_event_count }` + `RecoveryStatus{ state: RecoveryState }`. Adds `relaunched` to the ui's 2-value enum (resume failed → fresh relaunch is a distinct outcome the §8 row names). Frozen at the **start of 4.1** (seam-first, before 4.1 logic consumes it). | additive (one enum value) | scales to B1/B2/B3 — `replayed`/`relaunched` cover the replay-only pole, `resumed` the live pole | low | **4.5** |
| **C2 — richer shape (per-session detail, timings)** | Add recovery latency, per-resource lease-reclaim detail, etc. | larger | over-specified before live data | premature (LESSON §14 — freeze load-bearing fields, defer the rest) | 3.0 |
| **C3 — defer the freeze again** | Keep daemon-internal; ui stays provisional. | none yet | — | the ui has waited 4 phases; defers the cross-track unblock again | 2.0 |

**Recommendation: C1**, frozen at the head of 4.1 (§2.5-seam discipline: snapshot-test + CONTRACT bump +
the Appendix-A/`daemon/CLAUDE.md` row I author hot). The `relaunched` third value is the one real shape
question — the §8 "resume fails → relaunch + restart affordance" row implies a distinct outcome from a clean
`resumed`/`replayed`. **Mostly downstream of fork (b):** the survival granularity decides whether `resumed`
(live pole) is even reachable in MVP — so rule (b) first, and (c)'s value set follows.

**Gates:** slice 4.1 (the freeze is its first layer).

---

### Fork (d) — the live INV-SEC-1 security pass + tool-policy  *(CATEGORY-1 SAFETY CHECKPOINT — P4 kickoff)*

**Plain language:** this is the moment a real agent's real tool-calls (a `bash`, a file edit) first reach the
live Gateway and get allowed-or-denied for real. The interception was built and *adversarially* verified in
043 — but only against synthetic payloads. Going live exposes three things the user must rule on. **This slice
(4.0b) gets its own security-reviewer pass on every layer, no exceptions.**

The good news from grounding: the **policy wiring is mechanically clean** — `AgentMutationPolicy` *wraps*
`CatalogPolicy` (delegates all non-agent actions unchanged, only raises `agent.*` to Deny), so the runtime
swap is a one-line `main.rs` change with no composition puzzle. That narrows the fork to three real questions:

**d.1 — the wall-clock approval wait (the security-flagged surface).** When a mutating tool needs human
approval, the hook call *blocks* the agent while the daemon waits for the human. How long, and what cancels
the wait? A live cancellation race (session dies / daemon shuts down mid-wait) must fail **closed** (→ Deny).
- *Recommendation:* a bounded timeout (proposal: **configurable, default ~5 min**) → `timeout → Deny`;
  session-death and daemon-shutdown both cancel → Deny; the `decision_sink` is fired exactly once. Pin the
  cancellation paths with fault-injection. **Your input wanted on the default timeout** (UX vs. safety: too
  short denies legitimate slow approvals; the fail-closed direction is non-negotiable).

**d.2 — the split tool-policy (carry-forward, lead-flagged for return-review).** The conservative
deny-unknown denies *benign* tools too, which would cripple a live agent. Two sub-rulings:
- **(i) benign-internal auto-allow:** `TodoWrite` (no FS/git/external/exfil surface — risk-0 per Q2) should
  auto-allow so the agent stays functional. *Recommendation: allow this narrow internal set.*
- **(ii) agent network-egress policy:** `WebFetch`/`WebSearch` carry a **data-exfil dimension** (a secret in
  a URL crosses the trust boundary) — *not* trivially benign. *Recommendation: a separate user-owned egress
  policy (default: require-approval), NOT folded into the benign set.* **This is a user policy call.**

**d.3 — acknowledge the dedicated security pass + the known MVP tradeoff.** Supervised Claude **cannot use
MCP tools** in MVP (they fall through the coverage gap → denied). That's a recorded 043 tradeoff; going live
makes it user-visible. *Recommendation: accept for MVP (documented); the receiver-side `CoverageGap` deny is
the primary control, `permissions.deny` is defense-in-depth.*

| Sub-decision | Recommended default | Non-negotiable | Your call on |
|---|---|---|---|
| d.1 approval-wait timeout | ~5 min, configurable | fail-closed on timeout/cancel/death | the timeout *value* |
| d.2(i) `TodoWrite` auto-allow | allow (benign-internal) | no FS/git/external in the allow-set | confirm the set |
| d.2(ii) `WebFetch`/`WebSearch` | separate egress policy, require-approval | egress ≠ benign | the egress policy |
| d.3 MCP-denied + security pass | accept for MVP + own review | the live slice gets the security pass | acknowledge |

**Gates:** slice 4.0b — and this is the **category-1 safety checkpoint** the lead flagged for P4 kickoff. No
live-wiring slice is authored until you rule d.1/d.2/d.3.

---

## 5. What this means for sequencing (the recommendation in one line)

Rule the **4 forks** → I author 4.0a (fork a) + 4.0b (fork d, CAT-1) first, then 4.0c, then 4.1 (forks b+c),
then 4.2 / 4.3. **Meanwhile 3.5 (terminal-attach benchmark, fork-free) is already dispatched** and builds in
parallel, so the track is productive while the forks are with you. **Fork (b) is the one to rule first** — it
cascades into (c) and into the whole shape of 4.1.

## 6. Cross-references
- Tracker: `IMPLEMENTATION_PLAN.md` Phase 4 (4.1/4.2/4.3) + the Carry-forward "P4 pins" + "044→P4
  telemetry-hardening pins".
- Anchors: `ARCHITECTURE.md §8` (recovery flows), `§17` (failure-mode contract), `§5.1` (Session machine +
  `stale` recompute-on-rebuild), `§9.1` (the adapter AS-BUILT notes + the P4 deferrals), `§10` (background
  jobs), `§18` (the deferred daemon-restart-recovery-latency budget — committed when the P4 survival bench
  lands, analogous to 3.5 for terminals).
- Lessons in force: §23 (observation-event vs Gateway), §25 (the PTY-primary adapter + #9/#10), §26 (the
  interception INV-SEC-1 discipline), §27 (telemetry deltas/sink).
- Live seams: `main.rs:71`, `harness/claude/intercept.rs:196`, `gateway/policy.rs:107`, `harness/mod.rs:162/182`,
  `terminal/mod.rs:163`, `ui/src/contracts/provisional.ts:24-37`.

---

## 7. Rulings (2026-06-12) + the B2 survival Finding

### 7.1 — FINDING: B2 "agent process outlives the daemon + live in-flight-turn reattach" is not free under PTY-primary

**The mechanism (grounded, both harnesses).** The cat-4-resolved Claude adapter launches `claude` as a
**daemon child** via 3.4's `PortablePtyHost`, where **the daemon holds the PTY master**. When the daemon
dies/restarts, its master fd closes → POSIX terminal hangup → the slave's foreground process group gets
**SIGHUP** (+ stdin EOF) → the interactive `claude` **terminates**. Codex is the same shape: `app-server
--stdio` is a stdio child; daemon death closes the pipes → EOF/SIGPIPE → exit. This is exactly what 3.4
already models (`TerminalProcessExited` on child exit). **So the agent process does NOT survive the daemon's
death** — and the §17 "agent/PTY dies (daemon alive)" row is the *opposite* case (child dies, daemon lives).

**What B2-strict therefore requires** (new architecture, not in 3.4): for the agent to outlive the daemon,
it must NOT be a direct daemon PTY-child — i.e. a **detachable-session broker** (tmux/abduco-class, the
daemon attaches/detaches to a surviving PTY holder) or a setsid'd agent under a surviving master-holder.
That is a meaningful new subsystem. And the survival itself — the agent *actually* surviving + the reattach
to a *live* in-flight turn — is a **live-process property**, so it is **0.1/0.3-HITL-verify-only** (the
reattach *logic* is FakeBroker-testable; the survival is not deterministically unit-testable).

**The reconsider point (heavy ≠ correct).** §8-as-written already specifies survival as
`--resume`/`thread/resume` = **relaunch-and-resume-from-transcript** (a fresh process resumes the
conversation from the harness's incrementally-persisted transcript) — call it **B2-achievable**. It delivers
the user's stated goal — **complete, automatic O-2 survival** (the conversation continues across a restart
with no human "restart session" click) — **without the broker**, with fully deterministic resume/replay
DECISION logic, and the live `--resume`/`thread/resume` call as the 0.1/0.3-HITL follow-on. The *only* thing
B2-achievable loses vs B2-strict is the literal in-flight turn at the instant of an abrupt crash — which
`--resume` largely reconstructs from the persisted transcript anyway. **Marginal fidelity gained: one
partial turn. Cost of gaining it: a detachable-PTY-broker subsystem + a non-deterministic survival path.**

**Recommendation:** confirm **B2-achievable** as "complete O-2 survival" (auto-resume, no broker, matches §8,
deterministic logic test-first) — OR accept the broker subsystem for **B2-strict** with eyes open. Either
way 4.1 builds the deterministic resume/reattach DECISION logic test-first; the difference is *only* whether a
PTY-broker subsystem enters P4 scope. **Not silently downgrading to B1, not silently eating the complexity —
surfacing the cost for the user's call** (lead's ⚠️ instruction).

### 7.2 — (c) `ResumeResult` §2.5-seam freeze — the value set, keyed to the B2 flavor

Replaces the daemon-internal `ResumeResult{resumed_live: bool, replayed_event_count}` (`harness/mod.rs:162`);
reconciles the ui provisional `ResumeMode("resumed"|"replayed")` + `RecoveryState`/`RecoveryStatus`
(`provisional.ts:24-37`). Frozen at the head of 4.1 (snapshot-test + CONTRACT bump + the Appendix-A /
`daemon/CLAUDE.md` row I author hot).

```rust
// shared/ freeze (wire = snake_case TEXT):
pub enum ResumeMode {                 // ui adds the new values to its 2
    Resumed,        // harness-native --resume/thread/resume succeeded (relaunch-and-resume-from-transcript)
    Replayed,       // no harness resume → serialized-scrollback replay + relaunch
    Relaunched,     // resume failed → fresh relaunch + "restart session" affordance (§8 "else…" tail)
    // ReattachedLive,  ← ADD iff B2-strict (reconnected to the SURVIVING same-process in-flight turn;
    //                     distinct from Resumed = a NEW process from transcript). Omit under B2-achievable.
}
pub struct ResumeResult { pub mode: ResumeMode, pub replayed_event_count: u64 }
pub enum RecoveryState { Recovering, Recovered, RecoveryFailed }  // == the ui's existing 3, frozen as-is
```

**Per LESSON 14 (freeze load-bearing, defer the rest):** freeze the **3-value** set (`resumed|replayed|
relaunched`) under B2-achievable; add the **4th** (`reattached_live`) only if the user confirms B2-strict.
Decide (c)'s value count the same beat as the §7.1 Finding.

> **✅ CONFIRMED (away-authority, 2026-06-12; ⚠️ return-review):** B2-strict ruled → **`ResumeMode = {Resumed,
> Replayed, Relaunched, ReattachedLive}` (4 values)** + `RecoveryState` = the ui's existing 3, frozen as-is.
> **Freezes at the head of 4.1** (the §2.5-seam — CONTRACT bump + schema-snapshot; the ui adds
> `relaunched`+`reattached_live` to its provisional 2). Not yet authored (4.0b-1/4.0b-2 precede 4.1).

### 7.3 — (a) drive-loop concurrency: opt-1 (task/session) vs opt-3 (actor/session), in light of B2 — for the user's call

**The daemon's existing idiom IS an actor idiom.** The write-actor (LESSON 9) is literally *a dedicated
thread + an mpsc mailbox + a command loop*; the accept-loop is already *task-per-connection*. So **opt-3
(session-as-supervised-actor) is the daemon's OWN write-actor pattern applied to sessions — not a new
framework, low novelty risk.** The user's instinct (3 ≈ best-practice) is well-founded *and* cheap here.

**B2 mildly favors opt-3.** A session under B2 is a long-lived, stateful, reattachable entity whose control
surface grows — pause/resume, the inbound client `{pause}`/`{resume}` (6.3d), reattach-on-daemon-restart,
policy-grant, kill. A **mailbox-actor** models that as messages to the session-actor, with a **supervisor**
parent tracking `JoinHandle`s + owning the restart/reattach lifecycle — exactly the shape B2's survival
wants. **opt-1 (inline task)** hand-rolls the same control-routing without the mailbox discipline → it works,
but converges toward opt-3 as the control surface grows. Either way, **mutations still funnel to the single
write-actor** (INV-SEC-1 — the session-actor is an *edge* actor that proposes intents, never a 2nd mutator).

**My read:** in light of B2, **opt-3 pays off** (the daemon already has the pattern; B2's reattach/control
lifecycle is what a mailbox-actor is for) — but the cost delta over opt-1 is small (both = a Tokio task + a
`spawn_blocking` read-pump; opt-3 adds the mpsc mailbox + a supervisor parent). **The user's final call.**
Whichever (a) lands, it sets the `HarnessAdapter` async signature + the supervisor module shape that every
P4 slice lives in — hence the HOLD.

### 7.4 — d-rulings (final) → fold into the 4.0b brief; record in Decisions-tabled at the round seal
**d.1** approval-wait = a **configurable §6.2 policy knob (default ~5 min)**; **fail-closed on
timeout/cancel/session-death LOCKED** (INV-SEC-1, not a knob). **d.2** split: `TodoWrite` (+ benign-internal
non-mutation) **auto-allow**; `WebFetch`/`WebSearch` **require-approval** — two separate policies. **d.3**
MCP/Task/bg-subagent **denied = confirmed MVP**. (These feed 4.0b, which is HELD on (a).)

---

## 8. FINALIZED P4 slice order (all forks ruled 2026-06-12 — B2-strict · opt-3 · 4-value ResumeMode · broker in scope)

**HOLD lifted.** Rulings: **(b) B2-strict** (live-process reattach; the detachable-terminal **broker
subsystem is in P4 scope** — a user-ruled §8 EXTENSION, §8.1 below) · **(a) opt-3** (session-as-supervised-
actor; sets the async trait signature + the supervisor shape) · **(c)** freeze `ResumeMode = resumed |
replayed | relaunched | reattached_live` (the 4th per B2-strict) · **d.1/d.2/d.3** as §7.4.

**The cat-1 boundary (decisive for the order):** *launching an agent session is itself a mutation* (it spawns
a process + creates session state) → session-create is a **Gateway action** (INV-SEC-1). And a live agent that
can call tools is unsafe until the **live interception** is wired. So the **live-agent spawn (E) and the live
interception (F) MUST land together** as the cat-1 slice — everything before it is **FakeHarness-driven
scaffolding** (no live agent, no mutation). That cleaves P4 cleanly:

| # | Slice | Pieces | Cat-1? | Test posture | Anchors | Module |
|---|---|---|---|---|---|---|
| **4.0a** | **opt-3 supervisor spine** (DISPATCH FIRST; IN PROGRESS) | the adapter driven via **`spawn_blocking` the SYNC trait** (Step-2.5-ruled — trait UNCHANGED, NO `async-trait` dep; the leaner route over async-ify) · `SessionActor` (Tokio task + mpsc mailbox + §5.1 status state, owns the adapter + terminal read-pump) · `SessionSupervisor` (spawns/tracks/supervises actor `JoinHandle`s via a `JoinSet`) · the `SessionLauncher`/`TerminalBroker` **SEAM** (daemon-owned-PTY impl now; the survival broker swaps in at 4.1). Spawned in `main.rs` (mirrors `spawn_reaper`); **FakeHarness/FakePty-driven, NO live agent, no event emission (cat-1 enforced STRUCTURALLY — no `WriteHandle` in the module).** | No | deterministic, test-first | §9.1, §5.1, §10 | NEW `daemon/src/session/` |
| **4.0b** | **CAT-1: live INV-SEC-1 interception + the Gateway session-create executor** | the §6.3 `session.create`/`session.kill` action types + executors (spawn the real agent via the supervisor) · the `AgentMutationPolicy` runtime swap (`main.rs:71`) · `route_intercept`→live-hook transport · the per-session `decision_sink` · the §6.2 wall-clock approval-wait knob (default ~5 min, **fail-closed** on timeout/cancel/death) · the split tool-policy (d.2). **The real Claude launch + live interception land together.** | **YES — own security pass; surfaces to lead→user first** | decision logic test-first; live hook = behavior-pinned + the security review | §9.1, §6.3, §6.2, §15 (INV-SEC-1) | `gateway/`, `harness/claude/`, `session/`, `main.rs` |
| **4.0c** | live telemetry pump + sink-bind (the 044 P4 deferral) | production `TelemetryEventSink`→`WriteHandle::append` · periodic pump (statusLine `refreshInterval`) · non-monotonic-cost clamp→≥0 · `metric_quality` degrade-on-guard | No (non-safety) | test-first | §9.1, §18 | `harness/claude/`, `session/` |
| **4.1** | **survival (B2-strict) — the §8 EXTENSION** | the §2.5-seam **`ResumeResult` freeze** (4-value `ResumeMode`) · the deterministic **resume/replay/reattach DECISION logic** (FakeBroker/FakeHarness) · projection rebuild + lease reclaim on restart · the **detachable-terminal broker subsystem** (swapped behind the 4.0a seam) · the reattach protocol (deterministic part) · the "restart session" affordance. **LIVE broker-reattach survival = the 0.1/0.3-HITL follow-on (verify-only).** | invariant (lease/audit) | DECISION logic test-first; live survival = HITL follow-on | §8 (+ EXTENSION), §17, §5.1 | `session/broker.rs`, `harness/resume.rs`, `bootstrap.rs` |
| **4.2** | supervised-child-death recovery (daemon alive) | process-group reaper → `SessionFailed`/`TerminalProcessExited`/`TerminalPTYFailed` → fail in-flight `ActionRequest` + release lease; Codex pipe-drop vs crash | invariant | test-first | §17, §8 | `harness/supervisor.rs` |
| **4.3** | background jobs + §17 failure-mode surfaces | heartbeat/status pollers (derive `stale` by age) · WAL checkpointer · sidecar supervisor (ping/restart/backoff) · the §17 failure-table events the UI renders | invariant | test-first | §10, §17 | `jobs/` |

**Dispatch plan:** 4.0a now (non-cat-1). **4.0b surfaces to the lead → user for the cat-1 Step-2.5 design
review (as 043) before sign-off** — its safety calls: the live `decision_sink` wait/cancel semantics ·
audit-fault-vs-policy-deny at the live sink · the live Claude permission-rule grammar. 4.0c after 4.0a. 4.1
after 4.0b (the broker needs the live launch path). 4.2/4.3 chain off 4.0a/4.1.

### 8.1 — B2-strict = a user-ruled §8 architecture EXTENSION (recorded, not silently expanded)
B2-strict (the agent process outliving the daemon via a detachable-terminal broker, reconnect to the live
in-flight turn) **exceeds §8 as written** (§8 specifies `--resume`/`thread/resume` = relaunch-and-resume). The
user ruled it in (present, eyes-open re: the broker cost + the non-deterministic/HITL-only survival
verification). Treated as an **authorized architecture extension**: → **Decisions-tabled** (user-ruled,
2026-06-12) · → **§8 extended** with the detachable-terminal/broker survival subsystem (forward-note now; full
prose with the 4.1 design) · → **flagged for the next `/arch-finalize`**. The deterministic resume/replay/
reattach DECISION logic stays test-first; the live broker-reattach SURVIVAL is the labelled 0.1/0.3-HITL
follow-on. Any piece that genuinely can't be pinned deterministically gets flagged at decomposition.
