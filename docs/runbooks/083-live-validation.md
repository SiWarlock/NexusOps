# Runbook — 083 live GitHub Merge/Review validation (via the dev-client smoke CLI)

> **What this is for.** Validate the live, authenticated GitHub write path end-to-end — keychain auth → live-writes toggle → a real PR merge / review — **from the terminal, without the UI** (the UI PR Merge/Review buttons are still disabled placeholders; ui-track owns wiring them). Every step submits an EXISTING audited action through the real Gateway pipeline — the CLI bypasses nothing (each write is risk-3 + per-action approved).
>
> **Authored:** 2026-06-26 (086 / `smoke_cli_083_live_validation_driver`). Reachable surface: the `dev-client`-feature smoke binary.

## Prerequisites
- macOS; `$HOME` set. The daemon writes `~/Library/Application Support/NexusOps/` (db, pidlock, `gateway.sock`).
- `gh` installed + authenticated for the GitHub account you'll connect (the 083 interim reuses `gh auth token`; the no-`gh` device-flow is 084, not yet built).
- A **real test repo + an open PR** you're willing to merge — OR you create one via the chain (`create-pr`). Per the §63/082 rule, `merge-pr` fail-closes `NotFound` on an out-of-band PR, so the merged PR must be one the daemon created+folded (`create-pr`), not an arbitrary existing PR.
- The repo must be registered as a project in the daemon (so `project_id`/`repo_id` resolve) — via the normal project add/rescan path.

## Build + run
```bash
# Terminal 1 — the daemon (leave running)
cargo run -p nexusopsd

# Terminal 2 — the dev-client smoke binary (talks to the same gateway.sock)
cargo build --release -p nexusopsd --features dev-client
DAEMON=./target/release/nexusopsd
```

## Step 0 — add a project (rescan)
Before the GitHub chain, register the repo as a project (so `create`/the cockpit can target it):
```bash
$DAEMON smoke rescan --path <repo>        # risk-0 → auto-executes (no approve) → ProjectRescanned → proj_project
```
> **⚠️ Pending follow-on — reading the `project_id`.** As of 088 the `rescan` action carries `project_id:None`, so the `proj_project` projector healthy-skips it: **the project is not yet registered and no id is printed** → you can't yet chain into `smoke create --project <id>`. The fix (the rescan `project_id` mint+print follow-on, the #1 daemon next-slice) makes `rescan` **mint + print `project_id=<minted>`**. **Once that lands**, the full add→use chain is: `smoke rescan --path <repo>` (prints `project_id=<minted>`) → `smoke create --project <minted> --prompt "..."` → `smoke queue` → `smoke approve`.

## The validation chain (each `submit`-style step → approve it)
The connect/toggle/create/merge/review steps SUBMIT an audited action and print an **approval id**; approve each with the existing `smoke approve <id>` (the per-action human approval is the gate you're validating). `connect-gh` is the one exception (a peer-authed IPC trigger, no approval).

```bash
# 1. Source the gh token into the keychain (peer-authed; no approval; prints the keychain_ref)
$DAEMON smoke connect-gh --provider github --account <your-gh-login>

# 2. Register the connection (audited, risk-2) — carries the keychain_ref POINTER, never a token
$DAEMON smoke connect --provider github --keychain-ref <ref-from-step-1> --account <your-gh-login>
$DAEMON smoke approve <approval-id>      # → prints the connection_id

# 3. Flip live-writes ON for that connection (audited, risk-2) — the governance gate
$DAEMON smoke set-live-writes --connection <connection_id> --enabled true
$DAEMON smoke approve <approval-id>

# 4. Create a PR the daemon folds into proj_pull_request (audited, risk-3) — seeds a mergeable row
$DAEMON smoke create-pr --project <project_id> --head <feature-branch> --base main --title "test PR" --body "live-validation"
$DAEMON smoke approve <approval-id>       # → the live PR is created on GitHub + folded

# 5a. Merge it (audited, risk-3, cat-1) — SHA-pinned to the approved head
$DAEMON smoke merge-pr --repo <repo_id> --pr <pr_number> --sha <head_sha> --method squash
$DAEMON smoke approve <approval-id>        # → the live merge hits GitHub

# 5b. (Optional — the Review half) submit a review
$DAEMON smoke submit-review --repo <repo_id> --pr <pr_number> --sha <commit_id> --event approve --body "LGTM"
$DAEMON smoke approve <approval-id>
```

Use `smoke queue` to list pending approvals and `smoke audit` to see the immutable audit trail at any point.

## What "success" looks like
- Step 1 returns a `keychain_ref` (never a token). Steps 2–5 each print an approval id, and after `approve` the action reaches GitHub (the merge appears on the PR; the review posts).
- With live-writes OFF (default), the authed clients fail closed (unauth → `AuthFailed`) — every write stays gated behind the toggle AND the per-action approval.

## Notes / gotchas
- **UI contract skew:** running `create-pr` adds a PR row to `proj_pull_request`. The current UI (0.38) `.strict()`-rejects the daemon's (0.46) newer PR fields → a single PR row degrades the whole cockpit to read-only. So **test the UI cockpit on a FRESH DB first** (`rm ~/Library/Application\ Support/NexusOps/nexusops.db*` while the daemon is stopped), and run this live chain separately, until the ui-track lands the 0.46 regen / `.passthrough()` fix.
- **gh per-account interim:** a repo whose owner ≠ the connected `gh` account won't resolve a token (connect the matching account). The no-`gh` device-flow login is 084 (queued).
- **dev-client tests are CI-dark:** the builder tests run only under `cargo test --features dev-client` (a standing CI residual; tracked).
