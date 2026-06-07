# AgentOps Studio — Claude Design Handoff

## Purpose of this document

This document is a comprehensive product, UI, UX, and design-system handoff for **AgentOps Studio**, a desktop-first developer tool for orchestrating AI coding agents such as Claude Code and OpenAI Codex.

This is intended to be pasted into or uploaded to Claude Design before creating:

1. A design system.
2. A UI kit.
3. A high-fidelity product mockup.

**Important constraint:** Do not create a color taxonomy or color palette in this design pass. The design system may define semantic states, hierarchy, density, spacing, typography, components, interaction behavior, iconography, surfaces, and layout rules, but should avoid specifying a formal color taxonomy.

---

# 1. Product Overview

## Product name

**AgentOps Studio**

## One-line description

AgentOps Studio is a local-first AI coding operations console for orchestrating Claude Code, OpenAI Codex, and other coding agents across projects, terminal sessions, git worktrees, GitHub issues, Linear tickets, pull requests, and observability traces.

## Short product explanation

AgentOps Studio helps developers safely manage many parallel AI coding sessions without losing visibility, control, or trust.

It combines:

- A project/session manager.
- A terminal multiplexer for Claude Code, Codex, and similar coding harnesses.
- An integrated code editor and diff/review workspace for inspecting and safely editing agent changes.
- An agent observability dashboard.
- A coding orchestration platform.
- A GitHub and Linear task router.
- A worktree, branch, commit, PR, and merge control plane.
- A lightweight IDE-style operational workspace with file explorer, editor tabs, diagnostics, diffs, and review affordances.

The product should not feel like a normal chatbot, normal IDE, or generic SaaS dashboard. It should feel like an **air-traffic-control system for AI coding agents**.

## Mission statement

Help developers run many AI coding agents in parallel while maintaining full visibility into what each agent is doing, where it is working, what task it owns, what code it changed, how much context it has consumed, what it is blocked on, and what is ready for human review.

## Core promise

Every agent, task, terminal, branch, PR, context window, token cost, and human decision point is visible from one operational control plane.

## Product category

AgentOps Studio is best described as an:

> AI coding operations console.

It is part:

- IDE.
- Code editor.
- Terminal session manager.
- AI agent orchestration system.
- Observability platform.
- Git/worktree manager.
- GitHub/Linear task execution platform.
- Engineering command center.

---

# 2. Target User

## Primary user

A senior engineer, staff engineer, founder, technical lead, or AI-native developer who is using multiple coding agents in parallel.

They may be running:

- Claude Code sessions.
- Codex sessions.
- Agent teams with a lead agent and worker agents.
- Multiple project-specific worktrees.
- GitHub issue-driven sessions.
- Linear ticket-driven sessions.
- Long-running implementation, debugging, refactor, testing, documentation, and review tasks.

## User mindset

The user should feel like:

> A technical lead directing a team of junior engineers.

Not:

> A person juggling a dozen disconnected terminal tabs.

## User goals

The user wants to answer these questions quickly:

- Which projects are active?
- Which sessions are active, idle, stale, failed, completed, or waiting for me?
- Which agents are working right now?
- What is each agent working on?
- Which model and harness is each session using?
- Which GitHub issue or Linear ticket is linked to each session?
- Which repo, branch, worktree, commit, and PR does each session map to?
- What code did the agent change, and can I inspect or edit it without leaving the app?
- How much context has each session consumed?
- How many tokens and how much cost has each project/session used?
- Which sessions need permission, approval, review, or intervention?
- Which PRs are ready to merge?
- Which worktrees are dirty, stale, conflicted, or safe to remove?
- Which agent team workers are assigned to which subtasks?
- What happened recently across the project?

## Key pain points

The product should solve these problems:

1. **Terminal sprawl**  
   The user loses track of many Claude/Codex terminal sessions.

2. **Agent state ambiguity**  
   The user cannot tell whether an agent is working, stuck, idle, waiting for permission, or waiting for human input.

3. **Git/worktree confusion**  
   Parallel agent work creates many branches, worktrees, diffs, PRs, and merge states.

4. **Task routing friction**  
   GitHub issues and Linear tickets live outside the agent workflow and require manual copy/paste.

5. **Context/cost opacity**  
   The user cannot easily see token usage, context saturation, model selection, or cost by session/project.

6. **Review bottleneck**  
   The user becomes the bottleneck because agents frequently need permission, clarification, review, or merge decisions.

7. **Multi-agent complexity**  
   Team-lead/worker-agent structures become hard to visualize and control.

8. **Lack of operational timeline**  
   The user cannot reconstruct what happened, which decisions were made, which commands ran, and what changed.

---

# 3. Product Principles

## 1. Operational clarity over decoration

The UI should prioritize what is happening, what needs attention, and what can be acted on.

Do not make the interface feel like a marketing dashboard.

## 2. Terminal-first, not chatbot-first

Claude Code and Codex are terminal-native workflows. The product should respect that.

The embedded terminal should be a first-class surface, not a hidden implementation detail.

## 2a. Editor-first when inspecting code

The product also needs a real code editor surface. The user should be able to open files, inspect diffs, review agent changes, make small manual edits, view diagnostics, and compare branches/worktrees without jumping out to another IDE for every code-level decision.

The editor should complement the terminal rather than replace it: agents run in terminals, while humans inspect, review, and occasionally edit code in the editor workspace.

## 3. Human control stays central

The user should always know:

- What the agent is doing.
- What it is allowed to do.
- What it is asking for.
- What code it changed.
- What action the user is approving.
- How to pause, resume, kill, inspect, or redirect it.

## 4. Graph as operational map, not decoration

The project graph should be useful. It should show real relationships:

- Project → sessions.
- Team lead → worker agents.
- Session → ticket.
- Session → worktree.
- Worktree → branch.
- Branch → PR.
- PR → checks/review/merge state.
- Agent → terminal/transcript/context/tokens.

## 5. Attention should be obvious

Waiting, blocked, failed, stale, conflicted, or approval-needed states should visually rise to the top.

The user should not have to hunt for things that need them.

## 6. Local-first developer feel

The app should feel close to the user’s machine, repo, terminal, worktrees, and filesystem.

It should not feel like a disconnected cloud-only abstraction.

## 7. High density, clear hierarchy

The product is information-dense by nature. The UI should support density without becoming chaotic.

Use hierarchy, grouping, spacing, labels, badges, and progressive disclosure.

## 8. Every object should be inspectable

Sessions, issues, tickets, PRs, branches, worktrees, team leads, workers, tool calls, permission requests, and trace events should all have inspectable detail panels.

---

# 4. Core Domain Model

Claude Design should understand these product objects.

## Workspace

Top-level container for all projects.

Contains:

- Projects.
- Integrations.
- Global settings.
- Global event timeline.
- Global usage summaries.
- Global human-input inbox.

## Project

A repo-backed or workspace-backed development unit.

A project contains:

- Repo metadata.
- Claude/Codex sessions.
- Agent teams.
- Worktrees.
- Branches.
- Linked GitHub issues.
- Linked Linear tickets.
- PRs.
- Token and cost usage.
- Event history.
- Project-level settings.

## Session

A single running or historical AI coding session.

A session may be:

- Claude Code.
- Codex CLI.
- Gemini CLI.
- Another coding harness.

A session has:

- Status.
- Model.
- Harness.
- Terminal.
- Transcript.
- Linked task.
- Repo.
- Branch.
- Worktree.
- PR.
- Context usage.
- Token usage.
- Cost.
- Recent activity.
- Pending approvals.
- Tool calls.
- Files changed.

## Agent Team

A group of coordinated sessions.

Usually includes:

- Team lead session.
- Worker sessions.
- Shared objective.
- Delegated subtasks.
- Team-level status.
- Team-level token usage.
- Team-level branches/worktrees.
- Team-level PRs or output artifacts.

## Team Lead

A parent/manager agent that decomposes a task, delegates work, checks progress, or coordinates workers.

## Worker Agent

A spawned child session that owns a subtask.

Examples:

- Backend implementation.
- Test coverage.
- UI implementation.
- Docs.
- Bug investigation.
- Refactor.
- PR review fix.

## GitHub Issue

A synced issue from GitHub.

Used as:

- Source of work.
- Drag-and-drop task card.
- Session context.
- PR linkage.
- Acceptance criteria source.

