# AI Engineering Control Plane — Shared Object Model v0.1

> **Status:** Planning draft  
> **Date:** 2026-06-06  
> **Naming:** Open. Uses neutral working labels.

---

## 1. Purpose

This document defines the shared object model for the AI engineering control plane, Project Brain, Workflow Packs, and Action Gateway.

The goal is to stabilize the product nouns before writing the PRD, UX spec, data model, event model, and implementation plan.

For each object, this document captures:

- Definition
- Key fields
- Relationships
- Lifecycle/status states
- UI surfaces
- Events

---

## 2. Design principles for the object model

1. **Sessions are atomic.**  
   Every unit of agent execution should map to a Session.

2. **Everything should be linkable.**  
   Tasks, plan items, worktrees, branches, sessions, PRs, commits, approvals, and Project Brain evidence must form a traceable chain.

3. **The platform executes; Project Brain reasons.**  
   Project Brain may plan/request actions, but the Action Gateway executes them.

4. **Workflow templates and project instances are different.**  
   A Workflow Pack is reusable. A Workflow Instance is personalized to a project.

5. **Local identity matters.**  
   Local paths, worktrees, authenticated execution profiles, and repo roots are first-class.

6. **Every risky action is auditable.**  
   Actions should emit events with actor, target, risk, approval, and result.

---

## 3. Object relationship overview

```text
Workspace
  └── Project
        ├── Repository
        │     ├── Worktree
        │     │     └── Branch
        │     └── PullRequest
        ├── ImplementationPlan
        │     └── PlanTask
        ├── Task
        ├── Session
        │     ├── Terminal
        │     ├── Transcript
        │     ├── Artifact
        │     ├── Approval
        │     └── Event
        ├── AgentTeam
        │     ├── Lead Session
        │     └── Worker Sessions
        ├── WorkflowInstance
        │     ├── WorkflowCommand
        │     ├── SubagentDefinition
        │     ├── SkillDefinition
        │     └── HookDefinition
        └── ProjectBrainIndex
              ├── MemorySource
              ├── EvidenceItem
              ├── Decision
              └── EpisodeCard

ExecutionProfile
  └── used by Session / AgentTeam member

ActionGateway
  └── executes ActionRequest
        ├── may require Approval
        └── emits Event
```

---

## 4. Workspace

### Definition

The top-level local environment for a user.

### Key fields

- `workspace_id`
- `display_name`
- `root_settings_path`
- `projects[]`
- `execution_profiles[]`
- `integrations[]`
- `default_worktree_root`
- `global_policy`

### Relationships

- Has many Projects
- Has many Execution Profiles
- Has global integrations such as GitHub and Linear
- Has global Project Brain settings

### Status/lifecycle

- `not_configured`
- `setup_incomplete`
- `ready`
- `degraded`

### UI surfaces

- Global Command Center
- Settings
- Usage Dashboard
- Human Input Queue

### Events

- `WorkspaceCreated`
- `WorkspaceSetupCompleted`
- `WorkspaceSetupDegraded`
- `WorkspaceSettingsUpdated`

---

## 5. Project

### Definition

A local development project managed by the platform. Usually maps to one primary repository, but should allow future multi-repo projects.

### Key fields

- `project_id`
- `name`
- `repo_root`
- `canonical_root`
- `default_branch`
- `project_type`
- `linked_github_repo`
- `linked_linear_project`
- `brain_status`
- `workflow_instance_id`
- `policy_path`
- `created_at`
- `last_opened_at`

### Relationships

- Belongs to Workspace
- Has one or more Repositories
- Has many Sessions
- Has many Worktrees
- Has many Tasks
- May have ImplementationPlan
- May have WorkflowInstance
- Has ProjectBrainIndex

### Status/lifecycle

- `new`
- `registered`
- `indexing`
- `active`
- `degraded`
- `archived`

### UI surfaces

- Left sidebar
- Project Home / Observability Graph
- Plan View
- Code Editor
- Worktree/Git/PR Center
- Workflow Setup
- Settings

### Events

- `ProjectRegistered`
- `ProjectIndexed`
- `ProjectIndexDegraded`
- `ProjectOpened`
- `ProjectArchived`
- `ProjectSettingsUpdated`

---

## 6. Repository

### Definition

A git repository associated with a Project.

### Key fields

