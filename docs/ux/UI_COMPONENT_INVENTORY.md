# AI Engineering Control Plane — UI Component Inventory v0.1

> Status: Draft companion to UX & Information Architecture Spec v0.1  
> Date: 2026-06-07  
> Naming: parent platform name unresolved; use neutral product naming.  
> Constraint: no color taxonomy. Components should use semantic statuses, hierarchy, iconography, labels, density, motion, and layout states.

---

## 1. Purpose

This document gives Claude Design a comprehensive list of components required to create a realistic UI kit and prototype for the desktop AI engineering control plane.

Each component should be designed with:

- Compact developer-tool density
- Clear object ownership
- Explicit status states
- Keyboard/mouse affordances
- Error/loading/degraded variants
- Accessibility-safe status communication

---

## 2. App shell components

### 2.1 Desktop window frame

Purpose: establish desktop app context.

Variants:

- Default desktop app
- Local runtime disconnected
- Update available
- Remote access enabled

States:

- Active window
- Inactive window
- Fullscreen
- Command palette open

### 2.2 Workspace switcher

Fields:

- Workspace name
- Local machine name
- Runtime status
- Sync/index status

Actions:

- Switch workspace
- Open workspace settings
- Add workspace

### 2.3 Global command/search bar

Fields:

- Search placeholder
- Keyboard shortcut hint
- Scope indicator

Modes:

- Search objects
- Run commands
- Ask Project Brain
- Execute workflow command

### 2.4 Left project/session sidebar

Rows:

- Project row
- Session row
- Agent Team row
- Plan shortcut
- Worktree shortcut
- PR shortcut

Required states:

- Collapsed/expanded
- Active selection
- Waiting on human
- Active
- Idle
- Failed
- Completed
- Archived
- Context high
- Profile warning

### 2.5 Main workspace header

Fields:

- Breadcrumb
- Object title
- Status
- Primary action
- Secondary actions
- View tabs

### 2.6 Right inspector panel

Modes:

- Project inspector
- Session inspector
- Agent Team inspector
- Task inspector
- PlanTask inspector
- Worktree inspector
- Branch inspector
- PR inspector
- Approval inspector
- Event inspector

### 2.7 Project Brain drawer

Modes:

- Ask
- Plan
- Dispatch
- Review
- Decisions
- Memory
- Actions

States:

- Closed
- Docked
- Floating
- Split with inspector
- Scoped to project/session/file/PR/selection

### 2.8 Bottom event timeline

Variants:

- Workspace timeline
- Project timeline
- Session timeline
- Team timeline
- PR timeline
- Action timeline

Features:

- Filter chips
- Event severity/status
- Actor/source label
- Object links
- Timestamp
- Expand detail

### 2.9 Status bar

Items:

- Local runtime
- Project Brain index
- Current project
- Worktree/branch
- Execution Profile
- Active sessions
- Waiting approvals
- Token/context summary
- Last sync

---

## 3. Navigation and command components

### 3.1 Command palette

Sections:

- Recent objects
- Actions
- Workflow commands
- Project Brain prompts
- Navigation targets
- Pending approvals

States:

- Empty query
- Searching
- Results
- No results
- Action requires approval

### 3.2 Breadcrumb bar

Examples:

```text
Workspace / Project / Session
Project / Plan / Phase 2 / Task 3
Project / Worktrees / agent/eng-221
Project / PRs / #84
```

### 3.3 View tab group

Used for project tabs and object detail tabs.

States:

- Active
- Dirty/unsaved
- Warning
- Has badge count

### 3.4 Filter bar

Filter types:

- Status
- Model
- Harness
- Execution Profile
- Workflow
- Task source
- PR state
- Human input needed
- Date/time
- Risk level

---

## 4. Status, badge, and metadata components

### 4.1 Session status pill

Statuses:

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

### 4.2 Task status pill

Statuses:

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

### 4.3 Workflow Instance badge

Statuses:

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

### 4.4 Execution Profile badge

Fields:

- Provider
- Account alias
- Harness
- Status

Examples:

```text
Claude Max Main
Claude Max Secondary
Codex CLI Main
Claude Team Work
```

### 4.5 Harness badge

Examples:

```text
Claude Code
Codex CLI
Codex Cloud
Custom Shell
```

### 4.6 Model badge

Fields:

- Model family/name
- Context window if known
- Usage accuracy if known

### 4.7 Context usage meter

Variants:

- Compact ring
- Horizontal meter
- Inline badge
- Team aggregate

States:

- Normal
- Approaching limit
- Action threshold
- Hard stop
- Unknown

### 4.8 Token/cost usage meter

Variants:

- Session usage
- Project usage
- Execution Profile usage
- Workspace usage

Accuracy labels:

- Exact
- Estimated
- Unavailable

### 4.9 Risk badge

Levels:

```text
Read-only
Low
Medium
High
Critical
```

Must include text label, not only visual treatment.

### 4.10 Freshness/provenance badge

Fields:

- Last indexed
- Grounded at SHA
- Stale anchors count
- Graph warm/cold/degraded
- Transcript ingestion state

---

## 5. Project and session components

### 5.1 Project row

Fields:

- Project name
- Repo/provider
- Active count
- Waiting count
- Workflow state
- Brain freshness

### 5.2 Project card

Fields:

- Name
- Repo
- Local path
- Active sessions
- Waiting approvals
- Open PRs
- Current phase
- Workflow state
- Brain index state
- Usage summary

Actions:

- Open project
- Start session
- Open Brain
- Sync

### 5.3 Session row

Fields:

- Status
- Name/task
- Harness/model
- Execution Profile
- Context usage
- Worktree/branch
- Last activity
- Attention marker

Actions:

- Attach terminal
- Open editor
- Ask status
- More menu

### 5.4 Session card

Fields:

- Session title
- Current work
- Status
- Task link
- Branch/worktree
- Model/harness/profile
- Context/tokens
- PR/check state
- Files changed

### 5.5 Session completion card

Fields:

- Outcome summary
- Files changed
- Tests run
- Commits
- PR
- Known issues
- Project Brain indexed/unindexed

Actions:

- Create PR
- Open diff
- Summarize to Brain
- Archive
- Delete/keep worktree

---

## 6. Graph components

### 6.1 Graph canvas

Features:

- Pan/zoom
- Fit to screen
- Minimap
- Layout switcher
- Filter toolbar
- Selection state
- Group/collapse controls

### 6.2 Node base component

Fields:

- Node type icon
- Title
- Status
- Subtitle/owner
- Attention marker
- Metadata chips

States:

- Default
- Selected
- Hovered
- Active
- Waiting
- Failed
- Stale
- Completed
- Collapsed group

### 6.3 Project node

Shows:

- Project name
- Active/waiting count
- Workflow state
- Brain state

### 6.4 Session node

Shows:

- Session name
- Harness/model/profile
- Status
- Current task
- Context meter

### 6.5 Agent Team node

Shows:

- Objective
- Lead/orchestrator
- Worker count
- Waiting count
- Team status

### 6.6 Worker node

Shows:

- Role
- Assigned subtask
- Worktree/branch
- Current status
- Context tier

### 6.7 Task / PlanTask node

Shows:

- Source
- ID/title
- Status
- Phase/track
- Linked session count

### 6.8 Worktree / Branch node

Shows:

- Path/branch
- Dirty/conflict state
- Linked owner
- PR state

### 6.9 PR node

Shows:

- PR number
- Status
- Checks
- Review state
- Mergeability

### 6.10 Human input node

Shows:

- Request type
- Risk level
- Requesting session/team
- Age

### 6.11 Edge component

Relationship labels:

- owns
- assigned to
- spawned
- uses worktree
- tracks branch
- opens PR
- blocked by
- linked to
- supported by evidence
- governed by architecture

---

## 7. Terminal components

### 7.1 Embedded terminal panel

Required visual parts:

- Terminal chrome
- Session tab
- Scrollback
- Current command
- Connection state
- Copy/search controls

States:

- Connected
- Reconnecting
- Disconnected
- Read-only archived
- Waiting on input
- Session complete

