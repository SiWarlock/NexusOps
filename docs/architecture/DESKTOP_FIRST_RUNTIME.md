# Desktop-First Runtime Addendum v0.1

> Status: Draft  
> Scope: Cross-cutting product decision  
> Applies to: Product Canon, Shared Object Model, Action Gateway, Event Model, UX Spec, Platform PRD  
> Decision: The platform is a **desktop app**, not a web app.

---

## 1. Decision

The platform will be designed as a desktop-first application.

This changes the architecture and UX assumptions. The desktop app is not just a browser shell; it is the control plane that owns local terminals, repos, worktrees, credentials, git actions, session capture, and Project Brain integration.

---

## 2. Why desktop-first

The product's core workflows are local-machine workflows:

```text
run Claude Code / Codex terminals
attach to local PTYs
create git worktrees
edit local files
review diffs
run tests
inspect local repos
capture local session history
read local transcripts
manage local execution profiles
index project code/docs into Project Brain
```

A web-only app would require a local daemon anyway. A desktop app makes the local trust boundary explicit and gives the user a single place to supervise local execution.

---

## 3. Desktop app responsibilities

The desktop app owns:

```text
UI shell
left project/session sidebar
project graph
terminal windows/panes
code editor/review workspace
human input queue
task inbox
worktree/git/PR center
workflow setup UI
Project Brain drawer
Action Gateway UI
local event store
local projections
local runner
terminal/session manager
git/worktree manager
integration sync workers
execution profile manager
notification system
```

---

## 4. Local runtime shape

Recommended architecture:

```text
Desktop App
  UI process
  local app backend
  local event store
  local runner
  terminal manager
  git manager
  workflow runtime
  Project Brain service/client
  Action Gateway
  integration sync workers
```

The implementation may still use internal services, workers, or a local daemon, but those are implementation details of the desktop product.

---

## 5. Security boundary

The desktop app is the primary trust boundary.

It holds or brokers access to:

```text
local filesystem
local git repos
worktree paths
terminal processes
Claude/Codex execution contexts
GitHub/Linear tokens
Project Brain indexes
session transcript pointers
```

Any remote companion, plugin, or external integration must request actions through the desktop app's Action Gateway.

---

## 6. iOS companion stretch goal

A companion iOS app is a stretch goal for remote observability and limited control.

### 6.1 Possible use cases

```text
See which sessions are active
See which sessions are waiting on human input
Approve/deny low-risk permission prompts
Pause/resume sessions
Ask Project Brain a project-state question
Receive context/token/cost alerts
Check PR status
Ask an agent team lead for status
Send a short instruction to an existing session
```

### 6.2 Hard boundary

The iOS app should not become a remote shell.

It must not directly access:

```text
raw terminal PTYs
filesystem
credentials
full codebase
raw transcripts
raw environment variables
git remotes
Claude/Codex auth material
```

All remote actions route through:

```text
iOS Companion → encrypted relay/pairing channel → Desktop App → Action Gateway → local executor
```

### 6.3 MVP stance

Do not design MVP around the iOS app.

Design MVP so the future iOS companion is possible by:

```text
having a local event store
having redacted projections
having typed actions
having approvals
having a clear permission model
having durable audit events
```

---

## 7. Impact on artifacts

### Product Canon

Update platform definition to say:

```text
The platform is a desktop-first AI engineering control plane.
```

### Shared Object Model

Add or emphasize:

```text
Device
RemoteClient
LocalRunner
Terminal
EventProjection
```

### Action Gateway

Add remote-client actor mode and require all remote actions to route through the gateway.

### Event Model

Add remote companion events and redacted projections.

### UX Spec

Assume desktop-specific affordances:

```text
native windows / panes
keyboard shortcuts
terminal tabs
file picker
local notifications
menu bar / tray possible
system-level permission prompts
```

### Platform PRD

Make web app explicitly a non-goal for MVP unless a local desktop shell is still the product surface.

---

## 8. Open questions

```text
Which desktop framework should be used?
Should there be a background helper/daemon separate from the UI process?
Should the terminal/session runner survive UI restarts?
Should Project Brain run embedded, sidecar, or separate service?
How should app updates handle local stores and running sessions?
How should crash recovery reconnect to existing terminal sessions?
Can the app safely manage multiple Claude/Codex execution profiles at the OS process level?
What is the minimum useful iOS read-only projection?
Would iOS remote control require a hosted relay, local VPN/Tailscale, iCloud, or direct pairing?
What actions can be approved from iOS?
Should critical actions be desktop-only?
```

---

## 9. Strong decisions captured

```text
Desktop app is core product surface.
Web app is not MVP.
Local machine is the execution and trust boundary.
Future iOS companion is stretch goal.
Remote companion is observability-first, control-second.
Remote actions must use the Action Gateway.
Remote companion must never become direct remote shell access.
```
