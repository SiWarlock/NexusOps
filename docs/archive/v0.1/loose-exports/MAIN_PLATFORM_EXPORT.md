# Main Platform PRD v0.1

> **Working title:** AI Engineering Control Plane / the platform  
> **Status:** Draft v0.1 for architecture planning  
> **Naming:** OPEN. Do not treat any previous naming candidate as final.  
> **Primary form factor:** Desktop app with local runtime.  
> **Companion systems:** Project Brain, Workflow Packs, Action Gateway.  
> **Intended downstream use:** Input to architecture-draft, technical architecture, implementation planning, and revised design prototype generation.

---

## 1. Executive Summary

The platform is a **desktop-first AI engineering control plane** for running, supervising, reviewing, and shipping work from AI coding agents across multiple local projects.

It combines:

- agent session orchestration for Claude Code, Codex, and future harnesses;
- terminal multiplexing with first-class local terminals;
- worktree, branch, commit, pull request, and merge control;
- GitHub issue and Linear ticket intake;
- a review-focused code editor and diff workspace;
- project/session observability;
- agent team orchestration;
- workflow-pack support for custom scaffolds and project-specific commands;
- Project Brain integration for memory, retrieval, reasoning, and action planning;
- a permissioned Action Gateway that lets Project Brain propose and request actions safely.

The platform is **not** a generic IDE, not a chatbot wrapper, not a cloud SaaS, and not a replacement for Claude Code, Codex, GitHub, Linear, or the user’s existing local development environment. It is the control layer above them.

The main product thesis is:

> Once AI coding agents become parallel workers, the developer becomes a manager of distributed work. Existing tools still expose each agent as an isolated terminal, chat, branch, or PR. The platform gives the developer one desktop cockpit where every task, session, terminal, worktree, branch, PR, approval, workflow, and Project Brain memory object is visible, actionable, and auditable.

The first high-quality version should let the user answer these questions quickly:

```text
What projects are active?
Which agent sessions are running?
Which sessions are idle, blocked, failed, or waiting on me?
What task/ticket/plan item is each session working on?
Which model, harness, account/subscription, branch, and worktree is each session using?
What changed in the code?
What needs review?
What needs approval?
What is ready for PR or merge?
What does Project Brain know about this project, task, feature, file, or decision?
What action is Project Brain proposing, and is it safe to approve?
```

---

## 2. Product Definition

### 2.1 What the Platform Is

The platform is a **local desktop application and local execution runtime** that coordinates AI-assisted engineering work.

It has five major responsibilities:

1. **Dispatch work** from prompts, GitHub issues, Linear tickets, implementation plan tasks, architecture sections, PR failures, or ad hoc user instructions into agent sessions or agent teams.
2. **Supervise work** by tracking session status, terminal output, heartbeats, tool calls, human input needs, context usage, token usage, worktrees, branches, changed files, and events.
3. **Review work** through a code editor, changed files list, inline/side-by-side diffs, tests, diagnostics, PR state, comments, and agent explanations.
4. **Deliver work** through git actions, commits, branches, worktrees, pull requests, checks, reviews, merges, and archiving.
5. **Remember and reason** through Project Brain, which indexes code/docs/session history and can answer questions or propose actions through the Action Gateway.

### 2.2 What the Platform Is Not

The platform is not:

- a replacement for local git;
- a hosted cloud IDE;
- a general team SaaS in MVP;
- a direct remote shell product;
- a generic RAG chatbot;
- a generic project management tool;
- a replacement for GitHub or Linear;
- a replacement for Claude Code or Codex;
- a full IDE replacement in MVP;
- an autonomous system that mutates repos, tickets, credentials, or branches without permission.

### 2.3 Product Category

The product should define its own category:

```text
AI engineering control plane
AI coding operations console
Agentic development cockpit
Local-first agent orchestration desktop app
```

For internal planning, use **AI engineering control plane**.

---

## 3. Target Users and Personas

### 3.1 Primary Persona — Portfolio Builder / Solo Technical Lead

A senior developer running multiple local projects with Claude Code, Codex, custom workflows, and git worktrees.

Pain points:

- too many terminal tabs;
- too many branches/worktrees;
- difficult to remember what each agent is doing;
- difficult to know which session is blocked;
- difficult to trace a ticket from prompt to branch to PR;
- hard to recover context across sessions;
- hard to know when Project Brain/session memory should be consulted;
- hard to scale custom workflows across projects.

Success state:

> The user can open the desktop app and immediately see what is happening across all projects, what needs attention, which work is safe to review/merge, and what Project Brain remembers about any feature/task/decision.

### 3.2 Secondary Persona — Small-Team Tech Lead / Reviewer

A technical lead coordinating AI-generated work across a team or portfolio.

Pain points:

- review burden increases as agents produce more PRs;
- difficult to preserve architectural coherence;
- hard to audit why decisions were made;
- difficult to prevent accidental merges or risky automation.

Success state:

> The user can review agent outputs, inspect evidence, approve safe actions, reject risky ones, and preserve a decision trail.

### 3.3 Future Persona — Mobile Supervisor

The same primary user away from the desktop machine.

Pain points:

- wants to know if long-running agents are blocked;
- wants notifications for human-input-needed states;
- wants to approve/deny low-risk actions remotely;
- does not need direct shell access.

Success state:

> The user can observe project/session state and approve constrained actions from an iOS companion without exposing a raw remote shell.

---

## 4. Jobs To Be Done

### JTBD-1: Start Work Safely

When I receive a task, ticket, issue, or implementation-plan item, I want to launch an AI coding session in the right project/worktree/profile so that the work is isolated, traceable, and linked to its source.

### JTBD-2: Supervise Parallel Agents

When multiple agents are working, I want to see who is active, idle, blocked, stale, or waiting on me so that I can direct attention where it matters.

### JTBD-3: Review and Correct Code

When an agent produces changes, I want to inspect diffs, files, tests, diagnostics, and reasoning so that I can accept, reject, redirect, or ask for fixes.

### JTBD-4: Deliver Changes

When work is ready, I want to commit, push, open a PR, monitor checks, request fixes, and merge safely.

### JTBD-5: Use Project Memory

When I need project context, I want Project Brain to answer questions with evidence and help me understand when/how/why something was implemented.

### JTBD-6: Let the Brain Help, But Safely

When Project Brain knows what to do next, I want it to propose an action plan and request approval through the platform rather than silently mutating my system.

### JTBD-7: Support My Custom Workflows

When a project uses a scaffold/workflow like cc-crew, I want the platform to detect, personalize, expose, and run its commands and team flows without requiring every project to use that scaffold.

---

## 5. Product Principles

### 5.1 Desktop First

The desktop app is the primary product surface and the local machine is the execution/trust boundary.

The platform must assume:

- local repos;
- local worktrees;
- local terminals;
- local authenticated Claude/Codex contexts;
- local git credentials;
- local Project Brain stores;
- local action execution.

