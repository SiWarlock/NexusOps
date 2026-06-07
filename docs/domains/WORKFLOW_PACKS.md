# Workflow Packs Specification v0.1

> Status: Draft v0.1  
> Scope: AI engineering control plane / desktop-first platform  
> Related artifacts: Product Canon v0.1, Shared Object Model v0.1, Action Gateway Spec v0.1, Event Model & Audit Trail Spec v0.1, Project Brain PRD v2  
> Naming note: parent platform name is still OPEN; this document uses neutral product language.

---

## 1. Executive summary

Workflow Packs are the platform abstraction for project-specific AI engineering workflows.

A Workflow Pack is a reusable package of conventions, commands, templates, plan parsers, agent roles, launch recipes, hooks, skills, prompts, and upgrade rules. It lets the platform understand how a project wants AI work to happen without hardcoding one user's scaffolding into the product.

A Workflow Pack can be simple:

```text
A set of slash commands and prompts for a project.
```

Or advanced:

```text
A template-based scaffold that must be personalized against a project's architecture,
then used to generate project-specific commands, agents, docs, task plans, and team launch flows.
```

The key distinction:

```text
Workflow Pack
  Reusable package/template.

Workflow Instance
  Project-specific installed/personalized/generated result of a pack.

Workflow Personalization Run
  The process of applying a template pack to a project.
```

This distinction is mandatory. A template pack being available does not mean the project is ready to run its commands. Some packs require project-specific architecture, task plans, code-area mapping, and generated files before they become active.

---

## 2. Product definition

### 2.1 What Workflow Packs are

Workflow Packs are structured wrappers around repeatable engineering workflows.

They may include:

```text
- Commands
- Skills
- Subagents
- Hooks
- Prompt templates
- Scaffold templates
- Plan parsers
- Architecture parsers
- Team launch recipes
- Review recipes
- Test recipes
- Git/worktree recipes
- Project setup flows
- Upgrade flows
- Detection rules
- Status/readiness checks
- UI metadata
```

A Workflow Pack lets the platform answer:

```text
What workflow capabilities does this project have?
What commands are available?
Which commands require a personalized project instance?
What plan/task files can be parsed?
What agent team launch recipes exist?
Which files are owned by the workflow?
Which files are human-owned or foreign?
Can this workflow be upgraded safely?
What state is this workflow instance in?
```

### 2.2 What Workflow Packs are not

Workflow Packs are not:

```text
- A replacement for Claude Code, Codex, or future harnesses.
- A mandatory project format.
- A package manager for arbitrary untrusted scripts.
- A silent automation system that mutates projects without approval.
- A requirement for basic platform usage.
- A reason to make the product cc-crew-only.
```

A project can be useful in the platform without any Workflow Pack. The baseline product still supports projects, sessions, terminals, worktrees, git, code review, task inboxes, PRs, and Project Brain indexing.

### 2.3 Core thesis

Workflow Packs make the product adaptable without making the core product mushy.

The platform should provide strong generic primitives:

```text
Project
Task
ImplementationPlan
Session
AgentTeam
Worktree
Branch
PR
Approval
ActionRequest
Event
```

Workflow Packs should map project-specific conventions onto those primitives.

---

## 3. Design principles

### 3.1 Pack is not instance

A reusable pack and a project-specific instance are different objects.

The UI must never imply that a template pack is ready to run just because it is installed globally.

### 3.2 Optional by default

The platform must work with:

```text
Basic projects
  No workflow pack.

Claude-aware projects
  CLAUDE.md, .claude commands, skills, hooks, agents.

Workflow-pack projects
  A detected or installed workflow instance.

Template-pack projects
  A pack is available but needs personalization before use.
```

### 3.3 Detection is advisory, not proof

Detection rules should identify likely workflow files, but readiness must be verified through explicit checks.

```text
Detected files do not automatically mean Ready.
Manifest presence does not automatically mean healthy.
Command file presence does not mean safe to invoke.
```

### 3.4 Personalization is a first-class workflow

For template-based packs, applying the pack to a project is itself a workflow that can involve:

```text
Scanning the repo
Reading architecture docs
Inferring project values
Asking the user clarifying questions
Generating a plan
Previewing generated files
Requesting approval
Writing files
Reviewing diffs
Committing or leaving changes uncommitted
Activating the workflow instance
```