## Linear Ticket

A synced task from Linear.

Used similarly to GitHub issues.

## Worktree

An isolated git worktree used by an agent/session.

Tracks:

- Path.
- Branch.
- Dirty state.
- Session owner.
- Linked ticket.
- Linked PR.
- Last activity.
- Merge/conflict state.

## Branch

The git branch associated with a session/worktree.

## Pull Request

A GitHub PR associated with a branch/session/task.

Tracks:

- Draft/open/ready/merged/closed state.
- Checks.
- Review requests.
- Mergeability.
- Conflicts.
- Changed files.
- Linked issue/ticket.
- Agent author.

## Event

A timestamped activity in the system.

Examples:

- Session started.
- Agent requested permission.
- Tool call executed.
- Worktree created.
- Branch pushed.
- PR opened.
- Check failed.
- Human approved command.
- Agent became stale.
- Session exceeded context threshold.
- Team lead spawned worker.

---

# 5. Status Model

Do not define a color taxonomy, but do define clear semantic statuses.

## Session statuses

- Active
- Idle
- Waiting on human input
- Waiting on command permission
- Waiting on credentials/auth
- Waiting on external check
- Stale
- Failed
- Completed
- Paused
- Killed
- Archived

## Agent team statuses

- Planning
- Delegating
- Workers active
- Waiting on worker
- Waiting on human
- Merging outputs
- Review needed
- Failed
- Completed
- Paused

## Worktree statuses

- Clean
- Dirty
- Untracked files
- Conflicts
- Behind main
- Ahead of remote
- Ready for PR
- Linked to active session
- Stale
- Safe to remove
- Unsafe to remove

## PR statuses

- Draft
- Open
- Checks pending
- Checks passing
- Checks failing
- Review requested
- Changes requested
- Merge conflict
- Ready to merge
- Merged
- Closed

## Usage statuses

- Normal context usage
- High context usage
- Critical context usage
- Normal token usage
- High token usage
- Budget exceeded
- Project usage spike

## Attention states

Attention states are states that should rise visually in the UI and be sorted toward the top:

- Waiting on human input.
- Waiting on command permission.
- Failed.
- Stale.
- Merge conflict.
- Checks failing.
- Critical context usage.
- Budget exceeded.
- Unsafe worktree state.
- Authentication/integration error.

---

# 6. Information Architecture

The app should use a persistent desktop-style layout with multiple panels.

## Primary layout

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ Top Bar / Breadcrumb / Global Search / Primary Actions                       │
├───────────────┬──────────────────────────────────────────────┬───────────────┤
│ Left Sidebar  │ Main Workspace                               │ Right Inspector│
│ Projects      │ Project graph / terminal / inbox / git view   │ Selected object│
│ Sessions      │                                              │ details/actions│
├───────────────┴──────────────────────────────────────────────┴───────────────┤
│ Bottom Event Timeline / Activity Stream                                      │
└──────────────────────────────────────────────────────────────────────────────┘
```

## Persistent regions

### Top bar

Contains:

- App/workspace name.
- Breadcrumb.
- Global search.
- Command palette trigger.
- New session button.
- New agent team button.
- Sync integrations button.
- User/settings menu.

### Left sidebar

Contains:

- Workspace switcher.
- Search/filter.
- Projects.
- Nested sessions.
- Agent teams.
- Global human-input inbox.
- Usage summary.
- Integrations.
- Settings.

### Main workspace

Changes based on selected object or current route.

Main workspace routes:

- Global Command Center.
- Project Observability Graph.
- Session Terminal View.
- GitHub/Linear Task Inbox.
- Code Editor / Diff Review Workspace.
- Worktree/Git/PR Control Center.
- Agent Team View.
- Usage/Cost View.
- Settings/Integrations.

### Right inspector

Contextual detail panel that changes based on selected object.

It should be collapsible.

### Bottom timeline

Persistent activity/event feed.

Can be project-scoped, session-scoped, or global.

---

# 7. Left Sidebar UX

## Purpose

The left sidebar is the control surface for navigating projects and sessions.

It should make active and attention-needed work visible immediately.

## Structure

```text
Workspace
  Command/search

Projects
  ▾ Project A
      ⚠ Claude: waiting for approval
      ● Team: auth refactor
      ● Codex: issue #184
      ◐ Claude: writing tests
      ○ Idle: docs cleanup

  ▾ Project B
      ⚠ Codex: merge conflict
      ● Claude: API client
      ○ Completed: README update

Global
  Human Input Needed
  Token & Cost Usage
  Integrations
  Settings
```

## Sorting rules

Within each project, sessions should be sorted:

1. Waiting on human input.
2. Waiting on permission.
3. Failed.
4. Stale.
5. Active.
6. Paused.
7. Idle.
8. Completed.

Projects should also float upward when they contain sessions needing attention.

## Sidebar item metadata

Each session row should support:

- Status indicator.
- Harness icon/badge.
- Model shorthand.
- Short task name.
- Context usage mini-indicator.
- Token/cost mini-indicator.
- Linked ticket/issue ID.
- Branch or worktree shorthand.
- Last activity timestamp.
- Attention marker when needed.

## Sidebar interactions

- Click project → open Project Observability Graph.
- Click session → open Session Terminal View.
- Click agent team → open Agent Team View.
- Right-click session → contextual menu.
- Drag issue/ticket onto project → create new session.
- Drag issue/ticket onto session → append task to session context.
- Drag issue/ticket onto agent team → delegate through team lead.
- Collapse/expand projects.
- Pin favorite projects.
- Hide archived sessions.
- Filter by status, model, harness, repo, ticket source.

---

# 8. Main Screens

## Screen 1 — Global Command Center

### Purpose

Show the operational state of all projects and all active AI coding work.

### User question answered

“What is happening everywhere, and where do I need to intervene?”

### Layout

Top summary row:

- Active sessions.
- Waiting on human input.
- Waiting on permission.
- Open PRs.
- Checks failing.
- Tokens today.
- Cost today.
- Context risk count.
- Active agent teams.

Main content:

- Grouped project cards.
- Active session board.
- Attention-needed queue.
- Recent PR/review queue.
- Usage summary.

Right inspector:

- Shows selected project, session, event, PR, or attention item.

Bottom timeline:

- Global event timeline.

### Key modules

#### Attention Queue

A focused list of items requiring human action:

- Approve command.
- Provide clarification.
- Review diff.
- Resolve conflict.
- Re-auth integration.
- Merge decision.
- Re-run failed checks.
- Kill stale session.

Each item should have:

- Source project.
- Source session/team.
- Reason.
- Time waiting.
- Primary action.
- Secondary actions.

#### Active Work Board

Grouped by project.

Each card should show:

- Session/team name.
- Status.
- Model/harness.
- Task summary.
- Linked ticket.
- Branch/worktree.
- Context usage.
- Tokens/cost.
- Last activity.
- Quick actions.

#### Global Usage Snapshot

Shows:

- Tokens today.
- Tokens by project.
- Cost today.
- Cost by model.
- Highest context sessions.
- Budget warnings.
- Usage trend.

### Primary actions

- New session.
- New agent team.
- Sync GitHub/Linear.
- Open human-input inbox.
- Open usage details.
- Open command palette.

---

## Screen 2 — Project Observability Graph

### Purpose

This is the most important project-level screen.

It provides a live operational map of the project’s agents, tasks, worktrees, branches, PRs, and dependencies.

### User question answered

“What is happening inside this project, how are the agents connected, and what needs attention?”

### Layout

```text
Project Header
Graph Filter Bar

Central Graph Canvas

Right Inspector

