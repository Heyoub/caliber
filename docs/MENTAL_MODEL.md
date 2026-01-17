# CALIBER Mental Model

> How to think in CALIBER - A guide for AI agents

---

## Core Philosophy

CALIBER is a **hierarchical memory framework** for AI agents. It models memory the way humans remember: some things are fleeting (what you had for breakfast), others persist (how to ride a bike).

**The key insight:** Turns are cheap and disposable. Artifacts and Notes are valuable and persist.

---

## The Hierarchy

```
TENANT (Isolation boundary - one per organization)
  └── TRAJECTORY (Task/Goal - like a project)
        ├── SCOPE (Context window - like a conversation)
        │     ├── TURN (Ephemeral - dies when scope closes)
        │     └── TURN
        ├── ARTIFACT (Persists - important outputs)
        └── NOTE (Cross-trajectory knowledge)
```

### Entity Lifespans

| Entity | Lifespan | Analogy |
|--------|----------|---------|
| **Tenant** | Permanent | Your organization |
| **Trajectory** | Task duration | A project folder |
| **Scope** | Session | A conversation thread |
| **Turn** | Scope only | Individual messages |
| **Artifact** | Configurable | Saved documents |
| **Note** | Cross-trajectory | Learned knowledge |

---

## Memory Types

CALIBER maps to human-like memory categories:

| Type | Maps To | Lifespan | Example |
|------|---------|----------|---------|
| **Ephemeral** | Turns | Scope only | "The user just asked about X" |
| **Working** | Active scope | Session | "We're debugging the auth flow" |
| **Episodic** | Artifacts | Configurable | "I fixed bug #123 yesterday" |
| **Semantic** | Notes | Long-term | "TypeScript requires strict types" |
| **Procedural** | Notes (procedure type) | Permanent | "How to deploy to production" |

### What Survives When a Scope Closes?

When a Scope closes:
- ❌ **Turns are deleted** (ephemeral conversation)
- ✅ **Artifacts survive** (extracted value)
- ✅ **Notes survive** (learned knowledge)

This is intentional. Turns contain raw conversation data. Before closing a scope, extract valuable information into Artifacts or Notes.

---

## Token Budget Flow

Each Scope has a token budget. As you work, tokens accumulate:

```
Scope(token_budget=8000)
  ├─ Turn(500 tokens) → used: 500
  ├─ Turn(400 tokens) → used: 900
  ├─ Turn(600 tokens) → used: 1500
  ├─ ...
  └─ When 80% full → trigger summarization
```

### Summarization Triggers

CALIBER can auto-summarize when:
- Token usage exceeds threshold (e.g., 80% of budget)
- Scope is closing
- Turn count threshold reached
- Artifact count threshold reached
- Manual API trigger

---

## Abstraction Levels (L0 → L1 → L2)

Information compresses upward as context fills:

```
L0 (Raw):      "User asked about error handling in turn 3, wanted try/catch"
L1 (Summary):  "This trajectory focused on error handling patterns in TypeScript"
L2 (Principle): "Always validate at system boundaries, fail fast internally"
```

| Level | Name | Scope | Example |
|-------|------|-------|---------|
| L0 | Raw | Single turn/artifact | Direct observations |
| L1 | Summary | Within trajectory | Synthesized from multiple L0s |
| L2 | Principle | Cross-trajectory | High-level abstractions |

**Why?** L2 principles survive with minimal tokens. "Always validate inputs" is cheaper than storing 50 examples of input validation.

---

## Multi-Agent Coordination

CALIBER supports multiple agents working together:

### Delegation (Sub-task)

```
Agent A ──[Delegation]──→ Agent B
          (task + context)
              │
              └──→ Creates child trajectory
                   Works in own scope
                   Returns artifacts + result
```

Use when: You need a specialist agent for a sub-task but want to maintain overall control.

### Handoff (Transfer Control)

```
Agent A ──[Handoff]──→ Agent B
          (full context transfer)
              │
              └──→ Takes over trajectory
                   Agent A steps back
```

Use when: Another agent should completely take over (e.g., escalation to human).

### Message (Async Communication)