### 5.2 Human Attention Is the Scarce Resource

The UI should rank work by attention need, not alphabetically.

Priority order:

1. waiting on human input;
2. permission requested;
3. failed/stale/conflicted;
4. active/running tests;
5. changes ready;
6. idle;
7. completed/archived.

### 5.3 Session Is the Atomic Operational Unit

A session is the main unit that connects:

```text
task/ticket/plan item
agent harness
execution profile
terminal
worktree
branch
files changed
tool calls
approvals
usage
PR
summary
events
Project Brain memories
```

### 5.4 Preserve Native Agent Surfaces

Claude Code and Codex remain terminal-native. The platform wraps and observes them; it does not hide them behind a fake chat-only UI.

### 5.5 Code Review Is First-Class

The code editor is a first-class surface, but the MVP editor is review-focused rather than a full IDE replacement.

### 5.6 Every Action Is Auditable

Important actions must produce durable events:

- who/what requested it;
- what evidence was used;
- what approval was granted;
- what executor ran;
- what changed;
- whether rollback is available.

### 5.7 Project Brain Can Plan, Platform Executes

Project Brain may understand, reason, draft, recommend, and request actions. The platform owns credentials, permissions, local execution, audit, and rollback.

### 5.8 Workflow Packs Are Optional

The platform should work on a basic repo. Workflow Packs add rich conventions but are not required.

### 5.9 Risk Must Be Visible

The platform must surface:

- dangerous commands;
- high context usage;
- high cost usage;
- dirty worktrees;
- conflicts;
- stale sessions;
- failing checks;
- unreviewed generated code;
- weak Project Brain evidence;
- low-confidence session/project associations;
- incomplete workflow personalization.

---

## 6. Core Product Concepts

### 6.1 Project

A local engineering project, usually a git repository, with optional GitHub/Linear mapping, Project Brain index, workflow instance, and execution settings.

### 6.2 Session

One AI coding agent runtime instance, typically Claude Code or Codex, attached to a project/worktree/branch/task.

### 6.3 Agent Team

A grouped set of related sessions with a lead/orchestrator and worker sessions. Agent teams may be launched by workflow commands such as `/team-start`.

### 6.4 Execution Profile

A named local runtime/account context used to launch sessions. Examples:

```text
Claude Max Main
Claude Max Secondary
Claude Team Work
Codex CLI Main
Codex Cloud GitHub
```

Execution Profiles are explicit and auditable. They must not be treated as automatic subscription-hopping.

### 6.5 Worktree

An isolated git working directory, usually mapped to a branch and one or more sessions.

### 6.6 Task

An external or internal work item. Sources include:

- GitHub issue;
- Linear ticket;
- implementation plan task;
- architecture section;
- PR failure;
- ad hoc prompt.

### 6.7 Implementation Plan / Plan Task

A structured project plan parsed from files such as `MVP_TASKS.md`. Plan tasks may link to architecture anchors, Linear/GitHub items, sessions, worktrees, PRs, and commits.

### 6.8 Workflow Pack

A reusable template/package of commands, skills, agents, plan parsers, launch recipes, templates, and conventions.

### 6.9 Workflow Instance

A project-specific personalized/generated installation of a Workflow Pack.

### 6.10 Project Brain

The local-first project memory/retrieval/reasoning/action-planning system. It can be standalone, but the platform embeds it as a drawer and action-planning collaborator.

### 6.11 Action Gateway

The permissioned execution layer through which Project Brain, UI components, workflow commands, and future remote companion apps request actions.

### 6.12 Event

A durable timeline/audit record of anything important that happened in the platform.

---

## 7. Product Surfaces

The desktop app should include these primary surfaces.

### 7.1 First Launch / Setup Wizard

Guides machine setup:

- local runtime checks;
- Claude Code detection;
- Codex detection;
- git detection;
- terminal shell detection;
- Project Brain setup status;
- GitHub auth;
- Linear auth;
- default worktree root;
- execution profile creation;
- permission policy defaults;
- optional Workflow Pack library setup.

### 7.2 Global Command Center

A cross-project attention dashboard.

Shows:

- active sessions;
- sessions waiting on input;
- failed/stale sessions;
- active agent teams;
- open PRs;
- tasks assigned to agents;
- usage/context hotspots;
- recently completed work;
- remote status if companion access exists later.

### 7.3 Project Home / Observability Graph

A project-level operational graph connecting:

- project;
- sessions;
- agent teams;
- team leads;
- workers;
- worktrees;
- branches;
- plan tasks;
- GitHub issues;
- Linear tickets;
- PRs;
- approvals;
- Project Brain evidence/memory sources.

The graph must be operational, not decorative.

### 7.4 Project Sessions List

A dense list/table of sessions grouped by status, harness, execution profile, task, worktree, branch, PR, usage, and attention state.

### 7.5 Session Terminal View

A first-class terminal surface with session metadata and side inspector.

### 7.6 Code Editor / Diff Review Workspace

A review-focused editor showing file explorer, changed files, editor tabs, diffs, conflicts, tests, diagnostics, and ask-agent-on-selection actions.

### 7.7 Plan View

A structured view over implementation plan files such as `MVP_TASKS.md`, showing phases, tracks, tasks, anchors, links, sessions, PRs, status, and dispatch actions.

### 7.8 Task Inbox

GitHub Issues, Linear Tickets, PR review requests, ad hoc tasks, plan tasks, and assigned work.

### 7.9 Worktree / Git / PR Control Center

Fleet-management view for worktrees, branches, dirty states, commits, PRs, checks, conflicts, and merges.

### 7.10 PR Review Workspace

Review PR status, diff, checks, comments, linked sessions, linked tasks, and agent fix flows.

### 7.11 Agent Team View

Shows lead/orchestrator/worker topology, roles, worktrees, branches, terminals, TDD stage, context status, artifacts, and coordination timeline.

### 7.12 Workflow Setup + Command Registry

Detect, install, personalize, activate, inspect, and run Workflow Packs and commands.

### 7.13 Project Brain Drawer

Right-side or slide-over assistant surface for project memory, evidence, questions, suggested actions, and action-plan previews.

### 7.14 Human Input Queue

Global queue of approvals, clarifications, permission requests, findings, blockers, and escalations.

### 7.15 Action Gateway Review Modal

Structured approval surface for one action or a bundled action plan.

### 7.16 Execution Profiles Settings

Create and manage runtime/account contexts for Claude, Codex, and future harnesses.

### 7.17 Usage / Context Dashboard

Tracks tokens, cost, context limits, profile usage, project usage, session usage, model/harness usage, and budget alerts.

### 7.18 Events / Audit Log

Durable event explorer for sessions, tasks, workflows, Project Brain, Action Gateway, git, PRs, approvals, and integrations.

### 7.19 Settings / Integrations / Remote Access

Global settings, project settings, GitHub/Linear integrations, Project Brain policies, remote companion settings, and security posture.

