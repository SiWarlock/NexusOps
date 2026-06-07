# Design System Request — AI Engineering Control Plane Desktop App

I am attaching a set of product, architecture, UX, and component-planning docs for a desktop application currently working-named **AI Engineering Control Plane**. The final product name is not decided, so do not invent or lock in a brand name. Use neutral naming such as:

- “the platform”
- “AI Engineering Control Plane”
- “desktop control plane”
- “the app”

Your task is to create the **design system and UI kit foundation** for this platform.

Do not create the full app prototype yet. This pass is for the design system: foundations, layout rules, component library, component variants, interaction states, and example compositions that can later be used to create the full high-fidelity prototype.

## Attached docs and how to use them

Use the attached docs in this priority order:

### 1. `CLAUDE_DESIGN_PROTOTYPE_PROMPT.md`

Use this as the highest-level design direction. It describes the intended prototype scope and the major screens the design system must eventually support.

### 2. `UX_INFORMATION_ARCHITECTURE.md`

Use this as the main UX source of truth. It defines the desktop shell, navigation model, screen inventory, drawers, inspectors, workflows, and how the product behaves.

### 3. `UI_COMPONENT_INVENTORY.md`

Use this as the definitive component inventory. Every major component listed there should be represented in the design system, either as a fully specified component or as part of a clearly defined component family.

### 4. `PRD.md`

Use this to understand the product requirements, target users, MVP/P1/P2 scope, and major features. The design system should support the full product direction, not just the MVP.

### 5. `PRODUCT_CANON.md`

Use this to understand the product thesis, product principles, what the platform is, what it is not, and the overall experience we are trying to create.

### 6. `SHARED_OBJECT_MODEL.md`

Use this to understand the canonical system objects and relationships. The UI components should preserve the distinctions between objects like Project, Session, Agent Team, Worktree, Workflow Pack, Workflow Instance, Action Request, Approval, Pull Request, Event, and Project Brain evidence.

### 7. `ACTION_GATEWAY.md`

Use this to design permission, approval, review, risk, dry-run, action-plan, execution, rollback, and audit components.

### 8. `WORKFLOW_PACKS.md`

Use this to design Workflow Pack, Workflow Instance, personalization, command registry, launch recipe, and workflow lifecycle components.

### 9. `CC_CREW_WORKFLOW_PACK.md`

Use this to design cc-crew-specific components and states, including personalized scaffold flows, `MVP_TASKS.md`, `ARCHITECTURE.md` anchors, `/team-start`, `/tdd`, lead/orchestrator/implementer roles, context monitoring, escalation categories, and scaffold upgrade flows.

### 10. `PROJECT_BRAIN_INTERFACE.md`

Use this to design the Project Brain drawer, project memory UI, evidence chips, retrieval/provenance UI, action planning UI, and Project Brain-to-platform action flows.

### 11. `DESKTOP_FIRST_RUNTIME.md`

Use this to make the product feel like a desktop app with a local runtime, not a generic web SaaS dashboard.

### 12. `EVENT_MODEL_AND_AUDIT_TRAIL.md`

Use this to design event timeline, audit log, provenance, runtime state, system history, and traceability components.

If any docs conflict, prioritize them in the order above.

Do not use any legacy AgentOps Studio handoff docs as source of truth.

---

# Important constraints

## Do not include a color taxonomy

Do **not** create:

- A color palette
- Brand colors
- A semantic color scale
- A status color table
- Data visualization colors
- Light/dark color tokens
- Hex values
- Named color ramps

You may define **status semantics** and **visual treatment patterns** without assigning actual colors. For example:

- Active sessions need a live/high-visibility treatment.
- Waiting-on-human states need the strongest attention treatment.
- Failed states need an error treatment.
- Stale/degraded states need a warning treatment.
- Completed states need a muted resolved treatment.
- High context usage needs a capacity/risk treatment.

But do not define the actual colors yet.

## Desktop-first

This is a **desktop app**, not a web app.

Design for:

