# Event Model & Audit Trail Spec v0.1

> **Status:** Planning draft  
> **Date:** 2026-06-06  
> **Scope:** Durable event stream, audit trail, timeline model, Project Brain ingestion contract, and future remote observability/control support for the desktop AI engineering control plane.  
> **Naming:** Open. Uses neutral labels: **the platform**, **Project Brain**, **Workflow Packs**, and **Action Gateway**.  
> **Depends on:** Product Canon v0.1, Shared Object Model v0.1, Action Gateway Spec v0.1, Project Brain PRD v2.

---

## 1. Executive summary

The platform needs one durable timeline that connects everything a user cares about:

- agent sessions
- terminal activity
- worktrees
- git actions
- commits
- pull requests
- GitHub issues
- Linear tickets
- implementation-plan tasks
- workflow-pack runs
- Project Brain indexing/retrieval/action plans
- human approvals
- agent-team orchestration
- token/context/cost usage
- errors, failures, and stale states

This spec defines the **Event Model** and **Audit Trail**.

The event system is the platform's append-only memory of operational facts. It is how the desktop app can reconstruct state, how the project graph stays accurate, how the Human Input Queue knows what needs attention, how Project Brain learns what happened, and how a future iOS companion can observe and request control without directly touching local credentials or terminals.

The key principle:

> **Commands request change. Actions authorize and execute change. Events record that change happened.**

---

## 2. Desktop-first product decision

The platform is a **desktop app**, not a web app.

This changes the architecture in important ways:

- The main UI shell runs locally.
- Terminal sessions are local first-class surfaces.
- Git worktrees and code editor state live on the user's machine.
- Local credentials remain local.
- The Action Gateway, event store, local runner, Workflow Pack runtime, and Project Brain connector run locally.
- The event log is local-first and should work offline.
- Cloud or relay infrastructure is not required for the MVP.

The app may still expose local services internally, such as:

- local daemon / background helper
- local event bus
- local HTTP or IPC API
- local MCP server
- per-project workers
- terminal/session supervisor

But these are implementation details of a desktop-first product, not a hosted web application.

### 2.1 Desktop app internal architecture

Recommended local process model:

```text
Desktop UI process
  - app shell
  - sidebar
  - project graph
  - terminal panes
  - code editor / diff review
  - Project Brain drawer
  - approval UI

Local coordinator / daemon
  - event store
  - action gateway
  - project registry
  - session registry
  - workflow runtime
  - integration syncers
  - usage ledger

Local runner
  - Claude Code sessions
  - Codex sessions
  - terminals / PTYs
  - worktree operations
  - git operations
  - command execution

Project Brain service
  - indexer
  - retrieval API
  - evidence engine
  - action planner
  - event consumer
```

The UI may be restarted without losing operational history because the durable event store and projections live below it.

### 2.2 iOS companion stretch goal

A companion iOS app is a **stretch goal**, not an MVP requirement.

The iOS app should be framed as:

> **Remote observability and controlled intervention for running desktop-managed agent work.**

It should not be framed as a full remote IDE or cloud runner.

Potential iOS capabilities:

- view active projects and sessions
- see which sessions are active, idle, failed, or waiting on human input
- receive push notifications for approvals and blockers
- approve/deny low- or medium-risk requests
- ask Project Brain questions against redacted summaries
- send a message to a session or agent team
- pause/resume/kill sessions, subject to policy
- open PR/task/status summaries

Explicit non-goals for the iOS stretch goal:

- no raw local credential access on the phone
- no full local filesystem browsing by default
- no arbitrary shell command execution from iOS by default
- no full terminal streaming in the first version unless securely proxied and separately approved
- no direct git mutation from iOS for high-risk actions without strong confirmation and policy

The iOS app should consume **redacted event projections**, not raw local logs/transcripts.

---

## 3. Why the event model matters

The event model is the backbone for five platform capabilities.

### 3.1 Situational awareness

The UI needs to answer:

```text
What is active?
What is waiting on me?
What failed?
What changed?
What is ready for review?
What did this session do?
Which task/worktree/branch/PR does it belong to?
```

These answers come from projections over events.

### 3.2 Auditability

Every mutating action should be reconstructible:

```text
Who requested it?
What evidence supported it?
What preview was shown?
Who approved it?
What executor ran it?
What changed?
What failed?
Can it be undone?
```

### 3.3 Project Brain memory

Project Brain should ingest structured events so it can answer temporal/provenance questions:

```text
When did we implement feature Y?
Which session changed this file?
What task caused PR #84?
Why did the team decide to defer this architecture item?
Which agent team worked on Phase 2 Track B?
```

### 3.4 Workflow Pack observability