```
Agent A ──[Message]──→ Agent B
          (notification/query)
              │
              └──→ Processes when ready
                   May or may not respond
```

Use when: Agents need to communicate without blocking.

### Lock (Exclusive Access)

```
Agent A ──[Acquire Lock]──→ Resource
          (exclusive/shared)
              │
              └──→ Other agents wait
                   Release when done
```

Use when: Multiple agents might conflict on the same resource.

---

## Entity Reference Guide

### Trajectory

A trajectory is a **task container**. It represents a goal the agent is pursuing.

```typescript
{
  trajectory_id: "uuid",
  name: "Implement user auth",
  status: "Active" | "Completed" | "Failed" | "Suspended",
  parent_trajectory_id?: "uuid",  // For sub-tasks
  agent_id?: "uuid",
  outcome?: {
    status: "Success" | "Partial" | "Failure",
    summary: "What was accomplished",
    produced_artifacts: ["uuid", ...],
    produced_notes: ["uuid", ...]
  }
}
```

### Scope

A scope is a **context window**. It tracks token usage and contains turns.

```typescript
{
  scope_id: "uuid",
  trajectory_id: "uuid",
  name: "Implementation phase",
  token_budget: 8000,
  tokens_used: 1500,
  is_active: true,
  checkpoint?: {
    context_state: Uint8Array,  // Serialized state
    recoverable: true
  }
}
```

### Turn

A turn is a single **conversation message**. Ephemeral - deleted when scope closes.

```typescript
{
  turn_id: "uuid",
  scope_id: "uuid",
  sequence: 1,
  role: "User" | "Assistant" | "System" | "Tool",
  content: "The message text",
  token_count: 150,
  tool_calls?: [...],
  tool_results?: [...]
}
```

### Artifact

An artifact is **extracted value** from conversation. Persists after scope closes.

```typescript
{
  artifact_id: "uuid",
  trajectory_id: "uuid",
  scope_id: "uuid",
  artifact_type: "Code" | "Document" | "Decision" | "Summary" | ...,
  name: "auth-handler.ts",
  content: "export function authenticate() {...}",
  provenance: {
    source_turn: 5,
    extraction_method: "Explicit" | "Inferred" | "UserProvided",
    confidence: 0.95
  },
  ttl: "Persistent" | "Session" | "Scope" | "Duration(ms)"
}
```

### Note

A note is **cross-trajectory knowledge**. Learned facts that apply everywhere.

```typescript
{
  note_id: "uuid",
  note_type: "Convention" | "Strategy" | "Gotcha" | "Procedure" | ...,
  title: "TypeScript strict mode requirement",
  content: "This project uses strict TypeScript. Always add explicit types.",
  source_trajectory_ids: ["uuid", ...],
  source_artifact_ids: ["uuid", ...],
  access_count: 15  // How often this note was recalled
}
```

### Agent

An agent is a **registered worker**. It has capabilities and permissions.

```typescript
{
  agent_id: "uuid",
  agent_type: "coder" | "reviewer" | "planner" | ...,
  capabilities: ["write_code", "run_tests", ...],
  memory_access: {
    read: [{ memory_type: "Note", scope: "all" }],
    write: [{ memory_type: "Artifact", scope: "own_trajectory" }]
  },
  status: "active" | "idle" | "busy",
  current_trajectory_id?: "uuid",
  can_delegate_to: ["reviewer", "tester"]
}
```

---

## Common Patterns

### Pattern 1: Task Execution

```
1. Create Trajectory (name="Fix bug #123")
2. Create Scope (token_budget=8000)
3. Add Turns (conversation with user/tools)
4. Extract Artifacts (code changes, decisions)
5. Close Scope (turns deleted, artifacts persist)
6. Complete Trajectory (outcome recorded)
```

### Pattern 2: Knowledge Extraction

```
1. During work, identify reusable knowledge
2. Create Note with type:
   - Convention: "Always use async/await, not .then()"
   - Strategy: "Start with tests for complex features"
   - Gotcha: "The API returns 500 for auth failures, not 401"
3. Link to source artifacts/trajectories
4. Note persists across all future trajectories
```