---

## 8. Desktop-First Runtime Requirements

### DESK-1: Desktop App Required

The platform MUST ship as a desktop app for MVP.

### DESK-2: Local Runner Required

The platform MUST include or control a local runner responsible for:

- spawning terminals;
- launching Claude/Codex sessions;
- creating worktrees;
- running git commands;
- monitoring process status;
- recording events;
- routing Action Gateway requests;
- reading local project metadata;
- connecting to Project Brain.

### DESK-3: No Web App MVP

A browser web app MAY exist later as a local UI shell or remote companion backend, but it is not the MVP product surface.

### DESK-4: Local Trust Boundary

The local machine is the trust boundary for raw code, terminals, credentials, session logs, git actions, and Project Brain indexes.

### DESK-5: Local Credentials

The platform SHOULD rely on existing local auth contexts where possible, including:

- Claude Code local auth;
- Codex local auth;
- Git credentials;
- GitHub CLI or OAuth token;
- Linear OAuth/API token.

### DESK-6: Terminal Multiplexing

The desktop app MUST support multiple live terminal surfaces and allow terminals to be docked, popped out, attached, detached, and grouped by session/team.

### DESK-7: Process Resilience

The local runner SHOULD survive UI reloads/restarts where possible. Running sessions should be recoverable/re-attachable if the terminal/harness supports it.

### DESK-8: Native Notifications

The desktop app SHOULD support local notifications for waiting sessions, failed checks, permission requests, and completed sessions.

### DESK-9: Companion iOS Stretch

The iOS companion is P2/stretch. It should initially support observability, notifications, and constrained approvals. It MUST NOT be a raw remote shell in early versions.

---

## 9. Core User Flows

### 9.1 First-Time Setup

```text
Install desktop app
Launch setup wizard
Detect Claude Code / Codex / git / shell
Set default worktree root
Create execution profiles
Connect GitHub
Connect Linear
Set Project Brain setup state
Choose permission policy defaults
Add first project
Run initial project scan
Open Global Command Center
```

Requirements:

- setup must be idempotent;
- setup must be reversible;
- setup must be consented;
- degraded states must be visible;
- user must be able to skip optional integrations.

### 9.2 Add Local Project

```text
Click Add Project
Choose local repo path
Detect git state
Detect GitHub remote
Detect Linear mapping if available
Detect Project Brain index status
Detect Workflow Pack / Workflow Instance state
Detect implementation plan files
Choose default execution profiles
Choose worktree root
Finish add
```

### 9.3 Start Work from Linear/GitHub

```text
Open Task Inbox
Select Linear/GitHub item
Review description and acceptance criteria
Choose project/repo
Choose execution mode: session or team
Choose harness and execution profile
Choose worktree strategy
Preview generated prompt
Start session/team
Platform creates worktree/branch if requested
Platform launches terminal session
Task links to session/worktree/branch
Events recorded
```

### 9.4 Start Work from Implementation Plan

```text
Open project Plan View
Select plan task/phase/track
Review architecture anchors and acceptance criteria
Optionally link/create Linear/GitHub issue
Choose single agent or agent team
Choose workflow command if available
Preview plan
Approve
Create worktree/session/team
Link plan task to artifacts
```

### 9.5 Start Blank Session

```text
Select project
Click New Session
Choose harness
Choose execution profile
Choose worktree strategy
Enter prompt
Start
Attach terminal
```

### 9.6 Start Agent Team

```text
Select plan track / ticket / task
Choose Start Agent Team
Choose workflow recipe if available
Select lead/orchestrator/worker profiles
Preview team topology
Preview worktree/branch strategy
Approve action plan
Run team launcher
Open Agent Team View
```

### 9.7 Monitor Active Work

```text
Open Global Command Center
Review attention queue
Open project graph
Filter by waiting/active/stale/context high
Select session/team
Open terminal/editor/inspector
Act or defer
```

### 9.8 Respond to Human Input Request

```text
Human Input Queue shows request
User opens request
Platform shows context, session, task, command, risk, evidence
User approves/denies/edits
Action Gateway executes if approved
Session resumes or receives denial/instruction
Event recorded
```

### 9.9 Review Agent Changes

```text
Session says changes ready
Open Code Review Workspace
Inspect changed files
Review diff
Run tests/checks
Ask agent to explain selected code/diff
Request fix if needed
Commit or create PR
```

### 9.10 Create and Manage PR

```text
Review changes
Click Create PR
Platform generates title/body from task/session summary
User edits
Create PR through GitHub
Monitor checks
If checks fail, send failure context to agent
Review fixes
Merge when approved
Archive session/worktree as appropriate
```

### 9.11 Ask Project Brain

```text
Open Project Brain drawer
Ask project/session/file/PR/task question
Brain retrieves code/docs/sessions/events/evidence
Answer includes evidence chips
User clicks evidence to open code, session, PR, or plan task
```

### 9.12 Let Project Brain Propose Actions

```text
User asks Brain to do something
Brain produces action plan
Platform renders Action Gateway preview
User approves step-by-step or all
Platform executes through typed executors
Events recorded
Brain indexes the result later
```

### 9.13 Workflow Pack Personalization

```text
Open project Workflow tab
Select Workflow Pack
Platform detects prerequisites
Launch personalization run
Agent asks user for missing architecture/project values
Platform captures answers
Generated plan/diff shown
User approves writing files
Files generated
User reviews/commits
Workflow Instance becomes active
```

### 9.14 Remote Companion Stretch Flow

```text
Desktop app enables remote companion
Secure pairing established
iOS app shows projects/sessions/attention queue
Push notification for waiting input
User opens request
Can approve/deny low-risk action
High-risk actions require desktop confirmation
No raw shell access
```

---

## 10. Functional Requirements by Module

The following requirements use MUST / SHOULD / MAY semantics.

---

### 10.1 Project Management

#### PROJ-1: Add Project

The platform MUST allow users to add a local project by choosing a local path.

#### PROJ-2: Detect Git Repository

The platform MUST detect whether the project path is a git repo and identify:

- repo root;
- default branch;
- current branch;
- remote origin;
- dirty state;
- existing worktrees.

#### PROJ-3: Project Registry

The platform MUST maintain a local project registry with stable project IDs.

#### PROJ-4: Project Brain Status

The project view MUST show Project Brain status:

- not installed;
- setup required;
- index missing;
- indexing;
- ready;
- stale;
- degraded;
- opt-in needed for session memory.

#### PROJ-5: Workflow Detection

The project scan MUST detect Workflow Pack/Instance signals, including but not limited to:

- `.scaffolding/manifest.json`;
- `.project-brain/manifest.json`;
- `CLAUDE.md`;
- `.claude/commands/`;
- `.claude/skills/`;
- `.claude/agents/`;
- `ARCHITECTURE.md`;
- `MVP_TASKS.md` / `MVP_TASK.md`;
- docs directories;
- command definitions.