Workflow Packs need events for:

```text
personalization runs
command invocations
/team-start launches
orchestrator/implementer session registration
TDD slice progress
context warnings
workflow upgrades
manifest drift
```

### 3.5 Future remote control

A remote companion app should not connect directly to terminals or local files. It should consume a controlled event/projection stream and submit remote action requests through the Action Gateway.

---

## 4. Core principles

### 4.1 Events are facts

An event records something that already happened.

Examples:

```text
SessionStarted
ApprovalRequested
WorktreeCreated
PullRequestOpened
PlanTaskLinked
BrainActionPlanProposed
```

A command or request is not the same as an event. A user may request an action that is denied, fails, or never executes. Those outcomes are separate events.

### 4.2 Append-only by default

Events should be append-only. Corrections are represented as new events, not destructive edits.

Example:

```text
SessionLinkedToTask
SessionUnlinkedFromTask
SessionLinkedToTask
```

not:

```text
mutate SessionLinkedToTask in place
```

### 4.3 Local-first and offline-capable

The desktop app must work offline. Events are recorded locally and synced/exported only when explicitly configured.

### 4.4 Typed, versioned, and migration-safe

Every event type has a version. Unknown event versions should degrade gracefully rather than corrupting projections.

### 4.5 Privacy-aware

The event log must not become a secret dump.

Event payloads should store references to sensitive artifacts rather than raw content when possible:

```text
transcript_path reference, not full transcript
file path + hash, not full file content
redacted summary, not raw pasted secret
command fingerprint + preview, not secret-bearing args when unsafe
```

### 4.6 Projection-oriented

The UI should read from projections, not constantly recompute state from the raw event log.

Examples:

```text
ActiveSessionsProjection
ApprovalQueueProjection
ProjectGraphProjection
WorktreeStatusProjection
PlanProgressProjection
UsageLedgerProjection
AuditTrailProjection
```

### 4.7 Human action is first-class

Human decisions are events, not footnotes.

Examples:

```text
ApprovalGranted
ApprovalDenied
DecisionSaved
TaskPriorityChanged
PRMergeApproved
CommandEditedBeforeRun
```

### 4.8 Correlation is mandatory

Events in the same workflow must be connected through `correlation_id` and `causation_id`.

Example:

```text
BrainActionPlanProposed
  → ActionRequestCreated
    → ApprovalRequested
      → ApprovalGranted
        → WorktreeCreated
        → SessionStarted
        → PlanTaskLinked
```

All of these share one `correlation_id`.

---

## 5. Command vs action vs event

### Command

A command is an intent.

Examples:

```text
Start a Claude session for Plan Task 2.1
Create a worktree
Run /team-start backend
Create a PR
Refresh owned docs
```

Commands may come from:

- user UI
- keyboard shortcut
- Project Brain
- Workflow Pack recipe
- automation policy
- future iOS companion

### ActionRequest

An ActionRequest is the typed, permissioned representation of a command.

It includes:

- action type
- actor
- target objects
- inputs
- preconditions
- risk level
- preview
- required approval
- executor
- idempotency key

### Event

An event records the fact that something happened.

Examples:

```text
ActionRequested
ActionPreviewGenerated
ApprovalRequested
ApprovalGranted
ActionExecutionStarted
ActionExecutionSucceeded
ActionExecutionFailed
```

---

## 6. Event envelope

Every event should share a common envelope.

```json
{
  "event_id": "evt_01JZ...",
  "event_type": "SessionStarted",
  "event_version": 1,
  "occurred_at": "2026-06-06T16:22:31.123Z",
  "recorded_at": "2026-06-06T16:22:31.151Z",
  "workspace_id": "ws_default",
  "project_id": "proj_agentops",
  "actor": {
    "actor_type": "user",
    "actor_id": "user_cody",
    "display_name": "Cody"
  },
  "source": {
    "source_type": "desktop_ui",
    "source_id": "desktop_main"
  },
  "correlation_id": "corr_start_plan_task_221",
  "causation_id": "evt_previous",
  "idempotency_key": "create-session:proj_agentops:plan_221:claude-main",
  "object_refs": [
    {"object_type": "PlanTask", "object_id": "plan_task_221"},
    {"object_type": "Session", "object_id": "sess_abc"},
    {"object_type": "Worktree", "object_id": "wt_abc"}
  ],
  "sensitivity": "internal",
  "visibility": "project",
  "payload": {},
  "provenance": {
    "schema_version": "event-envelope-v1",
    "app_version": "0.1.0"
  },
  "integrity": {
    "payload_hash": "sha256:...",
    "previous_event_hash": "sha256:..."
  }
}
```

### 6.1 Required envelope fields

