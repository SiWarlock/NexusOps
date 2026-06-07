# AI Engineering Control Plane — UX & Information Architecture Spec v0.1

> Status: Draft for product/design alignment  
> Date: 2026-06-07  
> Naming: parent platform name is intentionally unresolved. Use **AI Engineering Control Plane** or **the platform** as a neutral working label.  
> Important design constraint: **do not include a color taxonomy** in this spec. Use semantic status treatments, density, hierarchy, iconography, labels, motion, and layout patterns. A visual system can later map statuses to colors.

---

## 1. Purpose

This document defines the product behavior, information architecture, user flows, screens, and components needed to create a comprehensive Claude Design prototype for the desktop AI engineering control plane.

The goal of the prototype is not merely to show a dashboard. It should represent the full product model:

- Desktop-first local app
- Multi-project AI coding operations console
- Claude Code / Codex terminal orchestration
- Code editor and diff review workspace
- Project Brain drawer
- Workflow Packs and project-specific Workflow Instances
- cc-crew-style implementation plans and `/team-start` agent teams
- Execution Profiles for multiple Claude/Codex accounts/subscriptions
- Git worktrees, branches, PRs, checks, merges, and conflicts
- GitHub / Linear task intake and linking
- Action Gateway approvals and auditability
- Token/context/usage visibility
- Event timeline and observability graph
- Stretch-goal companion iOS remote observability and controlled action approval

The intended output from Claude Design is a prototype that makes the platform feel like a real desktop product, not a generic SaaS dashboard or chatbot UI.

---

## 2. Product definition

The platform is a **desktop-first AI engineering control plane** for dispatching, supervising, reviewing, and merging work from Claude Code, Codex, and future agent harnesses across many projects.

It combines:

1. **Project operations** — projects, repos, local paths, integrations, workflow status.
2. **Task intake** — GitHub issues, Linear tickets, implementation plan tasks, ad hoc prompts.
3. **Agent orchestration** — Claude Code sessions, Codex sessions, agent teams, team leads, workers.
4. **Execution visibility** — terminals, session status, current command, context usage, token usage, approvals.
5. **Code review** — file tree, changed files, editor, inline diff, side-by-side diff, conflict resolver.
6. **Delivery control** — worktrees, branches, commits, PRs, checks, review state, merge flow.
7. **Project memory** — Project Brain retrieval, evidence chips, decisions, session memory, implementation history.
8. **Workflow system** — Workflow Packs, Workflow Instances, commands, plan parsers, launch recipes.
9. **Action safety** — typed actions, previews, approvals, risk levels, audit trail.

### 2.1 What the product is

The product is:

- A local desktop cockpit for managing multiple AI coding agents.
- A terminal multiplexer with project/task/branch/worktree awareness.
- A code review and diff workspace for human supervision.
- A project observability surface that shows agents, sessions, teams, branches, tickets, and PRs as connected objects.
- A task router that turns Linear/GitHub/plan work into agent sessions.
- A workflow launcher for custom command-based systems such as cc-crew.
- A Project Brain shell that can answer project questions and propose safe actions.

### 2.2 What the product is not

The product is not:

- A normal IDE replacement in v1.
- A chatbot-first coding app.
- A hosted SaaS control plane.
- A direct remote shell exposed to mobile.
- A generic project-management board.
- A decorative graph with no operational meaning.
- A hidden automation layer that mutates repos or tickets without approval.

---

## 3. Desktop-first product stance

The MVP surface is a desktop app.

The desktop app owns or coordinates:

- Local terminal sessions
- Local filesystem access
- Local worktrees and branches
- Local git operations
- Local Claude/Codex execution profiles
- Local Project Brain index access
- Local workflow-pack detection and personalization
- Local event log and audit trail
- Local credential boundaries

A web app is not the MVP product surface.

### 3.1 Desktop shell implications

The design should assume:

- Multi-pane desktop layout.
- Native-feeling app chrome or a custom titlebar.
- Persistent left navigation.
- Main workspace with tabs/views.
- Right inspector and Project Brain drawer.
- Bottom event timeline/status bar.
- Keyboard-first command palette.
- Terminal and editor surfaces that feel native to developer workflows.
- Local runtime and connectivity indicators.

### 3.2 iOS companion stretch goal

The iOS companion is not MVP. It should be represented only as a future settings/remote access concept and optional mobile mock screens.

Remote observability should come before remote control.

The iOS companion may eventually support:

- Viewing active projects and sessions.
- Seeing waiting-on-human input.
- Receiving push notifications for approvals.
- Reading session summaries.
- Approving safe/medium-risk actions.
- Opening PR/check summaries.
- Asking Project Brain project-state questions.

The iOS app must not become a direct remote terminal or unrestricted command runner. Any remote action should route through the Action Gateway.

---

## 4. Core mental model

The user is not merely chatting with AI. The user is acting like an engineering lead supervising a distributed team of AI coding workers.

The product should answer these questions instantly:

- What is running?
- What is blocked?
- What needs me?
- What changed?
- What is risky?
- Which task caused this work?
- Which session made this change?
- Which worktree/branch contains it?
- Which PR is it headed toward?
- Which model/account/profile is paying for it?
- What did Project Brain find as supporting evidence?
- What can I safely approve, reject, merge, or archive?

### 4.1 Atomic operational unit

The **Session** is the atomic operational unit.

A Session can be connected to:

- Project
- Repository
- Worktree
- Branch
- Terminal
- Task / PlanTask / Ticket / Issue
- Harness, such as Claude Code or Codex CLI
- Execution Profile
- Model
- Transcript
- Tool calls
- Approvals
- Files changed
- Commits
- Pull Request
- Project Brain episode card
- Events

Every UI surface should preserve this chain of ownership.

### 4.2 Canonical work path

