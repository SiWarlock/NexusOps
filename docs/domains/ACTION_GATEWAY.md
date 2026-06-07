# Action Gateway Spec v0.1

> **Status:** Draft planning artifact  
> **Scope:** Permissioned action layer for the AI engineering control plane and Project Brain  
> **Depends on:** Product Canon v0.1, Shared Object Model v0.1, Project Brain PRD v2, Workflow Packs Spec (future), Event Model Spec (future)  
> **Naming note:** Parent platform name is still open. This document uses neutral terms: **platform**, **Project Brain**, **Workflow Packs**, and **Action Gateway**.

---

## 1. Executive summary

The **Action Gateway** is the platform layer that lets Project Brain, the UI, workflow packs, and future automations request real work without bypassing user control.

It is the boundary between:

```text
Reasoning / planning / recommendations
```

and:

```text
Mutation / execution / side effects
```

Project Brain may understand the project, propose work, draft changes, and produce action plans. The platform owns execution. The Action Gateway sits between them and enforces:

```text
Typed action schemas
Risk classification
Precondition checks
Preview / dry-run requirements
User confirmation
Policy enforcement
Execution routing
Idempotency
Audit logging
Rollback where possible
Event emission
```

The gateway is what lets Project Brain evolve from a passive second brain into an active project co-pilot without silently mutating files, tickets, git state, sessions, workflow scaffolding, or credentials.

---

## 2. Product definition

### 2.1 What the Action Gateway is

The Action Gateway is a permissioned action orchestration layer.

It receives an `ActionRequest` or `ActionPlan`, validates it, classifies risk, generates a preview, requests approval if required, executes through the correct executor adapter, and emits events for audit and Project Brain memory.

### 2.2 What it is not

The Action Gateway is **not**:

```text
A chatbot
A general-purpose shell executor
A background automation engine with invisible side effects
A replacement for git, GitHub, Linear, Claude Code, or Codex
A place where Project Brain directly receives credentials
A way to bypass user permissions
A free-form "run whatever the model says" surface
```

### 2.3 Core thesis

> Project Brain can decide what should happen, but the Action Gateway decides whether it may happen, how it is previewed, who must approve it, which executor runs it, and how it is audited.

---

## 3. Why this exists

The platform is designed to coordinate many AI coding agents across projects, sessions, worktrees, plans, tickets, pull requests, and workflow packs. Project Brain eventually needs to do more than answer questions. It should be able to help with requests like:

```text
Start the next backend task.
Create a Linear issue from this plan task.
Launch /team-start for Phase 2 Track A.
Ask the active agent to explain this diff.
Create a worktree and start a Claude Code session.
Summarize this completed session and link it to the PR.
Generate a PR description from the task, commits, and session history.
Refresh stale owned docs and re-index them.
```

Without an Action Gateway, there are only two bad options:

```text
Option A: Project Brain is passive and cannot act.
Option B: Project Brain acts directly and becomes unsafe.
```

The gateway creates the third path:

```text
Project Brain plans. The gateway previews, gates, executes, and audits.
```

---

## 4. Design principles

### 4.1 No invisible mutation

No file, branch, worktree, ticket, PR, workflow file, global config, or agent session should be mutated invisibly. Every meaningful side effect must become an action with a record.

### 4.2 Typed actions only

The gateway must not execute arbitrary natural language. Natural language must be converted into typed `ActionRequest` or `ActionPlan` objects before execution.

### 4.3 Planning is separate from execution

Project Brain may propose actions. The gateway owns execution.

### 4.4 Preview before mutation

Any action that changes state must provide a preview or explain why preview is impossible.

### 4.5 Human control scales by risk

Low-risk actions can be fast. Medium-risk actions usually need confirmation. High-risk and critical actions need explicit, specific approval.

### 4.6 Policy before preference

Project, workspace, execution profile, integration, and security policies override Project Brain suggestions and user convenience.

### 4.7 Least privilege

An action should receive only the credentials, filesystem access, execution profile, integration scopes, and session handles required to perform that action.

### 4.8 Idempotency by default

Repeated requests should converge or no-op instead of duplicating sessions, comments, issues, branches, or workflow modifications.

### 4.9 Audit everything

Every requested, denied, approved, executed, failed, rolled back, or expired action should be represented in the event stream.

### 4.10 Evidence-linked actions

Actions proposed by Project Brain should include evidence: plan task, architecture anchor, session, code selection, PR check, ticket, command, or project memory that caused the recommendation.

### 4.11 Reversibility where possible

The gateway should expose rollback/undo paths when feasible. If rollback is not possible, the UI should say so before execution.

---

## 5. Actors and clients

### 5.1 Human user

The person who approves, edits, denies, or configures action policies.

### 5.2 Platform UI

The app surfaces action previews, approval cards, confirmation flows, queue state, progress, results, and audit history.

### 5.3 Project Brain

Project Brain is an action planner and evidence engine. It can request actions but should not directly perform privileged operations.

### 5.4 Workflow Pack Runtime

Workflow packs define commands, launch recipes, personalization flows, upgrade flows, plan parsers, and expected artifacts. They may request platform actions through the gateway.

### 5.5 Agent Session Adapters

Claude Code, Codex CLI, future SDKs, and other harnesses expose session actions such as start, resume, attach terminal, send message, pause, kill, and summarize.

### 5.6 Executor adapters

Executor adapters are the concrete implementations behind action types:

```text
Filesystem executor
Git executor
GitHub executor
Linear executor
Terminal/session executor
Workflow Pack executor
Project Brain index executor
Code editor/diff executor
Usage/cost executor
Notification executor
```

### 5.7 Policy engine

The policy engine decides whether an action is allowed, requires approval, requires a specific approver, must be blocked, or must be downgraded.

---

## 6. Capability modes

The Action Gateway should support five capability modes.

### 6.1 Read-only mode

No mutation. No confirmation required unless the data source is sensitive.

Examples:

```text
Open file
Open diff
Search project
Find related sessions
List worktrees
List plan tasks
List PR checks
Show workflow status
Ask Project Brain
```

### 6.2 Draft mode

Generate proposed content without committing it to an external system.

Examples:

```text
Draft PR description
Draft Linear issue
Draft task prompt
Draft review comment
Draft plan update
Draft session summary
Draft workflow personalization answers
```

### 6.3 Confirmed action mode

Single state-changing action with explicit user confirmation.

Examples:

```text
Create worktree
Create session
Link plan task to Linear issue
Send message to active agent
Create GitHub PR draft
Update a ticket status
Run Project Brain sync
```

### 6.4 Bundled workflow mode

A multi-step action plan approved as a workflow.

Examples:

```text
Start next backend task
Fix failing PR checks
Start /team-start for selected track
Personalize workflow pack for project
Create Linear issue + worktree + session + link everything
Review agent changes + create PR draft
```

### 6.5 Policy automation mode

Standing permission for a narrow class of actions under explicit constraints.

Examples:

```text
Auto-summarize completed sessions
Auto-link branch names to plan tasks
Auto-refresh Project Brain index after commit
Auto-create draft PR when tests pass and session completes
Auto-comment progress to Linear for low-risk status updates
```

Policy automation must be opt-in, narrow, revocable, and auditable.

---

## 7. Risk model

Risk level controls preview depth, confirmation requirements, allowed automation, and audit severity.

| Level | Name | Meaning | Default confirmation |
|---:|---|---|---|
| 0 | Read-only | No mutation and no sensitive data exfiltration | None |
| 1 | Low | Local UI or metadata change; easily reversible | Usually none or lightweight |
| 2 | Medium | Creates local state or sends bounded messages | Confirm unless policy allows |
| 3 | High | Changes git, files, tickets, PRs, workflow files, or runs commands | Explicit confirmation |
| 4 | Critical | Destructive, security-sensitive, irreversible, or global configuration | Explicit step-level confirmation; no broad automation |

### 7.1 Level 0 — Read-only

Examples:

```text
Open file
Open diff
Search memory
List sessions
Show PR checks
Show workflow manifest
Preview command definition
```

### 7.2 Level 1 — Low risk

Examples:

```text
Save local UI preference
Pin context chip
Create draft text in composer
Add local note
Open terminal attach view
Mark notification read
```

### 7.3 Level 2 — Medium risk

Examples:

```text
Create worktree
Create branch
Create agent session
Send instruction to an agent
Link task to session
Link plan task to Linear issue
Create draft GitHub PR
Create draft Linear issue
Run Project Brain sync
Summarize session into Project Brain memory
```

### 7.4 Level 3 — High risk

Examples:

```text
Run shell command
Edit files
Apply patch
Commit changes
Push branch
Create non-draft PR
Update Linear issue status
Update MVP_TASKS.md
Run workflow personalization that writes files
Run workflow upgrade
Refresh owned docs with layer-docs --update
```

### 7.5 Level 4 — Critical risk

Examples:

```text
Merge PR
Force push
Delete branch
Delete worktree with uncommitted changes
Change global Claude/Codex configuration
Modify credentials or secrets
Send raw transcript chunks to cloud model
Disable safety policy
Run arbitrary command outside project root
Change execution profile credentials
```

Critical actions should not be included in broad “approve all” workflows by default.

---

## 8. Action lifecycle

Every action moves through an explicit state machine.

```text
Drafted
  → Submitted
  → Normalized
  → Validated
  → RiskClassified
  → Previewed
  → AwaitingApproval
  → Approved / Denied / Expired / Cancelled
  → Queued
  → Executing
  → Succeeded / Failed / PartiallySucceeded
  → RolledBack / RollbackFailed / Archived
```

### 8.1 Drafted

A client has proposed an action, but it is not yet formally submitted.

### 8.2 Submitted

The gateway has received an action request.

### 8.3 Normalized

Natural-language or shorthand input has been converted into the canonical schema.

### 8.4 Validated

Required fields, resource IDs, permissions, and preconditions have been checked.

### 8.5 RiskClassified

The risk engine assigned a risk level and required confirmation mode.

### 8.6 Previewed

The gateway generated a preview or dry-run output.

### 8.7 AwaitingApproval

The action cannot continue until the user approves, edits, or denies it.

### 8.8 Approved / Denied / Expired / Cancelled

The user or policy resolved the approval.

### 8.9 Queued

The action is waiting for locks, resources, rate limits, or executor capacity.

### 8.10 Executing

The executor is performing the action.

### 8.11 Succeeded / Failed / PartiallySucceeded

Execution finished. If partial, the result must identify which steps succeeded and which did not.

### 8.12 RolledBack / RollbackFailed

If rollback was available and invoked, the result records the rollback outcome.

---

## 9. Core data model

### 9.1 ActionRequest

```ts
type ActionRequest = {
  id: string;
  idempotencyKey: string;
  createdAt: string;
  requestedBy: ActorRef;
  source: ActionSource;

  projectId?: string;
  workspaceId?: string;
  sessionId?: string;
  agentTeamId?: string;
  workflowInstanceId?: string;

  actionType: ActionType;
  intent: string;
  params: Record<string, unknown>;

  resourceRefs: ResourceRef[];
  evidenceRefs: EvidenceRef[];
  preconditions: Precondition[];

  requestedExecutionProfileId?: string;
  policyContext: PolicyContext;

  dryRunRequired: boolean;
  confirmationPreference: ConfirmationPreference;
  expiresAt?: string;
};
```

### 9.2 ActionPlan

An action plan is a sequence or graph of action requests.

```ts
type ActionPlan = {
  id: string;
  title: string;
  summary: string;
  requestedBy: ActorRef;
  projectId?: string;

  steps: ActionPlanStep[];
  dependencies: ActionDependency[];
  rollbackPlan?: RollbackPlan;

  overallRiskLevel: RiskLevel;
  approvalMode: 'approve_all' | 'step_by_step' | 'mixed' | 'blocked';
  evidenceRefs: EvidenceRef[];
  createdAt: string;
};
```

