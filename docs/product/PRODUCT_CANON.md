# AI Engineering Control Plane — Product Canon v0.1

> **Status:** Planning draft  
> **Date:** 2026-06-06  
> **Naming:** Open. This document intentionally uses neutral working labels: **the platform**, **Project Brain**, **Workflow Packs**, and **Action Gateway**.

---

## 1. Purpose of this document

This is the top-level product canon for the parent platform: the AI engineering control plane that manages projects, coding agents, terminals, worktrees, tickets, pull requests, implementation plans, custom workflows, and Project Brain.

This document is not the final PRD. It is the source-of-truth planning artifact that stabilizes the product shape before writing the PRD, UX spec, design system handoff, and implementation plan.

---

## 2. Working product definition

The platform is a local-first AI engineering control plane for dispatching, supervising, reviewing, and merging work from Claude Code, Codex, custom agent teams, and future coding-agent harnesses.

It combines:

- Project/session management
- Terminal orchestration
- Code editor and diff review
- Worktree, branch, git, and PR management
- GitHub and Linear task intake
- Agent observability
- Execution profile routing
- Workflow Pack support
- Project Brain memory, retrieval, reasoning, and action planning

The platform is best understood as **air traffic control for AI coding agents**.

---

## 3. Product thesis

AI coding tools are moving from single-turn assistants to parallel workers. Once developers can run multiple agents across multiple projects, the hard problem becomes operational control:

- Which agents are active?
- Which are blocked?
- What task is each agent working on?
- Which worktree, branch, PR, ticket, and implementation-plan item does each session belong to?
- Which sessions are using which model, harness, and subscription/account profile?
- What needs human approval?
- What changed?
- What can safely merge?
- Why was a change made, and where is the evidence?

The platform exists to solve that coordination problem.

The core bet:

> The developer of the near future is not just writing code with an assistant. They are managing a fleet of coding agents. They need an operational control plane, not another chat box.

---

## 4. What the product is

The platform is:

1. **An AI coding operations console**  
   It gives the user real-time visibility and control over many agent sessions.

2. **A terminal/session control plane**  
   It preserves native Claude Code and Codex terminal workflows while wrapping them with state, context, approvals, git metadata, and observability.

3. **A project-aware orchestration layer**  
   It knows projects, repos, worktrees, branches, tasks, plan items, PRs, sessions, and agent teams.

4. **A git/worktree safety layer**  
   It makes parallel agent work safe by isolating work in worktrees and exposing branch, dirty-state, diff, PR, and merge status.

5. **A review-focused code environment**  
   It includes a first-class code editor and diff review workspace focused initially on inspecting, editing, explaining, testing, and reviewing agent-created changes.

6. **A workflow runtime**  
   It detects and exposes project-specific workflows, including custom Claude/Codex scaffolding, commands, skills, subagents, hooks, plan files, and team launch recipes.

7. **A Project Brain host**  
   It integrates a standalone Project Brain system as a project memory, retrieval, reasoning, evidence, and action-planning layer.

---

## 5. What the product is not

The platform is not:

1. **Not just an IDE**  
   It may include a code editor, but the differentiator is orchestration and operational control.

2. **Not just a chatbot**  
   Chat is one surface. The product center is projects, sessions, worktrees, plans, diffs, PRs, approvals, and evidence.

3. **Not a replacement for Claude Code or Codex**  
   It should wrap and orchestrate these tools rather than hide or reimplement their native surfaces.

4. **Not a generic enterprise RAG tool**  
   Project Brain is project/code/workflow/session memory, not a broad enterprise search app.

5. **Not a mandatory scaffold system**  
   Custom scaffolding and Workflow Packs are supported, but projects without them must still work.

6. **Not cloud-first by default**  
   The likely architecture is local-first or local-runner-first because terminals, git, local files, worktrees, and credentials are core.

---

## 6. Target users

### Primary user: portfolio solo developer / technical builder

A developer running multiple projects locally, often with Claude Code, Codex, custom workflows, and many simultaneous agent sessions.

Needs:

- Manage many projects and sessions
- Avoid losing track of agent work
- Route tasks from Linear/GitHub/plans into agents
- Keep work isolated by branch/worktree
- Review code safely
- Recover context and history through Project Brain

### Secondary user: technical lead / reviewer

A lead overseeing multiple agent-produced branches and PRs.

Needs:

- See what is ready for review
- Identify risky or stale work
- Connect PRs back to tasks, sessions, and decisions
- Use Project Brain to explain why and how changes were made

### Later user: new teammate / onboarding user

Someone joining a project who needs guided, cited explanations of architecture, code, decisions, and recent implementation history.

Needs:

