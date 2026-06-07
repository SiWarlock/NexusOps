# cc-crew Workflow Pack Integration Spec v0.1

> Status: Draft v0.1  
> Scope: Concrete Workflow Pack integration for `claude-code-tdd-agent-crew-scaffold`  
> Related artifacts: Workflow Packs Spec v0.1, Project Brain PRD v2, Action Gateway Spec v0.1, Event Model & Audit Trail Spec v0.1  
> Important clarification: cc-crew is a template/scaffold workflow. It is not automatically ready to run in a project until personalized/generated from project-specific architecture and artifacts.

---

## 1. Executive summary

cc-crew should be modeled as the first rich Workflow Pack integration.

It is not merely a set of Claude commands. It is a project operating workflow that moves from PRD/research/architecture to an implementation plan, generates project-specific Claude scaffolding, then runs single-operator or team-based TDD implementation sessions.

The platform should support cc-crew without requiring it.

```text
Basic project
  No cc-crew; platform still works.

cc-crew template available
  The pack exists but has not been applied to this project.

cc-crew personalization needed
  Project has architecture/planning artifacts but no generated workflow instance.

cc-crew active instance
  Project has been personalized/generated and commands are ready.
```

The most important product requirement is to distinguish the template from the personalized instance.

---

## 2. Integration goals

```text
Detect cc-crew-related files.
Understand whether the project has an active generated instance.
Expose cc-crew commands in the Command Registry.
Parse MVP_TASKS.md into ImplementationPlan and PlanTask objects.
Map architecture anchors to plan tasks.
Expose /team-start as an agent team launch recipe.
Expose /tdd as a TDD slice workflow.
Surface context heartbeat/status when available.
Route scaffold generation, team start, upgrades, and file writes through the Action Gateway.
Allow Project Brain to reason over cc-crew artifacts and propose actions.
Keep cc-crew optional and decoupled.
```

---

## 3. Integration non-goals

```text
Do not make the platform depend on cc-crew.
Do not make Project Brain require cc-crew docs.
Do not mutate .scaffolding/manifest.json directly unless the cc-crew workflow explicitly owns that operation.
Do not assume a cc-crew template is project-ready before personalization.
Do not implement bidirectional Linear sync in MVP.
Do not rewrite the user's scaffold in this spec.
```

---

## 4. cc-crew as a Workflow Pack

### 4.1 Pack identity

```yaml
WorkflowPack:
  id: cc-crew
  displayName: cc-crew TDD Agent Crew
  sourceType: git_repo | local_directory
  sourceUri: https://github.com/SiWarlock/claude-code-tdd-agent-crew-scaffold
  supportedHarnesses:
    - claude_code
  capabilities:
    - template_scaffolding
    - personalization
    - commands
    - skills
    - subagents
    - plan_parser
    - architecture_anchor_parser
    - team_launch_recipe
    - tdd_slice_tracking
    - context_monitoring
    - upgrade_flow
```

### 4.2 Pack is template-based

The cc-crew pack requires project-specific generation/personalization.

Personalization depends on project artifacts such as:

```text
Architecture document
Planning docs
Implementation plan/task tracker
Code areas
Package layout
Project conventions
Mode selection: team vs single-operator
Optional command/subagent choices
```

Therefore:

```text
cc-crew pack installed != cc-crew instance ready.
```

---

## 5. cc-crew Workflow Instance states

```text
None
  No cc-crew indicators in project.

Template available
  The cc-crew pack is available globally or in the platform library.

Partial artifacts detected
  Some planning or architecture docs exist, but no generated instance.

Needs personalization
  The project can likely use cc-crew, but scaffold generation has not completed.

Personalization in progress
  scaffold-generate or equivalent generation flow is running.

Generated, review required
  Files were generated and need user review.

Active
  Generated instance exists and core files are present.

Ready for team mode
  Active instance has team commands/roles and required plan state.

Ready for single-operator mode
  Active instance has single-session commands and required plan state.

Degraded
  Instance exists but some commands/files/manifests are missing or stale.

Drift detected
  Generated files diverge from manifest/source pack expectations.

Upgrade available
  Source pack has newer version/templates/commands.

Detached
  User disabled cc-crew for the project.
```

---

## 6. Detection rules

### 6.1 High-confidence indicators

```text
.scaffolding/manifest.json
.scaffolding/brain.json or .project-brain/manifest.json with cc-crew producer metadata
Generated .claude/commands/ matching cc-crew command names
Generated .claude/agents/ matching cc-crew agent names
MVP_TASKS.md with cc-crew-style spec anchors
ARCHITECTURE.md with stable § anchors
```