### 3.5 Explicit ownership boundaries

Every file touched by a Workflow Pack should be classified as:

```text
Owned
  Generated or maintained by the workflow. The workflow may update it through approved flows.

Foreign
  Produced by humans or another tool. The workflow may read it, but must not overwrite it.

Supplemental
  Added by the workflow to fill a gap. Namespaced and clearly marked.

User-local
  Local config/state that should not be committed.
```

### 3.6 No silent mutation

All mutating pack operations go through the Action Gateway.

```text
Install pack
Apply pack
Personalize pack
Run command
Start team
Update owned docs
Upgrade instance
Write generated files
Modify hooks/settings
```

### 3.7 Desktop-first and local-runner-aware

Workflow Packs run in a desktop-first environment with local repositories, local terminals, local file access, local worktrees, and local credentials.

A Workflow Pack may expose remote-observable state later, but pack execution is local unless explicitly supported by a future cloud/relay runner.

### 3.8 Graceful degradation

If a pack is absent, partial, outdated, or unhealthy, the platform should degrade to baseline behavior rather than block the project.

---

## 4. Core objects

### 4.1 WorkflowPack

A reusable package/template.

```yaml
WorkflowPack:
  id: string
  name: string
  displayName: string
  description: string
  sourceType: local_directory | git_repo | bundled | marketplace | manually_defined
  sourceUri: string | null
  sourceVersion: string | null
  packVersion: string
  schemaVersion: string
  provider: user | bundled | third_party
  trustLevel: untrusted | user_trusted | bundled_trusted | verified
  supportedHarnesses:
    - claude_code
    - codex_cli
    - custom
  capabilities:
    - commands
    - skills
    - subagents
    - hooks
    - templates
    - plan_parser
    - agent_team_recipe
    - personalization
    - upgrade
  detectors: WorkflowDetector[]
  commands: WorkflowCommandDefinition[]
  skills: SkillDefinition[]
  subagents: SubagentDefinition[]
  hooks: HookDefinition[]
  planParsers: PlanParserDefinition[]
  launchRecipes: LaunchRecipeDefinition[]
  personalizationFlow: PersonalizationFlowDefinition | null
  upgradeFlow: UpgradeFlowDefinition | null
  fileOwnershipRules: FileOwnershipRule[]
  securityProfile: WorkflowSecurityProfile
```

### 4.2 WorkflowInstance

A project-specific installed or personalized result of a Workflow Pack.

```yaml
WorkflowInstance:
  id: string
  workflowPackId: string
  projectId: string
  status: available | detected | needs_personalization | personalizing | active | ready | degraded | drift_detected | upgrade_available | archived | detached
  instanceVersion: string | null
  generatedFromVersion: string | null
  generatedFromSha: string | null
  manifestPath: string | null
  rootPaths: string[]
  mode: basic | single_operator | team | custom | unknown
  taskTrackerPaths: string[]
  architectureDocPaths: string[]
  codeAreas: CodeArea[]
  enabledCommands: string[]
  enabledSkills: string[]
  enabledSubagents: string[]
  enabledHooks: string[]
  generatedFiles: GeneratedFile[]
  ownershipMap: FileOwnershipRecord[]
  readiness: WorkflowReadinessReport
  lastScannedAt: datetime
  lastActivatedAt: datetime | null
```

### 4.3 WorkflowPersonalizationRun

The process of applying a template Workflow Pack to a project.

```yaml
WorkflowPersonalizationRun:
  id: string
  workflowPackId: string
  projectId: string
  status: drafted | scanning | needs_input | plan_ready | awaiting_approval | generating | generated | review_required | applied | failed | cancelled
  sourceArchitectureDocs: string[]
  sourcePlanDocs: string[]
  inferredValues: object
  unresolvedQuestions: PersonalizationQuestion[]
  userAnswers: PersonalizationAnswer[]
  generationPlan: GeneratedFilePlan[]
  actionPlanId: string | null
  generatedDiffArtifactId: string | null
  resultingWorkflowInstanceId: string | null
  startedAt: datetime
  completedAt: datetime | null
```