| Field | Required | Description |
|---|---:|---|
| `event_id` | yes | Stable unique event ID. |
| `event_type` | yes | PascalCase event type. |
| `event_version` | yes | Integer schema version for the event type. |
| `occurred_at` | yes | When the fact happened. |
| `recorded_at` | yes | When the platform recorded it. |
| `workspace_id` | yes | Workspace boundary. |
| `actor` | yes | Who or what caused it. |
| `source` | yes | Which subsystem emitted it. |
| `correlation_id` | yes | Groups related events into one workflow. |
| `payload` | yes | Type-specific event data. |
| `sensitivity` | yes | Privacy classification. |
| `provenance` | yes | Schema/app/source metadata. |

### 6.2 Optional envelope fields

| Field | Description |
|---|---|
| `project_id` | Project scope if known. |
| `session_id` | Direct session scope if common enough to promote. |
| `agent_team_id` | Direct team scope if common enough to promote. |
| `causation_id` | Immediate prior event that caused this event. |
| `action_request_id` | Associated Action Gateway request. |
| `approval_id` | Associated approval. |
| `workflow_run_id` | Associated Workflow Pack run. |
| `object_refs` | Related objects. |
| `idempotency_key` | Deduplication key for repeated requests. |
| `visibility` | User/project/workspace/system visibility. |
| `integrity` | Hash chain or tamper-evidence metadata. |

---

## 7. Actor model

Actors may include:

```text
user
project_brain
action_gateway
workflow_runtime
local_runner
session_adapter
integration_syncer
system
remote_device
automation_policy
```

### 7.1 Actor examples

```json
{
  "actor_type": "project_brain",
  "actor_id": "brain_local",
  "display_name": "Project Brain"
}
```

```json
{
  "actor_type": "remote_device",
  "actor_id": "ios_cody_iphone",
  "display_name": "Cody's iPhone"
}
```

```json
{
  "actor_type": "session_adapter",
  "actor_id": "claude_code_adapter",
  "display_name": "Claude Code Adapter"
}
```

---

## 8. Source model

`source` describes the subsystem that emitted the event.

Examples:

```text
desktop_ui
local_daemon
action_gateway
terminal_supervisor
claude_code_adapter
codex_adapter
git_executor
github_syncer
linear_syncer
workflow_pack_runtime
project_brain_indexer
project_brain_retriever
project_brain_action_planner
usage_meter
remote_relay
```

Source matters because the audit trail should distinguish:

```text
User clicked button
Project Brain proposed action
Action Gateway approved action
Git executor performed action
GitHub syncer observed remote result
```

---

## 9. Sensitivity model

Events should carry sensitivity classification.

| Level | Meaning | Examples |
|---|---|---|
| `public` | Safe to display/export broadly. | Generic status labels. |
| `internal` | Normal project metadata. | Session started, PR opened. |
| `confidential` | Sensitive project details. | File paths, branch names, ticket details. |
| `secret` | Potential secrets/PII. | Raw command args with tokens, transcript excerpts. |
| `restricted` | Should not sync remotely without explicit consent. | Raw terminal stream, unredacted transcripts, credentials. |

Default should be conservative. For example, terminal output should default to `restricted` unless redacted/summarized.

---

## 10. Event taxonomy

### 10.1 Workspace and app lifecycle events

```text
AppStarted
AppExited
DesktopDaemonStarted
DesktopDaemonStopped
WorkspaceCreated
WorkspaceOpened
WorkspaceClosed
SettingsChanged
PolicyChanged
```

### 10.2 Project and repository events

```text
ProjectAdded
ProjectRemoved
ProjectOpened
ProjectClosed
ProjectIndexed
ProjectIndexUpdated
ProjectIndexFailed
RepositoryLinked
RepositoryUnlinked
RepositoryStatusChanged
ProjectPolicyChanged
ProjectBrainEnabled
ProjectBrainDisabled
```

### 10.3 Workflow Pack events

```text
WorkflowPackInstalled
WorkflowPackDetected
WorkflowPackUpdated
WorkflowPackRemoved
WorkflowInstanceDetected
WorkflowInstancePersonalizationStarted
WorkflowInstancePersonalizationQuestionAsked
WorkflowInstancePersonalizationAnswerRecorded
WorkflowInstancePersonalizationPlanGenerated
WorkflowInstancePersonalizationApproved
WorkflowInstancePersonalized
WorkflowInstanceActivated
WorkflowInstanceDriftDetected
WorkflowInstanceUpgradeAvailable
WorkflowInstanceUpgradeStarted
WorkflowInstanceUpgradeApplied
WorkflowInstanceDetached
```

### 10.4 Workflow command events