Bottom Project Event Timeline
```

### Project header

Should include:

- Project name.
- Repo name.
- Default branch.
- Active session count.
- Waiting count.
- Open PR count.
- Worktree count.
- Tokens/cost summary.
- Sync status.
- Primary actions.

### Graph canvas

The graph should show nodes and edges.

#### Node types

- Project node.
- Individual session node.
- Agent team node.
- Team lead node.
- Worker agent node.
- GitHub issue node.
- Linear ticket node.
- Worktree node.
- Branch node.
- Pull request node.
- Human input required node.
- Failed/stale warning node.

#### Edge types

- Owns.
- Spawned.
- Working on.
- Linked to.
- Uses worktree.
- Uses branch.
- Opened PR.
- Needs review.
- Waiting on.
- Depends on.
- Generated.
- Updated.

### Graph behavior

- Click node → open inspector.
- Double-click session node → open terminal.
- Double-click team node → open Agent Team View.
- Double-click PR node → open PR detail.
- Hover node → quick summary popover.
- Drag ticket onto graph → create new session or team.
- Drag ticket onto team lead → delegate.
- Drag session node near team → offer to attach to team.
- Filter by status, model, harness, source, branch, PR state.
- Group by team, status, worktree, or ticket source.
- Collapse completed nodes.
- Highlight attention-needed path.
- Show stale/failed/blocked nodes prominently.
- Show team lead → worker relationships clearly.

### Graph filters

Filters should include:

- Status.
- Harness.
- Model.
- Ticket source.
- Branch.
- Worktree.
- PR state.
- Attention-needed only.
- Active only.
- Hide completed.
- Group by team.
- Group by branch.
- Group by issue/ticket.

### Inspector content for selected session node

- Session name.
- Status.
- Model.
- Harness.
- Repo.
- Worktree.
- Branch.
- Linked issue/ticket.
- PR.
- Context usage.
- Token usage.
- Cost.
- Last heartbeat.
- Current task.
- Recent tool calls.
- Pending approvals.
- Files changed.
- Actions.

### Inspector content for selected team node

- Team objective.
- Lead agent.
- Workers.
- Worker status.
- Delegated subtasks.
- Team-level progress.
- Team-level tokens/cost.
- Shared blockers.
- Team outputs.
- Actions.

### Inspector content for selected PR node

- PR title.
- State.
- Checks.
- Reviews.
- Mergeability.
- Conflicts.
- Files changed.
- Agent author.
- Linked issue/ticket.
- Actions.

### Primary actions

- New session.
- New agent team.
- Attach terminal.
- Pause all project sessions.
- Ask all active sessions for status.
- Open task inbox.
- Open git/PR control center.
- Sync GitHub/Linear.
- View usage.

---

## Screen 3 — Session Terminal View

### Purpose

Allow the user to directly control, inspect, and communicate with a single Claude Code or Codex session.

### User question answered

“What is this agent doing, what does it need, and how do I interact with it?”

### Layout

```text
Session Header
Terminal Workspace
Right Session Inspector
Bottom Composer / Optional Timeline
```

### Session header

Should include:

- Project.
- Session name.
- Status.
- Model.
- Harness.
- Context usage.
- Token usage.
- Cost.
- Branch.
- Worktree.
- Linked ticket/issue.
- PR.
- Last heartbeat.
- Primary actions.

### Terminal workspace

Should support:

- Embedded terminal.
- Multiple terminal tabs if needed.
- Terminal transcript.
- Search within terminal output.
- Copy command/output.
- Open terminal externally.
- Attach/detach.
- Split view with diff.
- Split view with code editor.
- Pop-out terminal window.

### Right session inspector

Sections:

#### Current task

- Task title.
- Source: manual / GitHub / Linear / spawned worker.
- Description.
- Acceptance criteria.
- Current plan.
- Current step.

#### Pending approvals

Examples:

- Run command.
- Edit file.
- Install package.
- Access network.
- Push branch.
- Open PR.
- Merge branch.
- Delete worktree.

Each approval card should include:

- Requested action.
- Reason.
- Risk level.
- Command or operation.
- Files affected.
- Approve.
- Deny.
- Modify.
- Ask for explanation.

#### Files changed

- File path.
- Change type.
- Lines changed.
- Open diff action.
- Open file in editor action.

#### Recent tool calls

- Tool name.
- Timestamp.
- Result.
- Duration.
- Error state if any.

#### Git state

- Worktree.
- Branch.
- Dirty state.
- Last commit.
- Remote tracking.
- PR state.

#### Usage

- Context usage.
- Token usage.
- Cost.
- Model.
- Session duration.

### Bottom composer

The composer allows the user to send a message/instruction to the agent.

Should support:

- Plain instruction.
- Attach issue/ticket.
- Attach file context.
- Attach diff.
- Mention another session.
- Mention a PR.
- Mention branch/worktree.
- Request status.
- Request plan.
- Request tests.
- Request PR.
- Request stop/pause.

### Primary actions

- Send instruction.
- Approve command.
- Deny command.
- Pause.
- Resume.
- Kill.
- Archive.
- Create PR.
- Open diff.
- Open worktree.
- Ask for status.
- Compact/summarize context.
- Spawn worker.
- Convert to team.

---

## Screen 4 — GitHub/Linear Task Inbox

### Purpose

Turn GitHub issues and Linear tickets into dispatchable agent work.

### User question answered

“What work is available, what should I give to an agent, and where should it run?”

### Layout

```text
Inbox Header
Tabs / Filters
Task List
Task Detail Panel
Dispatch Panel
```

### Tabs

- GitHub Issues.
- Linear Tickets.
- PRs.
- Assigned to Agents.
- Needs Review.
- Blocked.
- Recently Completed.

### Filters

- Project.
- Repo.
- Label.
- Priority.
- Assignee.
- Status.
- Source.
- Milestone.
- Sprint/cycle.
- Has acceptance criteria.
- Has linked PR.
- Unassigned.
- Agent-ready.
- Needs clarification.

### Task card

Each task card should include:

- Source badge: GitHub or Linear.
- ID.
- Title.
- Status.
- Priority.
- Labels.
- Assignee.
- Project/repo.
- Estimate/size if available.
- Linked PR count.
- Assigned agent/session if any.
- Drag handle.
- Last updated.

### Task detail panel

Should include:

- Full description.
- Acceptance criteria.
- Labels.
- Comments.
- Attachments.
- Related PRs.
- Related commits.
- Suggested repo.
- Suggested branch name.
- Suggested worktree name.
- Suggested agent/harness.
- Suggested prompt.
- Risk/complexity estimate.
- Dispatch options.

### Dispatch behavior

User can:

- Drag task onto project.
- Drag task onto session.
- Drag task onto team.
- Drag task onto worktree.
- Click “Dispatch”.
- Create new session from task.
- Create new team from task.
- Assign task to existing session.
- Attach as context only.
- Convert task into subtasks.

### Drop behavior

```text
Drop onto project:
  Create a new session or agent team for this project.

Drop onto existing session:
  Add task context to the session and ask the agent to continue.

Drop onto team lead:
  Ask team lead to decompose and delegate the task.

Drop onto worker:
  Assign the task/subtask directly to worker.

Drop onto worktree:
  Run the task in that worktree/branch context.

Drop onto PR:
  Ask agent to address PR feedback or failing checks.
