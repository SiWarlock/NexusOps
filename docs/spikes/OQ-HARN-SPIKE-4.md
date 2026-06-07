# OQ-HARN-SPIKE-4 — Codex app-server schema-pin + CI schema-diff gate

| | |
|---|---|
| **MVP task** | 0.3 (Phase 0) |
| **Open question** | `OQ-HARN-SPIKE-4` — Codex `app-server` schema-pin + CI-regen policy |
| **Spec anchors** | `ARCHITECTURE.md §9.1` (Codex adapter), ADR-006 (one contract, two lifecycle models), `§16` (version-compat matrix) |
| **Status** | ⚠️ PARTIALLY RESOLVED — procedure + CI gate scaffolded; **live schema capture DEFERRED (codex CLI not installed)** |
| **Date** | 2026-06-07 |
| **Gates** | Phase 3 Codex adapter |

---

## 1. Decision / status

**The `codex` CLI is NOT installed on this build machine** (`command -v codex` → not found).
Per the brief's absent-tool rule, this spike **does not block**: the pin procedure + the
CI schema-diff gate are scaffolded and the **live schema capture is a one-command follow-up
the user runs once Codex is installed**. What ran vs deferred is explicit in §4.

The **stable method surface** the MVP binds to is already LOCKED in `ARCHITECTURE.md §9.1`
+ ADR-006, so Phase 3 can proceed against that contract; the schema bundle is a CI
drift-detector, not a blocker for adapter design.

---

## 2. Pinned method set + approval shapes (from `§9.1` / ADR-006 — authoritative)

MVP depends only on **stable** methods (NOT `experimentalApi`-gated):

| Method / notification | Role |
|---|---|
| `thread/start{cwd}` | start a session; returns thread id (no stdout race) |
| `thread/resume` | resume after daemon restart (O-2 survival) |
| `thread/list?cwd=` | re-associate sessions to a worktree after a pipe drop |
| `turn/start` | drive a turn |
| `thread/status/changed` | push status (no PTY scraping — §9.1 invariant #9) |
| `turn/completed`, `thread/tokenUsage/updated` | completion + telemetry pushes |
| `item/commandExecution/requestApproval` | **host-routed approval → Action Gateway** |

- **Approval shapes:** handle **modern + legacy** shapes, **or pin a min Codex version**
  (recommended: pin, so the adapter binds one shape). The modern shape is the host-routed
  `item/commandExecution/requestApproval` (+ `applyPatch`, `mcp_tool_call` elicitation).
- **No settable session id** (key on `cwd + returned thread_id`); **no context-window %**
  (`HarnessCapabilities.supportsContextMetadata=false` → UI renders "unknown", never 0%).
- **`-32001` overload = transient/retryable** (outbox backoff, not a terminal failure).
- **Rollout JSONL** (`~/.codex/sessions/...`) is forensic read only, **hardened 0600**
  (bug #21660 / §15 invariant #11) — pre-create the dir 0700 before launch.

These are the conformance-suite + golden-fixture targets for Phase 3 (`§14`).

---

## 3. Pin procedure + CI schema-diff gate (scaffolded)

**Scaffolding:** `docs/spikes/codex-schema/snapshot.sh` (executable) + this doc.

```bash
# once codex is installed (pins version + writes the reviewed baseline bundle):
docs/spikes/codex-schema/snapshot.sh capture     # -> codex-version.txt + codex-app-server-schema.json
git add docs/spikes/codex-schema/codex-*.{txt,json} && git commit   # commit as the baseline

# CI gate (fails on unreviewed version OR schema drift on a Codex bump):
docs/spikes/codex-schema/snapshot.sh check        # exit 1 on drift, 2 if codex absent
```

> **TODO-VERIFY (codex absent at authoring):** the capture sends a JSON-RPC `initialize`
> handshake to `codex app-server --stdio` and records the advertised surface. The exact
> introspection method name + response shape **must be confirmed against the installed
> codex** before the gate is trusted (marked `TODO-VERIFY` in the script). The script
> canonicalizes via `jq -S` so the committed baseline diffs cleanly.

**GitHub Actions gate (ready to enable once the baseline is committed):**

```yaml
# .github/workflows/codex-schema.yml  (add when codex is installed + pinned)
name: codex-app-server-schema
on: { pull_request: {}, push: { branches: [main] } }
jobs:
  schema-diff:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - run: npm i -g @openai/codex@<PINNED_VERSION>   # pin == codex-version.txt
      - run: bash docs/spikes/codex-schema/snapshot.sh check
```

---

## 4. What ran vs deferred

| Item | Status |
|---|---|
| Detect codex CLI | ✅ ran — **absent** (`command -v codex` → not found) |
| Stable method set + approval shapes | ✅ documented (from LOCKED §9.1 / ADR-006) |
| Pin procedure | ✅ scaffolded (`snapshot.sh capture`) |
| CI schema-diff gate | ✅ scaffolded (`snapshot.sh check` + Actions snippet) |
| **Live schema bundle capture** | ⛔ **DEFERRED** — needs codex installed (HITL/user one-liner) |
| **`TODO-VERIFY` introspection method** | ⛔ **DEFERRED** — confirm against installed codex |

---

## 5. Flags back to the orchestrator

- **Deferred (needs user/HITL):** install + pin `codex`, run `snapshot.sh capture`, commit
  the baseline, confirm the `TODO-VERIFY` introspection call, enable the Actions workflow.
  Low effort once codex is present; not on the 0.5 critical path for *contract* purposes
  (the method set is already LOCKED in §9.1).
- **No architecture change** — §9.1 + ADR-006 already pin the stable surface; this spike only
  adds the drift-detector. No `/arch-finalize` needed.