### 4.4 WorkflowCommand

A command exposed by a pack or instance.

```yaml
WorkflowCommand:
  id: string
  workflowPackId: string
  workflowInstanceId: string | null
  name: string
  displayName: string
  description: string
  sourcePath: string | null
  commandType: slash_command | skill | shell_command | platform_action | recipe
  argumentSchema: json_schema | null
  supportedContexts:
    - project
    - session
    - terminal
    - plan_task
    - plan_track
    - issue
    - pull_request
    - code_selection
    - worktree
    - agent_team
  supportedHarnesses:
    - claude_code
    - codex_cli
    - sdk
    - terminal_only
  requiresPersonalizedInstance: boolean
  createsSessions: boolean
  createsAgentTeam: boolean
  mutatesFiles: boolean
  riskLevel: 0 | 1 | 2 | 3 | 4
  previewBehavior: none | static_preview | dry_run | generated_diff
  invocationModes:
    - terminal_send
    - sdk_dispatch
    - action_gateway
```

### 4.5 LaunchRecipe

A structured workflow that starts one or more sessions.

```yaml
LaunchRecipe:
  id: string
  name: string
  description: string
  recipeType: single_session | agent_team | review_session | fix_session | doc_refresh | custom
  commandName: string | null
  requiredContext:
    - project
    - plan_task
    - plan_track
    - issue
    - architecture_anchor
  inputs: json_schema
  defaultWorktreeStrategy: none | existing | new_per_session | new_per_team | new_per_worker
  defaultBranchPattern: string
  sessionRoles:
    - role: lead
      harness: claude_code
      executionProfileSelector: project_default
    - role: implementer
      harness: claude_code
      executionProfileSelector: available_profile
  outputDetectionRules: OutputDetectionRule[]
  eventsEmitted: string[]
```

### 4.6 PlanParser

A parser that turns project planning files into `ImplementationPlan` and `PlanTask` objects.

```yaml
PlanParser:
  id: string
  workflowPackId: string
  displayName: string
  supportedFiles:
    - MVP_TASKS.md
    - IMPLEMENTATION_PLAN.md
  parserType: markdown_structured | frontmatter | json | custom
  sectionRules: object
  taskRules: object
  anchorRules: object
  statusRules: object
  outputObjects:
    - ImplementationPlan
    - PlanTask
```

---

## 5. Workflow lifecycle states

### 5.1 Workflow Pack lifecycle

```text
Available
  Pack exists in local library or can be imported.

Installed
  Pack files are installed locally and visible to the platform.

Enabled
  Pack is available for selection in projects.

Updated
  Pack source has changed or a newer version exists.

Disabled
  Pack remains installed but is not used.

Removed
  Pack has been removed from the local library.
```

### 5.2 Workflow Instance lifecycle

```text
Not present
  Project has no instance of this pack.

Detected
  Platform found signs of the workflow but has not verified readiness.

Needs personalization
  Template pack exists or partial files exist, but project-specific generation has not completed.

Personalizing
  Personalization flow is active.

Generated
  Files were generated, but not yet reviewed/activated.

Active
  Instance exists and can be used.

Ready
  Instance passed readiness checks for its advertised capabilities.

Degraded
  Instance can be used partially, but some commands/files/capabilities are missing or stale.

Drift detected
  Instance differs from its manifest, source pack, or expected generated state.

Upgrade available
  Source pack has newer machinery or templates.

Archived
  Instance is retained for history but not actively used.

Detached
  Project intentionally no longer uses the instance.
```

### 5.3 Command readiness states

```text
Available
  Command definition exists.

Hidden
  Command is not shown by default in the current context.

Disabled
  Command cannot run because required context is missing.

Needs personalization
  Command requires an active Workflow Instance.

Needs approval
  Command can run only after Action Gateway approval.

Ready
  Command can be invoked in the current context.

Running
  Command is currently executing.

Failed
  Last invocation failed.
```

---

## 6. Detection model

Workflow detection should run during:

```text
- Project add/import
- Project open
- Workflow tab open
- Manual rescan
- Git checkout/branch switch
- Manifest change
- Workflow Pack update
```

### 6.1 Detection inputs