- `repo_id`
- `project_id`
- `remote_url`
- `provider`
- `owner`
- `name`
- `local_path`
- `default_branch`
- `current_head_sha`
- `git_status`

### Relationships

- Belongs to Project
- Has many Worktrees
- Has many Branches
- Has many PullRequests
- Provides commits to Project Brain

### Status/lifecycle

- `connected`
- `missing_local_path`
- `dirty`
- `syncing`
- `detached`
- `archived`

### UI surfaces

- Project top bar
- Git/PR Center
- Settings
- Project Brain evidence chips

### Events

- `RepositoryConnected`
- `RepositorySynced`
- `RepositoryHeadChanged`
- `RepositoryStatusChanged`

---

## 7. Worktree

### Definition

A local git worktree used to isolate agent work.

### Key fields

- `worktree_id`
- `project_id`
- `repo_id`
- `path`
- `branch_name`
- `base_branch`
- `owner_session_id`
- `owner_agent_team_id`
- `linked_task_id`
- `dirty_state`
- `ahead_count`
- `behind_count`
- `last_commit_sha`
- `created_at`

### Relationships

- Belongs to Repository
- May be owned by Session or AgentTeam
- Links to Branch
- Links to Task or PlanTask
- May produce PullRequest

### Status/lifecycle

- `creating`
- `clean`
- `dirty`
- `untracked_files`
- `conflicts`
- `behind_base`
- `ahead_of_base`
- `pr_open`
- `merged`
- `prunable`
- `locked`
- `deleted`

### UI surfaces

- Worktree/Git/PR Center
- Session inspector
- Project graph
- Code Editor header

### Events

- `WorktreeCreated`
- `WorktreeDirtyStateChanged`
- `WorktreeLinkedToSession`
- `WorktreeDeleted`
- `WorktreePruned`

---

## 8. Branch

### Definition

A git branch associated with a worktree, task, session, or PR.

### Key fields

- `branch_id`
- `repo_id`
- `name`
- `base_branch`
- `head_sha`
- `tracking_remote`
- `owner_session_id`
- `linked_pr_id`

### Relationships

- Belongs to Repository
- May be checked out by Worktree
- May be linked to PullRequest
- May be produced by Session

### Status/lifecycle

- `local_only`
- `pushed`
- `behind_base`
- `ahead_of_base`
- `conflict_risk`
- `merged`
- `deleted`

### UI surfaces

- Session header
- Worktree/Git/PR Center
- Project graph
- PR view

### Events

- `BranchCreated`
- `BranchPushed`
- `BranchMerged`
- `BranchDeleted`

---

## 9. Task

### Definition

A unit of requested work from GitHub, Linear, PR review, ad hoc user prompt, or another external source.

### Key fields

- `task_id`
- `source`
- `external_id`
- `title`
- `description`
- `acceptance_criteria`
- `priority`
- `labels[]`
- `assignee`
- `linked_project_id`
- `linked_plan_task_id`
- `linked_sessions[]`
- `linked_prs[]`
- `status`

### Relationships

- Belongs to Project
- May link to PlanTask
- May create Session or AgentTeam
- May link to PR
- May sync with GitHub/Linear

### Status/lifecycle

- `unassigned`
- `queued`
- `assigned`
- `in_progress`
- `blocked`
- `needs_clarification`
- `changes_ready`
- `pr_opened`
- `needs_review`
- `requested_changes`
- `merged`
- `closed`
- `abandoned`

### UI surfaces

- Task Inbox
- Plan View
- Session inspector
- Project graph
- Project Brain drawer

### Events

- `TaskImported`
- `TaskLinkedToPlanTask`
- `TaskAssignedToSession`
- `TaskStatusChanged`
- `TaskSyncedToExternalSource`

---

## 10. ImplementationPlan

### Definition

A structured representation of a project implementation plan file such as `MVP_TASKS.md` or similar.

### Key fields

- `implementation_plan_id`
- `project_id`
- `source_path`
- `format`
- `title`
- `phases[]`
- `tracks[]`
- `tasks[]`
- `anchors[]`
- `current_phase`
- `last_parsed_sha`
- `parse_confidence`

### Relationships

- Belongs to Project
- Has many PlanTasks
- Links to Architecture anchors
- Links to Tasks, Sessions, Worktrees, PRs
- May be parsed by Workflow Pack