```

### Primary actions

- Sync GitHub.
- Sync Linear.
- Dispatch to new session.
- Dispatch to new agent team.
- Assign to existing session.
- Generate task prompt.
- Mark agent-ready.
- Open source issue/ticket.
- Create branch/worktree.

---

## Screen 4b — Code Editor / Diff Review Workspace

### Purpose

Give the user a first-class place to inspect, review, and lightly edit code produced by agents without leaving the orchestration console.

This screen should feel like a focused developer workspace, not a full replacement for VS Code. It should provide the code-level visibility needed to supervise agents safely.

### User question answered

“What exactly changed, can I inspect it, and can I make or request the right fix?”

### Layout

```text
Editor Header
File Explorer / Changed Files Panel
Code Editor Canvas
Right Review/Agent Inspector
Bottom Terminal / Problems / Test Output / Timeline Drawer
```

### Editor header

Should include:

- Project.
- Repo.
- Current worktree.
- Branch.
- Linked session/team.
- Linked issue/ticket.
- PR if present.
- Dirty state.
- Test/check state.
- Context/tokens for linked session.
- Primary actions.

### File explorer / changed files panel

Should support two modes:

1. **Repo explorer** for browsing files in the selected worktree.
2. **Changed files** for reviewing agent edits, diffs, conflicts, and staged/unstaged changes.

Each file row should show:

- Path.
- Change type.
- Additions/deletions if available.
- Conflict marker if applicable.
- Test/diagnostic marker if applicable.
- Agent/session that changed it if known.

### Code editor canvas

The editor should support:

- Tabs.
- Split panes.
- Syntax highlighting.
- Line numbers.
- Search in file.
- Go to file / fuzzy file open.
- Basic edit mode.
- Read-only/review mode.
- Inline diff view.
- Side-by-side diff view.
- Conflict resolution view.
- Inline comments/notes.
- Agent-generated explanations attached to code regions.
- Save/revert file actions.
- Stage/unstage file or hunk actions if git integration supports it.

### Right review/agent inspector

When a file or diff is selected, the inspector should show:

- Why the file changed.
- Which agent/session changed it.
- Related task/issue/ticket.
- Related PR/check failure/review comment.
- Agent explanation.
- Risk summary.
- Related tests.
- Recent commands that touched this file.
- Actions to ask the agent to explain, revise, test, revert, or split changes.

### Bottom drawer

The bottom drawer should be switchable between:

- Embedded terminal.
- Problems/diagnostics.
- Test output.
- Git diff summary.
- Agent timeline.
- Tool call log.

### Editor interactions

- Click a changed file from a session → open the file/diff in the editor.
- Click a file path in terminal output → open file at line in editor.
- Click a failing test/check → open relevant file and test output.
- Highlight code → ask linked agent to explain, refactor, test, or fix.
- Select a hunk → ask agent to revise only this hunk.
- Select a file → ask agent why it changed or whether it is safe.
- Drag a ticket into the editor → attach task context to the current worktree/session.
- Open editor from graph node, session view, worktree view, PR card, or terminal path.

### Primary actions

- Open file.
- Open diff.
- Save file.
- Revert file.
- Stage/unstage file.
- Resolve conflict.
- Ask agent to explain selected code.
- Ask agent to fix selected code.
- Ask agent to add/update tests.
- Send selected code as context.
- Open terminal at file/worktree.
- Open externally in local IDE.

---

## Screen 5 — Worktree / Git / PR Control Center

### Purpose

Prevent chaos from many parallel agents editing code across many branches and worktrees.

### User question answered

“What code changed, where did it change, what is safe to review, and what is safe to merge?”

### Layout

```text
Git Header
Worktree Table / Cards
Branch/PR Panels
Diff/Check Inspector
```

### Worktree table columns

- Worktree name.
- Path.
- Branch.
- Linked session.
- Linked agent team.
- Linked issue/ticket.
- Dirty state.
- Files changed.
- Last commit.
- Last activity.
- PR.
- Merge/conflict state.
- Actions.

### Worktree card variant

For card-based view:

- Worktree name.
- Branch.
- Session owner.
- Linked task.
- Dirty summary.
- PR summary.
- Last activity.
- Quick actions.

### Git actions

- Create worktree.
- Delete worktree.
- Open worktree in internal editor.
- Open worktree in external IDE.
- Open terminal at worktree.
- Checkout branch.
- Pull latest.
- Merge main.
- Rebase on main.
- Commit changes.
- Push branch.
- Open PR.
- Open diff.
- Open file in editor.
- Resolve conflicts.
- Run tests.
- Ask agent to fix checks.
- Mark safe to remove.

### PR grouping

Group PRs by:

- Draft.
- Checks pending.
- Checks failing.
- Needs review.
- Changes requested.
- Ready to merge.
- Merge conflict.
- Merged.

### PR card

Each PR card should include:

- PR number.
- Title.
- Branch.
- Linked task.
- Agent/session author.
- Checks status.
- Review status.
- Mergeability.
- Files changed.
- Last updated.
- Quick actions.

### Diff/check inspector

When a worktree, branch, or PR is selected, the inspector should show:

- Changed files.
- Diff summary.
- Test/check results.
- Risk summary.
- Review comments.
- Merge conflicts.
- Agent explanation.
- Suggested next action.

### Primary actions

- Create worktree.
- Open diff.
- Open editor.
- Ask agent to fix.
- Create PR.
- Re-run checks.
- Merge.
- Squash merge.
- Rebase.
- Resolve conflict.
- Archive completed worktree.

---

## Screen 6 — Agent Team View

### Purpose

Visualize and control a team lead agent and its spawned worker agents.

### User question answered

“How is this multi-agent task decomposed, who is doing what, and what needs coordination?”

### Layout

```text
Team Header
Lead Agent Panel
Worker Grid
Team Graph
Shared Timeline
Right Inspector
```

### Team header

Should include:

- Team name.
- Project.
- Objective.
- Status.
- Lead model/harness.
- Worker count.
- Linked task.
- Team-level branch/worktree strategy.
- Team-level tokens/cost.
- Primary actions.

### Lead agent panel

Should include:

- Objective.
- Current plan.
- Delegated tasks.
- Current coordination step.
- Blockers.
- Team summary.
- Lead terminal action.

### Worker grid

Each worker card should include:

- Worker name.
- Role/subtask.
- Status.
- Model/harness.
- Branch/worktree.
- Linked files.
- Progress summary.
- Context usage.
- Tokens/cost.
- Last activity.
- Open terminal.
- Open diff.
- Pause/resume/kill.

### Team graph

Shows:

- Team lead.
- Worker sessions.
- Subtask relationships.
- Branches/worktrees.
- PRs.
- Shared issue/ticket.
- Human input nodes.
- Merge/review nodes.

### Team controls

- Broadcast instruction.
- Ask lead for status.
- Ask all workers for status.
- Pause all.
- Resume all.
- Kill team.
- Add worker.
- Merge worker outputs.
- Collapse completed workers.
- Create team PR.
- Split team into separate PRs.
- Convert worker to standalone session.

### Primary actions

- Broadcast instruction.
- Open lead terminal.
- Open all terminals.
- Add worker.
- Pause all.
- Ask lead to reconcile outputs.
- Review team diff.
- Create PR.

---

# 9. Cross-Cutting UX Patterns

## Command palette

A global command palette should support:

- Create new session.
- Create new agent team.
- Search projects.
- Search sessions.
- Search issues/tickets.
- Search PRs.
- Open terminal.
- Approve pending permission.
- Pause/resume/kill session.
- Create worktree.
- Open diff.
- Sync integrations.
- Ask all agents for status.
- Open human-input inbox.
- Open usage dashboard.

## Human-input inbox

A global inbox for all places where the user is needed.

Items include:

- Permission requests.
- Clarification requests.
- Failed commands.
- Merge conflict decisions.
- PR review decisions.
- Authentication errors.
- Tool failures.
- Budget/context warnings.
- Agent asks.

Each item should provide:

- What happened.
- Why the user is needed.
- Related project/session/team.
- Risk level.
- Primary action.
- Secondary actions.
- Dismiss/snooze option.

## Drag-and-drop task routing

Dragging GitHub issues and Linear tickets should be a core interaction.

Important affordances:

- Clear drag handles.
- Drop target highlighting.
- Preview of what will happen.
- Confirmation only for high-impact dispatches.
- Ability to choose session/team/worktree/branch at drop time.

## Terminal popovers and docked panels

From graph, team, or session views, terminal sessions should be openable as:

- Full main view.
- Right dock.
- Bottom dock.
- Floating modal.
- Multi-terminal grid.
- External terminal.

## Editor and terminal co-location

The editor and terminal should be designed as complementary surfaces. Common layouts should include:

- Terminal-only mode for direct Claude/Codex control.
- Editor-only mode for code review and manual edits.
- Editor + terminal split view.
- Editor + diff + inspector layout.
- Multi-terminal grid with selected file/diff inspector.
- Pop-out terminal while keeping editor focused.

Important linking behavior:

- Terminal file paths should open in the editor.
- Diff files should open in the editor.
- PR review comments should open in the editor.
- Agent explanations should link to code regions.
- Selected code should be sendable to the agent as context.

## Progressive disclosure

The app is dense. Avoid overwhelming the user by revealing detail progressively:

- Sidebar shows compact status.
- Cards show summary.
- Inspector shows details.
- Modal/drawer shows full transcript/diff.
- Dedicated screen shows full operational control.

## Attention-first sorting

Anything blocked, failed, waiting, conflicted, or approval-needed should sort higher than active or idle work.

## Safe destructive actions

Destructive actions require confirmation:

- Kill session.
- Delete worktree.
- Force push.
- Merge PR.
- Delete branch.
- Discard changes.
- Archive session with uncommitted work.

## Context and cost visibility

Token, cost, and context usage should be visible at multiple levels:

- Session row.
- Session header.
- Project header.
- Usage dashboard.
- Inspector.
- Global command center.

## Trust-building UX

The UI should make agents auditable.

Show:

- What task they were given.
- What plan they made.
- What files they changed.
- What commands they ran.
- What tools they used.
- What approvals they requested.
- What PR they opened.
- What tests/checks ran.
- What failed.
- What was merged.

---

# 10. Design System Requirements

Do not include a color taxonomy or formal palette.

The design system should include:

- Typography.
- Spacing.
- Layout grid.
- Panel system.
- Border/elevation guidance.
- Iconography style.
- Density rules.
- Component anatomy.
- Component variants.
- Component states.
- Interaction behavior.
- Motion guidance.
- Accessibility guidance.
- Empty/loading/error states.
- Data visualization patterns without defining a color palette.

## Visual style

The product should feel:

- Technical.
- Dark-mode first.
- Dense.
- Calm.
- Operational.
- Professional.
- Developer-native.
- Terminal-friendly.
- Precise.
- Trustworthy.

The product should avoid:

- Generic AI sparkle visuals.
- Cartoon mascots.
- Excessive glow effects.
- Oversized marketing widgets.
- Chatbot-first layouts.
- Decorative graphs that do not encode real state.
- Hiding git/session details behind vague abstractions.

## Typography guidance

Use a practical combination of:

- A clean UI sans-serif for interface labels, cards, panels, and navigation.
- A monospace typeface for commands, paths, branches, tokens, model IDs, logs, file names, and terminal-adjacent metadata.

Typography should support:

- Dense tables.
- Compact cards.
- Terminal output.
- Code paths.
- Small metadata labels.
- Strong hierarchy between title, status, metadata, and action text.

## Layout guidance

The app is desktop-first.

Recommended layout behavior:

- Persistent left sidebar.
- Main workspace.
- Optional right inspector.
- Optional bottom timeline.
- Resizable panels.
- Collapsible panels.
- Split views.
- Pop-out terminal windows.
- Fullscreen terminal mode.
- Keyboard navigability.

## Spacing/density guidance

Support at least two density modes:

1. Comfortable  
   For initial use, demos, and lower information density.

2. Compact  
   For power users managing many sessions.

Components should be designed to work in compact mode without losing labels, status, or action clarity.

## Motion guidance

Use motion sparingly.

Appropriate motion:

- Subtle activity pulse for active sessions.
- Gentle attention animation for waiting/blocking states.
- Smooth expansion/collapse.
- Drag-and-drop target transitions.
- Terminal popover opening.
- Graph layout transitions.
- Timeline event insertion.

Avoid:

- Decorative animation.
- Distracting constant motion.
- Animations that obscure state.
- Excessive AI-themed effects.

## Iconography guidance

Use simple, technical, developer-oriented icons.

Icon categories:

- Project.
- Terminal.
- Session.
- Agent.
- Team.
- Team lead.
- Worker.
- GitHub.
- Linear.
- Branch.
- Worktree.
- Commit.
- PR.
- Merge.
- Conflict.
- Check.
- Warning.
- Waiting.
- Token/context.
- Cost.
- Model.
- Harness.
- Tool call.
- Timeline event.
- Settings.
- Integration.
- Search.
- Command.

---

# 11. Component Inventory

## Navigation components

### Workspace Switcher

Allows switching between workspaces.

States:

- Default.
- Open.
- Loading.
- Syncing.
- Error.

### Project Tree Sidebar

Persistent navigation tree.

Contains:

- Projects.
- Nested sessions.
- Agent teams.
- Global navigation items.

Variants:

- Full.
- Compact.
- Collapsed.
- Filtered.
- Search results mode.

States:

- Loading.
- Empty.
- Sync error.
- Offline.
- No projects.

### Project Group Row

A collapsible project row in the sidebar.

Should show:

- Project name.
- Repo shorthand.
- Count of active sessions.
- Count of attention-needed sessions.
- Token/cost mini summary if space allows.
- Expand/collapse control.

States:

- Collapsed.
- Expanded.
- Selected.
- Contains active sessions.
- Contains attention-needed sessions.
- Syncing.
- Error.

### Session Row

A nested sidebar item for a session.

Should show:

- Status.
- Harness.
- Model shorthand.
- Task summary.
- Linked issue/ticket.
- Context mini indicator.
- Last activity.

Variants:

- Claude Code session.
- Codex session.
- Worker session.
- Completed session.
- Archived session.

States:

- Active.
- Idle.
- Waiting.
- Failed.
- Stale.
- Paused.
- Completed.
- Selected.

### Agent Team Row

A nested sidebar item for an agent team.

Should show:

- Team name.
- Lead status.
- Worker count.
- Attention count.
- Objective shorthand.

States:

- Planning.
- Active.
- Waiting.
- Failed.
- Completed.
- Selected.

### Breadcrumb Bar

Shows current route.

Examples:

- All Projects.
- Project / Session.
- Project / Agent Team / Worker.
- Project / PR.
- Project / Worktree.

### Global Command Palette

Keyboard-first action/search surface.

Should support fuzzy search over:

- Projects.
- Sessions.
- Teams.
- Issues.
- Tickets.
- PRs.
- Worktrees.
- Commands.
- Settings.

---

## Status and metadata components

### Session Status Pill

Communicates current state.

Statuses:

- Active.
- Idle.
- Waiting on human.
- Waiting on permission.
- Stale.
- Failed.
- Completed.
- Paused.
- Killed.

### Model Badge

Shows model used.

Examples:

- Claude Opus.
- Claude Sonnet.
- GPT/Codex.
- Gemini.
- Local model.

### Harness Badge

Shows execution environment.

Examples:

- Claude Code.
- Codex CLI.
- Terminal.
- Custom agent.
- Team lead.
- Worker.

### Context Usage Ring

Shows context window usage.

Should support:

- Compact.
- Full.
- With numeric percentage.
- With threshold label.
- In table row.
- In card.
- In header.

### Token Usage Meter

Shows input/output token usage.

Should support:

- Session-level usage.
- Project-level usage.
- Team-level usage.
- Daily usage.
- Budget comparison.

### Cost Badge

Shows estimated cost.

Should support:

- Per session.
- Per project.
- Per team.
- Today.
- This week.
- Budget exceeded state.

### Heartbeat Indicator

Shows whether session is alive.

States:

- Live.
- Recently active.
- Stale.
- Disconnected.
- Unknown.

### Waiting on Human Alert

Reusable alert for blocked user action.

Should include:

- Reason.
- Source.
- Waiting duration.
- Primary action.
- Secondary action.
- Dismiss/snooze.

### Permission Request Card

Used when an agent asks to run a command or perform a sensitive action.

Should include:

- Requested action.
- Command or operation.
- Reason.
- Risk explanation.
- Files/resources affected.
- Approve.
- Deny.
- Modify.
- Ask for explanation.

### Stale Session Warning

Shows that a session has not produced output or heartbeat recently.

Actions:

- Ping session.
- Ask for status.
- Resume.
- Pause.
- Kill.
- Archive.

---

## Graph components

### Graph Canvas

Interactive canvas for project observability.

Features:

- Pan.
- Zoom.
- Select.
- Multi-select.
- Fit to view.
- Grouping.
- Filtering.
- Minimap.
- Layout controls.
- Collapse completed.
- Focus attention-needed.

### Project Node

Represents root project.

Shows:

- Project name.
- Repo.
- Active session count.
- Waiting count.
- PR count.
- Usage summary.

### Session Node

Represents a Claude/Codex session.

Shows:

- Session name.
- Status.
- Harness.
- Model.
- Task summary.
- Context usage.
- Linked branch/worktree.

### Agent Team Node

Represents a multi-agent team.

Shows:

- Team name.
- Objective.
- Lead status.
- Worker count.
- Attention count.
- Progress summary.

### Team Lead Node

Represents parent/lead agent.

Shows:

- Lead role.
- Current planning/delegation state.
- Worker count.
- Current blocker if any.

### Worker Node

Represents child agent.

Shows:

- Worker role.
- Subtask.
- Status.
- Branch/worktree.
- Progress summary.

### Issue/Ticket Node

Represents GitHub issue or Linear ticket.

Shows:

- Source.
- ID.
- Title.
- Priority.
- Status.
- Assigned session/team if any.

### Worktree Node

Represents git worktree.

Shows:

- Worktree name.
- Branch.
- Dirty/clean/conflict state.
- Linked session.

### Branch Node

Represents branch.

Shows:

- Branch name.
- Ahead/behind state.
- Linked PR.
- Last commit.

### PR Node

Represents pull request.

Shows:

- PR number.
- Title.
- State.
- Checks.
- Review state.
- Mergeability.

### Graph Edge

Represents relationship between objects.

Types:

- Owns.
- Spawned.
- Works on.
- Linked to.
- Uses.
- Opened.
- Needs.
- Depends on.
- Generated.
- Updated.

### Node Detail Popover

Hover/click preview.

Shows compact summary and primary actions.

### Graph Minimap

Shows viewport and large graph overview.

### Graph Filter Bar

Controls visible nodes/relationships.

Filters:

- Status.
- Model.
- Harness.
- Source.
- Branch.
- PR state.
- Team.
- Active only.
- Attention needed.
- Hide completed.

---

## Code editor and review components

### Code Editor Workspace

Main code inspection and editing surface.

Should support:

- File tabs.
- Split panes.
- Syntax highlighting.
- Line numbers.
- Search.
- Read-only mode.
- Edit mode.
- Diff mode.
- Conflict mode.
- Linked session/team context.

### File Explorer Panel

Navigates the selected repo/worktree.

Variants:

- Full repo tree.
- Changed files only.
- PR files only.
- Conflict files only.
- Search results.

### Editor Tab

Represents an open file, diff, test output, or generated artifact.

States:

- Clean.
- Dirty.
- Read-only.
- Diff.
- Conflict.
- Generated by agent.
- Has diagnostics.

### Inline Diff Viewer

Shows file changes in a unified diff format inside the editor.

Should support:

- Added/removed line indicators without relying only on color.
- Hunk-level actions.
- Comment on hunk.
- Ask agent to revise hunk.
- Stage/unstage hunk if supported.

### Side-by-Side Diff Viewer

Shows original and modified versions next to each other.

Should support:

- Synchronized scrolling.
- Hunk navigation.
- Inline comments.
- Open related terminal/session.

### Conflict Resolver View

Focused view for merge conflicts.

Should include:

- Current change.
- Incoming change.
- Base if available.
- Accept current.
- Accept incoming.
- Accept both.
- Manual edit.
- Ask agent to resolve.

### Problems / Diagnostics Panel

Shows lint, type, test, and runtime problems.

Fields:

- Severity.
- File.
- Line.
- Message.
- Source.
- Related session/check if available.
- Ask agent to fix action.

### Code Selection Action Bar

Appears when the user selects code.

Actions:

- Ask agent to explain.
- Ask agent to refactor.
- Ask agent to test.
- Ask agent to fix.
- Send as context.
- Create note.
- Copy path/selection.

### Agent Code Explanation Card

Attaches an agent explanation to a file, symbol, line range, or diff hunk.

Should include:

- Explanation.
- Source agent/session.
- Related task.
- Confidence/risk label if available.
- Ask follow-up action.

### Review Comment Card

Represents a code review comment tied to a line or hunk.

Should support:

- Human comment.
- Agent response.
- Resolved/unresolved state.
- Send to agent.
- Mark resolved.

### Test Output Panel

Shows test command output and failures.

Should support:

- Link failures to files/lines.
- Attach output to agent instruction.
- Re-run test action.
- Ask agent to fix action.

---

## Terminal and chat components

### Embedded Terminal Panel

Native terminal area.

Features:

- Terminal output.
- Input passthrough.
- Copy.
- Search.
- Scrollback.
- Attach/detach.
- Open external.
- Resize.
- Split.

### Terminal Tab Group

For multiple terminals.

Variants:

- Project terminals.
- Team terminals.
- Worker terminals.
- Session terminal + test terminal.

### Terminal Popover

A floating or docked terminal from a graph/team/session node.

### Agent Message Composer

Input for sending instructions to agent.

Features:

- Text input.
- Attach issue.
- Attach ticket.
- Attach file.
- Attach diff.
- Mention session.
- Mention PR.
- Send.
- Save prompt snippet.

### Tool Call Log

List of recent tool calls.

Should include:

- Tool name.
- Timestamp.
- Duration.
- Status.
- Result summary.
- Error details if failed.

### Session Transcript Drawer

Full conversation/transcript view.

Should include:

- User prompts.
- Agent messages.
- Tool calls.
- System events.
- Search/filter.
- Export option.

### Approval Prompt

Compact inline approval UI.

Variants:

- Command approval.
- File edit approval.
- Network access approval.
- Git operation approval.
- PR operation approval.

---

## Issues and tickets components

### GitHub Issue Card

Shows synced GitHub issue.

Fields:

- Issue number.
- Title.
- Repo.
- Labels.
- Assignee.
- Status.
- Linked PRs.
- Agent assignment.
- Drag handle.

### Linear Ticket Card

Shows synced Linear ticket.

Fields:

- Ticket ID.
- Title.
- Team/project.
- Priority.
- Status.
- Cycle.
- Assignee.
- Agent assignment.
- Drag handle.

### Draggable Task Card

Generic issue/ticket card for routing work.

States:

- Idle.
- Dragging.
- Droppable target active.
- Already assigned.
- Blocked.
- Agent-ready.
- Needs clarification.

### Ticket Detail Panel

Shows full task context.

Sections:

- Description.
- Acceptance criteria.
- Comments.
- Attachments.
- Linked PRs.
- Linked commits.
- Suggested prompt.
- Suggested branch/worktree.
- Dispatch options.

### Dispatch Target Selector

Selects where a task should go.

Targets:

- New Claude session.
- New Codex session.
- Existing session.
- New agent team.
- Existing agent team.
- Specific worker.
- Specific worktree.

### Acceptance Criteria Block

Structured list of requirements the agent should satisfy.

Should support:

- Checkbox style.
- Source-linked criteria.
- Agent-generated criteria.
- Human-edited criteria.

---

## Git and PR components

### Worktree Table

Dense table for many worktrees.

Columns:

- Name.
- Path.
- Branch.
- Session.
- Task.
- Dirty state.
- Files changed.
- Last commit.
- Last activity.
- PR.
- Actions.

### Worktree Card

Compact card version of a worktree.

Shows:

- Name.
- Branch.
- Session owner.
- Task.
- Dirty state.
- PR state.
- Quick actions.

### Branch Badge

Shows branch name.

Variants:

- Main/default.
- Agent branch.
- Feature branch.
- Stale branch.
- Conflict branch.

### Diff Summary Card

Shows changed files and line changes.

Fields:

- Files changed.
- Additions.
- Deletions.
- Key paths.
- Risk summary.
- Open diff action.
- Open file in editor action.

### PR Card

Shows PR state.

Fields:

- PR number.
- Title.
- Branch.
- Session/team author.
- Linked task.
- Checks.
- Review.
- Mergeability.
- Last updated.
- Actions.

### Check Status List

Shows CI/test checks.

Fields:

- Check name.
- Status.
- Duration.
- Failure summary.
- Re-run action.
- Ask agent to fix action.

### Merge Conflict Alert

Shows conflict state.

Fields:

- Branch.
- Files conflicted.
- Related session.
- Suggested action.
- Open resolver.
- Ask agent to resolve.

### Commit Timeline

Shows commits in a branch/worktree/PR.

Fields:

- Commit hash.
- Message.
- Author/session.
- Timestamp.
- Checks if relevant.

---

## Observability components

### Event Timeline

Chronological activity feed.

Event types:

- Session lifecycle.
- Tool call.
- Permission request.
- Git action.
- PR action.
- Integration sync.
- Error.
- Human action.
- Usage alert.

### Agent Trace Tree

Hierarchical trace of an agent’s work.

Should show:

- Prompt.
- Plan.
- Tool calls.
- File reads/writes.
- Commands.
- Errors.
- Outputs.
- Final result.

### Gantt Execution Timeline

Optional time-based view of sessions/tool calls.

Useful for:

- Long-running sessions.
- Team workers.
- Parallel task visualization.

### Cost/Tokens Chart

Usage visualization without a formal color taxonomy.

Should support:

- By session.
- By project.
- By model.
- By harness.
- Over time.
- Budget line/threshold.

### Tool Call Sequence Diagram

Shows ordered interaction between:

- User.
- Agent.
- Terminal.
- Filesystem.
- Git.
- GitHub.
- Linear.
- MCP/tool services.

### Error/Span Detail Panel

Shows details for failed operations.

Fields:

- Error message.
- Stack/log excerpt.
- Tool.
- Session.
- Timestamp.
- Retry action.
- Ask agent to explain/fix.

---

## Action components

### New Session Button

Starts a new Claude/Codex session.

Options:

- Project.
- Repo.
- Harness.
- Model.
- Worktree.
- Branch.
- Task source.
- Starting prompt.

### New Agent Team Button

Starts a team lead + workers.

Options:

- Objective.
- Project.
- Lead model/harness.
- Worker count.
- Worker roles.
- Worktree strategy.
- Task source.

### Attach Terminal Button

Opens terminal view for selected session.

### Pause/Resume/Kill Controls

Control agent execution.

Should include safe confirmations for destructive operations.

### Create PR Button

Creates a PR from branch/worktree/session.

### Merge PR Button

Merges PR.

Should show checks/review/conflict state before action.

### Request Agent Fix Button

Sends failing checks/review comments/conflicts back to the agent.

### Sync Integrations Button

Syncs GitHub/Linear/project metadata.

### Ask for Status Button

Requests a concise status update from one session, team, or all active sessions.

### Broadcast Instruction Button

Sends instruction to multiple workers or sessions.

---

# 12. Empty, Loading, and Error States

## Empty states

### No projects

Explain how to add a project/repo.

Actions:

- Add project.
- Connect GitHub.
- Import local repo.

### No active sessions

Prompt to start a new Claude/Codex session.

Actions:

- New session.
- Dispatch issue.
- Create agent team.

### No linked issues/tickets

Prompt to connect GitHub/Linear.

Actions:

- Connect GitHub.
- Connect Linear.
- Sync integrations.

### No worktrees

Explain worktree benefit.

Actions:

- Create worktree.
- Start session with new worktree.

### No PRs

Explain that PRs will appear when agents open them.

Actions:

- Create PR from selected branch.
- View branches.

## Loading states

Use skeletons for:

- Sidebar project tree.
- Graph canvas.
- Task inbox.
- Worktree table.
- PR list.
- Usage cards.
- Inspector details.

Use explicit sync states for:

- GitHub sync.
- Linear sync.
- Repo scan.
- Session heartbeat.
- Terminal reconnect.
- Graph layout.

## Error states

Errors should be actionable.

Examples:

- GitHub auth expired.
- Linear sync failed.
- Terminal disconnected.
- Worktree path missing.
- Branch deleted.
- PR not found.
- Agent process crashed.
- Model/harness unavailable.
- Token/cost estimate unavailable.
- Permission request expired.

Each error should include:

- What happened.
- Impact.
- Suggested fix.
- Retry action.
- Open logs action if useful.

---

# 13. Accessibility and Keyboard UX

## Accessibility goals

The UI is dense, so accessibility is critical.

Support:

- Keyboard navigation.
- Clear focus states.
- Text labels for icons.
- Non-color-only status indicators.
- Tooltips for compact metadata.
- Screen-reader-friendly status labels.
- Scalable text where possible.
- Sufficient contrast without requiring a specified palette.

## Keyboard shortcuts

Recommended shortcuts:

- Open command palette.
- New session.
- New agent team.
- Search projects/sessions.
- Open human-input inbox.
- Approve selected permission.
- Deny selected permission.
- Open selected terminal.
- Toggle right inspector.
- Toggle bottom timeline.
- Pause selected session.
- Ask selected session for status.
- Open diff.
- Open PR.
- Sync integrations.

## Non-color status indicators

Because this handoff excludes color taxonomy, status should also be communicated with:

- Text labels.
- Icons.
- Shape.
- Border treatment.
- Positioning.
- Sorting.
- Motion.
- Badges.
- Tooltips.
- Alert grouping.

---

# 14. Example Data for Mockups

Use this sample data in the mockup.

## Projects

### AgentOps Studio

Repo: `github.com/cody/agentops-studio`

Sessions:

- Claude: `ENG-221 GitHub OAuth Callback`
- Codex: `BUG-118 Worktree Cleanup`
- Team: `Agent Team — Linear Sync Pipeline`
- Claude: `UI Kit Component Pass`
- Codex: `PR #84 Fix Failing Checks`