### 9.3 ActionPlanStep

```ts
type ActionPlanStep = {
  stepId: string;
  label: string;
  actionRequest: ActionRequest;
  required: boolean;
  canSkip: boolean;
  rollbackActionType?: ActionType;
  status: ActionState;
};
```

### 9.4 ActionPreview

```ts
type ActionPreview = {
  actionRequestId: string;
  generatedAt: string;
  riskLevel: RiskLevel;
  riskReasons: string[];
  summary: string;
  changedResources: ResourceRef[];
  commandPreview?: CommandPreview;
  diffPreview?: DiffPreview;
  apiPayloadPreview?: ApiPayloadPreview;
  sessionPreview?: SessionPreview;
  workflowPreview?: WorkflowPreview;
  rollbackPreview?: RollbackPreview;
  cannotPreviewReason?: string;
};
```

### 9.5 Approval

```ts
type Approval = {
  id: string;
  actionRequestId?: string;
  actionPlanId?: string;
  requiredApprover: ActorRef | 'current_user' | 'project_owner';
  status: 'requested' | 'approved' | 'denied' | 'expired' | 'cancelled';
  riskLevel: RiskLevel;
  message: string;
  approvedAt?: string;
  deniedAt?: string;
  expiresAt?: string;
  approvalScope: 'single_action' | 'plan' | 'policy_grant';
  constraints?: ApprovalConstraint[];
};
```

### 9.6 ActionResult

```ts
type ActionResult = {
  actionRequestId: string;
  status: 'succeeded' | 'failed' | 'partially_succeeded' | 'cancelled';
  startedAt: string;
  finishedAt: string;
  executor: string;
  outputSummary: string;
  createdResources: ResourceRef[];
  changedResources: ResourceRef[];
  emittedEvents: string[];
  error?: ActionError;
  rollbackAvailable: boolean;
  rollbackActionRequestId?: string;
};
```

### 9.7 ActorRef

```ts
type ActorRef =
  | { type: 'user'; userId: string }
  | { type: 'project_brain'; brainId: string }
  | { type: 'workflow_pack'; workflowPackId: string }
  | { type: 'agent_session'; sessionId: string }
  | { type: 'system_policy'; policyId: string };
```

### 9.8 ResourceRef

```ts
type ResourceRef = {
  type:
    | 'project'
    | 'repo'
    | 'worktree'
    | 'branch'
    | 'file'
    | 'diff'
    | 'session'
    | 'agent_team'
    | 'plan_task'
    | 'implementation_plan'
    | 'workflow_pack'
    | 'workflow_instance'
    | 'workflow_command'
    | 'linear_issue'
    | 'github_issue'
    | 'pull_request'
    | 'execution_profile'
    | 'memory_source'
    | 'decision'
    | 'artifact';
  id: string;
  displayName?: string;
  uri?: string;
};
```

### 9.9 EvidenceRef

```ts
type EvidenceRef = {
  type:
    | 'project_brain_evidence_item'
    | 'file_anchor'
    | 'architecture_anchor'
    | 'plan_task'
    | 'session_episode'
    | 'commit'
    | 'pr_check'
    | 'terminal_event'
    | 'ticket'
    | 'workflow_manifest'
    | 'user_instruction';
  id: string;
  label: string;
  confidence?: 'exact' | 'likely' | 'loose' | 'unverified';
};
```

---

## 10. Action taxonomy

The platform should use namespaced action types.

```text
project.*
brain.*
workflow.*
plan.*
task.*
session.*
team.*
git.*
github.*
linear.*
code.*
review.*
usage.*
settings.*
```

### 10.1 Project actions

| Action | Risk | Description |
|---|---:|---|
| `project.open` | 0 | Open project in UI |
| `project.rescan` | 1 | Re-detect repo/workflow/plan metadata |
| `project.add` | 2 | Register project with the platform |
| `project.archive` | 3 | Archive project from active workspace |
| `project.remove` | 4 | Remove project registration and derived caches |

### 10.2 Project Brain actions

| Action | Risk | Description |
|---|---:|---|
| `brain.ask` | 0 | Ask Project Brain a question |
| `brain.find_related` | 0 | Retrieve related code/docs/sessions/tasks |
| `brain.sync` | 2 | Re-index project deltas |
| `brain.summarize_session` | 2 | Create/update episode card for a session |
| `brain.save_decision` | 2 | Save decision with evidence |
| `brain.refresh_owned_docs` | 3 | Run owned-doc refresh loop and re-index |
| `brain.send_raw_transcript_to_cloud` | 4 | Send raw transcript to cloud model; should require explicit per-action consent |

### 10.3 Workflow Pack actions

| Action | Risk | Description |
|---|---:|---|
| `workflow.detect` | 0 | Detect workflow pack/instance state |
| `workflow.view_manifest` | 0 | Open workflow manifest |
| `workflow.install_pack` | 2 | Install workflow pack locally |
| `workflow.personalize.preview` | 1 | Preview personalization plan |
| `workflow.personalize.run` | 3 | Run personalization that writes files |
| `workflow.upgrade.preview` | 1 | Preview upgrade plan |
| `workflow.upgrade.run` | 3 | Apply workflow upgrade |
| `workflow.command.invoke` | 2-3 | Invoke workflow command, risk depends on command |

### 10.4 Plan actions

| Action | Risk | Description |
|---|---:|---|
| `plan.open` | 0 | Open implementation plan |
| `plan.link_task` | 2 | Link plan task to session/ticket/PR |
| `plan.update_status` | 2-3 | Update plan task state |
| `plan.create_task_from_ticket` | 3 | Mutate implementation plan from ticket |
| `plan.create_ticket_from_task` | 2-3 | Create external ticket from plan task |

### 10.5 Session actions