#### PROJ-6: Project Settings

Each project SHOULD support project-level defaults:

- default execution profile;
- allowed execution profiles;
- default worktree root;
- branch naming pattern;
- default harness;
- permission policy;
- GitHub/Linear mappings;
- Project Brain policy;
- Workflow Pack preference.

---

### 10.2 Execution Profiles

#### PROF-1: Named Profiles

The platform MUST support named execution profiles representing local harness/account contexts.

#### PROF-2: Profile Fields

Each profile SHOULD include:

```text
id
displayName
provider
harness
accountAlias
authMethod
shellProfile
cliPath
defaultModel
defaultPermissionMode
defaultWorktreeRoot
allowedProjects
usagePolicy
status
lastUsedAt
```

#### PROF-3: Explicit Session Assignment

Every session MUST record the execution profile used to launch it.

#### PROF-4: Team Role Assignment

Agent team creation SHOULD allow different profiles for lead/orchestrator/worker roles.

#### PROF-5: Usage Tracking

Usage dashboards SHOULD group usage by profile when reliable data exists.

#### PROF-6: No Hidden Auto-Switching

The platform MUST NOT silently route sessions to alternate subscriptions/accounts without user-visible profile selection or policy approval.

---

### 10.3 Harness Adapters

#### HARN-1: Adapter Model

The platform MUST treat Claude Code, Codex, and future runtimes as adapters behind a common session abstraction.

#### HARN-2: Claude Code Adapter

MVP SHOULD support Claude Code interactive sessions, terminal attachment, resume where possible, command injection where safe, transcript capture where allowed, and status detection.

#### HARN-3: Codex Adapter

MVP SHOULD support Codex CLI sessions if feasible. If not, Codex support may be P1 but the data model must support it from the beginning.

#### HARN-4: Future Adapters

The architecture SHOULD support future adapters such as:

- Gemini CLI;
- Aider;
- OpenCode;
- custom shell commands;
- remote/cloud agent runners.

#### HARN-5: Capability Declaration

Each adapter MUST declare capabilities:

```text
supportsTerminal
supportsResume
supportsTranscriptRead
supportsToolCallParsing
supportsUsageMetadata
supportsContextMetadata
supportsCommandInjection
supportsSubagents
supportsHooks
supportsCloudTasks
```

---

### 10.4 Session Management

#### SESS-1: Create Session

The platform MUST create sessions from:

- blank prompt;
- GitHub issue;
- Linear ticket;
- plan task;
- architecture section;
- PR/check failure;
- Project Brain action plan;
- Workflow Command.

#### SESS-2: Session Metadata

Each session MUST track:

```text
session_id
project_id
harness
model
execution_profile_id
status
task_id / plan_task_id / issue_id / ticket_id
worktree_id
branch
terminal_id
agent_team_id if any
created_at
last_heartbeat_at
context_usage
token_usage
cost_estimate
files_changed
linked_pr
summary
```

#### SESS-3: Status Model

Session statuses MUST include:

```text
creating
starting
active
thinking
running_command
editing_files
running_tests
waiting_on_permission
waiting_on_human
waiting_on_external_service
idle
stale
failed
changes_ready
completed
archived
killed
```

#### SESS-4: Attention Sorting

Session lists MUST sort by attention priority before alphabetical order.

#### SESS-5: Attach Terminal

The user MUST be able to attach/open the terminal for a session.

#### SESS-6: Pause/Resume/Kill

The platform SHOULD support pause/resume/kill where the adapter/runtime supports it.

#### SESS-7: Session Summary

The platform SHOULD create or request a session summary when a session completes, idles, archives, or approaches context limits.

#### SESS-8: Session Archive

Completed sessions SHOULD be archivable with summary, artifacts, worktree/branch/PR state, and Project Brain ingestion status.

#### SESS-9: Session Ownership

Every code change surfaced in the UI SHOULD show which session/team produced it if known.

---

### 10.5 Terminal Management

#### TERM-1: Embedded Terminal

The desktop app MUST provide embedded terminal panels.

#### TERM-2: Terminal Tabs

The user SHOULD be able to view multiple terminals as tabs or panes.

#### TERM-3: Team Terminal Group

Agent Team View SHOULD group lead/orchestrator/worker terminals.

#### TERM-4: Terminal Context Bar

Each terminal MUST show context metadata:

```text
project
session
harness
model
execution profile
worktree
branch
task/ticket/plan link
status
```

#### TERM-5: Command/Approval Extraction

Where feasible, the platform SHOULD detect approval requests, command prompts, and blocked states from terminal output and adapter events.

#### TERM-6: Terminal Safety

Remote companion surfaces MUST NOT expose raw terminal input/output in early versions unless explicitly added behind a high-security policy.

---

### 10.6 Code Editor / Diff Review

#### EDIT-1: Review-Focused Editor

The MVP editor MUST support reviewing changed files and diffs from a selected session/worktree/PR.

#### EDIT-2: File Explorer

The editor SHOULD include a file explorer scoped to the selected project/worktree.

#### EDIT-3: Changed Files Panel

The editor MUST include a changed files panel for the selected worktree/session/PR.

#### EDIT-4: Editor Tabs

The editor SHOULD support multiple file tabs.

#### EDIT-5: Diff Modes

The editor MUST support inline diff and SHOULD support side-by-side diff.

#### EDIT-6: Conflict Resolver

The editor SHOULD support merge conflict visualization and resolution, likely P1 if too large for MVP.

#### EDIT-7: Diagnostics/Test Output

The editor SHOULD show diagnostics, problems, and test output when available.

#### EDIT-8: Ask Agent on Selection

The editor SHOULD let users select code or diff hunks and ask an agent to:

- explain;
- refactor;
- fix;
- add tests;
- align with architecture;
- compare to Project Brain evidence;
- resolve a review comment.

#### EDIT-9: Link to Project Brain

The editor SHOULD show evidence chips from Project Brain for relevant files, symbols, plan tasks, decisions, or prior sessions.

#### EDIT-10: External IDE Option

The platform SHOULD provide an “open in external IDE” action for the selected file/worktree.

---

### 10.7 Task Inbox / GitHub / Linear

#### TASK-1: Task Sources

The platform MUST support internal tasks and SHOULD support GitHub Issues and Linear Tickets in MVP or early P1.

#### TASK-2: Read-Only Sync First

GitHub/Linear intake SHOULD start with read/link behavior before deeper mutation/sync.

#### TASK-3: Task Card

Each task card SHOULD show:

```text
source
title
status
priority
labels
repo/project
assignee
acceptance criteria
linked plan task
linked session
linked worktree
linked PR
```

#### TASK-4: Dispatch Task

The user MUST be able to create a session/team from a task.

#### TASK-5: Drag and Drop

Drag-and-drop SHOULD be supported but every drag action MUST have an accessible button/menu alternative.

#### TASK-6: Task Linking