- Ask project history questions
- Follow evidence chips to code, commits, sessions, PRs, and docs
- Understand current implementation plan and architecture

---

## 7. Core product layers

The platform has six product layers.

### 7.1 Project Layer

Owns projects, repositories, local paths, settings, workflow instances, integrations, and Project Brain association.

### 7.2 Work Layer

Owns task intake from GitHub, Linear, implementation plans, architecture sections, PR review requests, and ad hoc user prompts.

### 7.3 Agent Layer

Owns sessions, agent teams, models, harnesses, execution profiles, terminals, subagents, and team roles.

### 7.4 Execution Layer

Owns terminal process management, tool calls, command invocation, approval requests, logs, context limits, token/cost tracking, and heartbeats.

### 7.5 Code Layer

Owns file tree, editor, changed files, diffs, diagnostics, tests, conflicts, and code-selection-to-agent actions.

### 7.6 Delivery Layer

Owns git worktrees, branches, commits, PRs, checks, reviews, merges, and session archival.

Project Brain cuts across all layers as memory and reasoning. Workflow Packs cut across all layers as project-specific operating rules.

---

## 8. Atomic operational unit

The **session** is the atomic operational unit.

Every meaningful unit of agent work should resolve to one or more sessions.

A session can be linked to:

- Project
- Repository
- Worktree
- Branch
- Task
- Plan task
- GitHub issue
- Linear issue
- Workflow command
- Agent team
- Execution profile
- Terminal
- Transcript
- Diffs
- Commits
- PRs
- Test results
- Approval requests
- Project Brain episode cards

This is the key to making the UI coherent.

---

## 9. Main product surfaces

### 9.1 Global Command Center

The workspace-level overview for all projects and sessions.

Primary question:

> What needs my attention across everything?

Shows:

- Active sessions
- Waiting sessions
- Failed/stale sessions
- Open PRs
- High context/cost sessions
- Recently completed work
- Human input queue

### 9.2 Project Home / Observability Graph

The project-level command surface.

Primary question:

> What is happening inside this project?

Shows:

- Project node
- Sessions
- Agent teams
- Worktrees
- Branches
- Tasks
- Plan tasks
- PRs
- Waiting approvals
- Status and relationships

The graph must be operational, not decorative. It should support filtering, selection, node actions, and terminal/editor popovers.

### 9.3 Session Terminal View

The native Claude Code / Codex session surface.

Primary question:

> What is this agent doing, and how do I intervene?

Shows:

- Embedded terminal
- Session header
- Model/harness/profile
- Context/tokens/cost
- Worktree/branch/task/PR
- Pending approvals
- Recent tool calls
- Changed files
- Message composer

### 9.4 Code Editor / Review Workspace

The first-class code review and editing environment.

Primary question:

> What changed, is it correct, and what should happen next?

Shows:

- File explorer
- Changed files
- Editor tabs
- Inline diff
- Side-by-side diff
- Conflict resolver
- Diagnostics
- Test output
- Agent comments
- Selection-to-agent actions

The editor starts as review-focused rather than full IDE replacement.

### 9.5 Task Inbox

The intake surface for GitHub, Linear, implementation plan tasks, and ad hoc work.

Primary question:

> What work can I dispatch to an agent?

Shows:

- GitHub issues
- Linear tickets
- PR review requests
- Plan tasks
- Assigned-to-agent tasks
- Needs-review tasks
- Drag/drop dispatch targets

### 9.6 Plan View

The implementation-plan surface.

Primary question:

> Where are we in the plan, and what should be worked next?

Shows:

- Phases
- Tracks
- Tasks
- Architecture anchors
- Linked tickets
- Linked sessions
- Linked worktrees
- Linked PRs
- Carry-forward items
- Open decisions
- Log/history

### 9.7 Worktree / Git / PR Control Center

The parallel-work safety surface.

Primary question:

> Which branches/worktrees/PRs exist, who owns them, and what is safe to merge?

Shows:

- Worktrees
- Branches
- Dirty state
- Linked sessions
- Linked tasks
- Changed files
- Commits
- PR state
- Checks
- Merge conflicts
- Merge actions

### 9.8 Agent Team View

The multi-agent orchestration surface.

Primary question:

> What is the team lead coordinating, what are workers doing, and how do their outputs reconcile?

Shows:

- Team objective
- Lead/orchestrator session
- Worker sessions
- Roles
- Tracks/subtasks
- Worktrees/branches
- Terminals
- Diffs
- PRs
- Team timeline
- Broadcast/pause/status controls

### 9.9 Workflow Setup / Command Registry

The project workflow surface.

Primary question:

> What project-specific workflow exists here, is it personalized, and what commands can I run?

Shows:

