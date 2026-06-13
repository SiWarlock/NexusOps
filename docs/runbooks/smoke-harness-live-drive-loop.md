# Runbook — live INV-SEC-1 drive-loop smoke harness (0.1-HITL)

> **What this is.** The authorized "see it work" rig for the **live INV-SEC-1 drive loop** (P4.0b-2):
> the daemon launches a **real `claude`** under supervision, an initial prompt drives it to make two
> tool calls, and you watch the interception **auto-allow a Read** and **gate a Bash** — approving or
> denying the Bash yourself over the IPC path. It is a **dev validation rig, not the cockpit** — the UX
> is deliberately CLI-clunky (poll a projection + `tail` the event log + approve/deny by id). The
> polished interactive terminal is the ui track's **6.3d**, not this.
>
> **Drive mechanism = Option G (lead-ruled 2026-06-13):** an additive `initial_prompt` on
> `session.create` is fed to the session's PTY via the existing `TerminalSession::write` seam. It does
> **not** touch the O-13 launch argv / the #10 enforcement surface, and **safety holds** — every tool
> call still routes through the unchanged `intercept` adjudication.
>
> **Status:** the exact dev-client subcommand name + build flag are confirmed at "harness ready" (the
> implementer settles Step-2.5 Q3/Q4). This runbook is written against the defaults: a feature-gated
> `nexusopsd` subcommand named `smoke`, built with `--features dev-client`. Adjust the two invocation
> lines if those change.

---

## 0. Prerequisites

- **macOS** (the MVP target; the daemon resolves `~/Library/Application Support/NexusOps`).
- A **Claude Max-plan** account (subscription/OAuth auth — the PTY-primary design deliberately avoids the API-key billing pool; see §1).
- The **`claude` CLI** on `PATH` (`which claude`).
- The daemon built **with the dev-client**: `cargo build --release --features dev-client` (from `daemon/`). The release binary is `target/release/nexusopsd`.

---

## 1. One-time auth setup (Max plan, clean env)