The default work path is:

```text
Task / PlanTask / Issue / Ticket
  → Dispatch
  → Session or Agent Team
  → Worktree + Branch
  → Terminal execution
  → Code changes
  → Tests/checks
  → Human review
  → Commit
  → PR
  → Merge
  → Archive
  → Project Brain memory
```

The prototype should make this path visible across screens.

---

## 5. UX principles

### 5.1 Attention first

The UI should prioritize what needs human attention over what is merely active.

Suggested attention order:

1. Human input required
2. Failed or blocked
3. High-risk approval pending
4. Merge conflict / failing checks
5. High context or high cost
6. Active
7. Idle
8. Completed
9. Archived

### 5.2 Preserve native agent surfaces

Claude Code and Codex sessions should remain accessible as real terminal surfaces. The platform wraps the terminal with context, status, approvals, diffs, and actions.

### 5.3 Never separate code from context

When reviewing code, show:

- Session that produced it
- Task or plan task that caused it
- Worktree and branch
- PR/check state
- Files changed
- Relevant Project Brain evidence
- Approval history

### 5.4 Every object shows ownership

Every object row/card/node should show enough metadata to avoid ambiguity:

- Project
- Session/team owner
- Harness/model/profile
- Worktree/branch
- Linked task/ticket/plan anchor
- Current status
- Last activity

### 5.5 Risk is visible

The user should see risk states before taking action:

- High-risk command
- Uncommitted dirty worktree
- Failing checks
- Conflict
- Stale Project Brain index
- Session context near limit
- Unknown execution profile state
- Workflow instance not ready
- Low-confidence session/project association
- Unverified Project Brain claim

### 5.6 Completion is explicit

A session should not simply disappear. Completion should produce:

- Summary
- Files changed
- Tests run
- Commits/PRs
- Known issues
- Recommended next action
- Archive/delete worktree option
- Project Brain indexing state

### 5.7 Graph plus list

Graphs are useful for relationships; lists are useful for scanning. Every graph-heavy surface should have list/table alternatives or side panels.

### 5.8 Command-first, not command-only

Power users should be able to use command palette and keyboard shortcuts. Every command should also exist as a visible UI action for discoverability and accessibility.

---

## 6. Top-level information architecture

### 6.1 Persistent desktop shell

```text
Desktop Window
├─ Titlebar / Workspace switcher / Sync + runtime indicators
├─ Left sidebar
│  ├─ Global command/search
│  ├─ Human Input Queue shortcut
│  ├─ Projects tree
│  ├─ Task Inbox
│  ├─ PRs / Review
│  ├─ Usage
│  └─ Settings
├─ Main workspace
│  ├─ Active screen content
│  ├─ Optional internal tabs
│  └─ Optional split panes
├─ Right panel stack
│  ├─ Inspector
│  └─ Project Brain drawer
├─ Bottom event timeline / activity rail
└─ Status bar
   ├─ Local runtime
   ├─ Project Brain index state
   ├─ Git branch/worktree
   ├─ Active profile
   ├─ Token/context summary
   └─ Last sync
```

### 6.2 Left sidebar hierarchy

```text
Workspace
  Command Center
  Human Input Needed
  Task Inbox
  PRs / Review
  Usage

Projects
  ▾ Project A
      ▾ Waiting
          Session rows
      ▾ Active
          Session rows
      ▾ Agent Teams
          Team rows
      ▾ Idle
          Session rows
      Plan
      Code
      Worktrees
      PRs
      Workflow
      Brain
  ▾ Project B
      ...

Settings
  Execution Profiles
  Integrations
  Workflow Packs
  Project Brain
  Remote Access
  Security / Policies
```

### 6.3 Project-level tabs

When a project is selected, the main area should support these project tabs:

1. **Overview** — observability graph + attention cards.
2. **Sessions** — table/board of sessions.
3. **Plan** — implementation plan / `MVP_TASKS.md` / plan-task linking.
4. **Code** — editor and changed files workspace.
5. **Tasks** — GitHub/Linear/ad hoc tasks scoped to project.
6. **Worktrees** — worktree/branch table.
7. **PRs** — pull requests, checks, review/merge state.
8. **Workflow** — Workflow Pack/Instance setup and command registry.
9. **Brain** — Project Brain index, memory sources, decisions, Q&A.
10. **Settings** — repo path, profiles, policies, integrations.

### 6.4 Right panel stack

The right side has two major modes:

```text
Inspector
  Details and actions for the currently selected object.

Project Brain
  Project-aware reasoning, evidence, memory, action planning, and suggested actions.
```

The right panel can be:

- Closed
- Inspector only
- Project Brain only
- Split inspector + Project Brain stacked
- Floating drawer overlay for Project Brain

### 6.5 Bottom event timeline

The bottom rail shows the current scope’s recent events:

- Workspace scope on Command Center
- Project scope on Project Overview
- Session scope on Session Terminal
- Team scope on Agent Team View
- PR scope on PR Review
- Action scope during Action Gateway flows

Events should be filterable by:

- Human input
- Sessions
- Git
- PRs
- Workflow
- Project Brain
- Errors
- Usage
- Security/policy

---

## 7. Core object surfaces

Each object should appear consistently across rows, cards, graph nodes, inspector panels, and search results.

### 7.1 Project

Primary display:

- Name
- Repo/provider
- Local path
- Workflow state
- Project Brain index freshness
- Active/waiting sessions
- Open PRs
- Tokens/cost today
- Integrations connected

### 7.2 Session

Primary display:

- Session name
- Status
- Harness
- Model
- Execution Profile
- Project
- Task/PlanTask/Ticket
- Worktree
- Branch
- Context usage
- Token usage
- Last activity
- Current action/command
- PR/check state if available

### 7.3 Agent Team

Primary display:

- Team objective
- Lead/orchestrator
- Worker count
- Worker statuses
- Track/phase/plan anchor
- Linked worktrees/branches
- Context tiers
- Escalations
- Combined output/PR state

### 7.4 Plan Task

Primary display:

- Phase / track / task title
- Status
- Architecture anchors
- Files expected/touched
- Linked Linear/GitHub item
- Linked sessions/teams
- Linked PRs
- Acceptance criteria
- Last update

### 7.5 Workflow Instance

Primary display:

- Workflow Pack name
- Instance status
- Personalization state
- Manifest path
- Generated from version/SHA
- Commands available
- Plan parser status
- Team launcher readiness
- Upgrade/drift state

### 7.6 Action Request

Primary display:

- Proposed by
- Action summary
- Risk level
- Target project/session/worktree/ticket
- Preview/dry-run result
- Required approval
- Timeout/expiration
- Evidence references
- Approve / deny / edit buttons

---

## 8. Status models for UI

Do not implement these as color names. Use semantic status labels, icons, badges, row ordering, emphasis, and motion.

### 8.1 Session statuses

```text
Creating
Starting
Active
Thinking
Running command
Editing files
Running tests
Waiting on permission
Waiting on human input
Waiting on external service
Idle
Stale
Failed
Completed
Archived
Killed
```

### 8.2 Task statuses

```text
Unassigned
Queued
Assigned
In progress
Blocked
Needs clarification
Changes ready
PR opened
Needs review
Requested changes
Merged
Closed
Abandoned
```

### 8.3 Worktree statuses

```text
Clean
Dirty
Untracked files
Conflicts
Behind base
Ahead of base
PR open
Merged
Prunable
Locked
Deleted
```

### 8.4 PR statuses

```text
Draft
Open
Checks pending
Checks failing
Needs review
Changes requested
Approved
Mergeable
Conflict
Merged
Closed
```

### 8.5 Workflow Instance statuses

```text
Not detected
Pack available
Needs personalization
Personalization in progress
Generated, review required
Active
Ready for team mode
Degraded
Drift detected
Upgrade available
Archived
Detached
```

### 8.6 Project Brain statuses

```text
Not configured
Indexing
Ready
Partial index
Stale
Graph degraded
Transcript ingestion off
Transcript ingestion active
Reindex required
Error
```

### 8.7 Approval statuses

```text
Requested
Previewed
Awaiting approval
Approved
Denied
Expired
Cancelled
Executing
Succeeded
Failed
Partially succeeded
Rollback available
Rolled back
```

### 8.8 Execution Profile statuses

```text
Available
Active
In use
Rate limited
Auth expired
Misconfigured
Disabled
Unknown
```

---

## 9. Screen specifications

Each screen below should be represented in the prototype or at minimum in the UI kit as a realistic state.

---

### Screen 1 — First Launch / Desktop Setup Wizard

**Purpose:** Configure the local machine so the app can safely orchestrate sessions, repositories, integrations, Project Brain, and workflows.

**Primary question:** “Is my local machine ready to run this product safely?”

**Layout:** Centered wizard inside desktop app shell.

**Steps:**

1. Welcome and product framing
2. Local runtime check
3. Claude Code / Codex detection
4. Execution Profiles setup
5. Project Brain setup
6. Git/GitHub/Linear connections
7. Workflow Pack library
8. Security and approval policy
9. Finish / Add first project

**Required components:**

- Setup progress stepper
- Runtime check row
- Host config permission card
- Execution Profile card
- Integration connection card
- Project Brain store card
- Security policy selector
- Consent prompt
- Repair action
- Continue/skip/back actions

**Important states:**

- All checks passed
- Tool missing but installable
- Tool missing and manual action required
- Auth expired
- Host config write requires consent
- Project Brain partial setup
- Setup repair available

**Prototype scenario:** Show a state where Claude Code is detected, Codex needs authentication, GitHub is connected, Linear is not connected, Project Brain store is ready, and Workflow Packs are available.

---

### Screen 2 — Global Command Center

**Purpose:** Workspace-wide situational awareness.

**Primary question:** “What needs my attention across all projects?”

**Layout:**

```text
Left sidebar + main dashboard + right inspector + bottom event timeline
```

**Main sections:**

1. Needs My Attention
2. Active Work
3. Agent Teams
4. Recently Completed
5. Risky / Stale
6. Open PRs
7. Usage Hotspots
8. Project Brain / index health

**Required components:**

- Attention summary cards
- Project activity group
- Session cards
- Agent team cards
- PR cards
- Token/context usage meters
- Human input cards
- Event timeline
- Right inspector for selected object

**Required data fields:**

- Active sessions count
- Waiting sessions count
- Open PR count
- Failing checks count
- Tokens today
- Cost today if available
- Profiles currently in use
- Project Brain index health

**Actions:**

- New session
- New agent team
- Open Human Input Queue
- Open Task Inbox
- Ask Project Brain across workspace
- Sync all integrations
- Open selected project/session/PR

**Empty state:** No active sessions; show “Add project,” “Connect GitHub/Linear,” and “Start first session.”

**Error state:** Local runtime unavailable; show limited read-only mode and repair action.

---

### Screen 3 — Project Home / Observability Graph

**Purpose:** The central project control surface.

**Primary question:** “What is happening inside this project right now?”

**Layout:**

```text
Project header
Graph toolbar + filters
Central graph canvas
Right inspector
Bottom event timeline
Optional left list overlay
```

**Project header fields:**

- Project name
- Repo/provider
- Local path/worktree root
- Workflow Instance status
- Project Brain status
- Active sessions
- Waiting sessions
- Open PRs
- Tokens/context summary

**Graph node types:**