### 6.2 Medium-confidence indicators

```text
CLAUDE.md with cc-crew references
area-level CLAUDE.md files
area-level LESSONS.md files
docs/planning/
docs/layers/
docs/briefs/
docs/sessions/
docs/team-handoffs/
docs/orchestrator-briefing.md
docs/team-protocol.md
docs/tdd-brief-template.md
```

### 6.3 Detection report example

```yaml
WorkflowDetectionReport:
  projectId: project_123
  candidates:
    - workflowPackId: cc-crew
      confidence: high
      detectedInstance: true
      manifestPath: .scaffolding/manifest.json
      readinessGuess: active
      evidence:
        - path: .scaffolding/manifest.json
          reason: cc-crew provenance manifest detected
        - path: MVP_TASKS.md
          reason: task tracker detected
        - path: .claude/commands/team-start.md
          reason: team command detected
```

---

## 7. Instance manifest handling

### 7.1 Read-only rule

The platform and Project Brain should treat cc-crew's `.scaffolding/manifest.json` as workflow-owned state.

They may read it for identity, provenance, generated files, code areas, and version information.

They should not hand-edit it.

If an update is needed, it should happen through an approved cc-crew generation or upgrade flow.

### 7.2 Platform-owned companion metadata

The platform may store companion metadata in its local database or a platform-owned manifest.

Examples:

```text
platform workflow instance id
last readiness check
last Project Brain ingest
last command registry scan
user UI preferences
manual plan-task links
manual Linear/GitHub links
```

This metadata should not be confused with cc-crew's own manifest.

---

## 8. Personalization flow

### 8.1 Trigger points

The platform may suggest personalization when:

```text
A user imports a project with architecture docs but no cc-crew instance.
A user selects cc-crew from Workflow Setup.
Project Brain recommends applying cc-crew.
A user attempts to run a cc-crew command that requires an active instance.
```

### 8.2 Flow

```text
1. User opens Project → Workflow.
2. Platform shows cc-crew available or detected as incomplete.
3. User selects Apply / Personalize cc-crew.
4. Platform scans project artifacts.
5. Platform identifies architecture docs, task docs, code areas, package manifests, existing .claude files.
6. Platform launches a scaffold-generation session or guided personalization run.
7. The generator infers values and asks unresolved questions.
8. Platform records questions/answers as personalization metadata.
9. Platform previews generated files and diff.
10. User approves writes through the Action Gateway.
11. Generated files are written.
12. Platform shows diff/review.
13. User activates workflow instance.
14. User may commit generated files.
```

### 8.3 Required approval cards

```text
Approve generated scaffold plan
Approve file writes
Approve modification of .claude files
Approve hooks/settings changes, if any
Approve commit, if requested
```

### 8.4 Generated review output

```text
Generated files
Modified files
Owned/foreign classification
Commands enabled
Agents enabled
Plan parser status
Team mode readiness
Single-operator readiness
Warnings
Recommended next action
```

---

## 9. cc-crew artifact map

The platform should recognize these as cc-crew-relevant sources.

```text
ARCHITECTURE.md
  Binding architecture contract with stable section anchors.

MVP_TASKS.md
  Implementation plan/task tracker with phases, tasks, spec anchors, state, carry-forward, and logs.

docs/planning/*.md
  Research, decisions, draft architecture, requirements, risks, and handoff artifacts.

docs/layers/OVERVIEW.md and docs/layers/*.md
  Layer documentation with plain/deep structure and file:line anchors.

docs/learn-site/content.json
  Structured layer/content payload, useful as fast ingest path for Project Brain.

<area>/LESSONS.md
  Area-specific lessons and gotchas.

CLAUDE.md and area CLAUDE.md files
  Project and area instructions/conventions.

.claude/commands/
  Slash commands.

.claude/agents/
  Subagent definitions.

docs/briefs/
  Implementation briefs and decision context.

docs/sessions/
  Session reports/summaries.

docs/team-handoffs/
  Team coordination and handoff state.

.scaffolding/manifest.json
  Workflow instance provenance and generated file ledger.
```

---

## 10. Plan parser: MVP_TASKS.md

### 10.1 Parser goals

Parse `MVP_TASKS.md` into platform objects:

```text
ImplementationPlan
PlanPhase
PlanTrack
PlanTask
ArchitectureAnchor
AcceptanceCriterion
CarryForwardItem
DecisionItem
LogEntry
```

### 10.2 Sections to preserve

```text
Current state
Next session target
Carry-forward
Deliverable map
Phase exit checklist
Phase sections
Spec anchors
Tasks
Files
Cross-doc invariants
Trims / nice-to-haves
Open decisions
Log
```

### 10.3 PlanTask fields

```yaml
PlanTask:
  id: stable_generated_id
  sourceFile: MVP_TASKS.md
  sourceAnchor: heading_or_checkbox_anchor
  title: string
  phase: string
  track: string | null
  status: not_started | in_progress | blocked | review | done | deferred
  specAnchors:
    - ARCHITECTURE.md §N
  files:
    - path
  crossDocInvariant: string | null
  acceptanceCriteria: string[]
  linkedLinearIssueId: string | null
  linkedGitHubIssueNumber: number | null
  linkedSessionIds: string[]
  linkedAgentTeamIds: string[]
  linkedPullRequestIds: string[]
```

### 10.4 Mutating plan files

The platform may update links/status/log entries only through approved actions.

MVP should start with:

```text
Manual linking stored in platform metadata.
Optional write-back later.
```

Recommended sequencing:

```text
P0: Link plan tasks to Linear/GitHub/session/PR in platform metadata.
P1: One-way create Linear/GitHub issue from plan task.
P2: Controlled write-back/sync with conflict review.
```

---

## 11. Architecture anchor parser

cc-crew architecture docs use stable section anchors conceptually represented as `§N` references.

The platform should parse:

```text
ARCHITECTURE.md §N
ARCHITECTURE.md §N.M
Section headings
Model/contract inventories
File:line anchors in architecture prose
```

Plan tasks should be able to link to architecture anchors so the platform can show:

```text
Task → architecture section
Session → task → architecture section
PR → session → task → architecture section
Code diff → task/architecture context
```

Project Brain should use those links as high-trust evidence.

---

## 12. Command registry for cc-crew

The platform should scan and expose cc-crew commands.

### 12.1 Expected command categories

```text
Planning / architecture
  arch-draft
  arch-finalize
  tasks-gen

Scaffolding
  scaffold-generate
  scaffold-upgrade

Team orchestration
  team-start
  team-end
  orchestrate-start
  orchestrate-end
  session-start
  session-end

Implementation
  tdd
  run-tests
  preflight
  wired
  check-arch
  context-check
  eval
  trace

Documentation / memory
  layer-docs
  learn-site
  ingest / Project Brain publish step, if present
```

Exact command names should be discovered from files rather than assumed.

### 12.2 Command details to show

```text
Name
Source file
Role/context
Arguments
Description
Allowed tools, if declared
Requires active instance?
Requires plan task?
Creates session/team?
Mutates files?
Risk level
Run button
Open definition
```

### 12.3 Invocation behavior

For MVP, commands can be invoked by sending command text into a Claude Code terminal.

Later, supported commands may be dispatched through SDK/non-terminal mechanisms when safe.

---

## 13. /team-start as Agent Team Launch Recipe

### 13.1 Recipe purpose

`/team-start` should be exposed as a structured agent-team launcher.

It should be invokable from:

```text
Project graph
Plan phase
Plan track
Plan task
Linear issue
GitHub issue
Project Brain action plan
Command palette
```

### 13.2 Inputs

```yaml
TeamStartInputs:
  track: string
  sourcePlanTaskId: string | null
  sourcePlanTrackId: string | null
  architectureAnchors: string[]
  worktreeStrategy: shared | per_team | per_worker
  branchName: string
  leadExecutionProfileId: string
  orchestratorExecutionProfileId: string | null
  implementerExecutionProfileIds: string[]
  promptAddendum: string | null
```

### 13.3 Roles

```text
Team lead
  Durable supervising session.

Orchestrator
  Plans/splits/coordinates implementation.

Implementer
  Does code work for a specific track/area/slice.
```

The platform should not require every project to use all roles, but it should display them when the workflow instance exposes them.

### 13.4 Launch action plan

A typical `/team-start` action plan:

```text
1. Validate workflow instance readiness.
2. Validate plan track/task context.
3. Create or select worktree.
4. Create branch.
5. Start lead session under selected execution profile.
6. Send /team-start <track> into terminal.
7. Detect spawned/registered sessions when possible.
8. Link sessions to AgentTeam object.
9. Link team to plan task/track.
10. Open Agent Team View.
```