```text
Files and directories
  CLAUDE.md
  .claude/
  .claude/commands/
  .claude/skills/
  .claude/agents/
  .claude/settings.*
  .claude/hooks.*
  .scaffolding/manifest.json
  .project-brain/manifest.json
  ARCHITECTURE.md
  MVP_TASKS.md
  IMPLEMENTATION_PLAN.md
  docs/planning/
  docs/layers/
  docs/sessions/
  docs/briefs/
  docs/team-handoffs/

Runtime state
  Available CLI commands
  Existing sessions
  Known terminal profiles
  Project Brain index metadata
  Git worktrees/branches

Configuration
  Workflow Pack library
  User settings
  Project settings
  Execution Profiles
```

### 6.2 Detection output

```yaml
WorkflowDetectionReport:
  projectId: string
  scannedAt: datetime
  candidates:
    - workflowPackId: string
      confidence: high | medium | low
      evidence:
        - path: string
          reason: string
      detectedInstance: boolean
      manifestPath: string | null
      readinessGuess: ready | partial | needs_personalization | unknown
  warnings:
    - code: string
      message: string
  recommendedActions:
    - actionType: string
      label: string
```

### 6.3 Readiness checks

Detection is not enough. The platform must run readiness checks before exposing high-level actions.

Examples:

```text
- Required files exist.
- Manifest schema is supported.
- Command files referenced by manifest exist.
- Plan parser can parse at least one task.
- Architecture anchors resolve or degrade with warnings.
- Required harness is available.
- Required execution profile exists.
- Required skills/subagents are installed or project-local.
- Working tree is not in a dangerous state.
- Pack source version is compatible with instance version.
```

---

## 7. Manifest model

The platform should support two manifest layers.

### 7.1 Pack manifest

A pack manifest describes the reusable package.

Suggested path:

```text
workflow-pack.json
```

Example:

```json
{
  "schemaVersion": "workflow-pack/v0.1",
  "id": "com.example.workflow.cc-crew",
  "name": "cc-crew",
  "displayName": "cc-crew TDD Agent Crew",
  "version": "0.1.0",
  "description": "Template-based Claude workflow for architecture-first TDD agent teams.",
  "capabilities": ["commands", "skills", "subagents", "templates", "plan_parser", "agent_team_recipe", "personalization", "upgrade"],
  "supportedHarnesses": ["claude_code"],
  "detectors": [
    {"kind": "file_exists", "path": ".scaffolding/manifest.json", "confidence": "high"},
    {"kind": "file_exists", "path": "MVP_TASKS.md", "confidence": "medium"},
    {"kind": "directory_exists", "path": ".claude/commands", "confidence": "medium"}
  ],
  "commands": [],
  "launchRecipes": [],
  "planParsers": []
}
```

### 7.2 Instance manifest

An instance manifest describes the generated/personalized project-specific result.

The platform should prefer the workflow's own instance manifest if one exists, but it must not assume every workflow uses the same manifest format.

The platform may create a sibling platform-owned manifest:

```text
.platform/workflows/<workflow_instance_id>.json
```

or store equivalent metadata in the local app database.

The instance manifest should record:

```text
Workflow pack id
Source pack version/SHA
Generated/personalized timestamp
Project id
Mode
Code areas
Task tracker path
Architecture path
Enabled commands
Enabled subagents
Generated files
Owned file ledger
Customization ledger
Upgrade base
Last readiness result
```

### 7.3 Manifest ownership rule

If a workflow already owns a manifest, the platform should treat it as read-only unless the pack explicitly exposes an approved update/upgrade action.

The platform may write its own metadata next to it, but it should not corrupt the workflow's source-of-truth manifest.

---

## 8. Personalization model

Template-based packs need a first-class personalization flow.

### 8.1 Personalization inputs

```text
Architecture documents
Implementation plans
Existing README/docs
Package manifests
Repository layout
Detected code areas
User answers
Project settings
Execution profiles
Preferred workflow mode
```

### 8.2 Personalization phases

```text
1. Select pack
2. Scan project
3. Determine if personalization is required
4. Infer project values
5. Ask unresolved questions
6. Generate personalization plan
7. Preview generated files
8. Request Action Gateway approval
9. Write generated files
10. Show diff/review
11. Activate workflow instance
12. Optionally commit changes
```