```text
WorkflowCommandDiscovered
WorkflowCommandInvoked
WorkflowCommandPreviewGenerated
WorkflowCommandStarted
WorkflowCommandOutputReceived
WorkflowCommandCompleted
WorkflowCommandFailed
```

### 10.5 Implementation plan and task events

```text
ImplementationPlanDetected
ImplementationPlanParsed
ImplementationPlanUpdated
PlanTaskDetected
PlanTaskStatusChanged
PlanTaskLinkedToLinear
PlanTaskLinkedToGitHub
PlanTaskLinkedToSession
PlanTaskLinkedToPullRequest
PlanTaskUnlinked
PlanTaskCreatedFromLinear
LinearIssueCreatedFromPlanTask
```

### 10.6 Integration events

```text
GitHubConnected
GitHubDisconnected
GitHubIssueSynced
GitHubPullRequestSynced
GitHubWebhookReceived
GitHubSyncFailed
LinearConnected
LinearDisconnected
LinearIssueSynced
LinearWebhookReceived
LinearSyncFailed
```

### 10.7 Worktree, branch, and git events

```text
WorktreeCreateRequested
WorktreeCreated
WorktreeCreateFailed
WorktreeStatusChanged
WorktreeDirtyStateChanged
WorktreeDeleted
WorktreePrunable
BranchCreated
BranchCheckedOut
BranchRebased
BranchMerged
CommitCreated
CommitAmended
CommitPushed
MergeConflictDetected
MergeConflictResolved
GitCommandStarted
GitCommandSucceeded
GitCommandFailed
```

### 10.8 Pull request events

```text
PullRequestDrafted
PullRequestOpened
PullRequestUpdated
PullRequestCheckStarted
PullRequestCheckPassed
PullRequestCheckFailed
PullRequestReviewRequested
PullRequestReviewSubmitted
PullRequestChangesRequested
PullRequestApproved
PullRequestMergeabilityChanged
PullRequestMerged
PullRequestClosed
```

### 10.9 Session events

```text
SessionCreateRequested
SessionStarting
SessionStarted
SessionAttachRequested
SessionAttached
SessionDetached
SessionStatusChanged
SessionHeartbeatReceived
SessionMessageSent
SessionMessageReceived
SessionToolCallObserved
SessionCommandObserved
SessionFileEditObserved
SessionWaitingOnPermission
SessionWaitingOnHuman
SessionResumed
SessionPaused
SessionFailed
SessionCompleted
SessionArchived
SessionKilled
SessionSummaryCreated
SessionEpisodeCardCreated
```

### 10.10 Terminal events

```text
TerminalCreated
TerminalAttached
TerminalDetached
TerminalInputSent
TerminalOutputReceived
TerminalScrollbackSnapshotCreated
TerminalProcessExited
TerminalPTYFailed
```

Raw terminal events can be high-volume and sensitive. The platform should not store all terminal output as first-class audit events by default. Instead, store:

- lifecycle events
- important status transitions
- redacted/summarized snippets
- references to local transcript/scrollback files

### 10.11 Agent team events

```text
AgentTeamCreateRequested
AgentTeamStarted
AgentTeamLeadRegistered
AgentTeamWorkerSpawned
AgentTeamWorkerRegistered
AgentTeamBroadcastSent
AgentTeamPlanUpdated
AgentTeamStatusChanged
AgentTeamWorkerCompleted
AgentTeamWaitingOnHuman
AgentTeamMergeStarted
AgentTeamMergeCompleted
AgentTeamCompleted
AgentTeamArchived
AgentTeamFailed
```

### 10.12 Approval and Action Gateway events

```text
ActionPlanProposed
ActionRequestCreated
ActionPreviewGenerated
ApprovalRequested
ApprovalGranted
ApprovalDenied
ApprovalExpired
ApprovalEscalated
ActionExecutionStarted
ActionExecutionSucceeded
ActionExecutionFailed
ActionRolledBack
ActionRollbackFailed
PolicyDecisionRecorded
```

### 10.13 Project Brain events

```text
BrainIndexStarted
BrainIndexCompleted
BrainIndexFailed
BrainSourceDiscovered
BrainSourceIngested
BrainSourceTombstoned
BrainAnchorResolved
BrainAnchorStale
BrainRetrievalRequested
BrainRetrievalCompleted
BrainAnswerGenerated
BrainEvidenceAttached
BrainActionPlanRequested
BrainActionPlanProposed
BrainMemorySaved
BrainDecisionSaved
BrainDocDriftDetected
BrainOwnedDocRefreshRequested
BrainOwnedDocRefreshCompleted
BrainForeignDocStalenessFlagged
```