- Desktop window chrome assumptions
- Resizable panes
- Docked panels
- Split views
- Terminal embedding
- Local runtime status
- Local file paths
- Local worktrees
- Keyboard-first workflows
- Command palette usage
- Dense developer data
- Long-running background sessions
- Multi-pane task supervision

Avoid:

- Marketing SaaS dashboard patterns
- Oversized web cards
- Landing-page styling
- Chatbot-only layouts
- Mobile-first assumptions
- Generic admin panel UI

The future iOS companion app is only a stretch goal. Do not design the iOS app in this pass. However, the design system may include small desktop indicators for future remote access status.

---

# Product framing

This product is an **AI engineering control plane**.

It helps a developer supervise many AI coding agents across many projects. The user is not simply chatting with one assistant. The user is managing a fleet of coding workers.

The platform combines:

- Claude Code terminal sessions
- Codex terminal sessions
- Future coding harness adapters
- Agent teams
- Team lead / orchestrator / implementer workflows
- Git worktrees
- Branches
- Commits
- Pull requests
- GitHub Issues
- Linear tickets
- Implementation plans
- Workflow Packs
- Project-specific scaffolded workflows
- Action approvals
- Project Brain memory and reasoning
- Code editing and diff review
- Token/context/usage tracking
- Event timelines and auditability

The product should feel like:

- A command center
- A terminal-aware desktop IDE companion
- An agent operations console
- A review-focused code workspace
- A git/worktree/PR control center
- A project-memory-aware orchestration tool
- A high-trust engineering cockpit

It should not feel like:

- A generic IDE
- A generic chatbot
- A generic project management tool
- A generic terminal multiplexer
- A generic observability dashboard
- A generic SaaS admin console

---

# Design system objective

Create a comprehensive design system and UI kit foundation that supports every major part of the platform.

The output should include:

1. Design system overview
2. Experience principles
3. Desktop layout system
4. Panel and shell architecture
5. Typography guidance
6. Spacing and sizing guidance
7. Iconography guidance
8. Elevation/layering guidance
9. Motion and feedback guidance
10. Status semantics without color taxonomy
11. Component families
12. Component anatomy
13. Component variants
14. Component states
15. Interaction patterns
16. Accessibility requirements
17. Empty states
18. Loading states
19. Error states
20. Degraded states
21. Approval/permission states
22. Example component compositions
23. UI kit page structure
24. Guidance for the later high-fidelity prototype pass

Again: do not include a color taxonomy.

---

# Core design principles

Use these principles throughout the design system.

## 1. Attention-first hierarchy

The UI should prioritize what needs the human.

Highest attention:

- Waiting on human input
- Waiting on permission
- Failed sessions
- Blocked sessions
- Merge conflicts
- Dangerous actions
- Failing checks
- High context usage
- High cost usage
- Stale/degraded runtime state

Lower attention:

- Normal active work
- Idle work
- Completed work
- Archived work

## 2. Operational clarity over decoration

The product can look polished and premium, but the design must optimize for operational clarity. Graphs, cards, badges, and panels must expose real state and useful actions.

## 3. Terminal-first, not chatbot-first

Claude Code and Codex terminal sessions are first-class. Do not hide them behind a fake chat-only interface.

## 4. Code review is first-class

The code editor and diff review workspace must be treated as a core product surface, not an afterthought.

## 5. Every object shows ownership

For any task, session, code change, PR, or event, the UI should make it clear:

- Which project it belongs to
- Which session or agent produced it
- Which execution profile was used
- Which worktree and branch own it
- Which ticket/task/plan item it maps to
- Which PR or artifact resulted
- Whether human approval is needed

## 6. Project Brain suggests; Action Gateway confirms

Project Brain can reason, recommend, draft, and propose actions. The platform executes actions through the Action Gateway with previews, risk treatment, approvals, and audit logs.

## 7. Workflow Packs are optional but powerful

The platform must work without a Workflow Pack, but when a project has one, the UI should expose its commands, plan structure, personalization state, team launch recipes, and lifecycle.

## 8. Desktop density