The platform MUST support manual linking between tasks, plan tasks, sessions, worktrees, branches, and PRs.

#### TASK-7: Linear Sync Staging

Linear sync SHOULD follow this sequence:

```text
P0: manual linking
P1: one-way create/update with confirmation
P2: controlled bidirectional sync with conflict resolution
```

---

### 10.8 Implementation Plan / Plan View

#### PLAN-1: Detect Plan Files

The platform MUST detect plan files such as `MVP_TASKS.md` and `MVP_TASK.md`.

#### PLAN-2: Parse Plan Structure

Where a parser exists, the platform SHOULD parse phases, tracks, tasks, anchors, acceptance criteria, deliverables, carry-forward items, decisions, and logs.

#### PLAN-3: Plan Task Object

Plan tasks SHOULD map to structured objects with:

```text
id
title
phase
track
source_anchor
architecture_anchor
status
dependencies
acceptance_criteria
linked_linear_issue
linked_github_issue
linked_sessions
linked_worktrees
linked_prs
```

#### PLAN-4: Dispatch from Plan

Users MUST be able to start a session/team from a plan task or track.

#### PLAN-5: Link to Architecture

Plan tasks SHOULD link to architecture anchors when present.

#### PLAN-6: Update Plan Status

Plan status updates SHOULD be explicit and auditable. Automatic status updates MAY be P1/P2 and should require clear policy.

---

### 10.9 Worktree / Git / Branch Management

#### GIT-1: Create Worktree

The platform MUST support creating a git worktree for a session or task.

#### GIT-2: Worktree Metadata

Each worktree MUST track:

```text
path
branch
project
linked session/team
linked task/plan task
status
changed files
last commit
linked PR
```

#### GIT-3: Worktree Statuses

Worktree statuses SHOULD include:

```text
clean
dirty
untracked_files
conflicts
behind_base
ahead_of_base
pr_open
merged
prunable
locked
deleted
```

#### GIT-4: Git Actions

The platform SHOULD support:

- create worktree;
- delete/archive worktree;
- checkout branch;
- commit;
- push;
- pull/rebase;
- merge main;
- open diff;
- resolve conflicts;
- create PR.

#### GIT-5: Action Gateway Required for Risky Git

Risky git actions MUST go through the Action Gateway.

Examples:

- delete worktree;
- delete branch;
- force push;
- merge PR;
- rebase public branch;
- commit generated code if policy requires approval.

#### GIT-6: Ownership Visibility

Every branch/worktree row SHOULD show its owning session/team/task if known.

---

### 10.10 Pull Requests

#### PR-1: PR Creation

The platform SHOULD create PRs from reviewed worktrees/branches.

#### PR-2: PR Metadata

Each PR SHOULD track:

```text
provider
repo
number
title
body
branch
base
linked task
linked plan task
linked session/team
checks
review status
mergeability
conflicts
files changed
```

#### PR-3: Generated PR Body

The platform SHOULD draft PR title/body from task context, session summaries, changed files, and Project Brain evidence.

#### PR-4: Checks Monitoring

The platform SHOULD show checks status and allow failed checks to be sent to an agent fix session.

#### PR-5: Merge Guard

Merging MUST be high-risk and require explicit confirmation unless the user configures a policy.

---

### 10.11 Agent Teams

#### TEAM-1: Agent Team Object

The platform MUST model agent teams even if full team orchestration is P1.

#### TEAM-2: Team Roles

Teams SHOULD support roles such as:

- team lead;
- orchestrator;
- implementer;
- tester;
- reviewer;
- documenter.

#### TEAM-3: Team Topology

Agent Team View MUST show parent/child relationships and session ownership.

#### TEAM-4: Team Launch Recipe

Workflow Packs SHOULD define team launch recipes such as `/team-start <track>`.

#### TEAM-5: Team Controls

The UI SHOULD support:

- broadcast instruction;
- ask lead for status;
- pause all;
- open all terminals;
- collapse completed workers;
- end team;
- merge/reconcile outputs.

#### TEAM-6: Team Artifacts

The team view SHOULD show worktrees, branches, diffs, PRs, tests, session summaries, and events per worker.

---

### 10.12 Workflow Packs

#### WF-1: Workflow Pack Abstraction

The platform MUST support a generic Workflow Pack abstraction.

#### WF-2: Workflow Instance Distinction

The platform MUST distinguish between reusable Workflow Packs and project-specific Workflow Instances.

#### WF-3: Lifecycle States

Workflow states SHOULD include:

```text
available
installed
detected
needs_personalization
personalization_in_progress
generated_review_required
active
ready_for_team_run
degraded
drift_detected
upgrade_available
archived
detached
```

#### WF-4: Personalization Flow

Template-based packs SHOULD support personalization runs that infer values, ask the user questions, preview generated files, request approval, write files, and produce a manifest.

#### WF-5: Command Registry

The platform MUST expose detected commands in a command registry.

#### WF-6: Command Metadata

Commands SHOULD include:

```text
name
source
type
description
file_path
input_schema
supported_contexts
supported_harnesses
creates_sessions
modifies_files
requires_personalized_instance
risk_level
```

#### WF-7: cc-crew Support

The platform SHOULD support the user’s cc-crew scaffold as a Workflow Pack integration without making it mandatory.

#### WF-8: No Mandatory Scaffold

Basic projects without workflow packs MUST remain first-class.

---

### 10.13 Project Brain Integration

#### BRAIN-1: Embedded Drawer

The platform MUST include a Project Brain drawer or panel.

#### BRAIN-2: Standalone + Platform-Native

Project Brain should remain usable standalone but expose APIs/events that let the platform embed and coordinate it.

#### BRAIN-3: Evidence-Based Answers

Project Brain answers MUST include evidence where available:

- file paths;
- line anchors;
- commits;
- PRs;
- sessions;
- plan tasks;
- architecture sections;
- docs;
- decisions;
- events.

#### BRAIN-4: Scope Control

The drawer MUST let the user control scope:

```text
current project
current session
current file
current diff
current PR
entire portfolio
```

#### BRAIN-5: Action Planning

Project Brain SHOULD be able to propose action plans.

#### BRAIN-6: No Direct Privileged Execution

Project Brain MUST NOT directly execute privileged platform operations. It must request execution through the Action Gateway.

#### BRAIN-7: Memory Sources

Project Brain should ingest/index, with appropriate consent and policy:

- code;
- docs;
- architecture;
- plan files;
- session summaries;
- session transcripts;
- commits;
- PRs;
- tickets;
- decisions;
- events;
- workflow metadata.

#### BRAIN-8: Session Memory Privacy

Session transcript ingestion MUST be opt-in per project and should default to local embeddings/redaction.

---

### 10.14 Action Gateway

#### AG-1: Typed Action Requests

All platform actions SHOULD be represented as typed ActionRequests.

#### AG-2: Risk Levels

The Action Gateway MUST classify actions by risk.

Suggested levels:

```text
0 read-only
1 low-risk local UI/state
2 medium-risk local mutation
3 high-risk repo/integration mutation
4 critical/destructive/security-sensitive
```

#### AG-3: Approval Required

Medium/high/critical actions MUST require approval unless policy explicitly allows auto-approval.

#### AG-4: Preview / Dry Run

Actions SHOULD provide preview/dry-run output where possible.

#### AG-5: Bundled Action Plans

The gateway MUST support bundled plans with step-by-step approval.

#### AG-6: Audit Events

Every requested, approved, denied, executed, failed, or rolled-back action MUST emit events.

#### AG-7: Brain Client

Project Brain MUST be treated as a client of the Action Gateway, not as an executor.

#### AG-8: Remote Companion Client

Future iOS companion actions MUST route through the Action Gateway.

---

### 10.15 Event Model / Audit Trail

#### EVT-1: Durable Events

The platform MUST record durable events for important actions and state transitions.

#### EVT-2: Event Categories

Events SHOULD include categories:

- project;
- session;
- terminal;
- task;
- plan;
- workflow;
- agent team;
- git;
- PR;
- approval;
- action;
- Project Brain;
- integration;
- usage;
- remote companion;
- system.

#### EVT-3: Timeline Surfaces

Events MUST feed:

- bottom activity timeline;
- project timeline;
- session timeline;
- agent team timeline;
- audit log;
- Project Brain ingestion.

#### EVT-4: Correlation IDs

Actions and related events SHOULD share correlation IDs.

#### EVT-5: Replay/Reindex

Project Brain SHOULD be able to consume events for memory/retrieval and reindex derived memory if needed.

---

### 10.16 Observability Graph

#### OBS-1: Project Graph

The project home MUST show an operational graph or graph/list hybrid.

#### OBS-2: Node Types

Graph node types SHOULD include:

```text
project
session
agent_team
team_lead
worker
plan_task
github_issue
linear_ticket
worktree
branch
pull_request
approval
human_input
Project Brain evidence/memory source
```

#### OBS-3: Node Status

Nodes MUST show status and attention states.

#### OBS-4: Graph Filters

Graph filters SHOULD include:

- status;
- model;
- harness;
- execution profile;
- task source;
- branch;
- worktree;
- PR state;
- context usage;
- waiting on human;
- agent team.

#### OBS-5: Graph Actions

Users SHOULD be able to click graph nodes to open inspector, terminal, editor, task, PR, or action plan.

#### OBS-6: List Fallback

The graph MUST have list/table alternatives for dense scanning and accessibility.

---

### 10.17 Human Input Queue

#### HIQ-1: Global Queue

The platform MUST have a global Human Input Queue.

#### HIQ-2: Request Types

Request types SHOULD include:

- command permission;
- clarification needed;
- blocked agent;
- finding;
- deferment approval;
- architectural decision;
- merge approval;
- workflow personalization approval;
- Project Brain action approval.

#### HIQ-3: Context Required

Every queue item MUST show enough context to decide:

- requesting session/team;
- project;
- task;
- requested action;
- risk;
- evidence;
- recommended response;
- consequences.

#### HIQ-4: Fast Decisions

The user SHOULD be able to approve, deny, edit, or defer from the queue.

---

### 10.18 Usage / Context / Cost Tracking

#### USAGE-1: Per Session

The platform SHOULD track token/context/cost per session when data is available.

#### USAGE-2: Per Project

The platform SHOULD aggregate usage by project.

#### USAGE-3: Per Profile

The platform SHOULD aggregate usage by execution profile.

#### USAGE-4: Accuracy Labels

Usage metrics MUST indicate whether they are exact, estimated, or unavailable.

#### USAGE-5: Budget Alerts

The platform SHOULD support budget/context alerts.

---

### 10.19 Settings and Integrations

#### SET-1: GitHub Integration

The platform SHOULD support GitHub issues, PRs, checks, comments, and merges through authenticated local/desktop flows.

#### SET-2: Linear Integration

The platform SHOULD support Linear ticket browsing, linking, and eventual issue creation/update.

#### SET-3: Integration Health

Settings MUST show integration health and auth status.

#### SET-4: Project Policies

Settings SHOULD expose policies for:

- Project Brain transport;
- session memory opt-in;
- Action Gateway approvals;
- execution profile allowlists;
- remote companion access;
- secret redaction;
- auto-approval rules.

---

## 11. Project Brain-Specific Requirements Inside the Platform

Project Brain is both a standalone product and a platform-native subsystem.

### 11.1 Platform Role

Within the platform, Project Brain acts as:

- project memory;
- evidence retriever;
- implementation history engine;
- decision ledger;
- explanation engine;
- action planner;
- future automation recommender.

### 11.2 Core Questions

It should answer:

```text
When did we implement feature X?
How did we implement feature Y?
Why did we choose approach Z?
Which session changed this file?
Which commit/PR introduced this behavior?
What architecture section governs this area?
What task/plan item produced this change?
What changed since I last worked on this project?
What should I review first?
What is the next task in this plan?
```

### 11.3 Evidence Chips

Every answer should include clickable evidence chips when available:

```text
file:line
commit
PR
session
plan task
architecture anchor
decision
Linear ticket
GitHub issue
event
```

### 11.4 Action Planning

Project Brain should produce structured action plans, such as:

```text
Create a worktree for this plan task.
Start a Claude Code session using Profile A.
Send this task context to the session.
Link the session to Linear ENG-221.
Open the Code Review Workspace.
```

### 11.5 Action Boundaries

Project Brain may:

- search;
- explain;
- summarize;
- draft;
- recommend;
- propose;
- request actions.

Project Brain must not directly:

- run shell commands;
- mutate git;
- create/delete worktrees;
- push/merge;
- update tickets;
- write workflow files;
- change settings;
- access credentials;
- start sessions outside the platform.

### 11.6 Brain Drawer UX

The drawer should include:

- scope selector;
- conversation;
- evidence chips;
- suggested follow-up questions;
- action plan cards;
- approval handoff to Action Gateway;
- memory/source view;
- decision log.

---

## 12. Workflow Packs and cc-crew Requirements

### 12.1 Generic Workflow Pack Model

Workflow Packs are optional extensibility bundles.

They may contain:

- templates;
- commands;
- skills;
- subagents;
- hooks;
- plan parsers;
- launch recipes;
- personalization flows;
- upgrade flows;
- manifest schemas;
- expected docs/artifacts.

### 12.2 Template vs Instance

The platform must clearly distinguish:

```text
Workflow Pack: reusable template/package.
Workflow Instance: personalized/generated project-specific installation.
```

### 12.3 Personalization Required

A template-based pack may not be ready after detection. It may require:

- architecture doc;
- plan files;
- user answers;
- generated commands;
- generated agents;
- manifest creation;
- file review;
- commit.

### 12.4 cc-crew as First Integration

The user’s cc-crew scaffold should be supported as a first Workflow Pack integration, but the platform must not require it.