### 8.3 Approval points

At minimum, a personalization flow must request approval before:

```text
- Writing project files
- Creating or modifying .claude files
- Creating or modifying hooks/settings
- Registering global skills
- Creating git commits
- Pushing branches
```

### 8.4 Generated file review

After generation, the platform should show:

```text
Generated files
Modified files
Deleted files, if any
Owned vs foreign classification
Diff by file
Readiness result
Recommended next action
```

---

## 9. Command registry

The platform should provide a Command Registry for every project.

### 9.1 Sources

Commands may come from:

```text
- Built-in platform actions
- Workflow Pack definitions
- Project-local slash commands
- User-global slash commands
- Claude skills
- Codex-compatible skills or prompts
- Shell scripts
- MCP tools
- Workflow launch recipes
```

### 9.2 Command list fields

```text
Name
Description
Source
Type
Harness support
Context support
Arguments
Risk level
Requires personalized instance
Creates session/team
Mutates files
Last run
Readiness
```

### 9.3 Invocation modes

```text
terminal_send
  Send command text into an attached terminal.

sdk_dispatch
  Dispatch through an SDK or non-interactive API when supported.

action_gateway
  Invoke a platform-native action or bundled action plan.

manual_copy
  Present command for manual copy if automation is unsafe or unsupported.
```

### 9.4 Command context binding

Commands should become more useful when invoked from an object.

Examples:

```text
Plan task → /implement-task with task anchor/context
Plan track → /team-start with track argument
PR → /review-pr with PR metadata
Code selection → /explain-selection with file/range context
Failed check → /fix-failing-check with logs
Worktree → /run-tests in worktree cwd
```

---

## 10. Plan parser requirements

Workflow Packs may define plan parsers that convert files into platform objects.

### 10.1 Parser outputs

```text
ImplementationPlan
PlanPhase
PlanTrack
PlanTask
ArchitectureAnchor
AcceptanceCriterion
Dependency
Status
LinkedIssue
LinkedSession
LinkedPR
```

### 10.2 Parser behavior

Parsers should:

```text
- Preserve source file path.
- Preserve heading hierarchy.
- Preserve source anchors.
- Capture task checkboxes/status when available.
- Capture architecture references.
- Capture file references.
- Capture acceptance criteria.
- Capture log/history sections separately from executable tasks.
- Avoid overwriting plan files unless explicitly invoked through an approved action.
```

### 10.3 Plan mutation

Plan mutation must go through the Action Gateway.

Examples:

```text
Update task status
Link Linear issue
Link GitHub issue
Link session
Add session summary
Add log entry
Add carry-forward item
Mark phase complete
```

---

## 11. Agent team recipes

Workflow Packs can expose agent team recipes.

### 11.1 Agent team recipe fields

```yaml
AgentTeamRecipe:
  id: string
  name: string
  objectiveTemplate: string
  roles:
    - id: lead
      displayName: Team Lead
      required: true
    - id: orchestrator
      displayName: Orchestrator
      required: false
    - id: implementer
      displayName: Implementer
      count: dynamic
  inputSchema: json_schema
  command: string | null
  worktreeStrategy: shared | per_team | per_worker
  branchPattern: string
  sessionNamingPattern: string
  expectedOutputs:
    - session_summary
    - commits
    - diff
    - pr
  escalationCategories: string[]
```

### 11.2 Team launch UX

A launch flow should ask:

```text
What is the source task/track?
What command/recipe should run?
Which worktree strategy?
Which execution profile per role?
Which branch naming pattern?
How many workers?
What should happen after completion?
```

### 11.3 Team observability

Agent team recipes should define how the platform can recognize:

```text
Lead session
Worker sessions
Orchestrator session
Task assignment
Current slice/track
Status heartbeat
Context threshold
Escalation state
Completion state
```

---

## 12. Action Gateway integration

Workflow Packs must use the Action Gateway for mutating operations.

### 12.1 Pack actions

```text
InstallWorkflowPack
EnableWorkflowPack
DisableWorkflowPack
RemoveWorkflowPack
RescanWorkflowPack
```

### 12.2 Instance actions