### 7.2 Terminal tab group

Fields:

- Session name
- Status
- Role if team
- Attention marker

### 7.3 Message composer

Modes:

- Send to session
- Broadcast to team
- Add task context
- Run workflow command
- Ask follow-up

Features:

- Attachment chips
- Selected context chips
- Command autocomplete
- Send button
- Approval preview if action-like

### 7.4 Tool call log

Fields:

- Tool/command
- Timestamp
- Status
- Files affected
- Risk
- Expand details

### 7.5 Permission request card

Fields:

- Requested command/action
- Session/team
- Reason
- Risk
- Preview
- Evidence/context
- Controls

---

## 8. Code editor and diff components

### 8.1 File explorer

Fields:

- Path
- Status marker
- Changed count
- Worktree root

States:

- Selected
- Modified
- Added
- Deleted
- Renamed
- Conflict
- Generated

### 8.2 Changed files panel

Fields:

- File path
- Change type
- Insertions/deletions
- Review status
- Owner session

### 8.3 Editor tab

States:

- Active
- Dirty
- Read-only
- Generated
- Conflict
- Diff mode

### 8.4 Editor canvas

Must support visual mock states:

- Code view
- Search result highlight
- Selected code
- Inline agent suggestion
- Diagnostics underline/marker

### 8.5 Inline diff hunk

Fields:

- File path
- Hunk header
- Added/removed lines
- Hunk status
- Comment count

Actions:

- Accept
- Reject
- Ask why
- Request fix
- Add comment
- Link to task

### 8.6 Side-by-side diff viewer

Fields:

- Before/after file headers
- Synchronized scroll
- Hunk controls
- Review status

### 8.7 Conflict resolver

Fields:

- Current/incoming/base
- Choose ours
- Choose theirs
- Edit manually
- Ask agent to resolve
- Mark resolved

### 8.8 Problems panel

Fields:

- Severity
- File/line
- Message
- Source
- Related session/check

### 8.9 Test output panel

Fields:

- Test command
- Status
- Failed tests
- Logs
- Rerun action
- Ask agent to fix

### 8.10 Code selection action menu

Actions:

- Ask Project Brain
- Ask active agent
- Explain
- Fix
- Add tests
- Refactor
- Link to task
- Show history
- Create review comment

---

## 9. Task, plan, and ticket components

### 9.1 Task card

Fields:

- Source
- ID
- Title
- Priority
- Labels
- Project/repo
- Status
- Linked plan task
- Suggested agent
- Suggested worktree

### 9.2 Ticket detail panel

Sections:

- Description
- Acceptance criteria
- Labels/priority
- Comments/activity
- Linked sessions
- Linked PRs
- Suggested prompt
- Dispatch controls

### 9.3 Plan phase row

Fields:

- Phase title
- Progress
- Current state
- Open tasks
- Active sessions
- PRs

### 9.4 Plan task row

Fields:

- Status
- Title
- Phase/track
- Architecture anchors
- Linked ticket
- Linked session/team
- PR
- Last update

### 9.5 Architecture anchor chip

Fields:

- Doc path
- Section/anchor
- Freshness state
- Open action

### 9.6 Plan linking panel

Displays mappings:

- PlanTask ↔ Linear issue
- PlanTask ↔ GitHub issue
- PlanTask ↔ Session
- PlanTask ↔ PR
- PlanTask ↔ Architecture anchor

---

## 10. Worktree, git, and PR components

### 10.1 Worktree card/table row

Fields:

- Name/path
- Branch
- Base
- Status
- Dirty files
- Linked session/team
- Linked task
- Last commit
- PR/checks

### 10.2 Branch badge

Fields:

- Branch name
- Ahead/behind
- Protected state
- Owner session/team

### 10.3 Git action toolbar

Actions:

- Create worktree
- Commit
- Push
- Rebase
- Merge main
- Create PR
- Open diff
- Resolve conflicts

### 10.4 Commit card

Fields:

- SHA
- Message
- Author/actor
- Timestamp
- Files changed
- Linked session/task

### 10.5 PR card

Fields:

- Number/title
- Branch/base
- Author/session
- Checks
- Review state
- Mergeability
- Linked task/plan

### 10.6 Checks panel

Fields:

- Check name
- Status
- Duration
- Log link
- Failure summary
- Ask agent to fix

### 10.7 Merge conflict alert

Fields:

- Files in conflict
- Branches involved
- Suggested resolution path
- Open resolver
- Ask agent to resolve

---

## 11. Agent team components

### 11.1 Agent team card

Fields:

- Objective
- Status
- Lead/orchestrator
- Worker count
- Waiting count
- Track/phase
- Context risk
- PR state

### 11.2 Team lead panel

Fields:

- Role
- Current plan
- Delegations
- Last status
- Blockers

### 11.3 Worker card

Fields:

- Role
- Subtask
- Status
- Harness/model/profile
- Worktree/branch
- Context tier
- Current command
- Terminal action

### 11.4 Team graph

Nodes:

- Lead
- Orchestrator
- Workers
- Worktrees
- Branches
- PRs
- Escalations

### 11.5 Team control bar

Actions:

- Broadcast
- Ask lead for status
- Pause all
- Resume all
- Open terminals
- End team
- Summarize
- Merge outputs

### 11.6 TDD slice tracker

Steps:

```text
0 Restate
1 Identify files
2 RED
2.5 Test-design review
3 Confirm RED
4 GREEN
5 Confirm GREEN
6 Refactor
7 Full suite
7.5 Reachability
8 Type + lint
9 Hot-route
10 Atomic commit
```

### 11.7 Escalation card

Categories:

- Critical/safety design question
- Finding
- Deferment approval
- Load-bearing architectural decision

---

## 12. Workflow Pack components

### 12.1 Workflow Pack card

Fields:

- Pack name
- Source
- Type
- Version
- Trust level
- Description
- Install/apply action

### 12.2 Workflow Instance status panel

Fields:

- Instance state
- Manifest path
- Generated from
- Last upgraded
- Mode
- Plan parser
- Commands available
- Team launcher readiness

### 12.3 Detection report

Fields:

- Confidence
- Detected files
- Missing files
- Readiness checks
- Recommended next action

### 12.4 Personalization stepper

Steps:

- Scan
- Infer values
- Ask questions
- Generate plan
- Approve plan
- Write files
- Review diffs
- Activate

### 12.5 Generated file review row

Fields:

- Destination path
- Template source
- Change type
- Status
- Diff action

### 12.6 Command registry table

Columns:

- Command
- Source
- Type
- Arguments
- Context required
- Readiness
- Risk
- Actions

### 12.7 Command card

Fields:

- Name
- Description
- Argument hint
- Source file
- Required context
- Run button
- Open definition

### 12.8 Launch recipe modal

Fields:

- Recipe name
- Source object
- Inputs
- Roles
- Execution Profiles
- Worktree strategy
- Preview action plan

---

## 13. Project Brain components

### 13.1 Brain drawer header

Fields:

- Scope
- Index status
- Evidence mode
- Action mode
- Privacy/transport

### 13.2 Brain message

Types:

- User message
- Answer
- Clarifying question
- Action proposal
- Evidence summary
- Refusal/insufficient evidence

### 13.3 Evidence chip

Types:

- File/line
- Architecture anchor
- PlanTask
- Session episode
- Commit
- PR
- Decision
- Ticket/issue
- Event
- Memory source

Fields:

- Label
- Source type
- Freshness/confidence
- Open action

### 13.4 Action plan card

Fields:

- Proposed by
- Goal
- Steps
- Risk levels
- Evidence
- Preview status
- Approval controls

### 13.5 Memory source card

Fields:

- Source name/path
- Type
- Producer
- Class: owned/foreign/supplemental
- Last indexed
- Freshness
- Chunk count

### 13.6 Decision log item

Fields:

- Decision title
- State: locked/proposed/open/deferred
- Date
- Source/evidence
- Linked architecture anchor
- Linked session/PR

### 13.7 Brain freshness banner

States:

- Ready
- Indexing
- Partial
- Stale
- Graph degraded
- Transcript ingestion disabled
- Error

