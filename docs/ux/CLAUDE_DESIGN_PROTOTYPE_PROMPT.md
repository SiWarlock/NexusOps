# Claude Design Prompt — Desktop AI Engineering Control Plane Prototype v0.1

Use this prompt in Claude Design after loading the UX/IA Spec and UI Component Inventory.

---

## Prompt

Create a high-fidelity desktop app prototype and UI kit for a product currently working-named **AI Engineering Control Plane**. The final name is not chosen, so do not over-brand the UI. Use neutral product naming in headers.

This is a **desktop-first local app**, not a web SaaS dashboard. It is an AI coding operations console for dispatching, supervising, reviewing, and merging work from Claude Code, Codex, and future coding agents across multiple local projects.

The app combines:

- Local project management
- Claude Code / Codex terminal session orchestration
- Agent teams with lead/orchestrator/worker roles
- Git worktree, branch, commit, PR, check, and merge management
- GitHub issue and Linear ticket intake
- Implementation plan parsing, especially `MVP_TASKS.md`
- Workflow Packs and project-personalized Workflow Instances
- Custom workflow command registry, including `/team-start`
- Code editor and diff review workspace
- Project Brain drawer for project memory, evidence, reasoning, and action planning
- Action Gateway for previews, approvals, risk classification, and audit logs
- Execution Profiles for multiple Claude/Codex account contexts
- Token/context/usage tracking
- Event timeline and auditability
- Future iOS companion for remote observability and controlled approvals

Do **not** create a generic dashboard. The product should feel like a serious developer desktop tool: terminal-aware, code-aware, git-aware, operationally dense, and safe.

Do **not** include a color taxonomy. You may use visual status treatments in the mockup, but do not output a palette section or color token taxonomy. All statuses should also be communicated by text labels, badges, icons, layout priority, or motion.

---

## Core product mental model

The user is an engineering lead supervising many AI coding agents. The app must answer:

- What is running?
- What is blocked?
- What needs human input?
- Which task/ticket/plan item is each agent working on?
- Which model/harness/account profile is each session using?
- Which worktree and branch owns the changes?
- Which PR/check/review state is next?
- What changed in code?
- What is risky?
- What does Project Brain know about this work?
- What action can the user safely approve?

The **Session** is the atomic operational unit. A session links together project, task, terminal, harness, model, execution profile, worktree, branch, code changes, PR, approvals, transcript, usage, and Project Brain memory.

---

## Desktop shell to design

Design a persistent desktop app shell with:

1. Custom or native-feeling titlebar.
2. Left project/session sidebar.
3. Global command/search bar.
4. Main workspace.
5. Right inspector panel.
6. Project Brain drawer that can replace or sit beside the inspector.
7. Bottom event timeline/activity rail.
8. Compact status bar.

The app should be dense but readable.

---

## Required screens

Create these screens as high-fidelity desktop mockups. Use realistic sample data.

### 1. First Launch / Setup Wizard

Show a local setup flow:

- Runtime checks
- Claude Code detected
- Codex needs auth
- GitHub connected
- Linear not connected
- Project Brain store ready
- Execution Profiles setup
- Workflow Pack library available
- Approval/security policy choice
- Add first project

Components:

- Setup stepper
- Runtime check rows
- Consent cards
- Integration cards
- Execution Profile cards
- Security policy selector

### 2. Global Command Center

Workspace-wide overview with:

- Needs My Attention
- Active Work
- Agent Teams
- Recently Completed
- Risky/Stale
- Open PRs
- Usage hotspots
- Project Brain health

Include left sidebar, right inspector, bottom event timeline.

### 3. Project Home / Observability Graph

Selected project: “AI Engineering Control Plane”.

Show an operational graph with nodes for:

- Project
- Sessions
- Agent team
- Lead/orchestrator/workers
- Linear ticket
- GitHub issue
- PlanTask
- Worktree
- Branch
- PR
- Human approval
- Project Brain evidence

Include graph filters for status, object type, model, harness, execution profile, workflow, plan phase, ticket source, PR state, and human input needed.

Right inspector should show selected node details.

### 4. Project Sessions List

Dense table/board of sessions grouped by:

- Waiting
- Active
- Agent teams
- Idle
- Completed

Columns:

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

### 5. Session Terminal View

Embedded terminal for a Claude Code session.

Header should show:

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
- Linked ticket/plan task
- PR state

Right inspector should show:

- Task summary
- Pending approvals
- Recent tool calls
- Files changed
- Test/check output
- Git state
- Actions

Include a structured approval card outside the terminal text.

### 6. Code Editor / Diff Review Workspace

First-class review-focused editor.

Include:

- File explorer
- Changed files panel
- Editor tabs
- Code editor canvas
- Inline diff mode
- Side-by-side diff mode
- Conflict resolver state
- Problems panel
- Test output panel
- Code selection action menu
- Diff hunk action bar