- Workflow Pack availability
- Workflow Instance status
- Personalization state
- Manifest state
- Commands
- Skills
- Subagents
- Hooks
- Plan parsers
- Team launch recipes
- Upgrade/drift state

### 9.10 Project Brain Drawer

The memory, reasoning, and action-planning surface.

Primary question:

> What does this project know, and what can it help me do next?

Shows:

- Ask mode
- Plan mode
- Review mode
- Decisions mode
- Memory mode
- Evidence chips
- Suggested actions
- Draft action plans
- Approval buttons

### 9.11 Human Input Queue

The global attention surface for approvals, blockers, and escalations.

Primary question:

> Where am I the bottleneck?

Shows:

- Permission requests
- Human clarification requests
- Safety/design escalations
- Deferment approvals
- Load-bearing decision requests
- Failed/stale session alerts

---

## 10. Project Brain relationship

Project Brain should be standalone but platform-native.

Standalone responsibilities:

- Ingest code, docs, git history, session history, PRs, tickets, and workflow artifacts
- Build structured metadata and vector indexes
- Answer project and portfolio questions
- Produce evidence chips
- Support historical implementation queries
- Track provenance, freshness, and staleness
- Draft recommendations and action plans

Platform-native responsibilities:

- Use shared object IDs
- Consume platform events
- Understand sessions, worktrees, tasks, PRs, workflows, and approvals
- Power the Project Brain drawer
- Request actions through the Action Gateway

Project Brain should not directly execute privileged operations. It should plan and request them through the platform.

---

## 11. Workflow Pack relationship

A Workflow Pack is a reusable project workflow package. A Workflow Instance is a project-specific personalized/generated version of that pack.

The platform must support projects in three modes:

1. **Basic Project**  
   Repo + sessions + tasks + worktrees, no special scaffold.

2. **Claude/Codex-Aware Project**  
   Has files such as `CLAUDE.md`, `.claude/`, custom commands, skills, agents, or hooks.

3. **Workflow-Pack Project**  
   Has a recognized personalized workflow instance with manifest, commands, plan parser, launch recipes, and lifecycle state.

Workflow Packs must not be mandatory.

---

## 12. Execution Profiles

Execution Profiles represent named local agent runtime/account contexts.

Examples:

- Claude Max — Main
- Claude Max — Secondary
- Claude Team — Work
- Codex CLI — Main
- Codex Cloud — GitHub Connected

Execution Profiles solve:

- Which account/subscription is this session using?
- Which projects can use this profile?
- Which sessions are running under it?
- Which profile should run a lead vs worker?
- How should usage be attributed?

Routing should be explicit, visible, and auditable. The product should avoid implying automatic subscription circumvention.

---

## 13. Action Gateway

The Action Gateway is the permissioned execution layer through which Project Brain and UI workflows request actions.

It owns:

- Action schemas
- Risk levels
- Permission checks
- Preview/dry-run behavior
- User confirmation
- Execution delegation
- Audit logs
- Rollback/undo metadata when available

Examples of actions:

- Create worktree
- Start session
- Send message to session
- Invoke workflow command
- Link plan task to Linear issue
- Create PR
- Open diff
- Ask agent to fix selection
- Update task status
- Run tests
- Archive session

Project Brain can propose actions. The platform decides whether and how to execute them.

---

## 14. UX principles

### 14.1 Attention first

Waiting, blocked, failed, risky, or human-needed states outrank ordinary active work.

### 14.2 Preserve native surfaces

Claude Code and Codex terminal sessions should remain accessible as real terminals.

### 14.3 Never separate code from context

When viewing code or diffs, show the linked session, task, plan item, worktree, branch, PR, and evidence.

### 14.4 Every object shows ownership

For any branch, diff, PR, or file change, the user should know which session/team/task caused it.

### 14.5 Make risk visible

Risk states include high context, high cost, stale sessions, dirty worktrees, failing checks, merge conflicts, dangerous commands, credential access, and unreviewed generated code.

### 14.6 Completion is explicit

Sessions should end with a summary, changed files, tests run, known issues, PR status, and recommended next action.

### 14.7 Graph plus list

Graphs show relationships; lists and tables support scanning. Power users need both.

---

## 15. Primary workflows

### 15.1 First-time setup

1. Install platform
2. Configure local runner
3. Connect GitHub
4. Connect Linear
5. Detect Claude Code / Codex
6. Add execution profiles
7. Add first project
8. Detect workflow/project files
9. Configure worktree root
10. Create first session

### 15.2 Start work from ticket

1. Open Task Inbox
2. Select GitHub/Linear task
3. Choose project/repo
4. Choose single session or agent team
5. Choose harness/model/profile
6. Choose worktree strategy
7. Review generated prompt/action plan
8. Start session/team
9. Link task/session/worktree/branch