### Status/lifecycle

- `not_found`
- `detected`
- `parsed`
- `partially_parsed`
- `stale`
- `out_of_sync`

### UI surfaces

- Plan View
- Project Home
- Workflow Setup
- Project Brain drawer

### Events

- `ImplementationPlanDetected`
- `ImplementationPlanParsed`
- `ImplementationPlanParseFailed`
- `ImplementationPlanUpdated`

---

## 11. PlanTask

### Definition

A structured task parsed from an ImplementationPlan.

### Key fields

- `plan_task_id`
- `implementation_plan_id`
- `project_id`
- `phase`
- `track`
- `title`
- `description`
- `source_anchor`
- `architecture_anchor`
- `files_hint[]`
- `dependencies[]`
- `acceptance_criteria[]`
- `linked_task_ids[]`
- `linked_session_ids[]`
- `linked_pr_ids[]`
- `status`

### Relationships

- Belongs to ImplementationPlan
- May link to Task
- May create Session or AgentTeam
- May link to ArchitectureDoc anchor
- May link to PRs and commits

### Status/lifecycle

- `not_started`
- `ready`
- `blocked`
- `in_progress`
- `in_review`
- `done`
- `deferred`
- `trimmed`
- `out_of_sync`

### UI surfaces

- Plan View
- Task Inbox
- Project graph
- Agent Team launcher
- Project Brain drawer

### Events

- `PlanTaskParsed`
- `PlanTaskLinkedToExternalTask`
- `PlanTaskAssignedToSession`
- `PlanTaskStatusChanged`
- `PlanTaskCompleted`

---

## 12. Session

### Definition

One running or historical agent execution instance. This is the atomic operational object.

### Key fields

- `session_id`
- `project_id`
- `agent_team_id`
- `display_name`
- `harness`
- `model`
- `execution_profile_id`
- `terminal_id`
- `worktree_id`
- `branch_name`
- `linked_task_id`
- `linked_plan_task_id`
- `linked_pr_id`
- `workflow_command_id`
- `status`
- `context_usage`
- `token_usage`
- `cost_estimate`
- `started_at`
- `last_heartbeat_at`
- `completed_at`

### Relationships

- Belongs to Project
- May belong to AgentTeam
- Uses ExecutionProfile
- Runs in Worktree
- Linked to Task/PlanTask/PR
- Has Terminal
- Has Transcript
- Emits Events
- Produces Artifacts
- Requests Approvals
- Produces Project Brain EpisodeCard

### Status/lifecycle

- `creating`
- `starting`
- `active`
- `thinking`
- `running_command`
- `editing_files`
- `running_tests`
- `waiting_on_permission`
- `waiting_on_human_input`
- `waiting_on_external_service`
- `idle`
- `stale`
- `failed`
- `completed`
- `archived`
- `killed`

### UI surfaces

- Left sidebar
- Global Command Center
- Project graph
- Session Terminal View
- Code Editor header
- Inspector
- Human Input Queue
- Project Brain drawer

### Events

- `SessionCreated`
- `SessionStarted`
- `SessionStatusChanged`
- `SessionHeartbeatReceived`
- `SessionMessageSent`
- `SessionToolCallObserved`
- `SessionApprovalRequested`
- `SessionCompleted`
- `SessionArchived`
- `SessionKilled`

---

## 13. Terminal

### Definition

The live or replayable terminal surface attached to a Session.

### Key fields

- `terminal_id`
- `session_id`
- `pty_id`
- `process_id`
- `cwd`
- `shell`
- `terminal_state`
- `scrollback_path`
- `last_output_at`

### Relationships

- Belongs to Session
- Produces Transcript / terminal log
- May emit tool/command observations

### Status/lifecycle

- `starting`
- `attached`
- `detached`
- `backgrounded`
- `closed`
- `crashed`

### UI surfaces

- Session Terminal View
- Agent Team View
- Terminal popover

### Events

- `TerminalAttached`
- `TerminalDetached`
- `TerminalOutputObserved`
- `TerminalClosed`
- `TerminalCrashed`

---

## 14. AgentTeam

### Definition

A coordinated group of sessions working toward one objective, usually with a lead/orchestrator and worker sessions.

### Key fields