```text
DetectWorkflowInstance
PersonalizeWorkflowPack
ActivateWorkflowInstance
DetachWorkflowInstance
CheckWorkflowReadiness
CheckWorkflowUpgrade
UpgradeWorkflowInstance
```

### 12.3 Command actions

```text
InvokeWorkflowCommand
PreviewWorkflowCommand
StartWorkflowLaunchRecipe
StartAgentTeamFromRecipe
RunWorkflowCheck
```

### 12.4 Risk guidance

```text
Read command metadata: Level 0
Run readiness check: Level 0
Preview personalization plan: Level 1
Write generated files: Level 2 or 3
Install global skills/hooks/settings: Level 3
Run command in terminal: Level 2 or 3 depending on command
Create worktree/session/team: Level 2
Commit/push/merge: Level 3 or 4 depending on branch
```

---

## 13. Event model integration

Workflow Packs should emit or cause events.

```text
WorkflowPackImported
WorkflowPackInstalled
WorkflowPackEnabled
WorkflowPackDisabled
WorkflowPackUpdated
WorkflowInstanceDetected
WorkflowInstanceReadinessChecked
WorkflowPersonalizationStarted
WorkflowPersonalizationQuestionAsked
WorkflowPersonalizationPlanGenerated
WorkflowPersonalizationApproved
WorkflowPersonalizationFilesWritten
WorkflowPersonalizationCompleted
WorkflowInstanceActivated
WorkflowInstanceDriftDetected
WorkflowInstanceUpgradeAvailable
WorkflowInstanceUpgraded
WorkflowCommandRegistered
WorkflowCommandInvoked
WorkflowCommandSucceeded
WorkflowCommandFailed
AgentTeamRecipeStarted
PlanParsed
PlanTaskLinked
```

Events should include correlation IDs tying together:

```text
Workflow instance
Action request
Session
Terminal
Worktree
Plan task
Agent team
PR
Project Brain evidence
```

---

## 14. Project Brain integration

Project Brain should understand Workflow Packs, but it should not directly execute pack operations.

### 14.1 Project Brain consumes

```text
Workflow pack metadata
Workflow instance metadata
Command registry
Plan parser output
Personalization run history
Workflow events
Session summaries
Generated docs
Manifest state
Readiness reports
Upgrade/drift reports
```

### 14.2 Project Brain can propose

```text
Apply this workflow pack
Run personalization
Start next plan task
Invoke a workflow command
Start an agent team from this track
Link this plan task to Linear
Refresh owned docs
Upgrade workflow instance
```

### 14.3 Project Brain must request actions through the gateway

Examples:

```text
User: "Start the next backend task."
Brain retrieves: plan tasks, active sessions, workflow readiness, execution profiles.
Brain proposes: create worktree + invoke team recipe + link session to task.
Gateway previews and requests approval.
Platform executes.
Brain indexes events and outcomes.
```

---

## 15. UI surfaces

### 15.1 Workflow Setup tab

Shows:

```text
Detected workflow state
Available packs
Active instance
Readiness report
Personalization status
Upgrade/drift status
Actions
```

### 15.2 Command Registry tab

Shows:

```text
Commands
Recipes
Skills
Subagents
Hooks
Readiness
Risk
Context support
Run/preview/open definition actions
```

### 15.3 Plan view

Shows parsed plans from Workflow Pack plan parsers.

```text
Phases
Tracks
Tasks
Anchors
Linked issues
Linked sessions
Linked PRs
Start session/team actions
```

### 15.4 Agent Team launch modal

Shows a structured launch flow for team recipes.

### 15.5 Project Brain drawer

Shows Brain-suggested workflow actions and evidence.

### 15.6 Human Input Queue

Shows workflow questions, personalization approvals, command approvals, and escalation events.

---

## 16. Security and trust

### 16.1 Pack trust levels

```text
Untrusted
  Imported from arbitrary local path or repo. Mutations require explicit approval.

User trusted
  User explicitly marked as trusted.

Bundled trusted
  Shipped with the platform.

Verified
  Future signed/verified ecosystem pack.
```

### 16.2 Pack permissions

Workflow Packs should declare requested permissions:

```text
Read project files
Write project files
Read global user config
Write global user config
Register skills
Register hooks
Run shell commands
Start sessions
Create worktrees
Create commits
Push branches
Open PRs
Update tickets
Access Project Brain index
```