This is a technical tool for expert users. Use high information density, but organize it carefully with hierarchy, grouping, and progressive disclosure.

---

# Core product objects to support visually

Design components for these objects. Do not flatten everything into generic cards.

## Workspace and project objects

- Workspace
- Project
- Repository
- Local path
- Runtime status
- Integration status
- Project Brain status
- Workflow status

## Git/code objects

- Worktree
- Branch
- Commit
- Changed file
- Diff hunk
- Conflict
- Pull Request
- Check run
- Review comment
- Merge action

## Work/task objects

- Task
- GitHub Issue
- Linear Ticket
- Implementation Plan
- Plan Phase
- Plan Track
- Plan Task
- Architecture Anchor
- Acceptance Criteria
- Linked issue/ticket/session/PR

## Agent/session objects

- Session
- Claude Code session
- Codex session
- Terminal
- Agent Team
- Team Lead
- Orchestrator
- Implementer
- Worker Agent
- Model
- Harness
- Execution Profile
- Context window
- Token usage
- Cost usage
- Tool call
- Permission request

## Workflow objects

- Workflow Pack
- Workflow Instance
- Workflow Personalization Run
- Workflow Command
- Slash Command
- Skill
- Subagent
- Hook
- Launch Recipe
- Scaffold Manifest
- Upgrade Check

## Project Brain objects

- Project Brain
- Memory Source
- Evidence Item
- Evidence Chip
- Episode Card
- Decision
- Retrieval Result
- Provenance Stamp
- Brain Action Plan
- Brain Suggested Action

## Action Gateway objects

- Action Request
- Action Plan
- Action Step
- Approval
- Risk Level
- Dry Run
- Preconditions
- Execution Result
- Rollback/Undo Availability
- Audit Entry

## Event/audit objects

- Event
- Timeline Item
- Audit Log Entry
- Runtime Event
- Session Event
- Git Event
- PR Event
- Workflow Event
- Brain Event
- Approval Event
- Integration Event

---

# Desktop app shell

Design a shell system with these persistent regions.

## Top app bar

The top bar should support:

- Workspace/project context
- Current selected project
- Global command palette trigger
- Runtime status
- Local daemon status
- Project Brain status
- GitHub/Linear sync status
- Execution profile indicators
- Waiting-on-human count
- Notifications
- Optional remote access status
- Settings shortcut

## Left sidebar

The sidebar should support:

- Workspace switcher
- Global Command Center
- Human Input Queue
- Project list
- Nested sessions under each project
- Active sessions sorted above idle/completed sessions
- Waiting-on-human sessions visually prioritized
- Agent teams expandable into lead/orchestrator/worker sessions
- Project shortcuts:
  - Overview
  - Sessions
  - Plan
  - Code
  - Worktrees
  - Tasks
  - PRs
  - Workflow
  - Commands
  - Brain
  - Events
  - Settings

Each project row should support:

- Project status
- Active count
- Waiting count
- Runtime/degraded indicator
- Workflow status
- Project Brain index status

Each session row should support:

- Session name
- Harness
- Model/profile
- Status
- Linked task/ticket
- Worktree/branch
- Context usage
- Waiting/failure indicator
- Team membership

## Main content region

The main content region should support:

- Global Command Center
- Project Home / Observability Graph
- Project Sessions List
- Session Terminal View
- Code Editor / Diff Review Workspace
- Plan View
- Task Inbox
- Worktree / Git / PR Control Center
- PR Review Workspace
- Agent Team View
- Workflow Setup
- Command Registry
- Human Input Queue
- Usage / Context Dashboard
- Events / Audit Log
- Settings / Integrations

## Right inspector

The inspector should show details/actions for the selected object.

It needs variants for:

- Project
- Session
- Agent Team
- Task
- Plan Task
- GitHub Issue
- Linear Ticket
- Worktree
- Branch
- Pull Request
- Workflow Pack
- Workflow Instance
- Workflow Command
- Approval
- Action Request
- Event
- Evidence Item
- Memory Source

## Project Brain drawer

The Project Brain drawer is separate from the normal inspector.

It should support:

- Ask
- Plan
- Review
- Memory
- Decisions
- Action planning

It should be able to show:

- Chat-like project-aware Q&A
- Evidence chips
- Source snippets
- Linked code
- Linked session history
- Linked commits
- Linked PRs
- Linked plan tasks
- Suggested actions
- Draft action plans
- Approval handoff into Action Gateway

## Bottom activity timeline

The bottom strip/panel should support:

- Workspace timeline
- Project timeline
- Session timeline
- Agent team timeline
- Git timeline
- Workflow timeline
- Approval timeline
- Brain timeline
- Filtered events
- Expandable details
- Jump-to-object links

---

# Required component families

Create design system components for all of the following families.

## 1. App shell components

- Desktop window shell
- Top app bar
- Runtime status indicator
- Local daemon status indicator
- Project Brain status indicator
- Integration sync indicator
- Global command palette trigger
- Notification/waiting count indicator
- Left sidebar
- Sidebar project group
- Sidebar session row
- Sidebar agent team row
- Sidebar nested worker row
- Sidebar section header
- Sidebar quick action
- Right inspector panel
- Project Brain drawer
- Bottom timeline panel
- Resizable pane divider
- Docked panel
- Split view
- Modal overlay
- Popover
- Drawer
- Toast / notification

## 2. Status and metadata components

- Status pill
- Harness badge
- Model badge
- Execution profile badge
- Context usage meter
- Token usage meter
- Cost usage badge
- Runtime health badge
- Stale/degraded badge
- Waiting-on-human indicator
- Waiting-on-permission indicator
- Failed/blocked indicator
- Completed/archived indicator
- Sync freshness stamp
- Provenance stamp
- Risk indicator
- Approval state badge
- Mergeability badge
- Check status badge
- Dirty worktree badge
- Conflict badge

Do not define colors for these badges. Define their role, shape, density, hierarchy, and state behavior only.

## 3. Project and session navigation components

- Project row
- Project card
- Project overview header
- Session row
- Session card
- Session table row
- Session group by status
- Session group by team
- Session search/filter bar
- Session quick actions
- Session health summary
- Session context/cost summary
- Agent team sidebar group
- Agent team summary card
- Worker session row

## 4. Graph and observability components

- Graph canvas
- Project node
- Session node
- Claude node
- Codex node
- Agent team node
- Team lead node
- Orchestrator node
- Implementer node
- Worker node
- Worktree node
- Branch node
- PR node
- GitHub Issue node
- Linear Ticket node
- Human Input Required node
- Approval node
- Project Brain/memory node
- Edge/relationship line
- Edge label
- Node cluster
- Node detail popover
- Graph minimap
- Graph filter bar
- Graph legend without color taxonomy
- Graph selected state
- Graph degraded/partial state
- Graph empty state
- List/table fallback for graph

Graph nodes should be operational. They must show state, ownership, and action affordances.

## 5. Terminal/session components

- Embedded terminal panel
- Terminal tab group
- Terminal session header
- Terminal status strip
- Terminal command marker
- Agent message composer
- Send instruction composer
- Tool call log
- Tool call row
- Permission prompt card
- Approval inline card
- Session transcript drawer
- Session summary panel
- Attach/detach terminal button
- Pause/resume/kill controls
- Session restart/resume controls
- Session linked task card
- Session linked worktree card
- Files changed mini-list
- Current activity indicator

## 6. Code editor and diff review components

- File explorer
- Changed files panel
- Editor tab bar
- Editor canvas
- File breadcrumb
- Code minimap placeholder
- Inline diff viewer
- Side-by-side diff viewer
- Diff hunk header
- Hunk action controls
- Accept/reject/request-change hunk actions
- Conflict resolver
- Conflict block
- Problems/diagnostics panel
- Test output panel
- Search/find bar
- Code selection action menu
- “Ask Project Brain” selection action
- “Ask active agent” selection action
- “Request fix” selection action
- “Add tests” selection action
- “Send to session” selection action
- Review comment composer
- Open externally button
- Linked session/worktree/branch/PR strip