### WTT RepoGraph

Repo: `github.com/cody/wtt-repograph`

Sessions:

- Claude: `Parser Memory Leak #184`
- Codex: `AST Snapshot Tests`
- Team: `Hybrid CodeGraph Wrapper`

### ST6 Weekly Commit

Repo: `github.com/cody/st6-weekly-commit`

Sessions:

- Claude: `Architecture Diagram`
- Codex: `Manager Review Flow`
- Claude: `Waiting — PR Feedback`

## GitHub issues

- `#184 Fix parser memory leak on large monorepos`
- `#191 Add session token usage tracking`
- `#203 Create PR health summary panel`
- `#207 Support worktree cleanup safety checks`

## Linear tickets

- `ENG-221 Add GitHub OAuth callback`
- `ENG-229 Build Linear sync pipeline`
- `ENG-237 Add human input inbox`
- `ENG-241 Implement agent team topology graph`

## PRs

- `#82 ENG-221 GitHub OAuth Callback`
- `#84 Fix failing checks for session usage`
- `#87 Linear sync pipeline draft`
- `#91 Worktree cleanup safety check`

## Agent teams

### Linear Sync Pipeline Team

Objective:

Build a robust Linear integration that syncs tickets, maps them to projects, and supports drag-and-drop dispatch into Claude/Codex sessions.

