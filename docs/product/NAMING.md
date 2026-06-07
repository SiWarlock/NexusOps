# Platform Naming Shortlist v1

> Scope: Names for the parent AI coding operations platform. Project Brain may keep its own name or become a named subcomponent. No trademark, domain, package, or company-name clearance has been completed.

---

## 1. Naming Goals

The parent platform name should communicate:

- Many agents/sessions being routed and supervised
- Developer control, not chatbot magic
- Worktrees, branches, tasks, terminals, PRs, and code review
- A calm, serious engineering cockpit
- Enough extensibility for Project Brain, Workflow Packs, Execution Profiles, and Action Gateway

Avoid names that over-index on:

- Generic “AI”
- Generic “AgentOps”
- “Conductor” because the user already references Conductor as a related product/category
- “IDE” because the product is broader than an IDE
- “Copilot” because it collides with an existing dominant term
- Neon/sparkle/generic AI branding

---

## 2. Recommended Naming Architecture

Recommended pair:

```text
Parent platform: Switchyard
Memory/brain service: Anchorlight
```

Tagline:

```text
Switchyard — the AI engineering control plane.
```

Subcomponent language:

```text
Anchorlight: project memory and evidence engine
Workflow Packs: project operating systems
Execution Profiles: runtime/account contexts
Clearance: action approvals and permissioning
Dispatch: task-to-agent launch flow
```

Why this pair works:

- Switchyard implies routing many tracks, sessions, branches, tickets, and agents.
- It avoids saying “AI” directly while still feeling operational.
- It fits the git/worktree metaphor: many tracks, many branches, controlled switching.
- Anchorlight remains a strong name for the evidence/trust/memory layer because it evokes anchors, grounding, and stale-vs-live illumination.

Caveat:

- “Switchyard” has prior technical uses and must be cleared before use.

---

## 3. Top Platform Name Candidates

### 1. Switchyard

Best for: the parent platform.

Meaning:

A control surface where work is routed between tracks. Strongly maps to agent sessions, worktrees, branches, and task dispatch.

Pros:

- Distinctive
- Operational
- Technical without being generic
- Works with git/worktree metaphors
- Good product language: “dispatch to Switchyard,” “Switchyard sessions,” “Switchyard Brain”

Cons:

- Some existing technical usage
- Industrial metaphor may feel less polished than “Flightdeck”

Suggested tagline:

```text
Switchyard — the AI engineering control plane.
```

---

### 2. Flightdeck

Best for: cockpit/air-traffic-control positioning.

Meaning:

The place where the pilot supervises many complex systems.

Pros:

- Easy to understand
- Premium/control-room feel
- Fits the “air traffic control for AI coding agents” metaphor

Cons:

- More generic
- Likely more naming collisions
- Less directly connected to git/worktrees

Suggested tagline:

```text
Flightdeck — mission control for AI coding agents.
```

---

### 3. Patchbay

Best for: routing/integration-heavy product.

Meaning:

A patchbay routes signals between devices. This maps to tasks, agents, terminals, GitHub, Linear, worktrees, PRs, and Project Brain.

Pros:

- Strong routing metaphor
- Developer/infrastructure feel
- Good for integrations

Cons:

- Audio/networking association may need explanation
- Less obviously about code operations

Suggested tagline:

```text
Patchbay — route coding work to the right agents, branches, and reviews.
```

---

### 4. Controlplane

Best for: explicit category positioning.

Meaning:

The layer that coordinates distributed workers.

Pros:

- Extremely accurate
- Enterprise/devops familiar
- Works with “AI coding control plane”

Cons:

- Very generic
- Harder to own
- Less memorable

Suggested tagline:

```text
Controlplane — a control plane for AI software teams.
```

---

### 5. Runloop

Best for: developer-native coordination metaphor.

Meaning:

The repeating cycle that processes events. Good fit for agents, events, approvals, and orchestration.

Pros:

- Developer-native
- Short
- Good for event-driven architecture

Cons:

- May already be used in developer tooling
- Less expressive about multi-agent routing

Suggested tagline:

```text
Runloop — supervise every agent, session, branch, and PR.
```

---

### 6. Forgeyard

Best for: build/workshop metaphor.

Meaning:

A place where work is forged, reviewed, and shipped.

Pros:

- Combines creation + operational yard
- More productizable than “Switchyard” for some audiences

Cons:

- “Forge” is heavily used in dev tools
- Slightly less precise

Suggested tagline:

```text
Forgeyard — coordinate AI agents from task to merged PR.
```

---

### 7. Crewdeck

Best for: agent-team-centric branding.

Meaning:

A deck for managing a crew of coding agents.

Pros:

- Easy to understand
- Ties to agent teams
- Pairs naturally with cc-crew

Cons:

- Over-indexes on teams and may not fit solo sessions, PRs, code editor, or Project Brain
- “Crew” has existing AI framework associations

Suggested tagline:

```text
Crewdeck — manage your AI coding crew.
```

---

### 8. RelayForge

Best for: workflow/action routing.

Meaning:

Work is relayed between tasks, agents, code, and review.

Pros:

- Active, product-y
- Captures handoffs

Cons:

- More SaaS-sounding
- Less memorable than Switchyard

Suggested tagline:

```text
RelayForge — relay work from ticket to agent to PR.
```

---

## 4. Project Brain Naming Candidates

If Project Brain is renamed, top candidates:

### Anchorlight

Best candidate for the memory/evidence engine.

Why:

- Encodes `file:line` anchors.
- Encodes live/stale visibility.
- Feels trustworthy and ownable.
- Works well as a subproduct: “Anchorlight inside Switchyard.”

### Lodestar

Good for guidance/truth.

Pros: memorable and navigational.  
Cons: more abstract and likely crowded.

### Graphkeep

Good for technical graph/memory positioning.

Pros: clear and technical.  
Cons: can imply a single native graph, which is not guaranteed.

### Throughline

Good for historical/project-memory questions.

Pros: maps to “how did this happen across time?”  
Cons: less obviously a developer tool.

### Cartulary

Good for authoritative project records.

Pros: unique and accurate.  
Cons: obscure.

Recommendation:

```text
Parent: Switchyard
Brain: Anchorlight
Full phrasing: Switchyard with Anchorlight project memory
```

---

## 5. Product Language Examples

```text
Open Switchyard.
Dispatch ENG-221 to Claude Max Main.
Start /team-start for Phase 2 backend.
Ask Anchorlight when we implemented auth callback.
Review the diff in Switchyard.
Clear the pending approval.
Archive the session and preserve the episode card.
```

```text
Switchyard Project Home
Switchyard Sessions
Switchyard Dispatch
Switchyard Worktrees
Switchyard Review
Switchyard Brain powered by Anchorlight
```

---

## 6. Immediate Recommendation

Use this internally for now:

```text
Switchyard
```

Use this as the descriptive category:

```text
AI engineering control plane
```

Use this as the memory subproduct:

```text
Anchorlight
```

Use this as the combined one-liner:

```text
Switchyard is an AI engineering control plane with Anchorlight project memory, built to dispatch, supervise, review, and merge work from Claude, Codex, and custom agent teams.
```

---

## 7. Clearance Checklist Before Committing

Before finalizing any name:

- Search exact company/product names
- Search npm package names
- Search GitHub org/repo names
- Search PyPI names
- Search domain availability
- Search X/Twitter/GitHub handles
- Search USPTO and relevant trademark databases
- Search App Store/Desktop app listings if shipping desktop
- Check confusion with major AI/developer tools

This document does not perform legal clearance.