The editor should be first-class but review-focused.

## 7. Plan and task components

- Implementation plan header
- Plan phase group
- Plan track group
- Plan task row
- Plan task card
- Plan task detail panel
- Architecture anchor link
- Acceptance criteria block
- Carry-forward block
- Decision/open question block
- Plan status indicator
- Plan linked session indicator
- Plan linked PR indicator
- Plan linked Linear/GitHub indicator
- Start session from task button
- Start agent team from track button
- Link task to ticket action
- Create ticket from plan task action
- Plan drift/stale anchor indicator
- Task source badge
- Task priority indicator
- Task dispatch target selector

## 8. GitHub/Linear task inbox components

- Task inbox shell
- Source tabs
- GitHub issue card
- Linear ticket card
- PR task card
- Task table row
- Task detail panel
- Task labels
- Task priority
- Task assignee
- Linked project/repo
- Suggested agent/harness/profile
- Suggested worktree name
- Drag handle
- Drop target
- Dispatch action
- Link to plan task
- Create plan task from ticket
- Sync status indicator
- Manual link state
- One-way create state
- Conflict/out-of-sync state placeholder

## 9. Worktree/Git/PR components

- Worktree card
- Worktree table
- Worktree status cell
- Dirty files summary
- Branch badge
- Ahead/behind indicator
- Commit summary
- Git action toolbar
- Create worktree modal
- Delete/archive worktree confirmation
- Rebase/merge main action
- Push action
- Commit action
- Conflict alert
- Conflict resolver entry point
- PR card
- PR table row
- PR grouped board
- Check run list
- Check failure row
- Mergeability panel
- Review status panel
- Create PR modal
- Merge confirmation modal
- Request agent fix button
- Failing checks → fix session flow card

## 10. Agent team components

- Agent team header
- Team objective card
- Team plan card
- Lead session card
- Orchestrator session card
- Implementer/worker card
- Worker terminal dock
- Team hierarchy tree
- Team topology graph
- Team timeline
- Team broadcast composer
- Ask lead for status button
- Pause all / resume all / end team controls
- Worker context meter
- Team context monitor
- Escalation card
- Escalation queue
- Merge/reconcile outputs panel
- Team artifact list
- Team PR/diff summary
- cc-crew `/team-start` launch card
- `/tdd` lifecycle tracker

## 11. Workflow Pack components

- Workflow Pack card
- Workflow Pack library list
- Workflow Instance card
- Workflow lifecycle state component
- Workflow setup panel
- Personalization run wizard
- Inferred values table
- Missing values prompt
- Generated files diff
- Personalization approval card
- Workflow manifest viewer
- Workflow health panel
- Workflow drift indicator
- Upgrade available card
- Upgrade check result
- Command registry table
- Workflow command card
- Slash command card
- Skill card
- Subagent card
- Hook card
- Launch recipe card
- Run command modal
- Command argument form
- Command preview
- Command execution result

Important: visually distinguish these states:

- Workflow Pack available
- Workflow Pack installed
- Workflow Instance detected
- Needs personalization
- Personalization in progress
- Generated, review required
- Active
- Ready for team mode
- Degraded
- Drift detected
- Upgrade available
- Archived/detached

## 12. Project Brain components

- Project Brain drawer
- Brain mode tabs:
  - Ask
  - Plan
  - Review
  - Memory
  - Decisions
  - Actions
- Brain message
- Brain answer
- Evidence chip
- Evidence list
- Source preview
- Code evidence card
- Session evidence card
- Commit evidence card
- PR evidence card
- Plan task evidence card
- Architecture anchor evidence card
- Memory source card
- Episode card
- Decision card
- Retrieval result card
- Provenance stamp
- Freshness/staleness banner
- Brain suggested action
- Brain action plan preview
- Brain draft output
- Brain confidence/grounding indicator
- “Send to Action Gateway” button
- “Ask follow-up” composer
- “Open evidence” action

Project Brain should feel powerful but bounded. It can suggest and plan actions, but the platform confirms execution through Action Gateway.