### 13.5 UI output

```text
Team objective
Track/task
Architecture anchors
Lead terminal
Orchestrator terminal
Implementer terminals
Worktree/branch
Current status
Escalations
Context levels
Artifacts
```

---

## 14. /tdd slice tracker

The platform should expose a cc-crew TDD slice tracker when it can identify the active `/tdd` flow.

Suggested states:

```text
Restate
Identify files
RED
Test design review
Confirm RED
GREEN
Confirm GREEN
Refactor
Full suite
Reachability
Type + lint
Hot route
Atomic commit
Done
Blocked
Escalated
```

The tracker should be observational first. It should not require the terminal workflow to change in MVP.

Later, cc-crew could emit structured events or markers for more reliable tracking.

---

## 15. Context monitoring integration

If cc-crew emits heartbeats or context status, the platform should map them into session/agent-team observability.

### 15.1 Data to capture

```text
Role
Track
Session id
Terminal id
Context percent
Context tier
Last heartbeat
Current slice
Escalation state
Close-out status
```

### 15.2 Context tiers

```text
OK
WARN
ACTION
HARD_STOP
UNKNOWN
```

### 15.3 UI mapping

```text
Sidebar session row
Agent team graph node
Session inspector
Human input queue
Project timeline
Usage dashboard
```

---

## 16. Escalation categories

cc-crew escalations should become first-class Human Input Queue items.

Suggested categories:

```text
Critical / safety design question
Finding
Deferment approval
Load-bearing architectural decision
Permission request
Context close-out required
Unknown / other
```

Each escalation should link to:

```text
Team
Session
Role
Plan task/track
Architecture anchor
Terminal transcript moment
Recommended action
```

---

## 17. Workflow upgrade handling

cc-crew has a scaffold upgrade concept. The platform should support it later through a workflow upgrade surface.

### 17.1 Upgrade states

```text
No upgrade information
Up to date
Upgrade available
Local modifications detected
Upgrade preview ready
Upgrade applied
Upgrade failed
```

### 17.2 Upgrade flow

```text
1. Check source pack version.
2. Compare instance manifest/generated files.
3. Detect local modifications.
4. Generate upgrade preview.
5. Show diff.
6. Ask approval through Action Gateway.
7. Apply upgrade.
8. Re-run readiness checks.
9. Emit events.
```

### 17.3 Safety rules

```text
Never clobber user edits silently.
Never mutate manifest by hand.
Always show diff before writes.
Use git/worktree isolation when possible.
```

---

## 18. Project Brain integration

Project Brain should understand cc-crew artifacts and use them as high-value memory sources.

### 18.1 Project Brain consumes

```text
ARCHITECTURE.md
MVP_TASKS.md
Planning docs
Layer docs
content.json
LESSONS.md
CLAUDE.md files
Briefs
Session docs
Team handoffs
Manifest
Platform events
Session episode cards
PR/commit links
```

### 18.2 Project Brain can answer

```text
What phase are we in?
What is the next task?
Which architecture section governs this task?
Which sessions worked on this plan item?
When did we implement this feature?
What code changed for this track?
Which decisions are still open?
Which docs are stale?
Which lessons apply to this new task?
```

### 18.3 Project Brain can propose actions

```text
Run cc-crew personalization.
Start /team-start for this track.
Create a session for this task.
Link plan task to Linear issue.
Create Linear issue from plan task.
Ask team lead for status.
Refresh owned docs.
Summarize completed session.
Update task state.
```

All mutations must route through the Action Gateway.

---

## 19. Linear/GitHub linking sequence

For cc-crew `MVP_TASKS.md`, the safest sequence is:

### P0: Manual linking

```text
Plan task ↔ Linear issue
Plan task ↔ GitHub issue
Plan task ↔ Session
Plan task ↔ PR
```

Links can live in platform metadata first.

### P1: One-way creation

```text
Create Linear issue from plan task.
Create GitHub issue from plan task.
Create plan task reference from issue.
```

### P2: Controlled sync

```text
Status sync
Title/description sync
Acceptance criteria sync
Comments/summaries sync
Conflict review UI
```

Do not start with automatic bidirectional sync.

---

## 20. UI surfaces specific to cc-crew

### 20.1 Workflow Setup

```text
cc-crew status
Template available / instance active / needs personalization
Manifest path
Generated from SHA/version
Mode: team / single-operator
Code areas
Task tracker
Architecture doc
Readiness checks
Actions: personalize, rescan, start team, check upgrade
```