- `agent_team_id`
- `project_id`
- `name`
- `objective`
- `workflow_recipe_id`
- `lead_session_id`
- `orchestrator_session_id`
- `worker_session_ids[]`
- `linked_plan_task_id`
- `linked_task_id`
- `worktree_strategy`
- `status`
- `started_at`
- `completed_at`

### Relationships

- Belongs to Project
- Has many Sessions
- May use WorkflowCommand or launch recipe
- Links to PlanTask/Task
- Produces Artifacts and PRs

### Status/lifecycle

- `draft`
- `starting`
- `active`
- `waiting_on_human`
- `blocked`
- `reconciling_outputs`
- `completed`
- `failed`
- `archived`

### UI surfaces

- Agent Team View
- Project graph
- Left sidebar
- Plan View
- Project Brain drawer

### Events

- `AgentTeamCreated`
- `AgentTeamStarted`
- `AgentTeamWorkerSpawned`
- `AgentTeamStatusChanged`
- `AgentTeamBroadcastSent`
- `AgentTeamCompleted`
- `AgentTeamArchived`

---

## 15. ExecutionProfile

### Definition

A named local runtime/account/subscription context used to run an agent session.

### Key fields

- `execution_profile_id`
- `display_name`
- `provider`
- `harness`
- `account_alias`
- `auth_method`
- `local_shell_profile`
- `cli_path`
- `default_model`
- `default_permission_mode`
- `project_allowlist[]`
- `usage_policy`
- `status`
- `last_used_at`

### Relationships

- Belongs to Workspace
- Used by Sessions
- May be assigned per AgentTeam role
- Used in usage/cost dashboards

### Status/lifecycle

- `not_configured`
- `ready`
- `auth_expired`
- `rate_limited`
- `disabled`
- `degraded`

### UI surfaces

- New Session flow
- Agent Team launcher
- Session header
- Inspector
- Settings
- Usage Dashboard

### Events

- `ExecutionProfileCreated`
- `ExecutionProfileValidated`
- `ExecutionProfileAuthExpired`
- `ExecutionProfileUsageUpdated`
- `ExecutionProfileDisabled`

---

## 16. WorkflowPack

### Definition

A reusable project-agnostic package of templates, commands, skills, subagents, hooks, plan parsers, and launch recipes.

### Key fields

- `workflow_pack_id`
- `name`
- `description`
- `source_repo`
- `source_version`
- `templates[]`
- `commands[]`
- `skills[]`
- `subagents[]`
- `hooks[]`
- `detectors[]`
- `plan_parsers[]`
- `launch_recipes[]`

### Relationships

- May produce WorkflowInstance
- Provides WorkflowCommands
- Provides plan parsers
- Provides AgentTeam recipes

### Status/lifecycle

- `available`
- `installed`
- `update_available`
- `disabled`
- `removed`

### UI surfaces

- Workflow Setup
- Settings
- Command Registry

### Events

- `WorkflowPackInstalled`
- `WorkflowPackUpdated`
- `WorkflowPackRemoved`

---

## 17. WorkflowInstance

### Definition

A project-specific personalized/generated instance of a WorkflowPack.

### Key fields

- `workflow_instance_id`
- `workflow_pack_id`
- `project_id`
- `status`
- `manifest_path`
- `generated_from_sha`
- `last_upgraded_from_sha`
- `mode`
- `track`
- `task_tracker_path`
- `architecture_doc_path`
- `code_areas[]`
- `optional_commands[]`
- `optional_subagents[]`
- `generated_files[]`
- `customization_ledger`

### Relationships

- Belongs to Project
- Created from WorkflowPack
- Has WorkflowCommands
- May parse ImplementationPlan
- May launch AgentTeams

### Status/lifecycle

- `detected`
- `needs_personalization`
- `personalization_in_progress`
- `active`
- `ready_for_team_run`
- `running`
- `drift_detected`
- `upgrade_available`
- `archived`
- `detached`

### UI surfaces

- Workflow Setup
- Project Home
- Command Registry
- Agent Team launcher
- Project Brain drawer

### Events

- `WorkflowInstanceDetected`
- `WorkflowInstancePersonalizationStarted`
- `WorkflowInstancePersonalized`
- `WorkflowInstanceDriftDetected`
- `WorkflowInstanceUpgradeAvailable`
- `WorkflowInstanceDetached`

---

## 18. WorkflowPersonalizationRun

### Definition