Lead:

- Claude Opus Team Lead

Workers:

- Worker 1: API integration
- Worker 2: Data model/schema
- Worker 3: UI task inbox
- Worker 4: Tests and mocks

---

# 15. Claude Design Prompt — Design System

Copy and paste this into Claude Design.

```text
Create a comprehensive design system for a desktop-first developer tool called AgentOps Studio.

AgentOps Studio is a local-first AI coding operations console for orchestrating Claude Code, OpenAI Codex, and other coding agents across projects, terminal sessions, git worktrees, GitHub issues, Linear tickets, pull requests, and observability traces.

This is not a normal IDE and not a chatbot app. It is an AI coding operations console: part IDE, part terminal multiplexer, part agent observability platform, part orchestration platform, part GitHub/Linear workflow manager, and part git/worktree/PR control plane.

The user is a senior engineer, technical lead, or AI-native developer running many AI coding agents in parallel. The design should help them instantly answer:
- Which projects are active?
- Which agents are working?
- Which agents are idle, stale, failed, or waiting on human input?
- What is each agent working on?
- Which GitHub issue or Linear ticket is linked to each session?
- Which model and harness is each session using?
- How much context, token usage, and cost has each session consumed?
- Which branch, worktree, commit, and PR belongs to each session?
- What needs review or approval?
- What can be safely merged?

Important constraint:
Do not create a color taxonomy or formal color palette in this design pass. You may define semantic states, hierarchy, typography, spacing, density, layout, component behavior, iconography, accessibility, and visual treatment, but avoid specifying a formal color system.

Design style:
- Dark-mode first.
- Dense but readable.
- Technical, calm, and operational.
- Inspired by VS Code, Linear, GitHub, Datadog, Raycast, terminal multiplexers, and AI agent observability tools.
- Avoid generic AI sparkle/glow visuals.
- Avoid marketing dashboard styling.
- Prioritize clarity, hierarchy, and human control.
- Communicate status through labels, iconography, shape, hierarchy, badges, sorting, and motion rather than relying only on color.

Core surfaces:
1. Left project/session sidebar.
2. Project observability graph.
3. Embedded terminal/session view.
4. Integrated code editor and diff/review workspace.
5. GitHub/Linear issue and ticket inbox.
6. Worktree, git, PR, and merge control center.
7. Agent team topology view.
8. Right-side contextual inspector.
9. Bottom event/activity timeline.
10. Global command palette.
11. Settings/integrations area.

Design system should include:
- Typography.
- Spacing.
- Layout grid.
- Panel system.
- Border/elevation guidance.
- Iconography style.
- Density rules.
- Component anatomy.
- Component variants.
- Component states.
- Interaction behavior.
- Motion guidance.
- Empty states.
- Loading states.
- Error states.
- Accessibility notes.
- Data visualization patterns without defining a formal color taxonomy.

Core components:
- Workspace switcher.
- Project tree sidebar.
- Collapsible project group.
- Session row.
- Agent team row.
- Breadcrumb bar.
- Global command palette.
- Session status pill.
- Model badge.
- Harness badge.
- Context usage ring.
- Token usage meter.
- Cost badge.
- Heartbeat indicator.
- Waiting on human alert.
- Stale session warning.
- Permission request card.
- Graph canvas.
- Project node.
- Session node.
- Agent team node.
- Team lead node.
- Worker node.
- Issue/ticket node.
- Worktree node.
- Branch node.
- PR node.
- Graph edge.
- Node detail popover.
- Graph minimap.
- Graph filter bar.
- Code editor workspace.
- File explorer panel.
- Editor tab.
- Inline diff viewer.
- Side-by-side diff viewer.
- Conflict resolver view.
- Problems/diagnostics panel.
- Code selection action bar.
- Agent code explanation card.
- Review comment card.
- Test output panel.
- Embedded terminal panel.
- Terminal tab group.
- Terminal popover.
- Agent message composer.
- Tool call log.
- Session transcript drawer.
- Approval prompt.
- GitHub issue card.
- Linear ticket card.
- Draggable task card.
- Ticket detail panel.
- Dispatch target selector.
- Acceptance criteria block.
- Worktree table.
- Worktree card.
- Branch badge.
- Diff summary card.
- PR card.
- Check status list.
- Merge conflict alert.
- Commit timeline.
- Event timeline.
- Agent trace tree.
- Gantt execution timeline.
- Cost/tokens chart.
- Tool call sequence diagram.
- Error/span detail panel.
- New session button.
- New agent team button.
- Attach terminal button.
- Pause/resume/kill controls.
- Create PR button.
- Merge PR button.
- Request agent fix button.
- Sync integrations button.
- Ask for status button.
- Broadcast instruction button.

Status model:
- Active.
- Idle.
- Waiting on human input.
- Waiting on command permission.
- Waiting on credentials/auth.
- Waiting on external check.
- Stale.
- Failed.
- Completed.
- Paused.
- Killed.
- Archived.
- PR open.
- PR ready.
- Checks failing.
- Merge conflict.
- High context usage.
- Critical context usage.
- High cost usage.
- Budget exceeded.

The design system should be practical enough to generate a UI kit and high-fidelity product mockup after this step.
```