| Action | Risk | Description |
|---|---:|---|
| `session.create` | 2 | Create a new agent session |
| `session.attach_terminal` | 0 | Attach to existing terminal |
| `session.send_message` | 2 | Send instruction to agent |
| `session.pause` | 2 | Pause a session |
| `session.resume` | 2 | Resume a session |
| `session.kill` | 3 | Kill a running session |
| `session.archive` | 2 | Archive completed session |
| `session.approve_permission` | 2-4 | Approve agent permission request; risk depends on underlying command |

### 10.6 Agent team actions

| Action | Risk | Description |
|---|---:|---|
| `team.create` | 2 | Create team container |
| `team.start` | 3 | Start lead/orchestrator/workers |
| `team.broadcast` | 2 | Send message to all team sessions |
| `team.pause_all` | 2 | Pause team sessions |
| `team.kill_all` | 3 | Kill team sessions |
| `team.ask_lead_status` | 1 | Ask team lead for status |
| `team.merge_outputs` | 3-4 | Combine team work outputs |

### 10.7 Git/worktree actions

| Action | Risk | Description |
|---|---:|---|
| `git.status` | 0 | Read status |
| `git.diff` | 0 | Read diff |
| `git.create_worktree` | 2 | Create worktree and branch |
| `git.delete_worktree` | 3-4 | Delete worktree; critical if dirty |
| `git.create_branch` | 2 | Create branch |
| `git.checkout` | 2-3 | Checkout branch/worktree |
| `git.commit` | 3 | Commit changes |
| `git.push` | 3 | Push branch |
| `git.rebase` | 3 | Rebase branch |
| `git.merge_main` | 3 | Merge main into branch |
| `git.force_push` | 4 | Force push |

### 10.8 GitHub actions

| Action | Risk | Description |
|---|---:|---|
| `github.list_issues` | 0 | List issues |
| `github.link_issue` | 2 | Link issue to platform object |
| `github.create_issue` | 2-3 | Create issue |
| `github.comment_issue` | 2 | Comment on issue |
| `github.create_pr_draft` | 2 | Create draft PR |
| `github.create_pr` | 3 | Create non-draft PR |
| `github.comment_pr` | 2 | Comment on PR |
| `github.request_review` | 2 | Request review |
| `github.merge_pr` | 4 | Merge PR |

### 10.9 Linear actions

| Action | Risk | Description |
|---|---:|---|
| `linear.list_issues` | 0 | List issues |
| `linear.link_issue` | 2 | Link issue to plan/session |
| `linear.create_issue` | 2-3 | Create issue from plan/task |
| `linear.update_issue` | 3 | Update title/body/status/assignee |
| `linear.comment_issue` | 2 | Post comment |
| `linear.sync_from_plan` | 3 | One-way create/update from implementation plan |
| `linear.bidirectional_sync` | 4 | Controlled bidirectional sync; future only |

### 10.10 Code/review actions

| Action | Risk | Description |
|---|---:|---|
| `code.open_file` | 0 | Open file in editor |
| `code.open_selection` | 0 | Open selection |
| `code.apply_patch` | 3 | Apply patch to files |
| `code.accept_hunk` | 3 | Accept diff hunk |
| `code.reject_hunk` | 3 | Reject diff hunk |
| `review.request_agent_fix` | 2 | Ask agent to fix selected code/diff |
| `review.request_agent_tests` | 2 | Ask agent to add tests |
| `review.create_comment` | 2 | Draft/create review comment |

### 10.11 Settings/security actions

| Action | Risk | Description |
|---|---:|---|
| `settings.view` | 0 | View settings |
| `settings.update_project_policy` | 3 | Update project policy |
| `settings.register_mcp` | 3 | Modify host MCP config |
| `settings.unregister_mcp` | 3 | Modify host MCP config |
| `settings.update_execution_profile` | 3-4 | Modify runtime/account profile |
| `settings.disable_safety_policy` | 4 | Disable safety controls |

---

## 11. Permission model

### 11.1 Permission scopes

```text
Workspace scope
Project scope
Repository scope
Worktree scope
Session scope
Agent team scope
Workflow instance scope
Integration account scope
Execution profile scope
Action type scope
Time-limited policy scope
```

### 11.2 Approval scopes

An approval can apply to:

```text
A single action
A specific step in a plan
A complete bundled workflow
A specific action type for a single session
A specific workflow recipe for a single project
A time-limited automation policy
```

### 11.3 Approval constraints

Approvals should be constrained. Examples:

```text
Only for project X
Only for session Y
Only inside worktree Z
Only for commands matching npm test / pnpm test / pytest
Only for branches matching agent/*
Only for draft PRs
Only for Linear comments, not status changes
Only for the next 30 minutes
Only up to N actions
Only if no uncommitted human changes exist
```

### 11.4 Standing grants

Standing grants should be narrow and visible.

Example:

```text
Allow Project Brain to auto-summarize completed sessions for Project A.
Allow the platform to auto-link branches named agent/<ticket-id> to matching Linear issues.
Allow Claude Code sessions in Project B to run npm test without approval.
```

Standing grants should be revocable from settings and visible in the audit log.

---

## 12. Policy engine

The policy engine evaluates every action before execution.

### 12.1 Policy inputs

```text
Action type
Risk level
Requested actor
Project policy
Workspace policy
Execution profile policy
Integration permissions
Workflow instance state
Session state
Worktree state
Branch protection
PR state
File sensitivity
Transcript privacy settings
User approval grants
```

### 12.2 Policy result

```ts
type PolicyDecision = {
  status: 'allow' | 'require_approval' | 'require_step_approval' | 'deny' | 'downgrade' | 'needs_more_context';
  reasons: string[];
  requiredApprovals: ApprovalRequirement[];
  constraints: ApprovalConstraint[];
  suggestedSaferAlternative?: ActionRequest | ActionPlan;
};
```

### 12.3 Examples