A run that applies a template WorkflowPack to a project and creates or updates a WorkflowInstance.

### Key fields

- `personalization_run_id`
- `project_id`
- `workflow_pack_id`
- `status`
- `input_architecture_doc`
- `inferred_values`
- `unresolved_questions[]`
- `user_answers[]`
- `generation_plan`
- `generated_diff`
- `approvals[]`
- `resulting_manifest_path`

### Relationships

- Belongs to Project
- Creates or updates WorkflowInstance
- May run through Session
- Emits Approvals and Artifacts

### Status/lifecycle

- `draft`
- `scanning`
- `awaiting_answers`
- `plan_ready`
- `awaiting_approval`
- `writing_files`
- `review_ready`
- `completed`
- `failed`
- `cancelled`

### UI surfaces

- Workflow Setup
- Project Brain drawer
- Code Editor / generated diff
- Human Input Queue

### Events

- `WorkflowPersonalizationStarted`
- `WorkflowPersonalizationQuestionAsked`
- `WorkflowPersonalizationPlanGenerated`
- `WorkflowPersonalizationApproved`
- `WorkflowPersonalizationFilesWritten`
- `WorkflowPersonalizationCompleted`

---

## 19. WorkflowCommand

### Definition

A command surfaced by a Workflow Pack or project-specific workflow instance.

### Key fields

- `workflow_command_id`
- `workflow_instance_id`
- `name`
- `source_file`
- `type`
- `description`
- `argument_schema`
- `supported_contexts[]`
- `supported_harnesses[]`
- `requires_personalized_instance`
- `creates_sessions`
- `risk_level`

### Relationships

- Belongs to WorkflowInstance or WorkflowPack
- May create Session or AgentTeam
- May require ActionGateway approval

### Status/lifecycle

- `available`
- `missing_context`
- `disabled`
- `running`
- `failed`

### UI surfaces

- Command Registry
- Command Palette
- New Session flow
- Agent Team launcher
- Project Brain drawer

### Events

- `WorkflowCommandDiscovered`
- `WorkflowCommandInvoked`
- `WorkflowCommandCompleted`
- `WorkflowCommandFailed`

---

## 20. SubagentDefinition

### Definition

A reusable project or user-level subagent definition discovered from workflow files or agent configuration.

### Key fields

- `subagent_definition_id`
- `project_id`
- `workflow_instance_id`
- `name`
- `role`
- `source_path`
- `description`
- `tools_allowed[]`
- `model_preference`
- `contexts[]`

### Relationships

- May belong to WorkflowInstance
- May be used by Session or AgentTeam

### Status/lifecycle

- `detected`
- `available`
- `invalid`
- `disabled`

### UI surfaces

- Workflow Setup
- Command Registry
- Agent Team launcher

### Events

- `SubagentDetected`
- `SubagentValidated`
- `SubagentUsed`

---

## 21. SkillDefinition

### Definition

A reusable skill or command bundle available to Claude/Codex workflows.

### Key fields

- `skill_definition_id`
- `name`
- `source_path`
- `scope`
- `description`
- `version`
- `provided_commands[]`
- `required_tools[]`

### Relationships

- May belong to WorkflowPack or WorkflowInstance
- May provide WorkflowCommands

### Status/lifecycle

- `installed`
- `available`
- `outdated`
- `invalid`
- `disabled`

### UI surfaces

- Workflow Setup
- Settings
- Command Registry

### Events

- `SkillDetected`
- `SkillInstalled`
- `SkillUpdated`
- `SkillDisabled`

---

## 22. HookDefinition

### Definition

A lifecycle automation hook available to an agent runtime or workflow.

### Key fields

- `hook_definition_id`
- `name`
- `scope`
- `event_type`
- `source_path`
- `command`
- `risk_level`
- `enabled`

### Relationships

- May belong to WorkflowInstance
- May emit platform Events
- May feed Project Brain

### Status/lifecycle

- `detected`
- `enabled`
- `disabled`
- `failed`

### UI surfaces

- Workflow Setup
- Settings
- Security/Permissions

### Events

- `HookDetected`
- `HookEnabled`
- `HookTriggered`
- `HookFailed`

---

## 23. ProjectBrainIndex

### Definition

The Project Brain store/index associated with a project.

### Key fields