- Project
- Session
- Agent Team
- Team lead
- Orchestrator
- Worker
- Task
- PlanTask
- GitHub Issue
- Linear Ticket
- Worktree
- Branch
- Pull Request
- Human input required
- Approval
- Project Brain memory/source
- Workflow command

**Graph edges:**

- Project owns session
- Session assigned task
- Session uses worktree
- Worktree tracks branch
- Branch opens PR
- Team lead spawned worker
- PlanTask maps to architecture anchor
- Ticket linked to PlanTask
- Approval blocks session
- Project Brain evidence supports task/session/PR

**Graph filters:**

- Status
- Object type
- Model/harness
- Execution Profile
- Workflow Pack
- Plan phase/track
- Ticket source
- Branch/worktree
- PR state
- Human input needed
- High context usage
- Stale/failing only

**Graph interactions:**

- Click node → right inspector
- Double-click session → open terminal
- Double-click worktree/branch → open code/diff
- Double-click PR → open PR review
- Drag task/plan task onto project → create new session
- Drag task onto team → start/delegate to team
- Drag task onto session → add to existing session
- Use lasso/select multiple sessions → bulk actions
- Collapse completed sessions
- Toggle graph/list split

**Empty state:** Project added, no sessions yet. Show task inbox, plan tasks, and “Start session.”

**Error state:** Graph unavailable; fall back to project session list and show graph-degraded notice.

---

### Screen 4 — Project Sessions List / Board

**Purpose:** Scannable alternative to graph view.

**Primary question:** “Which sessions are active, blocked, idle, or done?”

**Layouts:**

- Table view for density
- Board grouped by status
- Tree view grouped by project → team → session

**Columns:**

- Status
- Session
- Harness/model
- Execution Profile
- Task/PlanTask
- Worktree
- Branch
- Context
- Tokens
- Last activity
- PR
- Actions

**Actions:**

- Attach terminal
- Open editor/diff
- Ask for status
- Send message
- Pause/resume/kill
- Summarize
- Archive
- Create PR

**Important variants:**

- Single Claude Code session
- Single Codex session
- Team session with workers
- Stale session
- Failed session
- Waiting on approval
- Completed session with summary

---

### Screen 5 — Session Terminal View

**Purpose:** Native terminal session wrapped with operational context.

**Primary question:** “What is this agent doing and how can I intervene?”

**Layout:**

```text
Session header
Terminal tabs/main terminal
Right inspector
Bottom message composer
Bottom event timeline or compact session log
```

**Session header fields:**

- Project
- Session name
- Status
- Harness
- Model
- Execution Profile
- Context usage
- Token usage
- Worktree
- Branch
- Linked task/ticket/plan task
- PR state

**Right inspector sections:**

- Task summary
- Current activity
- Pending approvals
- Recent tool calls
- Files changed
- Test/check output
- Linked plan/ticket
- Git state
- Actions

**Composer behavior:**

- Send instruction to agent
- Include selected file/diff/task context
- Attach ticket/plan task
- Use command from registry
- Send to one session or team broadcast

**Approval behavior:**

Approval prompts should appear as structured cards outside terminal text:

- Requested command/action
- Reason/context
- Risk classification
- Preview/dry-run if available
- Approve
- Deny
- Edit instruction
- Approve once / approve for session / always ask

**Important states:**

- Running command
- Waiting on permission
- Waiting on human clarification
- Context near limit
- Terminal disconnected
- Session completed
- Session failed

---

### Screen 6 — Code Editor / Diff Review Workspace

**Purpose:** Human review and intervention surface for agent-produced code.

**Primary question:** “What changed, why, and is it safe?”

**Layout:**

```text
Project/session header
Left file explorer + changed files
Main editor/diff area
Right inspector / Project Brain drawer
Bottom problems/tests/output panel
```

**Required modes:**

- File view
- Inline diff
- Side-by-side diff
- Changed files review
- Conflict resolver
- Test output
- Problems/diagnostics
- PR review mode

**Required components:**

- File explorer
- Changed files tree
- Editor tab bar
- Editor canvas
- Diff hunk
- Hunk action bar
- Conflict block
- Test output panel
- Problems panel
- Code selection action menu
- Agent comment thread
- Review checklist

**Code selection actions:**

- Ask Project Brain to explain
- Ask active agent to explain
- Ask active agent to fix
- Ask active agent to add tests
- Send selected code to session
- Create review comment
- Link to PlanTask
- Show implementation history
- Show related sessions

**Diff hunk actions:**

- Accept hunk
- Reject hunk
- Request agent change
- Ask why this changed
- Add comment
- Link to architecture anchor

**Context strip:**

Always show:

- Session owner
- Task/PlanTask
- Worktree
- Branch
- PR
- Files changed
- Review state

**Empty state:** No file selected; show changed files and suggested review order.

**Error state:** Worktree path unavailable; show reconnect/open local path action.

---

### Screen 7 — Plan View / Implementation Plan

**Purpose:** Turn implementation plans into trackable work and agent dispatch.

**Primary question:** “What should be built next and what is its status?”

**Sources:**

- `MVP_TASKS.md`
- `MVP_TASK.md`
- Architecture docs
- Workflow Pack plan parser
- Manual plan/task import

**Layout:**

```text
Plan header
Phase/track tree
Task detail pane
Linked objects panel
Bottom plan activity log
```

**Plan header fields:**

- Plan file path
- Parser status
- Last parsed
- Architecture doc link
- Workflow Instance
- Current phase
- Completion summary
- Linear/GitHub link coverage

**Plan hierarchy:**

```text
Current state
Carry-forward
Phase
  Track
    PlanTask
      Acceptance criteria
      Files
      Cross-doc invariant
      Architecture anchors
      Linked sessions
      Linked PRs
```

**Task row fields:**

- Checkbox/status
- Task title
- Phase/track
- Architecture anchors
- Linked Linear/GitHub item
- Linked sessions/team
- Worktree/branch
- PR
- Last updated