## 13. Action Gateway components

- Action Review Modal
- Action Request card
- Action Plan card
- Action step list
- Risk level indicator
- Permission requirement row
- Dry-run result panel
- Preconditions panel
- Impact summary
- Affected objects list
- Approval decision controls
- Approve all
- Approve step-by-step
- Deny
- Edit action
- Require manual execution
- Execution progress state
- Execution result state
- Partial failure state
- Rollback availability indicator
- Audit log link
- High-risk confirmation pattern
- Critical action confirmation pattern

Action Gateway surfaces must feel trustworthy and explicit.

## 14. Event and audit components

- Timeline item
- Event row
- Event detail drawer
- Audit log table
- Event filter bar
- Actor badge
- Object reference chip
- Event severity/treatment
- Event source badge
- Session event
- Git event
- PR event
- Workflow event
- Approval event
- Brain event
- Integration event
- Runtime event
- Event replay/open target action
- Event empty state
- Event degraded state

## 15. Execution profile components

- Execution profile card
- Execution profile selector
- Execution profile badge
- Account alias
- Provider/harness indicator
- Default model setting
- Permission mode setting
- Usage summary
- Profile health
- Project allowlist
- Active sessions using profile
- Profile warning/degraded state
- Profile settings form

Execution Profiles represent different local account/runtime contexts, such as different Claude subscriptions or Codex accounts. Make this a first-class UI concept.

## 16. Usage/context components

- Usage dashboard
- Context usage meter
- Token usage chart placeholder
- Cost usage chart placeholder
- Per-session usage row
- Per-project usage row
- Per-profile usage row
- Budget/threshold indicator
- High-context warning
- Context limit risk card
- Usage by model/harness
- Usage by project
- Usage by agent team

Do not define data visualization colors.

## 17. Settings/integration components

- Settings shell
- Integrations list
- GitHub connection card
- Linear connection card
- Claude Code detection card
- Codex detection card
- Project Brain connection card
- Workflow Pack library settings
- Local runtime settings
- Security/permissions settings
- Remote access stretch-goal settings placeholder
- Setup status checklist
- Repair setup action
- Degraded integration card
- Reconnect action

---

# Required states

For each major component family, define relevant states.

At minimum include:

- Default
- Hover
- Focus
- Selected
- Active
- Disabled
- Loading
- Empty
- Error
- Warning/degraded
- Stale
- Waiting on human
- Waiting on permission
- Running
- Paused
- Failed
- Completed
- Archived
- Dirty
- Conflict
- Syncing
- Out of sync
- Requires approval
- Approved
- Denied
- Executing
- Partially failed
- Rolled back
- Read-only
- Locked
- High context usage
- High cost usage

Do not rely only on color for state. Use shape, iconography, labels, hierarchy, motion, density, and text.

---

# Interaction patterns to define

Create patterns for:

- Global command palette
- Search/filter
- Drag GitHub/Linear task to session/project/team/worktree
- Drag plan task to session/project/team
- Open terminal from session
- Open code editor from diff/file/PR/session
- Open Project Brain drawer from any object
- Ask Project Brain about selected code
- Ask active agent to fix selected code
- Request agent fix from failing PR check
- Create worktree from task
- Create session from task
- Start agent team from plan track
- Invoke workflow command
- Approve/deny Action Gateway request
- Step-by-step approval
- Dry-run before execution
- Link plan task to Linear/GitHub
- Link session to task
- Create PR from session
- Merge PR through Action Gateway
- Archive completed session
- Resolve degraded setup
- Open event target from audit timeline

---

# Accessibility requirements

The design system must support:

- Keyboard-first navigation
- Command palette access to major actions
- Visible focus states
- Screen-reader-friendly labels
- Non-color-only status communication
- Reduced motion option
- High density without tiny unreadable text
- Resizable panels
- Clear destructive action confirmation
- Clear distinction between read-only, draft, and execution states
- Safe handling of high-risk actions

---

# Output format

Please produce the design system in a structured way that could be translated into a Figma/Claude Design UI kit.