```text
Project Brain requests git.merge_pr into main.
Policy: require critical approval; cannot be included in approve-all.

Workflow Pack requests workflow.personalize.run.
Policy: allow preview; require explicit approval before writing files.

Agent requests npm test.
Policy: allow if project grant allows test commands in this worktree.

Agent requests rm -rf /.
Policy: deny.

Project Brain requests raw transcript summarization through cloud model.
Policy: deny unless project transcript ingestion is enabled and explicit per-action consent is granted.
```

---

## 13. Preview and dry-run requirements

### 13.1 Preview classes

| Preview class | Required for | Output |
|---|---|---|
| Read preview | read-only actions | data summary |
| Command preview | shell/session actions | command, cwd, env summary, profile, risk |
| Diff preview | file modifications | unified/side-by-side diff |
| Git preview | git actions | status, branch, worktree, changed files, conflicts |
| API payload preview | GitHub/Linear actions | method, resource, changed fields |
| Workflow preview | workflow pack actions | command, inputs, generated files, diff, spawned sessions |
| Session preview | session actions | harness, execution profile, worktree, prompt |
| Rollback preview | reversible actions | rollback steps and limitations |

### 13.2 Preview failures

If a preview cannot be generated, the gateway must explain why and upgrade the action risk if necessary.

Examples:

```text
Cannot preview arbitrary shell command effects.
Cannot preview external API side effects beyond payload.
Cannot preview agent behavior after receiving a message.
Cannot guarantee rollback after git push or PR merge.
```

### 13.3 Dry-run semantics

Where supported, a dry-run should use the underlying tool’s native check mode:

```text
git diff / git status / git merge --no-commit --no-ff preview where safe
Linear/GitHub payload validation without submit where possible
Workflow personalization generation into temp directory
Doc refresh into temp generation before swap
Patch application with check-only mode
```

If no native dry-run exists, the gateway should emulate preview from current state and mark it as estimated.

---

## 14. Confirmation UX requirements

The confirmation UI should be an action card or action plan drawer.

### 14.1 Single action card

A single action confirmation should show:

```text
Action title
Plain-language summary
Requesting actor
Project/session/worktree
Risk level
Why this action was proposed
Evidence chips
Resources changed
Preview/diff/command/API payload
Preconditions
Rollback availability
Policy reasons
Buttons: Approve, Deny, Edit, More details
```

### 14.2 Action plan card

A bundled workflow confirmation should show:

```text
Plan summary
Overall risk
Steps
Dependencies
Which steps need approval
Which steps are optional
Which steps are critical
Preview per step
Approve all eligible
Approve step-by-step
Edit plan
Deny plan
```

### 14.3 Human Input Queue

Actions awaiting approval should appear in the global Human Input Queue.

Sorting:

```text
Critical approvals
Blocked agent sessions
Failed actions
High-risk pending actions
Medium-risk pending actions
Low-risk informational actions
```

### 14.4 Editing an action

Users should be able to edit safe fields before approval:

```text
Branch name
Worktree name
Session prompt
Execution profile selection
PR title/body
Linear issue title/body
Workflow command arguments
Agent message
```

Critical fields should not be silently editable without re-running preview and risk classification.

---

## 15. Executor architecture

### 15.1 Logical flow

```text
Client / Project Brain / UI
  → Action Gateway API
  → Schema normalizer
  → Resource resolver
  → Policy engine
  → Risk classifier
  → Preview builder
  → Approval manager
  → Action queue
  → Executor adapter
  → Event bus + audit store
  → Project Brain memory ingestion
```

### 15.2 Executor adapters

Each executor adapter implements:

```ts
interface ActionExecutor<TParams, TResult> {
  actionTypes: ActionType[];
  validate(request: ActionRequest): ValidationResult;
  preview(request: ActionRequest): Promise<ActionPreview>;
  execute(request: ActionRequest, approval: ApprovalGrant): Promise<ActionResult>;
  rollback?(result: ActionResult): Promise<ActionResult>;
}
```

### 15.3 Required executors for MVP

```text
Project executor
Project Brain index executor
Git/worktree executor
Session executor
Workflow command executor
Code editor executor
GitHub executor, basic
Linear executor, basic
Event/audit executor
```

### 15.4 Executor isolation

Executors should run with least privilege:

```text
Filesystem actions scoped to project/worktree roots
Git actions scoped to repo/worktree
Session actions scoped to session handle
GitHub actions scoped to repo/account token
Linear actions scoped to workspace/project token
Workflow actions scoped to workflow instance
Execution profile actions scoped to selected runtime profile
```

---

## 16. Idempotency, concurrency, and locks

### 16.1 Idempotency keys

Every mutating action needs an idempotency key.

Examples:

```text
create worktree: project_id + branch_name + worktree_path
create session from plan task: project_id + plan_task_id + requested_mode + branch_name
create Linear issue from plan task: project_id + plan_task_id + target_team_id
create PR: repo_id + branch_name + base_branch
post comment: resource_id + generated_comment_hash
```

### 16.2 Resource locks

Locks should prevent conflicting mutations.

```text
Project lock
Worktree lock
Branch lock
Session lock
Agent team lock
Workflow instance lock
Integration resource lock
Brain index writer lock
```

### 16.3 Lock examples

```text
Only one git mutation per worktree at a time.
Only one workflow personalization run per project at a time.
Only one Project Brain write/index operation per project store at a time.
Multiple read actions may run concurrently.
A queued action must re-check preconditions after acquiring lock.
```

### 16.4 Stale preconditions

If state changed between preview and execution, the gateway must stop and refresh preview.

Examples:

```text
Branch changed
Worktree became dirty
PR checks changed
Plan task already linked
Session ended
Workflow instance moved from Needs Personalization to Active
Linear issue already created
```

---

## 17. Audit and event model

The Action Gateway should emit structured events for every lifecycle transition.

### 17.1 Core events