### 16.3 Forbidden by default

```text
Force push
Delete branches
Read secrets outside project scope
Modify credentials
Change global Claude/Codex auth state
Install arbitrary binaries without explicit approval
Run network scripts without explicit approval
```

---

## 17. Packaging and SDK

### 17.1 Local pack format

A pack can be represented as:

```text
workflow-pack.json
commands/
skills/
agents/
hooks/
templates/
parsers/
recipes/
README.md
```

### 17.2 Pack SDK

Future SDK functions:

```text
detect(project): WorkflowDetectionReport
readiness(project, instance): WorkflowReadinessReport
parsePlan(project, file): ImplementationPlan
listCommands(project, instance): WorkflowCommand[]
previewPersonalization(project, inputs): PersonalizationPlan
runPersonalization(project, approvedInputs): GeneratedDiff
previewCommand(command, context): CommandPreview
resolveLaunchRecipe(recipe, context): ActionPlan
checkUpgrade(instance): UpgradeReport
```

### 17.3 Versioning

```text
schemaVersion
packVersion
sourceVersion
instanceVersion
generatedFromSha
minimumPlatformVersion
maximumPlatformVersion
compatibilityWarnings
```

---

## 18. MVP scope

### MVP must support

```text
Detect project-local commands and basic Claude-aware files.
Represent Workflow Pack vs Workflow Instance.
Show Workflow Setup tab.
Show Command Registry tab.
Run readiness checks.
Parse at least one implementation plan format through a pack parser.
Invoke commands by sending text into an attached terminal.
Launch an agent team recipe through a structured UI.
Route mutating actions through Action Gateway.
Emit workflow lifecycle events.
Expose workflow metadata to Project Brain.
Support cc-crew as the first rich example pack.
```

### MVP may defer

```text
Pack marketplace.
Signed pack distribution.
Full bidirectional Linear sync.
Fully generic parser SDK.
Automated upgrades.
Cloud/remote pack execution.
Multi-user pack sharing.
```

---

## 19. P1 scope

```text
Workflow Pack import from git repo.
Workflow Pack manifest validation.
Workflow personalization runs with generated diff review.
Workflow instance upgrade checks.
Better plan parser schemas.
Command argument forms generated from schemas.
Per-role execution profiles for agent team recipes.
Pack-specific Project Brain action suggestions.
Workflow health dashboard.
```

---

## 20. P2 scope

```text
Signed packs.
Pack registry/marketplace.
Pack compatibility tests.
Custom workflow visual builder.
Bidirectional plan/ticket sync.
Remote observability for workflow runs.
Remote approval of workflow actions through companion app.
Team-shared workflow instances.
```

---

## 21. Non-goals

```text
Do not make every user adopt cc-crew.
Do not require workflow packs for baseline use.
Do not run arbitrary pack scripts without explicit trust and approval.
Do not hide command execution from the user.
Do not mutate workflow-owned manifests unless the pack exposes an approved mutation flow.
Do not turn Workflow Packs into a general SaaS plugin marketplace in MVP.
```

---

## 22. Open questions

```text
What should the first platform-native workflow-pack manifest schema be called?
Should the platform ship with built-in packs or import cc-crew as a user-installed pack?
How much of Claude Code skill/command parsing should be generic vs pack-specific?
Should workflow commands be normalized into one cross-harness command format?
How should package trust be represented before signed packs exist?
Should personalizations run via Claude Code terminal, SDK, or a platform-supervised session?
What is the minimum useful generic plan parser abstraction?
How should pack upgrades interact with uncommitted changes?
Can a workflow instance safely span multiple repos?
How should a future iOS companion display workflow actions without becoming a remote shell?
```

---

## 23. Acceptance criteria for v0.1 design

The spec is good enough for product/design if it lets us answer:

```text
What is a Workflow Pack?
What is a Workflow Instance?
How does a template pack become active in a project?
How does the platform detect readiness?
How are commands exposed?
How are plan tasks parsed?
How are agent teams launched?
How does Project Brain suggest workflow actions?
How does the Action Gateway keep workflow actions safe?
What does MVP include?
```