Organize the output into these sections:

1. **Design System Overview**
   - What this system is for
   - Product personality
   - Core UX principles
   - Desktop-first assumptions

2. **Foundations**
   - Typography guidance
   - Spacing/sizing guidance
   - Layout grid/panel guidance
   - Iconography guidance
   - Elevation/layering guidance
   - Motion/feedback guidance
   - Status semantics without colors
   - Accessibility principles

3. **Desktop Shell System**
   - Top bar
   - Sidebar
   - Main content region
   - Inspector
   - Project Brain drawer
   - Bottom timeline
   - Resizable panels
   - Modals/drawers/popovers

4. **Component Library**
   - Organize by the component families above.
   - For each component, define:
     - Purpose
     - Anatomy
     - Variants
     - States
     - Primary interactions
     - Accessibility notes
     - Example usage

5. **Complex Composite Components**
   - Session row
   - Agent team card
   - Project graph node
   - Action Gateway modal
   - Project Brain answer with evidence
   - Workflow Instance card
   - Plan task row
   - Worktree card
   - PR review card
   - Terminal header
   - Code diff hunk

6. **Example Layout Compositions**
   - Do not build the final prototype yet, but include small reference compositions for:
     - Global Command Center
     - Project Observability Graph
     - Session Terminal
     - Code Editor / Diff Review
     - Plan View
     - Task Inbox
     - Worktree / Git / PR Center
     - Agent Team View
     - Workflow Setup
     - Project Brain Drawer
     - Action Gateway Review Modal
     - Events / Audit Log

7. **UI Kit Page Structure**
   - Recommend how to organize the design system pages, for example:
     - Foundations
     - App Shell
     - Navigation
     - Status + Metadata
     - Sessions + Terminals
     - Code Editor + Diff
     - Graph + Observability
     - Plan + Tasks
     - Git + PRs
     - Agent Teams
     - Workflow Packs
     - Project Brain
     - Action Gateway
     - Events + Audit
     - Settings + Integrations
     - Empty/Loading/Error States
     - Prototype Compositions

8. **What to Build Next**
   - After the design system is approved, list the next prototype screens to create in order.

---

# Visual direction

Use a polished, premium, technical developer-tool aesthetic.

The product should feel:

- Dense but readable
- Calm but powerful
- Local-first
- Terminal-aware
- Trustworthy
- Operational
- Precise
- Slightly futuristic, but not gimmicky
- More like an engineering cockpit than a SaaS dashboard

Avoid:

- AI sparkle/glow clichés
- Oversized empty dashboard cards
- Marketing-site aesthetics
- Overly playful visuals
- Web-admin generic components
- Hiding the terminal
- Hiding approval/risk states
- Treating the graph as decorative
- Treating Project Brain as a generic chatbot
- Treating Workflow Packs as simple settings toggles

---

# Key product truths the design system must preserve

- The app is desktop-first.
- The session is the atomic unit of agent work.
- The terminal is first-class.
- The code editor and diff review workspace are first-class.
- Git worktrees are central to safe parallel agent work.
- Agent teams can include lead, orchestrator, and implementer sessions.
- Execution Profiles are first-class because sessions may run under different Claude/Codex accounts or subscriptions.
- Project Brain is standalone but integrated through a drawer/interface.
- Project Brain can propose actions but execution flows through Action Gateway.
- Workflow Packs are optional but powerful.
- A Workflow Pack template may require personalization before it becomes an active Workflow Instance.
- cc-crew is a supported Workflow Pack, but the platform must not require cc-crew.
- GitHub/Linear linking starts manual-first, then one-way creation, then eventual controlled bidirectional sync.
- The project graph is operational, not decorative.
- Human-input-needed states must be globally visible.
- Every important action should be auditable.
- Degraded/stale/partial states must be visible, not hidden.

---

# Final instruction

Create the design system and UI kit foundation only.

Do not create a final app mockup yet.

Do not include a color taxonomy.

Make the design system comprehensive enough that the next Claude Design pass can generate a high-fidelity desktop prototype representing the full platform.