```text
ActionRequested
ActionNormalized
ActionValidated
ActionRiskClassified
ActionPreviewGenerated
ActionApprovalRequested
ActionApproved
ActionDenied
ActionExpired
ActionQueued
ActionStarted
ActionSucceeded
ActionFailed
ActionPartiallySucceeded
ActionRollbackRequested
ActionRolledBack
ActionRollbackFailed
ActionArchived
```

### 17.2 Event payload minimum

```ts
type ActionEvent = {
  eventId: string;
  eventType: string;
  occurredAt: string;
  actor: ActorRef;
  actionRequestId?: string;
  actionPlanId?: string;
  projectId?: string;
  sessionId?: string;
  resourceRefs: ResourceRef[];
  riskLevel?: RiskLevel;
  summary: string;
  metadata: Record<string, unknown>;
  redactionApplied: boolean;
};
```

### 17.3 Audit log requirements

The audit log should record:

```text
Who requested action
Why action was proposed
What evidence supported it
What preview was shown
Who approved/denied it
What policy allowed or blocked it
What executor ran it
What changed
What events were emitted
Whether rollback was available or used
```

### 17.4 Redaction

Audit logs must not store secrets in plaintext. Command previews, environment summaries, transcript snippets, API payloads, and error logs need redaction before persistence.

---

## 18. Project Brain integration

### 18.1 Brain role

Project Brain should be able to:

```text
Plan actions
Explain why actions are recommended
Attach evidence
Generate draft content
Ask for missing inputs
Suggest safer alternatives
Submit ActionRequests to the gateway
Observe ActionEvents for future memory
```

Project Brain should not directly:

```text
Run shell commands
Modify files
Create branches/worktrees
Start terminals
Merge PRs
Update tickets
Change credentials
Modify global config
Bypass approval queues
```

### 18.2 Brain action flow

```text
User asks Project Brain: "Start the next backend task."
Brain retrieves plan, architecture, sessions, worktrees, tickets, workflow state.
Brain creates ActionPlan.
Gateway validates resources and policies.
Gateway previews each step.
UI asks user for approval.
Gateway executes approved steps.
Events feed back into Project Brain memory.
```

### 18.3 Brain answer with action plan

Project Brain should present actions as proposals, not hidden execution.

Example:

```text
I found the next unstarted backend task: Phase 2.3 Auth callback persistence.
It is anchored to ARCHITECTURE.md §Auth Flow and has no active session.

Proposed plan:
1. Create worktree agent/p2-auth-callback
2. Start Claude Code session using Execution Profile "Claude Max Main"
3. Send structured task prompt with architecture and plan anchors
4. Link the session to the plan task
5. Open the terminal and code review workspace

Approve all eligible steps or review step-by-step?
```

### 18.4 Evidence requirements

Project Brain-submitted actions should include evidence references whenever possible:

```text
Plan task
Architecture anchor
Relevant code anchor
Session episode
Ticket
PR/check failure
Workflow manifest
User instruction
```

---

## 19. Workflow Pack integration

### 19.1 Workflow Pack states

The Action Gateway must understand the difference between:

```text
Workflow Pack available
Workflow Pack installed
Workflow Instance detected
Workflow Instance needs personalization
Workflow Personalization in progress
Workflow Instance active
Workflow Instance drift detected
Workflow Instance upgrade available
Workflow Instance archived
```

### 19.2 Workflow personalization action

Personalization is a high-risk workflow because it writes project-specific files.

Required flow:

```text
Detect architecture/task artifacts
Infer placeholders and code areas
Ask user for missing values
Generate personalization plan
Preview generated files in temp area
Show diff
User approves writes
Write files
Update workflow manifest
Emit events
Offer commit action separately
```

### 19.3 Workflow command invocation

Workflow commands such as `/team-start` should be represented as typed actions.

```ts
type InvokeWorkflowCommandParams = {
  workflowInstanceId: string;
  commandName: string;
  args: Record<string, unknown>;
  targetSessionId?: string;
  projectId: string;
  worktreeId?: string;
  executionProfileIds?: Record<string, string>;
};
```

Risk depends on command behavior.

```text
Read-only command → low risk
Command that creates sessions → medium/high risk
Command that writes files → high risk
Command that pushes/merges/deletes → critical risk
```

### 19.4 `/team-start` as an action plan

The platform should model `/team-start` as a bundled plan, not a raw command.

Example steps:

```text
Validate workflow instance is active
Validate selected plan track exists
Create or select worktree
Create lead session
Invoke /team-start <track>
Detect spawned orchestrator/implementer sessions
Register agent team
Link team to plan track
Open Agent Team View
```

---

## 20. Execution Profiles integration

### 20.1 Execution Profile as bounded runtime context

An `ExecutionProfile` represents a specific local runtime/account context, such as a particular Claude Max account, Claude Team account, Codex CLI profile, or API-backed profile.

The Action Gateway does not own credentials. It references execution profiles by ID and asks the platform runtime to execute under that profile.

### 20.2 Profile selection rules

```text
Every session.create action must specify or resolve an execution profile.
Every team.start action must specify profiles for lead/orchestrator/workers or use project defaults.
The user must see which profile will run each session.
Profiles can be project-allowlisted.
Profiles can have usage policies.
Profiles cannot be silently switched by Project Brain.
```

### 20.3 Profile-sensitive risk

Actions may become higher risk depending on profile.

Examples:

```text
Starting an expensive model under a high-cost profile may require confirmation.
Using a work account in a personal project may be blocked.
Using a profile near usage limit may require confirmation or suggestion.
```

---

## 21. Git and worktree requirements

### 21.1 Create worktree

Required preview:

```text
Repo
Base branch
New branch name
Worktree path
Current git status
Whether branch already exists
Whether path already exists
Linked task/session
Rollback: remove worktree/branch if clean
```

### 21.2 Delete worktree

Risk depends on dirty state.

```text
Clean and merged → medium/high
Dirty → critical
Unpushed commits → critical
Linked active session → critical or blocked
```

### 21.3 Commit

Required preview:

```text
Files staged/unstaged
Diff summary
Commit message
Linked task/session
Author identity
Rollback: soft reset if not pushed
```

### 21.4 Push

Required preview:

```text
Remote
Branch
Commits to push
Protected branch status
Linked PR status
Rollback limitations
```

### 21.5 Merge PR

Critical action.

Required checks:

```text
PR approved or override reason
Checks passing or override reason
Merge conflicts absent
Branch protection satisfied
Linked session/task reviewed
User explicit confirmation
No approve-all by default
```

---

## 22. Task, plan, and ticket requirements

### 22.1 Plan-task linking

Plan task links should be medium risk because they mutate platform metadata and possibly plan files.

Preview:

```text
Plan task
Target ticket/session/PR
Existing links
Proposed link
Whether source file changes
```

### 22.2 Create Linear issue from plan task

Preview:

```text
Linear team/project
Title
Description
Acceptance criteria
Labels
Priority
Linked architecture anchors
Linked plan task
```

### 22.3 Bidirectional sync

Bidirectional sync should be P2 and critical/high risk until conflict handling exists.

Required before enabling:

```text
Source-of-truth policy
Field mapping
Conflict detection
User-visible diff
Rollback story
Audit trail
Per-field sync controls
```

---

## 23. Code editor and review requirements

### 23.1 Request agent fix for selected code

Medium risk because it sends instruction to agent but does not directly edit files.

Preview:

```text
Selected file/range
Target session
Execution profile
Prompt text
Attached context chips
Expected worktree
```

### 23.2 Apply patch

High risk because it changes files.

Preview:

```text
Unified diff
Files changed
Conflicts
Patch source
Rollback: reverse patch if clean
```

### 23.3 Accept/reject hunk

High risk.

Preview:

```text
Hunk before/after
File path
Session/PR owner
Dirty worktree state
```

---

## 24. Project Brain doc/index requirements

### 24.1 Brain sync

Medium risk. It mutates derived caches, not source files.

Preview:

```text
Project
Files changed since last index
Embedding model
Transport policy
Estimated chunks
Transcript ingestion status
```

### 24.2 Session summarization

Medium risk because session transcripts may contain sensitive content.

Requirements:

```text
Opt-in per project
Redaction before embedding
Exclude thinking blocks
Local embeddings by default for session data
Explicit consent before raw transcript chunks leave machine
```

### 24.3 Owned-doc refresh

High risk because it may run a producer that writes docs.

Required checks:

```text
Doc is OWNED, not FOREIGN
Workflow/doc skill available
Dirty working tree check
Preview generated doc changes
Do not clobber human edits
Re-embed changed chunks only after approval
```

### 24.4 Foreign docs

The gateway must block any attempt to auto-overwrite FOREIGN docs.

Allowed alternatives:

```text
Flag as stale
Create supplemental note
Open issue/task
Ask user to update manually
```

---

## 25. API sketch

The real implementation can be local IPC, HTTP, RPC, or internal service calls. The conceptual API should look like this.

### 25.1 Capabilities

```http
GET /action-gateway/capabilities
```

Returns supported action types, risk defaults, executor availability, and policy constraints.

### 25.2 Submit single action

```http
POST /action-gateway/actions
```

Request body:

```json
{
  "actionType": "git.create_worktree",
  "projectId": "proj_123",
  "requestedBy": { "type": "project_brain", "brainId": "brain_123" },
  "intent": "Create isolated worktree for Phase 2.3 auth callback task",
  "params": {
    "baseBranch": "main",
    "branchName": "agent/p2-auth-callback",
    "worktreePath": "../worktrees/agent-p2-auth-callback"
  },
  "resourceRefs": [
    { "type": "plan_task", "id": "task_phase_2_3" }
  ],
  "evidenceRefs": [
    { "type": "architecture_anchor", "id": "ARCHITECTURE.md#auth-flow", "label": "Auth Flow" }
  ],
  "dryRunRequired": true,
  "confirmationPreference": "confirm_if_required"
}
```

### 25.3 Submit action plan

```http
POST /action-gateway/action-plans
```

### 25.4 Preview

```http
POST /action-gateway/actions/{id}/preview
```

### 25.5 Approve

```http
POST /action-gateway/approvals/{id}/approve
```

### 25.6 Deny

```http
POST /action-gateway/approvals/{id}/deny
```

### 25.7 Execute

```http
POST /action-gateway/actions/{id}/execute
```

Usually called internally after approval.

### 25.8 Status/events

```http
GET /action-gateway/actions/{id}
GET /action-gateway/action-plans/{id}
GET /action-gateway/events?actionRequestId={id}
```

---

## 26. Example action plans

### 26.1 Start next plan task

```text
User asks Project Brain:
"Start the next backend task."
```

Action plan:

```text
1. plan.link_task? maybe skip if already linked
2. git.create_worktree
3. session.create
4. session.send_message
5. plan.link_task_to_session
6. project.open_session_terminal
```

Risk:

```text
Overall: Medium/High
Step 2: Medium
Step 3: Medium
Step 4: Medium
Step 5: Medium
```

Preview:

```text
Next task selected
Architecture anchors
Worktree path
Branch name
Execution profile
Prompt text
Linked resources
```

### 26.2 Start `/team-start` for a track

Action plan:

```text
1. workflow.detect
2. workflow.command.preview /team-start <track>
3. git.create_worktree
4. session.create lead
5. workflow.command.invoke /team-start <track>
6. team.register_spawned_sessions
7. plan.link_task_to_team
8. open Agent Team View
```

Risk:

```text
Overall: High
Critical if the command is configured to commit/push/merge automatically.
```

### 26.3 Fix failing PR checks

Action plan:

```text
1. github.get_pr_checks
2. brain.find_related failure history
3. git.create_worktree from PR branch or use existing worktree
4. session.create fix session
5. session.send_message with failing logs and related code
6. review.open_diff
```

Risk:

```text
Medium until changes are committed/pushed.
High for commit/push.
Critical for merge.
```

