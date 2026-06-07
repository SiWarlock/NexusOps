---
description: Pull a structured trace for a given id and format it for inspection. Usage: /trace <id>
allowed-tools: Bash, Read, Grep
argument-hint: "<id>"
---

Pull the structured trace for a given id and format the lifecycle for inspection.

Argument: `$ARGUMENTS` — the id of the run / request to inspect.

<!-- ▼ EXAMPLE BLOCK [id=trace-body]: /trace body — from the source project. Replace wholesale. ▼ -->

NexusOps is event-sourced: the append-only `events` log (ARCHITECTURE.md §7.1) is the spine, projections (`proj_*`, §7) are read-only derivatives, and every mutation flows through the Action Gateway pipeline (§6). `/trace` reconstructs ONE causal chain from the log. The id you pass is one of:

- a **`correlation_id`** — the whole flow (the mandatory envelope field that ties every event in one logical operation together, §7.1);
- an **`action_request_id`** — one trip through the Gateway pipeline (submit → preview → policy → approval → execute);
- an **`event_id`** — a single envelope you then walk outward via its `causation_id` (the immediate prior event, §7.1) and shared `correlation_id`.

## Procedure

1. **Local lookup first** — the daemon owns one SQLite DB (WAL) at `~/Library/Application Support/NexusOps/nexusops.db`; the `events` table is the source of truth (§7.2). Pull the chain ordered by `seq` (the canonical order — never `occurred_at`; clocks skew, §7.1):
   ```bash
   DB="$HOME/Library/Application Support/NexusOps/nexusops.db"
   # correlation_id → whole flow; or substitute action_request_id / event_id below
   sqlite3 -json "$DB" "
     SELECT seq, event_id, event_type, occurred_at, actor_type, actor_id,
            source_type, correlation_id, causation_id,
            action_request_id, approval_id, sensitivity, payload_json
     FROM events
     WHERE correlation_id = '$ARGUMENTS'
        OR action_request_id = '$ARGUMENTS'
        OR event_id = '$ARGUMENTS'
     ORDER BY seq ASC;" | head -400
   ```
   If you started from a single `event_id`, resolve its `correlation_id` from that row, then re-run the query on the `correlation_id` to get the full chain. Honor `sensitivity` (`public|internal|confidential|secret|restricted`, §7.1) — do NOT echo `payload_json` of `secret`/`restricted` events (terminal output defaults `restricted`); show the envelope + a redacted marker.

2. **Cross-check the Gateway state row** — for an `action_request_id`, the `action_requests` / `approvals` rows are canonical for execution; `proj_approval_queue` may lag (§7.2). Read the row to confirm the terminal status:
   ```bash
   sqlite3 -json "$DB" "SELECT action_request_id, action_type, risk, status FROM action_requests WHERE action_request_id = '$ARGUMENTS';"
   sqlite3 -json "$DB" "SELECT approval_id, action_request_id, decision, decided_at FROM approvals WHERE action_request_id = '$ARGUMENTS';"
   ```

3. **Format the lifecycle** for human inspection — render the chain as the Gateway pipeline (§6) it represents, anchored on `seq` and the causal links:
   ```
   correlation_id: <id>
   Pipeline (ordered by seq, causation_id chained):
     [seq=N]   ActionRequested        action_type=<t> risk=<0-4> actor=<actor_type:actor_id>
     [seq=N+1] ActionPreviewGenerated preview_class=<...>  (causation_id → seq N)
     [seq=N+2] PolicyEvaluated        decision=<allow|require_approval|require_step_approval|deny|downgrade|needs_more_context>
     [seq=N+3] ApprovalRequested      approval_id=<...>     (only if policy required it)
     [seq=N+4] ApprovalGranted|Denied decided_by=<actor>    edits?=<...>
     [seq=N+5] ActionExecuting        lease=<token> (stale-precondition re-check passed, §6.2/§16.4)
     [seq=N+6] ActionExecuted | ActionFailed(<reason>)
   Projections touched (one event → many, single commit txn, §7):
     <proj_* names updated by this chain, e.g. proj_approval_queue, proj_audit_trail, proj_session>
   Final outcome:
     <terminal Gateway status from the action_requests row>
   ```
   For a multi-step `ActionPlan` (O-3, §6.2), group by `plan_id` / `step_id`; each step is its own preview→approval→execute sub-chain sharing the plan's `correlation_id`.

4. **On a non-OK final status** — surface the failure precisely:
   - **`ActionFailed(stale_precondition)`** (§6.2/§16.4): the live source (§7.2 — git2/GitHub/keychain/harness) changed between approval and execute; the previewed diff/resource no longer matched, so the Gateway refused to execute a different mutation than was approved. Report what was re-read and what diverged.
   - **`PolicyDecision = deny`** (§6.2): which policy rule denied it (e.g. critical risk-4 action not in approve-all; `workflow.command.invoke` with null `input_schema` risk-floored to require approval).
   - **Integration failure** (§9/§17): transient (`429`/`5xx` → outbox backoff) vs terminal (`401`/`403` → `*SyncFailed` + profile→`auth_expired`). Say which.
   - **Harness/PTY/app-server exit** (§17): `SessionFailed` + `TerminalProcessExited` → in-flight action failed + lease released. Name the child that exited.
   Always state which `seq`/`event_type` emitted the terminal event and whether downstream projections were left consistent (the failing event still commits to the log — it is the audit record).

## Output

A single formatted lifecycle block keyed on `correlation_id` + the terminal Gateway status. Offer the raw `seq`-ordered envelope list (envelopes only, payloads redacted by `sensitivity`) only if the user requests a deep dive.

## Forbidden in this command

- **Treating a projection as truth.** `proj_*` rows are rebuildable read-only derivatives (§7.2); the `events` log + the `action_requests`/`approvals`/`leases` rows are canonical. If they disagree, the projection is lagging — report the canonical state, never the projection's.
- **Fetching or interpreting ids outside NexusOps's envelope contract.** If an id matches no `correlation_id` / `action_request_id` / `event_id` in this DB, say so; don't try to interpret a foreign id.
- **Inferring an absent stage.** The chain IS the log — if there's no `ActionPreviewGenerated` or `ApprovalGranted` event for a step, report "no event" (it never happened or was never recorded); never fabricate a stage the log doesn't contain.
- **Leaking sensitive payloads.** Never echo `payload_json` of `secret`/`restricted` events; only `keychain_ref` pointers ever live in the DB (§7.2) — there is no secret to surface anyway, but redact the marker explicitly.

<!-- ▲ END EXAMPLE BLOCK [id=trace-body] ▲ -->