Show context: session owner, task/plan task, worktree, branch, PR, and Project Brain evidence.

### 7. Plan View / Implementation Plan

Show parsed implementation plan from `MVP_TASKS.md`.

Include:

- Current state
- Carry-forward
- Phases
- Tracks
- PlanTask rows
- Architecture anchor chips
- Linked Linear/GitHub items
- Linked sessions/teams
- Linked PRs
- Task detail pane

Actions:

- Start session from task
- Start agent team from track
- Link Linear/GitHub item
- Create Linear issue from task
- Ask Project Brain

### 8. Task Inbox / GitHub + Linear

Tabs:

- GitHub Issues
- Linear Tickets
- PRs
- Plan Tasks
- Assigned to Agents
- Needs Review
- Completed

Show draggable task cards and a detail panel. Demonstrate drag/drop affordances to project, session, agent team, worktree, terminal composer, or plan task.

### 9. Worktree / Git / PR Control Center

Show:

- Worktree table/cards
- Branches
- Dirty state
- Linked sessions
- Linked tasks
- Last commits
- PR lanes
- Checks
- Merge conflicts

Actions:

- Create worktree
- Commit
- Push
- Rebase
- Merge main
- Create PR
- Resolve conflicts
- Request agent fix

### 10. PR Review Workspace

Show a PR produced by an agent/team.

Include:

- PR title/number
- Linked session/team
- Linked task/plan task/ticket
- Checks
- Review status
- Changed files
- Diff
- Project Brain review/evidence panel
- Actions to request fix, approve, merge, comment

### 11. Agent Team View

Show an active team launched through `/team-start`.

Include:

- Team objective
- Lead/orchestrator panel
- Worker cards
- Worker terminals as docked or popover panels
- Team graph
- Worktrees/branches/PRs
- Context tiers
- TDD slice tracker
- Escalation cards
- Broadcast controls

Roles:

- Team Lead
- Orchestrator
- Implementer: backend
- Implementer: frontend
- Implementer: tests

### 12. Workflow Setup + Command Registry

Show Workflow Pack and Workflow Instance behavior.

Include:

- Pack detected but not necessarily ready
- Instance status
- Personalization state
- Detected artifacts
- Manifest path
- Plan parser status
- Team launcher readiness
- Command registry table
- `/team-start`
- `/tdd`
- `/run-tests`
- `/context-check`
- `/scaffold-upgrade`

### 13. Workflow Personalization Review

Show the workflow template personalization flow:

- Inferred values
- Missing questions
- Generation plan
- Approval before writing files
- Generated file diff list
- Manifest preview
- Activate Workflow Instance

Make clear that a Workflow Pack template is not ready until personalized into a Workflow Instance.

### 14. Project Brain Drawer

Design as a persistent right-side drawer, not a generic chatbot.

Modes:

- Ask
- Plan
- Dispatch
- Review
- Decisions
- Memory
- Actions

Show a prompt like:

“Start the next backend task using the active workflow.”

The response should include:

- Retrieved task
- Evidence chips
- Suggested workflow command
- Proposed action plan
- Risk levels
- Approve all / approve step-by-step

Also show a historical answer:

“When did we implement the eval gate?”

Answer should include evidence chips for session, commit, PR, plan task, architecture anchor, and file/line.

### 15. Human Input Queue

Centralized list of blocked sessions and approvals.

Sections:

- Needs clarification
- Permission requests
- High-risk actions
- Workflow personalization approvals
- Project Brain action plans
- Agent team escalations

Each card must show risk, context, preview, and actions.

### 16. Action Gateway Review Modal / Panel

Show a Project Brain-proposed action plan:

- Create worktree
- Start `/team-start backend`
- Link team to PlanTask
- Open Agent Team View

Display:

- Summary
- Step list
- Risk levels
- Affected resources
- Preview/dry-run
- Required permissions
- Evidence
- Approval controls

### 17. Execution Profiles Settings

Show multiple Claude/Codex profiles:

- Claude Max Main
- Claude Max Secondary
- Claude Team Work
- Codex CLI Main
- Codex Cloud GitHub

Each profile should show auth state, default model, project allowlist, current sessions, and usage.

### 18. Usage / Context Dashboard

Show:

- Tokens today
- Cost today if available
- Context hotspots
- Usage by project
- Usage by session
- Usage by Execution Profile
- Exact/estimated/unavailable usage labels

### 19. Events / Audit Log

Show event timeline/table with filters.

Include events for:

- Session started
- Worktree created
- Workflow command invoked
- Approval requested
- Approval approved
- PR opened
- Project Brain answer generated
- Project Brain action plan proposed
- Workflow personalized

### 20. Settings / Integrations / Remote Access

Show:

- GitHub integration
- Linear integration
- Project Brain settings
- Workflow Pack library
- Security/approval policies
- Remote access/iOS companion stretch settings

Remote access should show notification-only and approval-capable modes, but not direct shell access.

---

## Required UI kit component families

Create components for:

- Desktop app shell
- Left project/session sidebar
- Global command palette
- Breadcrumbs
- View tabs
- Right inspector
- Project Brain drawer
- Bottom event timeline
- Status bar
- Project card/row
- Session row/card
- Agent team card
- Worker card
- Status pills
- Execution Profile badge/selector
- Harness/model badges
- Context and token usage meters
- Graph canvas
- Graph nodes and edges
- Embedded terminal
- Terminal tabs
- Message composer
- Tool call log
- Permission request card
- File explorer
- Changed files panel
- Editor tab
- Code editor canvas
- Inline diff hunk
- Side-by-side diff viewer
- Conflict resolver
- Problems panel
- Test output panel
- Code selection action menu
- Task card
- Ticket detail panel
- Plan phase row
- Plan task row
- Architecture anchor chip
- Worktree card/table row
- Branch badge
- Commit card
- PR card
- Checks panel
- Merge conflict alert
- Workflow Pack card
- Workflow Instance status panel
- Detection report
- Personalization stepper
- Generated file review row
- Command registry table
- Command card
- Launch recipe modal
- Evidence chip
- Brain answer card
- Brain action plan card
- Memory source card
- Decision log item
- Approval card
- Action plan review panel
- Action step row
- Risk explanation block
- Audit event card
- Setup stepper
- Runtime check row
- Integration card
- Security policy card
- Remote access pairing card
- Usage summary card
- Empty/loading/degraded/error states

---

## Sample data

Use this realistic sample data throughout the prototype.

Projects:

- AI Engineering Control Plane
- Project Brain
- cc-crew Scaffold Demo
- RepoGraph Parser
- Weekly Commit Automation

Execution Profiles:

- Claude Max Main
- Claude Max Secondary
- Claude Team Work
- Codex CLI Main
- Codex Cloud GitHub

Tasks:

- Linear ENG-221 — Add GitHub OAuth callback
- GitHub #184 — Fix parser memory leak
- PlanTask Phase 2.3 — Project observability graph
- PlanTask Phase 3.1 — Action Gateway approval cards
- GitHub PR #84 — Add workflow command registry

Sessions:

- Claude / ENG-221 GitHub OAuth callback
- Codex / GH-184 parser memory leak
- Claude Team / Phase 2 Observability Graph
- Claude / Docs drift refresh
- Codex / PR checks fix

Workflow:

- Workflow Pack: cc-crew
- Workflow Instance: Active
- Plan file: MVP_TASKS.md
- Architecture doc: ARCHITECTURE.md
- Team launcher: /team-start <track>
- Build loop: /tdd

Human input examples:

- Claude ENG-221 requests permission to run npm test.
- Codex PR-fix asks whether to update snapshots.
- Project Brain proposes creating a Linear issue from PlanTask Phase 2.3.
- Workflow personalization requests approval before writing generated files.

---

## Design style guidance

Use a dark or neutral technical developer-tool aesthetic if helpful, but do not define a color taxonomy. Emphasize:

- Dense but readable hierarchy
- Desktop app affordances
- Terminal-native credibility
- Code-review seriousness
- Operational clarity
- Attention-first sorting
- Clear status labels
- Strong object ownership
- Human control and safety
- Evidence and provenance

Avoid:

- Generic SaaS dashboard styling
- Oversized marketing cards
- AI sparkle/glow visual clichés
- Chatbot-first layout
- Decorative graphs
- Hiding terminal access
- Hiding approvals
- Hiding branch/worktree ownership
- Making Project Brain feel like a disconnected chat widget

---

## Prototype success criteria

The prototype succeeds if a viewer can understand, without verbal explanation:

1. This is a desktop app for managing AI coding agents.
2. Projects contain sessions, teams, tasks, worktrees, branches, PRs, and Project Brain memory.
3. Waiting-on-human input is globally visible.
4. Claude/Codex sessions are terminal-native but wrapped with context.
5. The code editor is a first-class review surface.
6. Implementation plan tasks can become sessions or agent teams.
7. GitHub/Linear items can be dragged/linked into agent workflows.
8. Workflow Packs can be detected, personalized, activated, and run.
9. `/team-start` creates observable lead/orchestrator/worker teams.
10. Project Brain can answer with evidence and propose actions.
11. The Action Gateway previews and approves risky actions.
12. Execution Profiles make account/subscription routing explicit.
13. Git/worktree/PR state is always tied back to sessions and tasks.
14. Events and audit history make the system trustworthy.
15. The iOS companion is a future observability/approval surface, not a remote shell.