### 26.4 Refresh stale owned docs

Action plan:

```text
1. brain.drift_check
2. classify docs as OWNED/FOREIGN/SUPPLEMENTAL
3. workflow.command.preview layer-docs --check
4. workflow.command.preview layer-docs --update into temp output
5. show diff
6. apply doc updates
7. brain.sync changed chunks
```

Risk:

```text
High because source docs are modified.
Blocked for FOREIGN docs.
```

### 26.5 Personalize workflow pack

Action plan:

```text
1. workflow.detect available pack
2. scan architecture/task artifacts
3. infer placeholders/code areas
4. ask user for missing values
5. generate personalization plan
6. generate files into temp area
7. show diff
8. write files after approval
9. write workflow manifest
10. offer git commit as separate action
```

Risk:

```text
High because project files are written.
Commit remains a separate high-risk action.
```

---

## 27. UI components needed

The design system should include these Action Gateway components:

```text
Action request card
Action plan drawer
Risk badge
Policy reason block
Evidence chip row
Preview panel
Command preview block
Diff preview block
API payload preview block
Rollback availability block
Approval buttons
Approve step-by-step control
Approve all eligible control
Deny with reason modal
Edit action modal
Permission grant modal
Human Input Queue row
Action progress timeline
Action audit timeline
Action failure card
Partial success recovery card
Policy automation settings table
Standing grant card
```

---

## 28. MVP scope

### 28.1 MVP must support

```text
Typed ActionRequest and ActionPlan model
Risk classification levels 0-4
Approval queue
Single-action approval
Action-plan preview
Action events/audit log
Basic policy engine
Git/worktree executor
Session executor
Workflow command executor, basic
Project Brain index executor
GitHub basic executor
Linear basic executor
Project Brain action planning integration
Execution profile selection for session/team actions
Idempotency keys for mutating actions
Resource locks for project/worktree/session
```

### 28.2 MVP action types

```text
brain.ask
brain.sync
brain.summarize_session
project.rescan
workflow.detect
workflow.command.invoke
plan.link_task
session.create
session.attach_terminal
session.send_message
session.pause
session.resume
git.status
git.diff
git.create_worktree
git.create_branch
github.create_pr_draft
linear.link_issue
linear.create_issue
code.open_file
review.request_agent_fix
```

### 28.3 MVP can defer

```text
Bidirectional Linear sync
Automatic PR merge
Force push
Full rollback engine
Multi-user approval chains
Policy automation marketplace
Fine-grained team RBAC
Workflow upgrade application
Owned-doc auto-refresh application
Critical action automation
```

---

## 29. P1 scope

```text
Bundled workflow approval
/team-start launch plan
Workflow personalization flow
Workflow upgrade preview
PR checks → agent fix workflow
Commit/push with preview
Create non-draft PR
Request review
Project Brain suggested actions inside drawer
Standing grants with TTL
Action replay for debugging
Partial-success recovery flows
Owned-doc refresh preview
One-way create Linear issue from plan task
```

---

## 30. P2 scope

```text
Controlled bidirectional Linear sync
Merge PR with full guardrail flow
Team/multi-user approval chains
Cross-project action plans
Policy automation mode
Custom workflow action schemas
Workflow Pack SDK for action definitions
Full rollback framework
Action simulation/sandboxing
Organization-level governance
Signed action bundles
```

---

## 31. Non-goals

```text
No silent Project Brain execution
No arbitrary untyped shell-command executor
No auto-merge in MVP
No bidirectional ticket sync in MVP
No broad standing grant like "do anything in this repo"
No direct credential access from Project Brain
No global config mutation without explicit consent
No raw transcript cloud processing without explicit consent
No workflow pack writing to a project before personalization approval
```

---

## 32. Open questions

### Product questions

```text
What is the minimum useful action set for the first demo?
Should Project Brain be allowed to auto-create draft text without approval?
Should session.send_message always require approval or be policy-allowable per session?
Should create_worktree require approval every time or be allowlisted per project?
What actions can be batched under Approve All in MVP?
What is the default policy for running tests?
```

### Technical questions

```text
What is the gateway transport: local HTTP, IPC, in-process service, or MCP tool?
How should executor adapters be registered?
How should action schemas be versioned?
Where does the audit log live?
How should action events feed Project Brain without circular dependency?
How should locks work across app restarts?
How should idempotency keys be derived consistently?
```

### Security questions

```text
What commands are safe enough for project-level standing grants?
How should shell-command risk be classified?
How should secrets be redacted from command previews and logs?
How should execution profile credentials be isolated?
What approval is required before sending raw transcript snippets to cloud models?
```

### UX questions

```text
Where does the approval queue live globally?
Should the Project Brain drawer show action plans inline or as a separate panel?
How much detail should the default action card show before expanding?
How should partial success recovery be presented?
How do we keep frequent low-risk approvals from becoming annoying?
```

---

## 33. Decisions captured in this draft

```text
Project Brain should be action-capable, but only through the Action Gateway.
The platform owns execution, permissions, credentials, and audit logs.
All mutating actions should be typed and previewed.
Risk levels 0-4 should drive approval behavior.
Critical actions should not be included in approve-all workflows by default.
Workflow Pack personalization is a high-risk action because it writes files.
The scaffold/template distinction matters: workflow pack vs workflow instance.
Execution Profile selection is part of session/team action planning.
Bidirectional Linear sync should not be MVP.
Merge PR should be critical and explicit.
Session transcript processing should remain under the strictest privacy gate.
```

---

## 34. Relationship to future artifacts

This spec should feed directly into:

```text
Event Model and Audit Trail Spec
Security, Permissions, and Safety Spec
Workflow Packs Spec
UX / Information Architecture Spec
Screen-by-Screen UI Requirements
Main Platform PRD
Project Brain PRD v3
```

The next recommended artifact is the **Event Model and Audit Trail Spec**, because the Action Gateway cannot be implemented cleanly without a durable event stream.