### 12.5 cc-crew Artifacts

The cc-crew integration should understand:

```text
ARCHITECTURE.md
MVP_TASKS.md / MVP_TASK.md
docs/planning/*.md
docs/layers/*
docs/learn-site/content.json
LESSONS.md
CLAUDE.md
.claude/commands/
.claude/skills/
.claude/agents/
.scaffolding/manifest.json
docs/briefs/
docs/sessions/
docs/team-handoffs/
```

### 12.6 cc-crew Commands

The command registry should expose commands such as:

```text
/team-start
/team-end
/orchestrate-start
/orchestrate-end
/session-start
/session-end
/tdd
/preflight
/run-tests
/check-arch
/wired
/context-check
/eval
/trace
/scaffold-generate
/scaffold-upgrade
```

Actual commands must be discovered from the project/workflow instance rather than hardcoded.

### 12.7 Team Start

The platform should expose `/team-start <track>` as an Agent Team launch recipe when the workflow instance is active and ready.

### 12.8 TDD Slice Tracker

For cc-crew sessions, the platform should optionally display the TDD lifecycle stage:

```text
0 Restate
1 Identify files
2 RED
2.5 Test-design review
3 Confirm RED
4 GREEN
5 Confirm GREEN
6 Refactor
7 Full suite
7.5 Reachability
8 Type + lint
9 Hot-route
10 Atomic commit
```

### 12.9 Escalation Mapping

cc-crew escalation categories should map into Human Input Queue items:

- critical/safety design question;
- finding;
- deferment approval;
- load-bearing architectural decision.

---

## 13. Security, Privacy, and Safety Requirements

### 13.1 Local-First Security

Raw code, terminals, local credentials, transcripts, and Project Brain indexes should remain local by default.

### 13.2 Secret Redaction

The platform should integrate with Project Brain redaction rules for transcript/session memory ingestion.

### 13.3 Credential Handling

The platform should avoid directly storing credentials where possible. It should use OS keychain or existing CLI auth contexts.

### 13.4 Dangerous Commands

The platform should detect and gate dangerous commands, including:

- deletion;
- force push;
- credential access;
- secret file reads;
- destructive filesystem operations;
- merge to protected branch;
- external network operations where policy requires approval.

### 13.5 Remote Companion Safety

Future remote access must be constrained:

- observability before control;
- no raw shell initially;
- Action Gateway required;
- high-risk actions require desktop confirmation;
- all remote activity audited.

### 13.6 Project Brain Safety

Project Brain action plans must show evidence, uncertainty, risk, and preview before execution.

---

## 14. UX Requirements

### 14.1 App Shell

The app shell should include:

```text
left project/session sidebar
main workspace
right inspector / Project Brain drawer
bottom activity timeline
command palette
global status area
```

### 14.2 Sidebar

The sidebar should show:

- workspace/global entries;
- projects;
- nested sessions;
- teams;
- attention indicators;
- active/waiting sorted first.

### 14.3 Right Inspector

Inspector changes based on selection:

- project;
- session;
- team;
- task;
- plan task;
- worktree;
- branch;
- PR;
- approval;
- workflow command;
- Project Brain evidence.

### 14.4 Project Brain Drawer

The drawer should be accessible globally and should not erase the selected context.

### 14.5 Command Palette

The command palette should expose platform actions, workflow commands, session actions, Project Brain actions, and navigation.

### 14.6 Empty States

Empty states should teach the product model:

- no projects;
- no sessions;
- no Project Brain index;
- no workflow instance;
- no task integration;
- no worktrees;
- no execution profiles;
- no PRs.

### 14.7 Degraded States

Degraded states must be visible:

- Project Brain stale;
- CodeGraph unavailable;
- integration auth expired;
- workflow personalization incomplete;
- execution profile offline;
- session usage unavailable;
- transcript ingestion disabled;
- remote companion disconnected.

### 14.8 Accessibility

Drag-and-drop must have non-drag alternatives. Graph views must have list/table alternatives.

---

## 15. MVP Scope

The MVP should prove the core loop:

> Add a local project, create isolated agent sessions, observe them, review changes, approve actions, and preserve enough context/evidence for Project Brain to help.

### MVP Must-Haves

```text
Desktop app shell
Local runner
Add local project
Project/session sidebar
Execution Profiles
Claude Code session launch
Codex-ready adapter model, even if Codex full support is P1
Worktree creation and linking
Embedded terminal
Session status tracking
Human Input Queue
Basic Action Gateway
Basic event timeline
Review-focused code editor
Changed files + diff view
Manual task/plan/session/worktree/PR linking
Project Brain drawer, read-only + draft/action-plan preview
Workflow Pack detection states
cc-crew detection at least
Plan file detection for MVP_TASKS.md/MVP_TASK.md
Basic Project Graph or graph/list hybrid
Git basics: status, commit, push, create PR if feasible
GitHub PR/issue read/link if feasible
Linear read/link if feasible
Usage/context display with exact/estimated/unavailable labels
Settings for profiles, projects, integrations, policies
```

### MVP Nice-to-Haves

```text
Codex CLI launch
PR checks monitoring
Failed-checks-to-agent flow
/team-start launcher if cc-crew instance active
Command registry execution
Session summaries into Project Brain
Ask-agent-on-code-selection
Native desktop notifications
```

### MVP Non-Goals

```text
Full IDE replacement
Cloud-hosted runner
Multi-user collaboration
Bidirectional Linear sync
Fully autonomous merges
Mobile companion
Full cc-crew upgrade system
Cross-project graph moat
Full transcript commit-linking
```

---

## 16. P1 Scope

P1 should deepen the differentiated workflows.

```text
Codex full adapter
Agent Team View
Workflow personalization UI
cc-crew /team-start integration
TDD slice tracker
PR checks + agent fix flows
One-way Linear creation from plan tasks
Project Brain action plans through Action Gateway
Session summaries and episode cards
Workflow Command Registry maturity
Conflict resolver
Project graph filters
Usage budgets
Remote notification groundwork
```

---

## 17. P2 / Stretch Scope

```text
Companion iOS app
Remote observability
Remote low-risk approvals
Bidirectional Linear sync with conflict resolution
Workflow Pack marketplace/import
Workflow upgrade previews
Cross-project Project Brain impact lens
Multi-user/team mode
Policy automation
Cloud/remote runner optionality
Advanced analytics/evals
```

---

## 18. Companion iOS Stretch Goal

The iOS app is a stretch goal and should not drive MVP architecture beyond ensuring event/action APIs are clean.

### 18.1 Initial Companion Capabilities

```text
View project/session status
Receive push notifications for human-input-needed
View session summaries
View PR/check state
View usage/context alerts
Approve/deny low-risk actions
Send short instruction to selected session only via Action Gateway policy
```

### 18.2 Initial Companion Non-Goals