### 15.3 Start work from implementation plan

1. Open Plan View
2. Select phase/track/task
3. Review architecture anchors and dependencies
4. Choose single agent or team workflow
5. Invoke command or plain prompt
6. Create/link worktree and session
7. Track progress against plan task

### 15.4 Monitor active work

1. Open Global Command Center or Project Home
2. Filter by waiting/active/stale/risky
3. Select session/team
4. Inspect current activity
5. Open terminal/editor/PR as needed
6. Approve, redirect, pause, or archive

### 15.5 Respond to blocked session

1. Human Input Queue shows request
2. User opens approval context
3. Platform shows command/task/session/risk
4. User approves, denies, edits, or escalates
5. Session resumes or status updates
6. Event is logged

### 15.6 Review agent changes

1. Session indicates changes ready
2. Open Code Review Workspace
3. Review changed files and diffs
4. Run tests or inspect output
5. Ask agent to explain/fix selected code
6. Commit or request changes
7. Create/update PR

### 15.7 Create and merge PR

1. Review diff
2. Generate PR body from task/session summary
3. Create PR
4. Track checks/reviews
5. Ask agent to fix failing checks
6. Approve/merge when ready
7. Archive session/worktree

### 15.8 Use Project Brain to act

1. User asks Project Brain for help
2. Brain retrieves context and evidence
3. Brain proposes an action plan
4. Platform previews risk and steps
5. User approves all or step-by-step
6. Platform executes through Action Gateway
7. Brain indexes resulting events/artifacts

---

## 16. MVP / P1 / P2 scope

### 16.1 MVP

- Local project registry
- Project/session sidebar
- Execution Profiles
- Claude Code session support
- Codex session support if feasible
- Worktree creation and linking
- Embedded terminal
- Basic code editor and diff review
- Basic task inbox
- Manual GitHub/Linear linking
- Plan View for parsed implementation plans
- Workflow detection
- Command registry
- Invoke workflow command from UI
- Human Input Queue
- Basic Project Brain drawer
- Action Gateway scaffolding
- Event timeline
- Create PR
- Archive completed sessions

### 16.2 P1

- Agent Team View
- Workflow Pack personalization flow
- cc-crew Workflow Pack integration
- `/team-start` launch recipe
- Plan task to Linear one-way creation
- PR checks integration
- Request agent fix from failing check
- Context/token/cost budgets
- Session summaries into Project Brain
- Decision log
- Advanced graph filters
- Conflict resolver

### 16.3 P2

- Controlled bidirectional Linear sync
- Cross-project Project Brain workflows
- Multi-user/team sharing
- Cloud runner or remote execution
- Policy automation mode
- Workflow Pack marketplace/import
- Automated PR-to-plan reconciliation
- Advanced agent performance analytics

---

## 17. Strong decisions so far

1. The product is an AI engineering control plane, not just an IDE or chatbot.
2. The session is the atomic operational unit.
3. Project Brain remains standalone but platform-native.
4. Project Brain can be action-capable through the Action Gateway.
5. Workflow Packs are first-class.
6. Workflow Packs and Workflow Instances are distinct.
7. Custom scaffolding is optional, not mandatory.
8. The code editor is first-class but review-focused first.
9. Execution Profiles are first-class.
10. The graph must be operational, not decorative.
11. Human-input-needed is a global attention queue.
12. Linear sync starts with linking, then one-way creation, then bidirectional sync later.
13. All important actions are auditable.
14. Naming is open.

---

## 18. Open questions

1. Is the main app desktop, web with local daemon, or both?
2. What is the MVP harness scope: Claude Code only, or Claude Code + Codex?
3. What terminal capture mechanism is reliable enough for MVP?
4. How are execution profiles mapped to local authenticated Claude/Codex contexts?
5. What is the minimum useful embedded editor?
6. How much git automation is safe in MVP?
7. How should Workflow Pack schemas be standardized?
8. Should Workflow Pack personalization run through a platform-supervised Claude Code session?
9. What Project Brain actions can run without confirmation?
10. What actions always require confirmation?
11. What event schema should Project Brain and the platform share?
12. What is the first demo workflow?

---

## 19. Artifact dependencies

This canon feeds:

- Shared Object Model
- Event Model
- Action Gateway Spec
- Workflow Packs Spec
- UX / Information Architecture Spec
- Main Platform PRD
- Design System Handoff
- Security and Permissions Spec
- Roadmap / Phasing Plan

---

## 20. Next artifact to refine

The next artifact should be the **Shared Object Model**, because it turns the product canon into concrete nouns, fields, relationships, states, UI surfaces, and events.