### 10.14 Code editor and review events

```text
EditorFileOpened
EditorSelectionCreated
EditorDiffOpened
EditorReviewCommentCreated
EditorHunkAccepted
EditorHunkRejected
EditorAgentFixRequested
EditorConflictResolverOpened
EditorConflictResolved
DiagnosticsUpdated
TestOutputCaptured
```

### 10.15 Usage and metering events

```text
TokenUsageObserved
ContextUsageObserved
CostUsageEstimated
CostUsageFinalized
UsageBudgetWarningRaised
UsageBudgetExceeded
ExecutionProfileUsageUpdated
```

Usage events should distinguish exact, estimated, and unavailable metrics.

### 10.16 Security and policy events

```text
SecretDetected
SensitiveOutputRedacted
PolicyViolationDetected
DangerousCommandDetected
CredentialAccessAttempted
RemoteDeviceRegistered
RemoteDeviceRevoked
RemoteActionRequested
RemoteActionDeniedByPolicy
```

---

## 11. Core event examples

### 11.1 SessionStarted

```json
{
  "event_type": "SessionStarted",
  "event_version": 1,
  "project_id": "proj_agentops",
  "session_id": "sess_123",
  "object_refs": [
    {"object_type": "Session", "object_id": "sess_123"},
    {"object_type": "Worktree", "object_id": "wt_123"},
    {"object_type": "PlanTask", "object_id": "plan_task_2_3"}
  ],
  "payload": {
    "session_name": "Claude / Phase 2.3 Auth Callback",
    "harness": "claude_code",
    "model": "claude-opus",
    "execution_profile_id": "exec_claude_max_main",
    "worktree_path": "/Users/cody/dev/project/.worktrees/phase-2-3-auth",
    "branch": "agent/phase-2-3-auth",
    "task_source": "implementation_plan"
  }
}
```

### 11.2 ApprovalRequested

```json
{
  "event_type": "ApprovalRequested",
  "event_version": 1,
  "project_id": "proj_agentops",
  "session_id": "sess_123",
  "action_request_id": "act_456",
  "approval_id": "appr_789",
  "payload": {
    "risk_level": 2,
    "reason": "Agent requested permission to run test suite",
    "requested_action_type": "session.approve_command",
    "preview": {
      "command_display": "npm test -- --runInBand",
      "cwd": "/Users/cody/dev/project/.worktrees/phase-2-3-auth"
    },
    "expires_at": "2026-06-06T17:30:00Z"
  }
}
```

### 11.3 BrainActionPlanProposed

```json
{
  "event_type": "BrainActionPlanProposed",
  "event_version": 1,
  "project_id": "proj_agentops",
  "payload": {
    "user_request": "Start the next backend task.",
    "summary": "Project Brain found Phase 2.3 as the next backend plan task and proposes starting a cc-crew team.",
    "evidence_refs": [
      {"object_type": "PlanTask", "object_id": "plan_task_2_3"},
      {"object_type": "ArchitectureAnchor", "object_id": "arch_auth_flow"}
    ],
    "proposed_actions": [
      "worktree.create",
      "agent_team.start",
      "workflow_command.invoke",
      "plan_task.link_session"
    ],
    "requires_confirmation": true
  }
}
```

### 11.4 PullRequestOpened

```json
{
  "event_type": "PullRequestOpened",
  "event_version": 1,
  "project_id": "proj_agentops",
  "payload": {
    "provider": "github",
    "repo": "org/repo",
    "pr_number": 84,
    "title": "Implement Phase 2.3 auth callback persistence",
    "branch": "agent/phase-2-3-auth",
    "base_branch": "main",
    "session_id": "sess_123",
    "plan_task_id": "plan_task_2_3",
    "url": "provider-managed-link-reference"
  }
}
```

### 11.5 BrainOwnedDocRefreshCompleted

```json
{
  "event_type": "BrainOwnedDocRefreshCompleted",
  "event_version": 1,
  "project_id": "proj_agentops",
  "payload": {
    "source_path": "docs/layers/02-auth.md",
    "producer": "layer-docs",
    "old_content_hash": "sha256:old",
    "new_content_hash": "sha256:new",
    "reason": "Anchor drift detected after commit abc123",
    "reembedded_chunks": 8,
    "unchanged_chunks_reused": 41
  }
}
```

---

## 12. Event storage

### 12.1 Recommended storage shape

For the desktop MVP:

```text
Local SQLite database
  events table
  event_payload JSON column
  projections tables
  outbox table
  schema_migrations table
```

Suggested tables:

```text
events
  event_id primary key
  event_type
  event_version
  occurred_at
  recorded_at
  workspace_id
  project_id nullable
  actor_type
  actor_id
  source_type
  source_id
  correlation_id
  causation_id nullable
  action_request_id nullable
  approval_id nullable
  sensitivity
  visibility
  payload_json
  payload_hash
  previous_event_hash nullable

projection_offsets
  projection_name
  last_event_id
  last_processed_at

outbox
  outbox_id
  destination
  event_id
  payload_json
  status
  retry_count
  next_attempt_at
```

SQLite is a good default because:

- it works locally
- it is durable
- it supports WAL
- it is easy to inspect
- it fits desktop-first architecture
- it can support projections without requiring a server

### 12.2 Event log vs large artifacts

Do not store large artifacts directly in the event log.

Large artifacts should be separate objects referenced from events:

```text
terminal transcript
Claude/Codex JSONL transcript
large diff
full file content
PR patch
test logs
embedding chunks
screen recordings
```

Events should store:

```text
artifact_id
path/reference
content_hash
size
redaction_status
sensitivity
summary
```

---

## 13. Projections

The raw event log is for correctness and audit. The UI reads projections.

### 13.1 Required MVP projections

```text
ProjectActivityProjection
  active/waiting/failed/idle/completed session counts
  active agent teams
  open PRs
  blocked tasks

SessionProjection
  current status
  task links
  worktree/branch
  model/harness/execution profile
  last heartbeat
  token/context usage
  pending approvals

ApprovalQueueProjection
  approvals sorted by risk and age
  requester
  affected session/team/project
  preview summary

WorktreeProjection
  branch
  dirty state
  owner session/team
  linked task
  PR status

PlanProgressProjection
  phase/task status
  linked sessions
  linked PRs
  linked tickets

ProjectGraphProjection
  nodes and edges for project observability

AuditTrailProjection
  human-readable ordered timeline of important events

UsageLedgerProjection
  tokens/context/cost by session, project, model, execution profile
```

### 13.2 Projection rules

- Projections must be rebuildable from the event log.
- Projections must track their last processed event.
- Projection rebuild should be safe after app crash.
- Projection corruption should not corrupt raw events.
- Projection updates should be incremental.

---

## 14. Audit trail requirements

### 14.1 What must be auditable

The following must always create audit events:

```text
creating/killing/pausing/resuming sessions
sending messages to sessions
creating/deleting worktrees
running git mutations
creating/updating/merging PRs
linking/unlinking tasks
creating/updating Linear/GitHub issues
running workflow-pack personalization
running workflow commands
running /team-start
Project Brain action plans
approval requests and decisions
doc refresh actions
policy changes
execution profile changes
remote-device registration/revocation
```

### 14.2 Audit event display

The UI should provide audit views at multiple scopes:

```text
Workspace audit log
Project timeline
Session timeline
Agent team timeline
Task timeline
Worktree timeline
PR timeline
Action request detail
Approval history
```

### 14.3 Audit entry UX

A human-readable audit entry should show:

```text
What happened
Who/what caused it
When it happened
What object it affected
What evidence was used
Whether it was approved
What changed
Whether it succeeded or failed
Links to related objects
```

Example:

```text
11:42 AM · Project Brain proposed starting Phase 2.3 backend team.
Evidence: MVP_TASKS.md Phase 2.3, ARCHITECTURE.md §Auth Flow.
Approved by Cody.
Actions executed: created worktree, started /team-start backend, linked team to plan task.
```

---

## 15. Relationship to Project Brain

Project Brain should consume events but not own the event store.

### 15.1 Project Brain consumes

```text
SessionStarted
SessionCompleted
SessionSummaryCreated
SessionEpisodeCardCreated
CommitCreated
PullRequestOpened
PullRequestMerged
PlanTaskStatusChanged
PlanTaskLinkedToSession
WorkflowInstancePersonalized
WorkflowCommandInvoked
DecisionSaved
ApprovalGranted
ApprovalDenied
BrainOwnedDocRefreshCompleted
```

### 15.2 Project Brain emits or requests

Project Brain may emit low-risk informational events:

```text
BrainRetrievalRequested
BrainAnswerGenerated
BrainEvidenceAttached
BrainActionPlanProposed
BrainMemorySaved
BrainDecisionSaved
```

For mutating operations, Project Brain should request actions through the Action Gateway, which then emits the authoritative action/approval/execution events.

### 15.3 Evidence chips and event references

Project Brain answers should be able to cite events as evidence:

```text
Session sess_123 started on June 6, 2026
Commit abc123 was created by session sess_123
PR #84 was opened from branch agent/phase-2-3-auth
Plan task Phase 2.3 was linked to PR #84
```

This requires stable event references and object IDs.

---

## 16. Relationship to Action Gateway