### 20.2 Plan View

```text
MVP_TASKS.md phases/tracks/tasks
Spec anchors
Status
Linked issues
Linked sessions
Linked PRs
Start session/team action
```

### 20.3 Command Registry

```text
cc-crew commands
Source files
Arguments
Risk
Context support
Run/Open definition
```

### 20.4 Agent Team View

```text
Team lead
Orchestrator
Implementers
Track
Plan task
Worktree/branch
Terminals
TDD progress
Context tiers
Escalations
Artifacts
```

### 20.5 Project Brain Drawer

```text
Plan-aware Q&A
Next-task suggestions
Workflow action proposals
Evidence chips from architecture/tasks/sessions/commits
```

---

## 21. Event taxonomy additions

```text
CcCrewDetected
CcCrewReadinessChecked
CcCrewPersonalizationStarted
CcCrewPersonalizationQuestionAsked
CcCrewPersonalizationPlanGenerated
CcCrewPersonalizationFilesWritten
CcCrewPersonalizationCompleted
CcCrewInstanceActivated
CcCrewPlanParsed
CcCrewPlanTaskLinked
CcCrewTeamStartRequested
CcCrewTeamStarted
CcCrewTeamMemberDetected
CcCrewTddSliceStarted
CcCrewTddSliceAdvanced
CcCrewEscalationRaised
CcCrewContextThresholdCrossed
CcCrewSessionCloseoutRequested
CcCrewUpgradeChecked
CcCrewUpgradeApplied
```

These may be implemented as generic workflow events with `workflowPackId=cc-crew` rather than separate concrete event names.

---

## 22. MVP requirements

### MVP should include

```text
Detect cc-crew active/incomplete states.
Show cc-crew status in Workflow Setup.
Parse MVP_TASKS.md into Plan View.
Parse ARCHITECTURE.md anchors enough to link tasks to architecture.
Show cc-crew commands in Command Registry.
Invoke /team-start from UI by sending command into Claude Code terminal.
Create AgentTeam object for /team-start launches.
Show team lead/orchestrator/implementer hierarchy when known.
Link plan task/track to session/team/worktree/PR.
Expose cc-crew artifacts to Project Brain.
Route mutating operations through Action Gateway.
Emit workflow events.
```

### MVP may defer

```text
Fully automated scaffold personalization.
Automatic command schema inference.
Bidirectional Linear sync.
Automatic scaffold upgrades.
Reliable spawned-session detection without workflow emitted markers.
Full TDD step tracking if not emitted structurally.
Remote iOS control.
```

---

## 23. P1 requirements

```text
Guided cc-crew personalization flow.
Generated file preview/diff review.
Workflow instance activation.
Upgrade check/preview.
Structured /team-start form.
Per-role execution profile selection.
Better context heartbeat ingestion.
Plan task write-back for links/status with conflict handling.
Create Linear/GitHub issue from plan task.
Project Brain action proposals for next task/team start.
```

---

## 24. P2 requirements

```text
Controlled bidirectional plan/ticket sync.
Automatic PR-to-plan reconciliation.
Full scaffold upgrade application.
Structured cc-crew event emission.
Remote observability of cc-crew teams.
Remote approval of escalations through companion app.
Shared/team workflow instances.
```

---

## 25. Open questions

```text
What exact fields should cc-crew expose in .scaffolding/manifest.json for platform readiness?
Should the platform maintain a separate cc-crew instance companion manifest?
Should /team-start run in one shared worktree or one worktree per implementer by default?
Can cc-crew emit structured markers/events for team membership and TDD stage changes?
How much of scaffold-generate should be platform-supervised vs terminal-native?
Should Project Brain be able to draft edits to MVP_TASKS.md or only store links in platform metadata initially?
What is the safest first write-back path for plan task ↔ Linear links?
How should execution profiles be assigned across lead/orchestrator/implementers?
What is the minimum reliable way to detect spawned sessions?
```

---

## 26. Acceptance criteria for v0.1

This integration spec is good enough if it lets us design:

```text
A Workflow Setup screen that shows cc-crew readiness accurately.
A Plan View that understands MVP_TASKS.md.
A Command Registry that exposes cc-crew commands.
A /team-start launcher that creates an AgentTeam object.
A Project Brain drawer that understands cc-crew artifacts.
A safe Action Gateway path for personalization, command invocation, and upgrades.
```