### Pattern 3: Context Management

```
1. Check scope.tokens_used vs scope.token_budget
2. If approaching limit (>80%):
   a. Summarize old turns → create Summary artifact
   b. Create Note with L1/L2 abstractions
   c. Optional: Create checkpoint for recovery
   d. Close scope, open new one
3. New scope starts fresh but artifacts/notes persist
```

### Pattern 4: Sub-task Delegation

```
1. Agent A working on trajectory
2. Needs specialist help → Create Delegation to Agent B
3. Agent B creates child trajectory
4. Agent B works, produces artifacts
5. Agent B completes delegation with result
6. Agent A receives artifacts, continues
```

---

## TTL (Time-to-Live) Reference

```typescript
type TTL =
  | "Persistent"   // Never expires
  | "Session"      // Expires when session ends
  | "Scope"        // Expires when scope closes
  | "Ephemeral"    // Alias for Scope
  | "ShortTerm"    // ~1 hour
  | "MediumTerm"   // ~24 hours
  | "LongTerm"     // ~7 days
  | "Permanent"    // Alias for Persistent
  | { Duration: number }  // Custom milliseconds
```

---

## Graph Relationships (Edges)

CALIBER supports linking entities with typed relationships:

| Edge Type | Meaning | Example |
|-----------|---------|---------|
| Supports | A backs up B | Artifact supports Note |
| Contradicts | A conflicts with B | Two conflicting facts |
| Supersedes | A replaces B | Updated artifact |
| DerivedFrom | A came from B | Summary from turns |
| RelatesTo | Generic relation | Related topics |
| SynthesizedFrom | Output from multiple inputs | L1 from multiple L0s |

---

## Quick Reference

### When to Create What

| Situation | Create |
|-----------|--------|
| Starting a new task | Trajectory |
| Beginning conversation | Scope |
| Each message exchange | Turn |
| Valuable output (code, decision) | Artifact |
| Reusable knowledge | Note |
| Sub-task for specialist | Delegation |
| Transfer control completely | Handoff |
| Async notification | Message |
| Prevent conflicts | Lock |

### What Persists?

| Entity | After Scope Close | After Trajectory Complete |
|--------|-------------------|---------------------------|
| Turn | ❌ Deleted | ❌ Deleted |
| Artifact | ✅ Persists | ✅ Persists |
| Note | ✅ Persists | ✅ Persists |
| Scope | Record remains | Record remains |
| Trajectory | - | Record remains |

---

## API Endpoints Quick Reference

| Resource | Endpoints |
|----------|-----------|
| Trajectories | `POST /trajectories`, `GET /trajectories/:id`, `PATCH`, `DELETE` |
| Scopes | `POST /scopes`, `GET`, `PATCH`, `POST :id/checkpoint`, `POST :id/close` |
| Turns | `POST /turns`, `GET /turns/:id` |
| Artifacts | `POST /artifacts`, `GET`, `PATCH`, `DELETE`, `POST /batch` |
| Notes | `POST /notes`, `GET`, `PATCH`, `DELETE`, `POST /batch` |
| Agents | `POST /agents/register`, `POST /heartbeat`, `DELETE` |
| Locks | `POST /locks/acquire`, `DELETE /release`, `PATCH /extend` |
| Messages | `POST /messages`, `GET`, `POST :id/acknowledge` |
| Delegations | `POST /delegations`, `POST :id/accept`, `POST :id/reject`, `POST :id/complete` |
| Handoffs | `POST /handoffs`, `POST :id/accept`, `POST :id/complete` |
| Search | `POST /search` |
| DSL | `POST /dsl/validate`, `POST /dsl/parse` |

---

## Summary

1. **Hierarchy**: Tenant → Trajectory → Scope → Turn
2. **Persistence**: Artifacts and Notes survive; Turns don't
3. **Token Management**: Budgets with auto-summarization
4. **Abstraction**: L0 → L1 → L2 compression
5. **Coordination**: Delegation, Handoff, Message, Lock

Think of CALIBER as giving your agent a proper memory system - not just a context window, but a structured way to remember what matters and forget what doesn't.