- `brain_index_id`
- `project_id`
- `store_path`
- `vector_model`
- `chunker_version`
- `schema_version`
- `ingested_from_sha`
- `last_indexed_at`
- `freshness_state`
- `session_memory_enabled`
- `policy`

### Relationships

- Belongs to Project
- Has MemorySources
- Has EvidenceItems
- Has EpisodeCards
- Has Decisions
- Consumes Events

### Status/lifecycle

- `not_created`
- `indexing`
- `ready`
- `partial`
- `stale`
- `degraded`
- `rebuilding`
- `disabled`

### UI surfaces

- Project Brain drawer
- Project Home
- Settings
- Evidence chips

### Events

- `BrainIndexCreated`
- `BrainIndexUpdated`
- `BrainIndexStale`
- `BrainIndexRebuilt`
- `BrainIndexDegraded`

---

## 24. MemorySource

### Definition

A source document, code file, transcript, PR, ticket, commit, or other artifact ingested by Project Brain.

### Key fields

- `memory_source_id`
- `project_id`
- `source_type`
- `path_or_external_url`
- `producer`
- `doc_type`
- `doc_class`
- `content_hash`
- `last_ingested_sha`
- `freshness_state`
- `trust_rank`

### Relationships

- Belongs to ProjectBrainIndex
- Produces chunks/EvidenceItems
- May link to WorkflowInstance or Session

### Status/lifecycle

- `discovered`
- `ingested`
- `stale`
- `foreign_changed`
- `owned_refresh_available`
- `tombstoned`

### UI surfaces

- Project Brain Memory tab
- Evidence chips
- Drift/Docs view

### Events

- `MemorySourceDiscovered`
- `MemorySourceIngested`
- `MemorySourceStale`
- `MemorySourceTombstoned`

---

## 25. EvidenceItem

### Definition

A cited source used to support a Project Brain answer or action plan.

### Key fields

- `evidence_item_id`
- `project_id`
- `source_type`
- `source_id`
- `path`
- `line_range`
- `commit_sha`
- `session_id`
- `pr_id`
- `confidence`
- `freshness_state`
- `excerpt`

### Relationships

- Belongs to ProjectBrainIndex
- May reference file, commit, PR, session, transcript, ticket, or plan task
- Supports BrainAnswer or ActionPlan

### Status/lifecycle

- `live`
- `stale`
- `moved`
- `unverified`
- `unavailable`

### UI surfaces

- Project Brain drawer
- Evidence chips
- Code Editor
- PR Review

### Events

- `EvidenceItemCreated`
- `EvidenceItemInvalidated`
- `EvidenceItemResolved`

---

## 26. EpisodeCard

### Definition

A privacy-redacted, summarized representation of an agent session transcript for Project Brain.

### Key fields

- `episode_id`
- `session_id`
- `project_id`
- `tool`
- `model`
- `start_ts`
- `end_ts`
- `git_branch`
- `user_intents[]`
- `files_touched[]`
- `key_decisions[]`
- `errors_fixed[]`
- `outcome_summary`
- `linked_commit_ids[]`
- `redacted`
- `association_confidence`

### Relationships

- Belongs to ProjectBrainIndex
- Derived from Session/Transcript
- May link to commits, files, PRs, PlanTasks, Decisions

### Status/lifecycle

- `pending`
- `summarized`
- `redacted`
- `embedded`
- `commit_linked`
- `low_confidence`
- `disabled`

### UI surfaces

- Project Brain drawer
- Session summary
- Evidence chips
- Project timeline

### Events

- `EpisodeCardCreated`
- `EpisodeCardRedacted`
- `EpisodeCardEmbedded`
- `EpisodeCardLinkedToCommit`

---

## 27. Decision

### Definition

A project decision captured from docs, sessions, reviews, Project Brain, or user input.

### Key fields

- `decision_id`
- `project_id`
- `title`
- `status`
- `decision_type`
- `rationale`
- `source_ids[]`
- `architecture_anchor`
- `plan_task_id`
- `session_id`
- `commit_sha`
- `created_at`
- `created_by`

### Relationships

- Belongs to Project
- May link to ArchitectureDoc, PlanTask, Session, PR, commit, EvidenceItems

### Status/lifecycle

- `proposed`
- `open`
- `locked`
- `deferred`
- `superseded`

### UI surfaces

- Project Brain Decisions tab
- Plan View
- Architecture view
- Evidence chips