---

# 16. Claude Design Prompt — UI Kit and Mockup

Copy and paste this into Claude Design after the design system is created.

```text
Using the AgentOps Studio design system, create a high-fidelity desktop web app UI kit and product mockup for an AI coding operations console.

AgentOps Studio orchestrates Claude Code, OpenAI Codex, and other coding agents across multiple projects. It integrates with GitHub and Linear, manages agent terminal sessions, creates git worktrees, tracks branches/PRs/merges, and provides observability for active sessions and agent teams.

Important constraint:
Do not create or rely on a formal color taxonomy. Use the existing visual treatment from the design system and communicate status through labels, hierarchy, icons, shape, spacing, badges, sorting, and motion cues.

Create the following screens:

Screen 1: Global Command Center
- Persistent left sidebar with projects and nested sessions.
- Active, waiting, failed, and stale sessions sorted to the top.
- Top stats: active sessions, waiting on input, waiting on permission, open PRs, checks failing, tokens today, cost today.
- Main area with grouped project/session cards.
- Attention-needed queue.
- Bottom activity timeline.
- Right inspector showing selected session/project/task details.

Screen 2: Project Observability Graph
- Selected project: “AgentOps Studio”.
- Central graph showing project node, individual Claude/Codex sessions, an agent team lead, worker agents, linked GitHub issues, Linear tickets, worktrees, branches, and PRs.
- Nodes should show status: active, idle, waiting on human input, waiting on permission, stale, failed, completed.
- Waiting and failed nodes should be visually prominent.
- Active nodes should show what they are working on.
- Team lead should visibly connect to spawned worker sessions.
- Include graph filters for status, model, harness, issue source, branch, worktree, PR state.
- Right inspector should show details for the selected node.
- Bottom event timeline should show recent agent/session/git events.

Screen 3: Session Terminal View
- Embedded native terminal for a Claude Code session.
- Header shows project, session name, model, harness, status, context usage, tokens, cost, branch, worktree, linked issue, and PR.
- Right panel shows task summary, files changed, pending approvals, recent tool calls, git state, usage, and actions.
- Include message composer to send instructions to the agent.
- Include permission approval UI.
- Include affordance to pop out, dock, or open terminal externally.

Screen 4: Code Editor / Diff Review Workspace
- Integrated code editor for the selected project/worktree/session.
- File explorer and changed-files panel.
- Editor tabs with syntax-highlighted files.
- Inline and side-by-side diff review.
- Conflict resolver view.
- Problems/diagnostics panel.
- Test output panel.
- Right inspector showing why a file changed, which agent changed it, related issue/ticket/PR, risk summary, and agent actions.
- Ability to select code and ask the linked agent to explain, revise, test, or fix it.
- Ability to open terminal paths, PR review comments, failing tests, and changed files directly in the editor.
- Include affordance to open the same worktree externally in the user’s local IDE.

Screen 5: GitHub/Linear Task Inbox
- Tabs for GitHub Issues, Linear Tickets, PRs, Assigned to Agents, Needs Review, Blocked, Recently Completed.
- List of draggable task cards.
- Ticket detail panel with description, labels, priority, acceptance criteria, linked repo, suggested agent, suggested worktree name, and suggested prompt.
- Show interaction affordance that a ticket can be dragged into a session terminal, project graph, worktree, or agent team.

Screen 6: Worktree / Git / PR Control Center
- Worktree table/cards with branch, linked session, linked team, dirty state, last commit, linked ticket, PR, merge/conflict state, and actions.
- Git action controls: create worktree, delete worktree, commit, rebase, merge main, open diff, resolve conflicts, run tests, push branch.
- PR cards grouped by draft, checks pending, checks failing, needs review, ready to merge, merge conflict, merged.
- Include merge conflict alert and checks panel.
- Include right inspector for selected worktree/PR/diff.

Screen 7: Agent Team View
- Team lead at top with objective, current plan, delegated tasks, and blockers.
- Worker agents below with assigned subtasks.
- Team graph showing lead to workers to branches/worktrees/PRs.
- Controls for broadcast instruction, pause all, ask lead for status, ask all workers for status, add worker, merge worker outputs, collapse completed workers.
- Include terminals as popover or docked panels for multiple worker sessions.

Overall UX:
- The project observability graph should be the home base for each project.
- Terminals, tasks, PRs, branches, worktrees, and agent teams should all attach to the graph rather than feeling like disconnected tabs.
- The UI should feel like an air-traffic-control system for AI coding agents.
- Prioritize operational clarity, human control, and trust.
- Make waiting-on-human and approval-needed states impossible to miss.
- Make terminal access first-class.
- Make git/worktree/PR state visible and safe to manage.
```