---

## 14. Action Gateway and approval components

### 14.1 Approval card

Fields:

- Request title
- Requesting actor
- Target
- Risk level
- Reason
- Preview
- Evidence
- Expiration
- Controls

### 14.2 Action plan review panel

Sections:

- Summary
- Step list
- Affected resources
- Preview/dry-run
- Permissions
- Rollback
- Audit note

### 14.3 Action step row

Fields:

- Step number
- Action type
- Target
- Preconditions
- Risk
- Preview
- Status

### 14.4 Risk explanation block

Fields:

- Risk level
- Why classified this way
- Required approval
- Safer alternative if available

### 14.5 Approval controls

Buttons/actions:

- Approve
- Deny
- Edit
- Approve step
- Approve all
- Defer
- Open evidence
- Open terminal/diff

### 14.6 Audit event card

Fields:

- Event type
- Actor
- Timestamp
- Action ID
- Target
- Result
- Evidence links

---

## 15. Setup, settings, and integration components

### 15.1 Setup stepper

Steps:

- Runtime
- Agents
- Profiles
- Project Brain
- Integrations
- Workflow Packs
- Security
- First project

### 15.2 Runtime check row

Fields:

- Component
- Status
- Version
- Required/optional
- Repair/install action

### 15.3 Integration card

Fields:

- Provider
- Status
- Account
- Scope
- Last sync
- Connect/reconnect

### 15.4 Execution Profile settings card

Fields:

- Name
- Provider
- Harness
- Account alias
- Auth state
- Default model
- Project allowlist
- Active sessions

### 15.5 Security policy card

Fields:

- Policy name
- Approval defaults
- Allowed actions
- Restricted actions
- Remote access permissions

### 15.6 Remote access / iOS pairing card

Fields:

- Remote access status
- Paired devices
- Capability mode
- Last connection
- Revoke action
- Audit link

---

## 16. Usage and metering components

### 16.1 Usage summary card

Fields:

- Tokens
- Cost if available
- Sessions
- Profile
- Time period
- Accuracy label

### 16.2 Context meter

Fields:

- Current context
- Limit
- Threshold state
- Session/team owner

### 16.3 Usage table

Columns:

- Project/session/profile
- Input tokens
- Output tokens
- Cost
- Context peak
- Accuracy
- Trend

### 16.4 Budget alert

Fields:

- Scope
- Threshold
- Current usage
- Suggested action

---

## 17. Empty, loading, degraded, and error states

### 17.1 Empty states

Required empty states:

- No projects
- Project has no sessions
- No tasks connected
- No implementation plan
- No Workflow Instance
- No Execution Profiles
- No PRs
- No events
- Project Brain not indexed

Each empty state should have:

- Plain explanation
- Recommended next action
- Secondary learn-more/action

### 17.2 Loading states

Required loading states:

- Project indexing
- Graph rendering
- Integration sync
- Terminal attaching
- Workflow scan
- Plan parser running
- PR checks refreshing
- Project Brain retrieving

### 17.3 Degraded states

Required degraded states:

- Local runtime unavailable
- Project Brain stale
- Code graph unavailable
- Transcript ingestion disabled
- Workflow Pack detected but not ready
- Execution Profile auth expired
- Integration rate limited
- Worktree missing

### 17.4 Error states

Required error states:

- Session failed
- Terminal disconnected
- Git action failed
- PR creation failed
- Workflow personalization failed
- Action preview failed
- Indexing failed
- Permission denied

---

## 18. Minimum UI kit deliverable

Claude Design should produce at least:

- Desktop app shell
- Sidebar/navigation system
- Project/session tree
- Status badge set
- Object cards/rows
- Graph node/edge system
- Terminal panel
- Code editor/diff components
- Task/ticket components
- Plan/PlanTask components
- Worktree/git/PR components
- Agent team components
- Workflow Pack/Instance components
- Command registry
- Project Brain drawer
- Evidence chips
- Action Gateway approvals
- Event timeline/audit log
- Usage/context components
- Setup/settings/integration components
- Remote access stretch components