### Events

- `DecisionCaptured`
- `DecisionStatusChanged`
- `DecisionLinkedToEvidence`

---

## 28. Artifact

### Definition

A concrete output produced by a session, team, workflow, or action.

### Key fields

- `artifact_id`
- `project_id`
- `type`
- `producer_session_id`
- `producer_agent_team_id`
- `path`
- `external_id`
- `summary`
- `created_at`
- `linked_task_id`
- `linked_plan_task_id`

### Relationships

- May belong to Session or AgentTeam
- May be PR, diff, commit, test result, generated doc, summary, or plan update
- May be ingested by Project Brain

### Status/lifecycle

- `created`
- `review_ready`
- `accepted`
- `rejected`
- `superseded`
- `archived`

### UI surfaces

- Session inspector
- Code Editor
- PR Review
- Project Brain drawer
- Event timeline

### Events

- `ArtifactCreated`
- `ArtifactReviewed`
- `ArtifactAccepted`
- `ArtifactArchived`

---

## 29. PullRequest

### Definition

A GitHub or provider pull request linked to branch, worktree, task, session, or team.

### Key fields

- `pr_id`
- `repo_id`
- `provider`
- `number`
- `title`
- `body`
- `branch_name`
- `base_branch`
- `linked_session_id`
- `linked_agent_team_id`
- `linked_task_id`
- `linked_plan_task_id`
- `checks_status`
- `review_status`
- `mergeability`
- `url`

### Relationships

- Belongs to Repository
- Links to Branch
- May be produced by Session/AgentTeam
- May close Task/PlanTask
- Ingested by Project Brain

### Status/lifecycle

- `draft`
- `open`
- `checks_pending`
- `checks_failing`
- `needs_review`
- `changes_requested`
- `approved`
- `mergeable`
- `conflict`
- `merged`
- `closed`

### UI surfaces

- PR Review
- Worktree/Git/PR Center
- Project graph
- Session inspector
- Task Inbox

### Events

- `PullRequestCreated`
- `PullRequestUpdated`
- `PullRequestChecksChanged`
- `PullRequestReviewChanged`
- `PullRequestMerged`
- `PullRequestClosed`

---

## 30. Approval

### Definition

A human decision point required for a session, command, action, or Project Brain plan.

### Key fields

- `approval_id`
- `project_id`
- `requester_type`
- `requester_id`
- `action_type`
- `risk_level`
- `summary`
- `details`
- `requested_at`
- `resolved_at`
- `resolved_by`
- `decision`

### Relationships

- May belong to Session, AgentTeam, WorkflowPersonalizationRun, or ActionRequest
- Appears in Human Input Queue
- Emits Event

### Status/lifecycle

- `requested`
- `approved`
- `denied`
- `edited`
- `auto_approved_by_policy`
- `expired`
- `escalated`

### UI surfaces

- Human Input Queue
- Session Terminal View
- Project Brain drawer
- Workflow Setup
- Action Plan modal

### Events

- `ApprovalRequested`
- `ApprovalApproved`
- `ApprovalDenied`
- `ApprovalExpired`
- `ApprovalEscalated`

---

## 31. ActionRequest

### Definition

A typed request to perform an operation through the Action Gateway.

### Key fields

- `action_request_id`
- `requester_type`
- `requester_id`
- `project_id`
- `action_type`
- `input_payload`
- `risk_level`
- `requires_confirmation`
- `dry_run_result`
- `approval_id`
- `status`
- `result_payload`
- `created_at`
- `completed_at`

### Relationships

- Created by UI, Project Brain, workflow, or automation policy
- May require Approval
- Executes against sessions, git, integrations, files, or workflows
- Emits Events

### Status/lifecycle

- `draft`
- `preview_ready`
- `awaiting_approval`
- `approved`
- `executing`
- `succeeded`
- `failed`
- `cancelled`
- `rolled_back`

### UI surfaces

- Project Brain drawer
- Action Plan modal
- Human Input Queue
- Event timeline

### Events

- `ActionRequested`
- `ActionPreviewGenerated`
- `ActionApproved`
- `ActionExecuted`
- `ActionFailed`
- `ActionRolledBack`

---

## 32. Event

### Definition

An immutable audit/timeline record describing something that happened in the platform.

### Key fields