The daemon strips `ANTHROPIC_API_KEY` from the spawned `claude` child (`ClaudeLaunchSpec::env_mutations` — §15 #8, so the agent rides subscription/OAuth auth, not the API-key pool). Set up the OAuth token and keep the env clean:

```bash
# 1. Mint a Max-plan OAuth token (interactive — opens a browser):
claude setup-token        # → prints / stores CLAUDE_CODE_OAUTH_TOKEN

# 2. Export it for the daemon's shell (the daemon inherits it; the child inherits it from the daemon):
export CLAUDE_CODE_OAUTH_TOKEN="<token from setup-token>"

# 3. Ensure NO API key is set (belt-and-suspenders — the daemon strips it anyway):
unset ANTHROPIC_API_KEY
env | grep -i anthropic   # expect: nothing (or only CLAUDE_CODE_OAUTH_TOKEN)
```

> Run `! claude setup-token` in this session if you'd like its output captured here; the `!` prefix
> runs it in-session.

---

## 2. Start the daemon

Start it from a **sensible project directory** — the launched `claude`'s cwd is the daemon's cwd
(`std::env::current_dir()`, MVP), so the demo prompt's `Read ./CLAUDE.md` must resolve there. The repo
root works:

```bash
cd /Users/dreddy/Documents/Dev/AI-tools/ai-engineering-control-plane/NexusOps
./daemon/target/release/nexusopsd
# → "nexusopsd: started (contract 0.27.0, db user_version N)"
# → "nexusopsd: GatewayPort listening at gateway.sock"
```

Leave it running in this terminal. Key paths it owns (under `~/Library/Application Support/NexusOps/`):

| File | What |
|---|---|
| `gateway.sock` | the UDS the dev-client + the `claude` `PreToolUse` hook connect to |
| `events.jsonl` | the **redacted** event mirror — the live audit feed to `tail -f` |
| `integrity-incidents.jsonl` | the durable §17 alarm (an audit-fault would land here; expect empty) |

---

## 3. Launch a supervised session with a demo prompt

In a **second terminal** (the dev-client is a short-lived UDS client):

```bash
NX=./daemon/target/release/nexusopsd

# Drive a real claude: a Read (auto-allow) then a Bash (gated). Two distinct tool calls.
$NX smoke create \
  --project "proj_00000000000000000000000000" \
  --prompt 'Use the Read tool to read the file ./CLAUDE.md. Then use the Bash tool to run `ls -la`. Do these as two separate tool calls, then stop.'
# → prints the ActionAck: { session_id: sess_…, action_request_id: act_…, status: … }
```

What happens inside the daemon: `session.create` (risk-0, auto-execute) → the `SessionExecutor` →
the live `PtyLauncher` spawns `claude --permission-mode default --settings <0600 temp>` with the
`PreToolUse` hook wired to `nexusopsd hook` → the executor writes your prompt to claude's PTY.

---

## 4. Observe the interception

claude runs the prompt. Expect the two tool calls to be handled differently:

- **Read** → `agent.file_read` (**risk-0 → auto-allowed** instantly by the interception; no approval needed).
- **Bash `ls -la`** → `agent.bash` (**risk-2 → gated**; the `PreToolUse` hook **blocks** up to 5 min waiting for your decision).

Watch it two ways:

```bash
# (a) tail the redacted event log — you'll see ActionRequested / ActionApprovalRequested / ActionApproved|ActionDenied:
tail -f ~/Library/Application\ Support/NexusOps/events.jsonl

# (b) poll the approval queue for the pending Bash:
$NX smoke queue
# → lists pending approvals: { approval_id: appr_…, action_type: "agent.bash", … }
```

The Read should appear **adjudicated-allowed with no pending approval**; the Bash should appear as a
**pending approval** in the queue.

---

## 5. Approve or deny the gated Bash

```bash
# Approve it — claude's blocked hook receives "allow" and the Bash runs:
$NX smoke approve appr_<id-from-queue>

# …or deny it — claude's hook receives "deny" and the Bash is blocked:
$NX smoke deny appr_<id-from-queue> "smoke test — denying to see the block"
```

After you decide, the `decision_sink` resolves → the hook returns the verdict → claude either runs `ls`
(approve) or reports the tool was blocked (deny). The `events.jsonl` tail shows `ActionApproved` /
`ActionDenied{reason}` accordingly.

---

## 6. What you just validated (the 0.1-HITL checks — record the outcomes)

These are the empirical validations brief 051 / session 020 flagged as pending a real Claude:

- [ ] **The live loop runs** — a real `claude` launched under the daemon, supervised, and made tool calls.
- [ ] **Auto-allow works** — the Read (`agent.file_read`, risk-0) sailed through with no approval.
- [ ] **Gating works** — the Bash (`agent.bash`, risk-2) blocked and waited for your IPC `approve`/`deny`.
- [ ] **The hook grammar is honored** — claude actually respected the daemon's allow/deny `PreToolUse` output (the exact grammar was previously unvalidated; confirm allow→runs, deny→blocked).
- [ ] **The deny baseline** — (optional) prompt claude to use a Task subagent or an MCP tool; the `permissions.deny: ["mcp__*","Task"]` baseline + the receiver `CoverageGap` deny should block it.
- [ ] **Hook-miss fails closed** — (optional, advanced) confirm a tool with no cached allow blocks rather than silently proceeding.
- [ ] **No integrity alarm** — `integrity-incidents.jsonl` stays empty (no audit-write fault on the happy path).

---

## 7. Cleanup

```bash
$NX smoke kill sess_<id>     # (if implemented) stop the supervised session
# Ctrl-C the daemon terminal → graceful drain + the PidLock releases.
# The generated per-session settings live at $TMPDIR/nexusops-claude-settings-<session_id>.json (0600).
```

---

## 8. Troubleshooting

- **The prompt doesn't submit / claude sits idle.** This is the known **#1 risk** (the prompt is written
  to the PTY immediately post-spawn and may race claude's TUI input-handler init). Try, in order:
  (a) re-run `smoke create` (the second launch often catches a warm TUI); (b) the implementer's chosen
  submit terminator may not match — the fallback is the other of `\r`/`\n` (confirmed at the live run);
  (c) a small pre-write settle / driving the write off the `SessionStart` hook is the documented follow-up
  if early-input loss is consistent. **The interception itself is unaffected** — this is purely the
  dev-drive convenience.
- **`session.create` returns a policy/precondition error.** Check the daemon log; confirm `project_id`
  is present (any string works — the resource_ref carries it) and the daemon is the live build (contract 0.27.0).
- **The hook can't reach the daemon.** Confirm the daemon is running + `gateway.sock` exists; the hook
  **fails closed** (denies the tool) when it can't reach the daemon — that's by design.
- **`claude` auth failures.** Re-mint `claude setup-token`; confirm `CLAUDE_CODE_OAUTH_TOKEN` is exported
  in the **daemon's** shell and `ANTHROPIC_API_KEY` is unset.

---

## 9. Provenance

- **Brief:** `docs/briefs/053-P4-0b-2-smoke-harness-live-drive-loop.md`
- **Drive loop it exercises:** brief 051 / session 020 (the cat-1 live INV-SEC-1 drive loop, `bd7523b`).
- **Drive-mechanism ruling:** Option G (lead, 2026-06-13) — `pty.write` seam, not the O-13 argv.
- **Follow-on:** the interactive-terminal UX = ui-track 6.3d (Option B); the prompt-feed timing hardening = a small daemon follow-up if the live run shows early-input loss.