The Action Gateway uses events for:

```text
requested actions
previews
risk classifications
policy decisions
approval requests
approval decisions
execution start
execution result
rollback attempts
failures
```

Canonical flow:

```text
ActionPlanProposed
ActionRequestCreated
ActionPreviewGenerated
PolicyDecisionRecorded
ApprovalRequested
ApprovalGranted / ApprovalDenied
ActionExecutionStarted
ActionExecutionSucceeded / ActionExecutionFailed
ActionRolledBack / ActionRollbackFailed
```

The Action Gateway should be the only subsystem that emits authoritative `ActionExecution*` events.

---

## 17. Relationship to Workflow Packs

Workflow Packs need events to become observable.

### 17.1 Workflow Pack lifecycle

```text
WorkflowPackInstalled
WorkflowPackDetected
WorkflowInstanceDetected
WorkflowInstancePersonalizationStarted
WorkflowInstancePersonalized
WorkflowInstanceActivated
WorkflowInstanceDriftDetected
WorkflowInstanceUpgradeAvailable
WorkflowInstanceUpgradeApplied
```

### 17.2 cc-crew-style team events

For a cc-crew-style team run:

```text
WorkflowCommandInvoked /team-start backend
AgentTeamStarted
AgentTeamLeadRegistered
AgentTeamWorkerSpawned orchestrator
AgentTeamWorkerSpawned implementer
SessionStarted lead
SessionStarted orchestrator
SessionStarted implementer
PlanTaskLinkedToSession
SessionWaitingOnHuman
AgentTeamCompleted
```

### 17.3 TDD slice progress

Workflow Packs may define custom event subtypes or payloads for internal progress.

Example:

```text
WorkflowSliceProgressed
  workflow_pack_id: cc-crew
  command: /tdd
  step: RED
  status: completed
```

The base event model should allow Workflow Packs to extend payloads without creating platform-level schema churn.

---

## 18. Relationship to Execution Profiles

Execution Profiles appear in events whenever a session or agent team uses a specific local account/runtime context.

Events should capture:

```text
execution_profile_id
provider
harness
model
account_alias
usage_policy
```

Relevant events:

```text
ExecutionProfileCreated
ExecutionProfileUpdated
ExecutionProfileSelected
ExecutionProfileUsageUpdated
ExecutionProfileLimitWarning
SessionStarted
AgentTeamWorkerSpawned
TokenUsageObserved
CostUsageEstimated
```

Do not store secrets or credentials in event payloads.

---

## 19. iOS companion architecture options

The iOS companion is not MVP, but the event model should not block it.

### Option A: End-to-end encrypted relay

Desktop app publishes selected redacted projections to a relay. iOS reads those projections and sends action requests back through the relay.

Pros:

- works outside the home network
- good UX
- push notifications possible

Cons:

- requires cloud relay infrastructure
- requires device registration and revocation
- requires encrypted sync protocol
- must be very careful with secrets

### Option B: User-owned secure tunnel

iOS connects to desktop through Tailscale, WireGuard, or another user-controlled tunnel.

Pros:

- less platform cloud responsibility
- strong privacy posture
- can be power-user friendly

Cons:

- harder setup
- less consumer-friendly
- push notifications are harder

### Option C: Notification-only bridge

Desktop sends push notifications for waiting approvals, but actions must be completed on desktop.

Pros:

- much safer
- easier first stretch goal
- less remote-control risk

Cons:

- not true remote control
- user still needs desktop access

### Recommended sequencing

```text
P2a: notification-only companion
P2b: read-only observability companion
P2c: low/medium-risk approval companion
P3: controlled remote actions through Action Gateway
```

The phone should never directly run commands. It should submit remote `ActionRequest`s to the desktop-owned Action Gateway.

---

## 20. Remote event additions for iOS stretch

```text
RemoteDevicePairingStarted
RemoteDevicePaired
RemoteDeviceRevoked
RemoteProjectionPublished
RemoteProjectionSyncFailed
RemoteNotificationSent
RemoteNotificationOpened
RemoteActionRequested
RemoteActionPreviewGenerated
RemoteApprovalRequested
RemoteApprovalGranted
RemoteApprovalDenied
RemoteActionDeniedByPolicy
RemoteSessionMessageRequested
RemoteSessionMessageSent
```

Remote events require extra metadata:

```text
device_id
device_name
device_public_key
remote_capability_scope
network_path
relay_id nullable
projection_version
redaction_policy_id
```

---

## 21. Event retention and export

### 21.1 MVP retention

For MVP:

- keep audit-critical events indefinitely unless user deletes the project/workspace
- keep high-volume terminal-output references with configurable retention
- keep raw transcript references only under project policy
- allow Project Brain episode cards to survive after raw transcript retention expires, if policy allows