- `event_id`
- `timestamp`
- `actor_type`
- `actor_id`
- `event_type`
- `project_id`
- `target_type`
- `target_id`
- `summary`
- `metadata`
- `correlation_id`
- `risk_level`

### Relationships

- May link to any object
- Consumed by Project Brain
- Displayed in timelines
- Used for audit and replay

### Status/lifecycle

Events are immutable. Corrections are appended as new events.

### UI surfaces

- Bottom timeline
- Project timeline
- Session timeline
- Audit log
- Project Brain evidence

### Event families

- Workspace events
- Project events
- Session events
- Terminal events
- Workflow events
- Git/worktree events
- Task events
- PR events
- Approval events
- Project Brain events
- Integration events
- Security events

---

## 33. IntegrationConnection

### Definition

A configured connection to an external system such as GitHub or Linear.

### Key fields

- `integration_connection_id`
- `workspace_id`
- `provider`
- `account_alias`
- `auth_status`
- `scopes[]`
- `last_sync_at`
- `sync_status`
- `project_mappings[]`

### Relationships

- Belongs to Workspace
- Maps to Projects
- Imports Tasks and PullRequests

### Status/lifecycle

- `not_connected`
- `connected`
- `auth_expired`
- `syncing`
- `degraded`
- `disabled`

### UI surfaces

- Settings
- Task Inbox
- Project Settings

### Events

- `IntegrationConnected`
- `IntegrationSyncStarted`
- `IntegrationSyncCompleted`
- `IntegrationAuthExpired`

---

## 34. UsageRecord

### Definition

A usage/cost/context record for a session, project, model, harness, or execution profile.

### Key fields

- `usage_record_id`
- `project_id`
- `session_id`
- `execution_profile_id`
- `model`
- `harness`
- `input_tokens`
- `output_tokens`
- `context_usage_percent`
- `cost_estimate`
- `accuracy_level`
- `measured_at`

### Relationships

- Belongs to Session or Project
- Aggregates by ExecutionProfile
- Feeds Usage Dashboard

### Status/lifecycle

- `exact`
- `estimated`
- `unavailable`

### UI surfaces

- Session header
- Project Home
- Usage Dashboard
- Inspector

### Events

- `UsageMeasured`
- `UsageEstimated`
- `UsageThresholdExceeded`

---

## 35. Object chains to preserve

### 35.1 Ticket to merge chain

```text
Task
  → Session or AgentTeam
  → Worktree
  → Branch
  → Diff
  → Commit
  → PullRequest
  → Review
  → Merge
  → Project Brain episode/evidence
```

### 35.2 Plan to implementation chain

```text
ImplementationPlan
  → PlanTask
  → Architecture anchor
  → WorkflowCommand
  → Session or AgentTeam
  → Worktree/Branch
  → PR
  → Done status
  → Decision/Episode evidence
```

### 35.3 Brain action chain

```text
Project Brain query
  → Evidence retrieval
  → Action plan
  → ActionRequest
  → Approval
  → Execution
  → Event
  → Re-index / memory update
```

### 35.4 Workflow personalization chain

```text
WorkflowPack
  → WorkflowPersonalizationRun
  → Generated files/diff
  → Approval
  → WorkflowInstance
  → Command Registry
  → Session/Team launch
```

---

## 36. Next artifact dependencies

This object model should feed:

- Event Model and Audit Trail Spec
- Action Gateway Spec
- Workflow Packs Spec
- UX / Information Architecture Spec
- Main Platform PRD
- Data Model / Storage Architecture
- API Contract

---

## 37. Open questions

1. Should Worktree belong strictly to Repository, or should the platform support virtual worktrees across multi-repo Projects?
2. Should Task and PlanTask be separate objects or should PlanTask be a specialized Task subtype?
3. How should branch ownership work when multiple sessions touch the same branch?
4. How should AgentTeam outputs reconcile into one PR versus multiple PRs?
5. What is the canonical ID for a Claude/Codex session when the platform wraps an existing terminal session?
6. How much of terminal/tool-call state can be captured reliably without relying on fragile transcript formats?
7. Should Project Brain EpisodeCards be generated automatically on session completion or only on archive?
8. How should execution profiles expose usage limits without encouraging automatic account hopping?
9. How should WorkflowCommand schemas be declared for older command formats that have no explicit input schema?
10. What is the minimum ActionRequest schema for MVP?