**Actions:**

- Start session from task
- Start agent team from phase/track
- Link Linear/GitHub item
- Create Linear issue from task
- Create GitHub issue from task
- Open architecture anchor
- Ask Project Brain about task
- Update task status
- Add carry-forward item
- Save decision

**Important behavior:**

Manual linking is P0. One-way creation is P1. Controlled bidirectional sync is P2.

**Empty state:** No implementation plan detected. Offer “Create/import plan,” “Use Workflow Pack parser,” or “Continue without plan.”

**Error state:** Plan parser found ambiguous structure; show raw file preview and mapping suggestions.

---

### Screen 8 — Task Inbox / GitHub + Linear

**Purpose:** Convert external work into local agent sessions and plan links.

**Primary question:** “What work is available, assigned, blocked, or ready for review?”

**Tabs:**

- GitHub Issues
- Linear Tickets
- PRs
- Plan Tasks
- Assigned to Agents
- Needs Review
- Completed

**Task card fields:**

- Source icon/name
- ID
- Title
- Priority
- Labels
- Assignee
- Repo/project
- Status
- Linked plan task
- Linked session/team
- Suggested harness/profile
- Suggested worktree/branch

**Task detail panel:**

- Description
- Acceptance criteria
- Comments
- Linked docs/architecture
- Linked plan task
- Suggested dispatch prompt
- Suggested workflow command
- History/events

**Drag/drop behavior:**

- Drop on project → new session
- Drop on session → add context to session
- Drop on team → delegate/decompose
- Drop on worktree → start/continue in worktree
- Drop into terminal composer → insert structured task context
- Drop into Plan View → link to plan task

**Actions:**

- Dispatch to Claude Code
- Dispatch to Codex
- Start agent team
- Link to plan task
- Create plan task from ticket
- Create ticket from plan task
- Ask Project Brain for context
- Mark blocked / needs review

---

### Screen 9 — Worktree / Git / PR Control Center

**Purpose:** Manage parallel code work safely.

**Primary question:** “Which branches/worktrees/PRs are safe, dirty, blocked, or mergeable?”

**Layout:**

```text
Top summary
Worktree table/cards
Branch/commit panel
PR lanes
Right inspector
Bottom git event log
```

**Worktree fields:**

- Name/path
- Branch
- Base branch
- Status
- Dirty files
- Linked session/team
- Linked task/plan task
- Last commit
- PR
- Checks
- Risk state

**PR lanes:**

- Draft
- Checks pending
- Checks failing
- Needs review
- Approved
- Mergeable
- Conflict
- Merged

**Git actions:**

- Create worktree
- Delete/archive worktree
- Lock worktree
- Create branch
- Checkout branch
- Pull/rebase from base
- Merge main
- Commit
- Push
- Create PR
- Update PR
- Merge PR
- Open conflict resolver
- Request agent fix

**Safety behavior:**

High-risk actions must route through Action Gateway:

- Delete worktree
- Push
- Force push
- Merge
- Modify protected branch
- Resolve conflicts with agent

**Empty state:** No worktrees beyond main. Show “Create worktree” and “Start session from task.”

---

### Screen 10 — PR Review Workspace

**Purpose:** Review, repair, and merge agent-produced work.

**Primary question:** “Can this PR safely merge?”

**Layout:**

```text
PR header
Checks/reviews summary
Changed files + diff
Right inspector
Bottom PR event timeline
```

**Header fields:**

- PR number/title
- Branch/base
- Author/session/team
- Linked task/plan task/ticket
- Review state
- Checks state
- Mergeability
- Risk summary

**Panels:**

- Description/summary
- Checks
- Reviews/comments
- Changed files
- Test output
- Project Brain evidence
- Agent session summary

**Actions:**

- Ask Project Brain to review
- Ask agent to fix failing checks
- Request changes
- Approve
- Merge
- Squash merge
- Rebase
- Update PR body
- Comment on PR
- Link to plan/ticket

---

### Screen 11 — Agent Team View

**Purpose:** Supervise lead/orchestrator/worker teams.

**Primary question:** “What is this team doing and how are workers coordinated?”

**Layout:**

```text
Team header
Lead/orchestrator panel
Worker grid/list
Team graph
Terminals dock/popovers
Artifacts panel
Escalation panel
Bottom team timeline
```

**Header fields:**

- Objective
- Source task/plan phase/track
- Workflow command
- Team status
- Active workers
- Waiting workers
- Context tiers
- Linked branches/PRs

**Lead/orchestrator panel:**

- Current plan
- Delegated tasks
- Last coordination message
- Escalations
- Ask lead for status

**Worker cards:**

- Role
- Subtask
- Status
- Harness/model/profile
- Context tier
- Worktree/branch
- Files touched
- Terminal attach
- Current command
- PR/diff state

**Team graph:**

- Lead → orchestrator → implementers
- Workers → worktrees/branches
- Branches → PRs
- Escalations → Human Input Queue

**Controls:**

- Broadcast instruction
- Ask lead for status
- Pause all
- Resume all
- Open all terminals
- End team
- Summarize team
- Merge/reconcile outputs
- Collapse completed workers

**cc-crew-specific display:**

- `/team-start <track>` command
- Track name
- TDD slice progress
- Context tier
- Escalation category
- Team-end/orchestrate-end/session-end state

---

### Screen 12 — Workflow Setup + Command Registry

**Purpose:** Detect, personalize, activate, and run project-specific workflows.

**Primary question:** “What workflow capabilities are available in this project, and are they ready?”

**Layout:**

```text
Workflow status header
Detected artifacts
Personalization status
Command registry
Plan parser status
Launch recipes
Upgrade/drift panel
```

**Workflow status fields:**