---

# 17. Suggested First Mockup Scope

For the first mockup, prioritize these screens in this order:

1. **Project Observability Graph**  
   This is the unique center of gravity for the product.

2. **Session Terminal View**  
   This proves the product is terminal-first and useful for real Claude/Codex workflows.

3. **Code Editor / Diff Review Workspace**  
   This proves the product supports code-level supervision, review, and manual intervention.

4. **GitHub/Linear Task Inbox**  
   This shows how work enters the system.

5. **Worktree/Git/PR Control Center**  
   This shows how parallel agent work remains safe.

6. **Agent Team View**  
   This shows the multi-agent orchestration story.

7. **Global Command Center**  
   This ties all projects together.

---

# 18. Product Differentiation Notes

AgentOps Studio should not be positioned as:

- Just another IDE.
- Just another chatbot.
- Just another terminal wrapper.
- Just another GitHub client.
- Just another observability dashboard.

It should be positioned as:

> The operational layer for AI-native software development.

The key differentiation is that it unifies:

- Agent sessions.
- Terminal surfaces.
- Code editor and diff/review surfaces.
- Project state.
- Human intervention points.
- GitHub/Linear work.
- Git worktrees.
- Branch/PR lifecycle.
- Token/context/cost visibility.
- Multi-agent team topology.
- Session observability.

The most important UX decision:

> Make the project graph the operational home base, and make every terminal, editor surface, issue, ticket, branch, worktree, PR, and agent team attach to it.

This prevents the product from feeling like a collection of tabs and makes it feel like a true command center.