```text
Raw terminal shell
Full code editor
High-risk git actions without desktop confirmation
Credential management
Workflow personalization
Bulk automation
```

### 18.3 Architecture Implication

Remote companion requires a secure relay or tunnel model later. MVP should not depend on it.

Potential future approaches:

- local desktop app publishes encrypted status to user-controlled relay;
- push notification provider for attention events;
- desktop approves paired devices;
- all remote control requests become ActionGateway ActionRequests;
- high-risk actions require desktop-side confirmation.

---

## 19. Success Metrics

### 19.1 Activation

```text
Time to add first project
Time to create first execution profile
Time to launch first session
Time to create first worktree-backed session
Time to open first review diff
```

### 19.2 Operational Value

```text
Number of sessions supervised per project
Human-input requests resolved from queue
Blocked/stale sessions detected before user manually finds them
Tasks linked to sessions/worktrees/PRs
PRs created from agent sessions
```

### 19.3 Trust

```text
% actions with preview before execution
% risky actions routed through Action Gateway
Audit event completeness
Project Brain answer evidence click-through
User-reported confidence in Brain answers
```

### 19.4 Review Quality

```text
Time from changes-ready to reviewed
Failed checks routed back to agent
Diff review actions taken
Merge conflict detection/resolution
```

### 19.5 Project Brain

```text
Questions answered with evidence
Feature-history queries answered
Session summaries indexed
Actions proposed and approved
Actions proposed and denied
```

### 19.6 Performance

```text
App launch time
Project scan time
Terminal attach latency
Graph render time
Diff open latency
Event write latency
Project Brain drawer response latency
```

---

## 20. Risks and Mitigations

### RISK-1: Product Scope Too Large

Mitigation: MVP must focus on project/session/worktree/terminal/review/Action Gateway basics.

### RISK-2: Terminal/Harness Status Detection Is Brittle

Mitigation: adapter capability model, version-tolerant parsers, degraded states.

### RISK-3: Git Automation Can Be Dangerous

Mitigation: Action Gateway risk classification, previews, confirmations, audit logs, rollback where possible.

### RISK-4: Project Brain Could Become Too Powerful Too Early

Mitigation: read-only and draft modes first; confirmed actions next; policy automation later.

### RISK-5: Workflow Packs Could Overfit cc-crew

Mitigation: generic Workflow Pack abstraction and Basic Project / Claude-Aware Project / Workflow-Pack Project modes.

### RISK-6: Execution Profiles Could Look Like Subscription Circumvention

Mitigation: explicit user-owned profiles, no hidden routing, usage transparency, project allowlists.

### RISK-7: Code Editor Scope Creep

Mitigation: review-focused editor first; external IDE integration; no full IDE replacement in MVP.

### RISK-8: Mobile Companion Security

Mitigation: observability first, no raw shell, Action Gateway required, high-risk desktop confirmation.

### RISK-9: Project Brain Session Memory Privacy

Mitigation: opt-in per project, local embeddings by default, redaction, exclude thinking blocks, explicit cloud consent.

### RISK-10: Graph Becomes Decorative

Mitigation: every graph node must have actions, status, inspector, filters, and list fallback.

---

## 21. Open Questions

```text
What desktop stack should be used?
How should the local runner communicate with the UI?
Which harness launches in MVP: Claude Code only or Claude Code + Codex?
How reliable is Codex transcript/session association?
How should execution profiles map to local authenticated contexts?
How much terminal state can be recovered after restart?
What is the minimal useful code editor?
What Project Brain APIs are needed for MVP?
What Action Gateway actions are P0?
What event storage backend should the platform use?
How should remote companion pairing work later?
What is the final product name?
What exactly is the first demo workflow?
```

---

## 22. Architecture Inputs and Constraints

The architecture draft should account for:

```text
Desktop app shell
Local runner/process manager
Terminal manager
Harness adapter layer
Project registry
Execution profile manager
Worktree/git manager
Task integration adapters
Project Brain client/API
Action Gateway
Event store/audit log
Workflow Pack runtime
Code editor/review service
Usage/context collector
Settings/policy store
Future remote companion API
```

Important constraints:

```text
Local-first by default
No cloud runner in MVP
No raw remote shell in stretch companion MVP
Project Brain indexes remain local by default
Action Gateway owns execution permissions
Workflow Packs are optional
cc-crew is first-class but not required
Session is the atomic operational unit
Every important state change emits an event
```

---

## 23. Artifact Dependency Map

This PRD is the umbrella product requirements document. More detailed requirements live in the supporting artifacts.

```text
Project Brain PRD v2
  Detailed memory/retrieval/action-planning requirements.

Product Canon
  Product thesis, principles, scope, and vision.

Shared Object Model
  Definitions and relationships for all core nouns.

Action Gateway Spec
  Typed actions, risk, approvals, execution, audit.

Event Model and Audit Trail Spec
  Durable event taxonomy and timeline requirements.

Desktop-First Runtime Addendum
  Desktop/local-runner and remote companion implications.

Workflow Packs Spec
  Generic workflow-pack abstraction.

cc-crew Workflow Pack Integration Spec
  Specific scaffold integration, personalization, commands, /team-start, /tdd.

UX / IA Spec
  Navigation, app shell, screens, states, and behavior.

UI Component Inventory
  Component list for design system and prototype.

Claude Design Prototype Prompt
  Design handoff prompt for full prototype generation.
```

---

## 24. Recommended Next Architecture-Draft Prompt Inputs

When invoking the architecture-draft skill, use this PRD plus the artifact bundle and ask it to produce:

```text
1. System context diagram
2. Desktop app architecture
3. Local runner architecture
4. Process/session/terminal lifecycle architecture
5. Harness adapter architecture
6. Action Gateway architecture
7. Event store/audit log architecture
8. Workflow Pack runtime architecture
9. Project Brain integration architecture
10. Git/worktree/PR architecture
11. Settings/policy/security architecture
12. Future iOS companion architecture options
13. MVP technical slice
14. Risks/spikes
15. Open architecture decisions
```

---

## 25. Appendix: MVP Demo Scenario

A strong MVP demo should be:

```text
1. User opens desktop app.
2. Adds a local project.
3. Platform detects git repo, Project Brain state, and cc-crew workflow signals.
4. User creates an execution profile for Claude Code.
5. User opens Plan View and selects a plan task.
6. Platform creates a worktree and starts a Claude Code session.
7. Session appears in sidebar and project graph.
8. Agent requests permission; Human Input Queue surfaces it.
9. User approves through Action Gateway.
10. Agent edits files.
11. User opens Code Review Workspace and reviews diff.
12. User asks Project Brain why a related feature was implemented a certain way.
13. Brain answers with evidence chips.
14. User asks Brain to create a PR.
15. Brain proposes an action plan.
16. User approves.
17. Platform creates PR, records events, and links task/session/worktree/PR.
```

This proves the core thesis without requiring every advanced feature.

---

*End of Main Platform PRD v0.1.*