- Workflow Pack
- Workflow Instance state
- Personalization status
- Manifest path
- Generated-from version/SHA
- Plan parser
- Team mode readiness
- Upgrade status

**Detected artifacts:**

- `.scaffolding/manifest.json`
- `.project-brain/manifest.json`
- `CLAUDE.md`
- `.claude/commands/`
- `.claude/skills/`
- `.claude/agents/`
- `ARCHITECTURE.md`
- `MVP_TASKS.md`
- `docs/layers/`
- `docs/sessions/`
- `docs/team-handoffs/`

**Command registry fields:**

- Command name
- Source
- Type
- Description
- Arguments
- Required context
- Readiness
- Risk
- Invocation mode
- Run button
- Open definition

**Actions:**

- Apply Workflow Pack
- Run personalization
- Review generated files
- Activate Workflow Instance
- Run command
- Start team
- Re-scan workflow
- Check upgrade
- Preview upgrade
- Detach workflow

**Important states:**

- Pack available, not installed
- Detected but not personalized
- Personalization in progress
- Generated, review required
- Active
- Degraded
- Drift detected
- Upgrade available

---

### Screen 13 — Workflow Personalization Review

**Purpose:** Make template-to-project generation visible and reviewable.

**Primary question:** “What will this Workflow Pack generate or change in this repo?”

**Layout:**

```text
Personalization stepper
Inferred values
Questions/answers
Generated file list
Diff preview
Approval cards
Action summary
```

**Required components:**

- Inferred value table
- Missing value question card
- User answer form
- Generation plan card
- Generated file diff list
- Manifest preview
- Approval card
- Commit suggestion

**Approval pauses:**

1. Approve generation plan before writing files.
2. Review generated files before commit/handoff.

**Actions:**

- Edit inferred value
- Answer missing question
- Approve plan
- Regenerate
- Accept file
- Reject file
- Commit changes
- Activate instance

---

### Screen 14 — Project Brain Drawer

**Purpose:** Project-aware memory, reasoning, evidence, and action planning.

**Primary question:** “What does the project know, and what can it help me do next?”

**Drawer modes:**

- Ask
- Plan
- Dispatch
- Review
- Decisions
- Memory
- Actions

**Header fields:**

- Scope: workspace/project/session/file/PR/selection
- Index status
- Evidence mode
- Action mode
- Privacy/transport indicator

**Conversation answer requirements:**

- Answer text
- Evidence chips
- Freshness/staleness stamp
- Confidence/verification state
- Related code chips
- Related session chips
- Related commit/PR chips
- Suggested actions

**Evidence chip types:**

- File/line
- Architecture anchor
- PlanTask
- Session episode
- Commit
- PR
- Decision
- Linear/GitHub item
- Event
- Memory source

**Action plan response:**

When the user asks Project Brain to act, the drawer should show:

- Proposed action plan
- Steps
- Risk levels
- Targets
- Evidence
- Preview/dry-run status
- Approval controls
- Edit plan option
- Approve all / step-by-step

**Example user prompts:**

- “When did we implement feature Y?”
- “Start the next backend task.”
- “What is blocking this PR?”
- “Ask the active agent to fix this failing check.”
- “Create a Linear issue from this plan task.”
- “Run the doc refresh for stale owned docs.”
- “Show the session where we fixed the eval failures.”

**Action boundary:** Project Brain proposes and requests. The Action Gateway executes.

---

### Screen 15 — Human Input Queue / Approvals

**Purpose:** Centralized triage for blocked agents and pending actions.

**Primary question:** “Where do I need to intervene?”

**Sections:**

- Needs clarification
- Permission requests
- High-risk actions
- Failed checks needing decision
- Workflow personalization approvals
- Project Brain action plans
- Agent team escalations

**Approval card fields:**

- Request type
- Requesting actor/session/team/brain
- Target project/worktree/branch
- Reason
- Risk level
- Preview
- Evidence/context
- Expiration
- Actions

**Actions:**

- Approve
- Deny
- Edit instruction
- Open terminal
- Open diff
- Ask Project Brain
- Approve once
- Set rule
- Escalate/defer

**Important behavior:**

The queue should be accessible globally from sidebar, status bar, keyboard shortcut, and notification.

---

### Screen 16 — Action Gateway Review Modal / Panel

**Purpose:** Review and approve typed action plans.

**Primary question:** “What exactly will happen if I approve this?”

**Layout:**

```text
Action summary
Risk banner
Step list
Preview/dry-run results
Affected resources
Evidence
Permissions required
Approval controls
Audit note
```

**Step display fields:**

- Step number
- Action type
- Target
- Risk level
- Preconditions
- Preview
- Rollback availability
- Status

**Controls:**

- Approve all
- Approve step-by-step
- Deny
- Edit plan
- Remove step
- Require manual execution
- Save as policy if eligible

**Important states:**

- Preview pending
- Preview succeeded
- Preview unavailable
- Stale preconditions
- Requires elevated permission
- Partially approved
- Executing
- Partially succeeded
- Failed with recovery actions

---

### Screen 17 — Execution Profiles Settings

**Purpose:** Manage multiple Claude/Codex runtime/account contexts.

**Primary question:** “Which account/profile does this session use?”

**Layout:**

```text
Profile list
Profile detail
Usage/current sessions
Project defaults
Safety policy
```

**Profile fields:**

- Display name
- Provider
- Harness
- Account alias
- Auth method
- CLI path/shell profile
- Default model
- Default permission mode
- Project allowlist
- Current sessions
- Usage state
- Last used

**Actions:**

- Add profile
- Test profile
- Set project default
- Disable profile
- Re-authenticate
- Assign to session/team role
- View usage

**Important behavior:**

Sessions and agent team workers must explicitly show their Execution Profile. The platform should not silently hop accounts.

