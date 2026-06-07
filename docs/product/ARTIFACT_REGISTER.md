# Platform Artifact Register v1

> Purpose: Track the product artifacts that should come out of planning for the AI coding operations platform and Project Brain. This register prevents decisions from getting lost as the product expands.

---

## 1. Artifact Strategy

This product has too many interlocking ideas to keep in chat memory. From this point forward, decisions should be captured into named artifacts.

The artifact set should separate:

- Product definition
- Formal PRDs
- Object models
- Event/action contracts
- Workflow pack specs
- UX specs
- Design-system handoffs
- Security requirements
- Roadmap and decision logs

The design system should be downstream of the product model, not the other way around.

---

## 2. Recommended Artifact Set

### A1. Platform Product Canon

Purpose: One top-level explanation of what the parent platform is.

Contents:

- Product thesis
- Problem statement
- Target users
- Jobs to be done
- Product principles
- What the product is / is not
- Core product layers
- Core modules
- Core objects
- Major workflows
- MVP/P1/P2 scope
- Key open questions
- Decision log summary

Status: Not created yet.

Priority: Immediate.

---

### A2. AgentOps Platform PRD

Purpose: Formal product requirements for the main orchestration/control-plane product.

Contents:

- Product overview
- Personas
- Jobs to be done
- Functional requirements
- Non-functional requirements
- Project/session management
- Terminal management
- Code editor/review workspace
- Worktree/git/PR management
- Task inbox
- Linear/GitHub integrations
- Agent team orchestration
- Execution profiles
- Human input queue
- Project graph
- Usage/token/context tracking
- Settings
- MVP scope
- Success metrics

Status: Not created yet.

Priority: High.

---

### A3. Project Brain PRD

Purpose: Standalone-but-platform-native PRD for Project Brain.

Contents:

- Memory ingestion
- Code/docs/session indexing
- Vector/structured stores
- Evidence chips
- Historical implementation queries
- Session memory
- Workflow-pack awareness
- Action planning
- Action Gateway integration
- Privacy and safety
- MVP/P1/P2

Status: Created as `project_brain_prd_v2_platform_native.md`.

Priority: Immediate.

---

### A4. Shared Object Model and Data Model

Purpose: Define the nouns of the system so the platform, Project Brain, and workflow packs share concepts.

Objects to include:

- Workspace
- Project
- Repository
- Worktree
- Branch
- Task
- PlanTask
- ImplementationPlan
- Session
- AgentTeam
- ExecutionProfile
- WorkflowPack
- WorkflowInstance
- WorkflowPersonalizationRun
- WorkflowCommand
- SubagentDefinition
- SkillDefinition
- HookDefinition
- ProjectBrainMemorySource
- Decision
- Artifact
- PullRequest
- Approval
- Event
- ActionPlan

For each object:

- Definition
- Key fields
- Relationships
- Lifecycle states
- Events emitted
- UI surfaces

Status: Not created yet.

Priority: Immediate.

---

### A5. Event Model and Audit Trail Spec

Purpose: Define how platform state changes become durable, indexable events for Project Brain and auditability.

Contents:

- Event naming conventions
- Event envelope
- Event versioning
- Project events
- Workflow events
- Session events
- Agent team events
- Git/worktree events
- PR events
- Task/ticket events
- Approval events
- Brain events
- Artifact events
- Replay/reindex behavior
- Audit log retention

Example events:

- ProjectAdded
- WorkflowPackDetected
- WorkflowInstancePersonalized
- PlanTaskLinked
- SessionStarted
- SessionEnded
- AgentTeamStarted
- WorktreeCreated
- CommitCreated
- PROpened
- ApprovalRequested
- ApprovalResolved
- BrainActionRequested
- BrainActionApproved
- BrainActionExecuted

Status: Not created yet.

Priority: High.

---

### A6. Action Gateway Spec

Purpose: Define how Project Brain can safely take actions through the platform.

Contents:

- Action schema
- Action categories
- Risk levels
- Permission model
- Preview/dry-run behavior
- Confirmation behavior
- Audit logging
- Rollback/undo expectations
- Bundled workflows
- Policy automation
- Allowed/disallowed Brain actions

Status: Not created yet.

Priority: Immediate, because it shapes Project Brain architecture.

---

### A7. Workflow Packs Specification

Purpose: Define the generic abstraction that supports cc-crew and other project workflows.

Contents:

- Workflow Pack concept
- Workflow Instance concept
- Workflow Personalization Run
- Detection rules
- Manifest rules
- Lifecycle states
- Command registry
- Plan parsers
- Team launch recipes
- Install/update/upgrade flows
- Template-vs-active-instance distinction

Status: Not created yet.

Priority: Immediate.

---

### A8. cc-crew Workflow Pack Integration Spec

Purpose: Dedicated integration spec for the user’s cc-crew scaffold.

Contents:

- Source repo
- Template/personalization flow
- Expected files
- Manifest interpretation
- `MVP_TASKS.md` parser
- `ARCHITECTURE.md` anchor mapping
- `/team-start` launcher
- `/tdd` lifecycle
- Team lead/orchestrator/implementer roles
- Context monitoring
- Escalation categories
- Upgrade handling
- UI surfaces

Status: Not created yet.

Priority: High.

---

### A9. UX / Information Architecture Spec

Purpose: Define app behavior before generating more UI mockups.

Contents:

- Navigation model
- App shell
- Left sidebar
- Right inspector
- Project Brain drawer
- Bottom timeline
- Project dashboard
- Session terminal view
- Code editor view
- Plan view
- Task inbox
- Worktree/git/PR center
- Agent team view
- Workflow setup view
- Command registry
- Human input queue
- Settings
- Empty/loading/error states
- Keyboard shortcuts
- Drag/drop behavior

Status: Not created yet.

Priority: High.

---

### A10. Screen-by-Screen UI Requirements

Purpose: Detailed requirements for mockups and Claude Design prompts.

Screens:

- Global Command Center
- Project Observability Graph
- Session Terminal View
- Code Editor / Diff Review Workspace
- Plan View
- Task Inbox
- Worktree / Git / PR Control Center
- Agent Team View
- Workflow Setup
- Execution Profiles Settings
- Project Brain Drawer
- Usage Dashboard
- Command Registry
- Human Input Queue

For each screen:

- Purpose
- Primary user question
- Required components
- Required states
- Primary actions
- Secondary actions
- Data shown
- Empty state
- Error state

Status: Not created yet.

Priority: Medium-high.

---

### A11. Design System / UI Kit Handoff

Purpose: Revised design-system handoff for Claude Design after product model stabilizes.

Contents:

- Component inventory
- Layout principles
- Density rules
- Typography direction
- Status treatments
- Interaction states
- Graph components
- Terminal components
- Code editor components
- Plan/task components
- Workflow components
- Brain/action components
- Accessibility notes

Status: Existing v2 handoff exists; needs future update.

Priority: Later, after product/UX specs.

---

### A12. Integration Requirements Spec

Purpose: Define external integrations.

Contents:

- Claude Code integration
- Codex CLI integration
- Claude/Codex execution profiles
- GitHub issues/PRs
- Linear issues
- Git worktrees
- Local filesystem
- Terminal sessions
- MCP
- Future cloud runner

Status: Not created yet.

Priority: Medium-high.

---

### A13. Security, Permissions, and Safety Spec

Purpose: Define safe operation boundaries.

Contents:

- Execution profile boundaries
- Credential handling
- Local command permissions
- Dangerous command detection
- Git action permissions
- Merge permissions
- Secret redaction
- Project Brain action permissions
- Audit logging
- Auto-approval policies
- Policy levels

Status: Not created yet.

Priority: High.

---

### A14. Roadmap and Phasing Plan

Purpose: Keep scope controlled.

Contents:

- MVP
- P1
- P2
- Deferred features
- Build sequence
- Dependency map
- Risks
- Technical spikes
- Design spikes

Status: Not created yet.

Priority: High.

---

### A15. Open Questions and Decision Log

Purpose: Track decisions and unresolved questions.

Contents:

- Product questions
- Architecture questions
- UX questions
- Integration questions
- Decisions made
- Date
- Reasoning
- Revisit trigger

Status: Started below.

Priority: Immediate.

---

## 3. Current Decisions to Preserve

### D1. The parent platform is an AI coding operations console

It is not just an IDE, chatbot, terminal manager, observability dashboard, or task board.

### D2. Session is the atomic operational unit

Sessions connect task, model, harness, execution profile, worktree, terminal, trace, code changes, PR, context usage, token usage, approvals, and history.

### D3. Project Brain is standalone now, platform-native later

Project Brain should ship as a useful local memory engine, while using shared IDs and future platform event/action contracts.

### D4. Project Brain should be action-capable through an Action Gateway

It should not directly execute privileged platform operations.

### D5. Workflow Packs are first-class

Custom scaffolding should be represented as a reusable workflow pack that can become a project-specific workflow instance.

### D6. cc-crew is a template workflow pack, not a ready-to-run requirement

The scaffold requires personalization after architecture and task artifacts exist.

### D7. Execution Profiles are first-class

Sessions and agent team workers should explicitly show which Claude/Codex account/subscription/runtime profile they are using.

### D8. Code editor is first-class but review-focused first

The platform should support code review, diff inspection, conflict resolution, diagnostics, and ask-agent-on-selection before trying to replace VS Code entirely.

### D9. Linear/GitHub implementation-plan sync should start with linking

Manual linking comes before one-way creation; bidirectional sync is later and requires conflict handling.

### D10. Human-input-needed is attention-first

Waiting/blocked/approval-needed sessions should outrank normal active sessions in navigation and dashboards.

---

## 4. Current Open Questions

1. What is the parent platform name?
2. What is the final Project Brain name?
3. Is the platform a desktop app, web app with local daemon, or both?
4. What is the first supported runtime: Claude Code only or Claude Code + Codex?
5. What is the minimum viable code editor?
6. What actions can Project Brain take without confirmation?
7. What actions always require confirmation?
8. How should Execution Profiles map to local authenticated Claude/Codex contexts?
9. What is the first demo workflow?
10. How much of cc-crew parsing belongs in the generic Workflow Pack layer vs the cc-crew integration spec?
11. How should Project Brain store plan-task-to-Linear mappings?
12. How should workflow personalization runs be represented and replayed?
13. Which event schema should platform and Project Brain share?
14. What should the Action Gateway minimum schema be?
15. How should transcript redaction be tested?

---

## 5. Suggested Artifact Creation Order

1. Project Brain PRD v2 — created.
2. Platform Product Canon.
3. Shared Object Model.
4. Action Gateway Spec.
5. Workflow Packs Spec.
6. UX / Information Architecture Spec.
7. AgentOps Platform PRD.
8. cc-crew Workflow Pack Integration Spec.
9. Security, Permissions, and Safety Spec.
10. Design System / UI Kit Handoff v3.

