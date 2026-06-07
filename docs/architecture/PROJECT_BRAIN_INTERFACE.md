# Project Brain ↔ Main Platform Interface Notes v0.1

> **Purpose:** Minimal Project Brain interface artifact for the main platform architecture draft. This is not the full Project Brain PRD. The full Project Brain-specific docs live in the separate Project Brain artifact bundle.

---

## 1. Boundary

The main platform and Project Brain should be built alongside each other, but with a clean boundary.

```text
Project Brain owns:
  memory
  retrieval
  evidence
  reasoning
  historical implementation questions
  recommendations
  action planning

Main platform owns:
  desktop UI
  local terminals
  Claude/Codex sessions
  worktrees
  git operations
  GitHub/Linear actions
  credentials
  permissions
  action execution
  audit logs
```

Project Brain can plan or request actions, but all mutations go through the platform's Action Gateway.

---

## 2. Integration mode

Project Brain should remain standalone as a local-first service/library, but be platform-native:

- It can run independently for indexing, retrieval, and project Q&A.
- It should use shared IDs for platform objects where available.
- It should consume platform events when integrated.
- It should expose APIs used by the platform's Project Brain drawer.
- It should not directly create terminals, change git state, modify tickets, or run workflow commands outside the Action Gateway.

---

## 3. Shared IDs and references

The platform and Project Brain should converge on these shared identifiers:

```text
workspace_id
project_id
repo_id
worktree_id
branch_name
commit_sha
session_id
agent_team_id
execution_profile_id
workflow_pack_id
workflow_instance_id
workflow_command_id
implementation_plan_id
plan_task_id
architecture_anchor
linear_issue_id
github_issue_number
pr_number
action_request_id
event_id
artifact_id
evidence_item_id
```

---

## 4. Inputs Project Brain should ingest from the platform

When integrated with the platform, Project Brain should ingest or reference:

- Projects and repos.
- Implementation plans and plan tasks.
- Architecture anchors and docs.
- Code and docs.
- Commits and changed files.
- Pull requests and reviews.
- GitHub issues and Linear tickets.
- Session summaries.
- Redacted/opt-in session transcript episode cards.
- Agent team summaries.
- Workflow pack metadata and workflow instance state.
- Decisions.
- Action Gateway action plans/results.
- Event log entries.

---

## 5. Outputs Project Brain should provide to the platform

Project Brain should provide:

- Evidence-backed answers.
- Evidence chips for code, docs, commits, sessions, PRs, events, and plan tasks.
- Historical implementation answers, such as “when/how did we implement feature Y?”
- Current-state summaries.
- Plan/task recommendations.
- Review explanations.
- Diff explanations.
- Draft PR descriptions.
- Draft task prompts.
- Action plans for Action Gateway review.

---

## 6. Project Brain drawer behavior

The platform should expose Project Brain as a drawer or side panel with modes:

```text
Ask
Plan
Review
Decisions
Memory
Actions
```

The drawer should support scope chips:

```text
Entire project
Current session
Current file
Current diff
Current PR
Current plan task
Current agent team
```

Every substantive answer should show evidence chips and freshness/provenance state.

---

## 7. Action planning flow

Example:

```text
User: Start the next backend task.

Project Brain:
  Retrieves current implementation plan, architecture anchors, active sessions,
  workflow instance state, execution profiles, worktree state, and linked tickets.

Project Brain proposes:
  1. Create worktree agent/p2-backend-auth
  2. Start Claude Code session under Claude Max Main profile
  3. Link session to PlanTask phase-2-backend-auth
  4. Send generated task prompt
  5. Open session terminal

Platform:
  Normalizes the plan as Action Gateway actions.
  Shows preview and risk.
  User approves.
  Platform executes.
  Events are emitted.
  Project Brain indexes resulting evidence later.
```

---

## 8. Safety requirements

- Project Brain must not silently mutate files, tickets, git state, sessions, or workflow configuration.
- Every requested mutation must become a typed Action Gateway request.
- High-risk and critical actions must require explicit human approval.
- Project Brain answers and proposed actions should cite evidence when available.
- Project Brain should respect project privacy policies and transcript-ingestion consent.

---

## 9. Architecture draft implications

The architecture draft should define:

- Project Brain service boundary.
- IPC/API between desktop platform and Project Brain.
- Shared object IDs and reference types.
- How Project Brain consumes the event log.
- How Project Brain stores evidence references back to platform objects.
- How Action Gateway accepts Project Brain action plans.
- How Project Brain handles unavailable or stale platform data.
- Whether Project Brain runs as an embedded process, sidecar daemon, local service, or library.