---

### Screen 18 — Usage / Context Dashboard

**Purpose:** Track tokens, context, cost, and profile usage.

**Primary question:** “Where are tokens/context/cost being spent?”

**Views:**

- Workspace usage
- Project usage
- Session usage
- Execution Profile usage
- Model/harness usage
- Context-limit risk

**Components:**

- Usage summary cards
- Per-project usage table
- Per-session usage table
- Context usage rings/meters
- Budget alerts
- High-cost session list
- Estimate accuracy badge

**Important note:** Usage accuracy can vary by adapter. Show whether usage is exact, estimated, or unavailable.

---

### Screen 19 — Events / Audit Log

**Purpose:** Durable history and traceability.

**Primary question:** “What happened, who/what caused it, and what evidence exists?”

**Layout:**

```text
Event filter sidebar
Event table/timeline
Event detail inspector
Related object links
```

**Filters:**

- Time
- Project
- Session
- Agent team
- Actor
- Event type
- Risk level
- Action ID
- Approval status
- Workflow
- Integration

**Event detail fields:**

- Event type
- Timestamp
- Actor
- Source
- Target object
- Correlation ID
- Causation ID
- Sensitivity
- Summary
- Payload preview
- Related evidence

---

### Screen 20 — Settings / Integrations / Remote Access

**Purpose:** Configure integrations, security, policies, Project Brain, Workflow Packs, and future iOS pairing.

**Sections:**

- General
- Local runtime
- Execution Profiles
- GitHub
- Linear
- Project Brain
- Workflow Packs
- Security and approvals
- Event/audit retention
- Remote access / iOS companion

**Remote access stretch panel:**

- Remote access disabled/enabled
- Pair iOS device
- Allowed remote capabilities
- Approval-only mode
- Notification-only mode
- Last remote connection
- Revoke device
- Audit remote actions

---

## 10. Required user flows

### Flow A — First-time setup

```text
Open app
Run local checks
Grant host-config permissions
Add Execution Profiles
Connect GitHub/Linear
Configure Project Brain
Choose approval policy
Add first project
```

### Flow B — Add project

```text
Add project
Select local repo path
Detect git/repo status
Detect Workflow Pack/Instance
Detect Project Brain state
Connect GitHub/Linear project mapping
Choose default Execution Profiles
Choose worktree root
Run first index/sync
Open Project Home
```

### Flow C — Personalize Workflow Pack

```text
Open project workflow tab
Select Workflow Pack
Review detected architecture/plan docs
Run personalization
Answer missing questions
Approve generation plan
Review generated files/diff
Activate Workflow Instance
Run command registry scan
```

### Flow D — Start work from implementation plan

```text
Open Plan View
Select PlanTask or track
Review architecture anchors and acceptance criteria
Choose single session or agent team
Choose harness/model/profile
Choose worktree strategy
Preview prompt/command
Approve action plan
Open terminal/team view
```

### Flow E — Start work from Linear/GitHub

```text
Open Task Inbox
Select ticket/issue
Review details and linked plan task
Choose dispatch target
Create or choose worktree
Choose agent/harness/profile
Start session
Ticket links to session/worktree/branch
```

### Flow F — Start `/team-start` agent team

```text
Open Plan View or Workflow Commands
Select phase/track
Choose /team-start launch recipe
Assign profiles to lead/orchestrator/workers
Choose worktree strategy
Preview action plan
Approve
Team starts
Open Agent Team View
```

### Flow G — Respond to blocked session

```text
Human Input Queue shows request
Open approval card
Review command/context/risk
Open terminal/diff if needed
Approve/deny/edit
Session resumes or receives new instruction
Event logged
```

### Flow H — Review agent changes

```text
Session reports changes ready
Open Code Review Workspace
Review changed files
Inspect diff
Ask Project Brain or agent about suspicious hunk
Request fix or add tests
Run tests/checks
Commit
Create PR
```

### Flow I — Fix failing PR checks

```text
Open PR Review
See failing check
Ask Project Brain to summarize failure
Request agent fix
Platform creates/uses worktree
Session receives failure context
Agent pushes fix
Checks rerun
PR returns to review
```

### Flow J — Project Brain action plan

```text
Ask Project Brain to do something
Brain retrieves evidence and proposes action plan
Action Gateway normalizes and previews
User approves all or step-by-step
Platform executes actions
Events emitted
Brain indexes resulting artifacts
```

### Flow K — Self-updating docs

```text
Project Brain detects stale owned docs
Brain proposes doc refresh
Action Gateway shows affected docs and commands
User approves
Workflow/doc skill runs
Changed docs re-ingested
Freshness state updates
```

### Flow L — Archive completed work

```text
Session completed
Open completion summary
Review files/tests/PR state
Create/merge PR if needed
Summarize session into Project Brain
Archive session
Delete or retain worktree
Update plan/ticket status
```

### Flow M — iOS companion stretch

```text
Enable remote access in desktop settings
Pair mobile device
Set notification-only or approval-capable mode
Receive mobile notification for waiting session
Open summary on iOS
Approve safe action or defer
Desktop executes through Action Gateway
Audit event recorded
```

---

## 11. Interaction rules

### 11.1 Drag and drop

Drag sources:

- GitHub issue
- Linear ticket
- PlanTask
- File
- Diff hunk
- PR
- Session
- Workflow command

Drop targets:

- Project
- Session
- Agent Team
- Worktree
- Terminal composer
- Plan task row
- Project Brain drawer
- Code editor

Drop outcomes should always preview what will happen before mutation.

### 11.2 Command palette

The command palette should support:

- Open project/session/PR/task
- Start session
- Start agent team
- Run workflow command
- Ask Project Brain
- Create worktree
- Create PR
- Open Human Input Queue
- Approve pending action
- Open recent file
- Search events