### 21.2 Export

The platform should eventually export event history as:

```text
JSONL
SQLite snapshot
Markdown audit report
Project Brain ingest bundle
```

### 21.3 Deletion

Local-first deletion should be explicit and scoped:

```text
delete project from platform registry
delete event history for project
delete transcript references
delete Project Brain index
delete generated projections
delete local worktrees
```

Deleting audit events may break provenance. The UI should warn when deletion removes the ability to answer historical questions.

---

## 22. Event schema versioning

Every event type has a version.

Rules:

- Additive fields are minor-compatible.
- Removing or changing field meaning requires a new event version.
- Projections must handle unknown versions by skipping with a visible degraded-state event.
- Migration tools can re-project but should not mutate raw historical event records.

Events may include both:

```text
event_version
payload_schema_uri
```

---

## 23. Failure modes

| Failure | Required behavior |
|---|---|
| Event write fails | Action should fail closed if audit is required. |
| Projection update fails | Raw event remains; projection marks degraded and can rebuild. |
| Duplicate event observed | Use event ID/idempotency key to dedupe. |
| Unknown event type | Store raw event, flag unsupported, do not crash. |
| Corrupt payload | Quarantine event, flag audit integrity warning. |
| Sensitive payload detected | Redact, downgrade sync/export eligibility, emit `SensitiveOutputRedacted`. |
| Remote sync fails | Keep local truth; remote projections become stale visibly. |
| Clock skew | Use both `occurred_at` and `recorded_at`; show uncertainty if needed. |

---

## 24. MVP scope

MVP should include:

```text
Local append-only event store
Common event envelope
Event type registry
Core session events
Core worktree/git events
Core approval/action events
Core Project Brain indexing/action-plan events
Core plan/task linking events
Core PR events
Core usage events
Project timeline projection
Session projection
Approval queue projection
Project graph projection
Audit trail view
Event export as JSONL for debugging
```

MVP should not include:

```text
iOS companion
cloud relay
team sync across machines
remote command execution
full terminal event persistence
complex event-sourced replay UI
cryptographic tamper-evident hash chain as a hard requirement
```

---

## 25. P1 scope

P1 should add:

```text
Workflow Pack event extensions
cc-crew team/TDD progress events
Project Brain event-backed provenance answers
PR review/check event integration
Linear/GitHub webhook event mapping
decision log events
projection rebuild UI
more detailed usage ledger
policy-driven event retention
```

---

## 26. P2 / stretch scope

P2 should add:

```text
iOS read-only observability companion
remote notification stream
remote approval flow for low/medium-risk actions
encrypted projection sync
device registration/revocation
signed event export/import
cross-device event redaction policies
```

P3 or later:

```text
full remote control
remote terminal streaming
multi-user shared event log
team-level event sync
enterprise audit bundles
```

---

## 27. Open questions

```text
What storage engine should be canonical for the event store: SQLite only, or SQLite + append-only JSONL mirror?
Should the raw event log be hash-chained from MVP, or only later?
How much terminal output should be retained by default?
Which events should Project Brain index immediately vs summarize first?
Should Project Brain episode cards be emitted by Project Brain or by the session-history ingestor?
What event payload fields are allowed to sync to a future iOS app?
What remote action risk level is allowed from iOS?
Should iOS approvals require biometric confirmation?
Should remote approval require desktop unlock / presence signal for high-risk actions?
How should event deletion interact with Project Brain's historical answers?
```

---

## 28. Decisions captured in this spec

```text
D-001: The platform is a desktop app, not a hosted web app.
D-002: The event store is local-first and should work offline.
D-003: iOS companion is a stretch goal for remote observability/control, not MVP.
D-004: Remote/iOS control must go through the Action Gateway, never directly to terminals/git.
D-005: Events are facts; commands and action requests are not events until recorded as facts.
D-006: Project Brain consumes events and may propose action plans, but mutating actions are executed by the platform.
D-007: Terminal output and transcripts are high-sensitivity; store references/summaries by default, not raw output as normal audit events.
D-008: UI state should be driven by projections over events.
```

---

## 29. Next artifact dependencies

This spec feeds directly into:

```text
UX / Information Architecture Spec
Workflow Packs Spec
cc-crew Workflow Pack Integration Spec
Security, Permissions, and Safety Spec
Desktop Runtime Architecture Spec
Project Brain Event Ingestion Spec
Main Platform PRD
```

The most logical next artifact is **Workflow Packs Spec**, because Workflow Packs define many of the event types that will make custom scaffolds, command registries, personalization runs, and agent-team recipes observable.