### 11.3 Context menus

Context menus should be object-aware. Examples:

Session context menu:

- Attach terminal
- Open editor/diff
- Send message
- Ask status
- Pause/resume/kill
- Summarize
- Archive

PlanTask context menu:

- Start session
- Start team
- Link ticket
- Open architecture anchor
- Ask Project Brain
- Mark status

Diff hunk context menu:

- Ask why
- Request fix
- Add tests
- Add review comment
- Link to task

### 11.4 Keyboard-first interactions

Suggested shortcuts for prototype:

```text
Cmd/Ctrl+K      Command palette
Cmd/Ctrl+Shift+B Project Brain drawer
Cmd/Ctrl+Shift+I Inspector
Cmd/Ctrl+Shift+H Human Input Queue
Cmd/Ctrl+Shift+T New session
Cmd/Ctrl+Shift+Y New agent team
Cmd/Ctrl+Shift+P Task Inbox
Cmd/Ctrl+Shift+E Code editor
Cmd/Ctrl+Shift+G Project graph
```

### 11.5 Notifications

Notification types:

- Human input needed
- Approval requested
- Session completed
- Session failed
- Context near limit
- PR checks failed
- PR ready to merge
- Workflow personalization needs review
- Project Brain index stale/error

---

## 12. Component requirements by surface

The full component inventory is provided in the companion UI Component Inventory document. At minimum, the prototype must include these component families:

1. App shell and navigation
2. Project/session tree
3. Status pills and badges
4. Execution Profile badges/selectors
5. Project graph nodes/edges
6. Terminal surfaces
7. Code editor and diff components
8. Task/ticket cards
9. Plan/task components
10. Worktree/branch/PR components
11. Agent team components
12. Workflow Pack/Instance components
13. Command registry components
14. Project Brain drawer components
15. Evidence chips
16. Action Gateway approval components
17. Event timeline/audit components
18. Usage/context components
19. Setup/integration components
20. Empty/loading/error/degraded states

---

## 13. Prototype sample data

Use realistic sample data rather than placeholders like “Lorem ipsum.”

### Projects

```text
AI Engineering Control Plane
Project Brain
cc-crew Scaffold Demo
RepoGraph Parser
Weekly Commit Automation
```

### Execution Profiles

```text
Claude Max Main
Claude Max Secondary
Claude Team Work
Codex CLI Main
Codex Cloud GitHub
```

### Sessions

```text
Claude / ENG-221 GitHub OAuth callback
Codex / GH-184 parser memory leak
Claude Team / Phase 2 Observability Graph
Claude / Docs drift refresh
Codex / PR checks fix
```

### Tasks

```text
Linear ENG-221 — Add GitHub OAuth callback
GitHub #184 — Fix parser memory leak
PlanTask Phase 2.3 — Project observability graph
PlanTask Phase 3.1 — Action Gateway approval cards
GitHub PR #84 — Add workflow command registry
```

### Workflow state

```text
Workflow Pack: cc-crew
Instance state: Active
Plan file: MVP_TASKS.md
Architecture doc: ARCHITECTURE.md
Team launcher: /team-start <track>
Build loop: /tdd
```

### Human input examples

```text
Claude ENG-221 requests permission to run npm test.
Codex PR-fix asks whether to update snapshots.
Project Brain proposes creating a Linear issue from PlanTask Phase 2.3.
Workflow personalization requests approval before writing generated files.
```

---

## 14. Claude Design prototype coverage checklist

The prototype should include at least these screens/states:

```text
1. First Launch / Setup Wizard
2. Global Command Center
3. Project Home / Observability Graph
4. Project Sessions List
5. Session Terminal View
6. Code Editor / Diff Review Workspace
7. Plan View
8. Task Inbox
9. Worktree / Git / PR Control Center
10. PR Review Workspace
11. Agent Team View
12. Workflow Setup + Command Registry
13. Workflow Personalization Review
14. Project Brain Drawer
15. Human Input Queue
16. Action Gateway Review Modal
17. Execution Profiles Settings
18. Usage / Context Dashboard
19. Events / Audit Log
20. Settings / Integrations / Remote Access
```

For a shorter prototype, combine some screens, but do not omit these concepts:

- Desktop shell
- Project/session sidebar
- Project graph
- Terminal
- Code editor/diff
- Plan tasks
- GitHub/Linear task intake
- Worktrees/PRs
- Workflow Packs
- Agent teams
- Project Brain action drawer
- Action Gateway approvals
- Execution Profiles
- Event timeline

---

## 15. Non-goals for the design prototype

The design prototype should not attempt to solve:

- Full backend implementation
- Real terminal emulation
- Real graph layout algorithm
- Real code syntax engine
- Real mobile remote execution
- Trademark/naming
- Final color palette/taxonomy
- Enterprise multi-user RBAC
- Hosted SaaS admin UI

---

## 16. Accessibility and usability requirements

- All drag/drop actions must have button/menu equivalents.
- Status must not rely on color alone.
- Keyboard navigation should reach all primary actions.
- Approval cards must clearly state risk and action consequences.
- Terminal and code text should have readable contrast in final design.
- Dense views should support filtering, grouping, and search.
- Graph view should have list alternatives.
- Motion should be subtle and avoid distracting from terminal/code work.
- Destructive actions should require confirmation and show target resources.

---

## 17. Design intent summary

The app should feel like:

```text
Air traffic control for AI coding agents,
with the seriousness of a git/PR control plane,
the density of a developer tool,
the immediacy of a terminal multiplexer,
the review power of a code editor,
and the memory of a project-aware second brain.
```

It should not feel like:

```text
A generic SaaS dashboard
A chatbot wrapper
A decorative graph demo
A ticket board with AI labels
A normal IDE clone
A web app pretending to be local
```

