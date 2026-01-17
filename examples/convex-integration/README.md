# CALIBER + Convex Integration Example

This example demonstrates how to integrate CALIBER (Cognitive Agent Long-term Intelligence, Behavioral Episodic Recall) with Convex, providing AI agents running on Convex with enterprise-grade memory capabilities.

## What is CALIBER?

CALIBER is a hierarchical memory framework for AI agents that provides:

- **Trajectories**: Task containers that track agent work from start to completion
- **Scopes**: Context windows within trajectories for managing token budgets
- **Artifacts**: Extracted values (code, decisions, plans) that persist beyond conversations
- **Notes**: Cross-trajectory knowledge that accumulates over time
- **Turns**: Ephemeral conversation messages within scopes

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Convex Application                        │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────┐ │
│  │   Frontend      │  │  Convex Backend │  │   Actions   │ │
│  │   (React/etc)   │──│  (Queries/Mut.) │──│ (Node.js)   │ │
│  └─────────────────┘  └─────────────────┘  └──────┬──────┘ │
└───────────────────────────────────────────────────┼─────────┘
                                                    │
                                                    │ HTTP/REST
                                                    ▼
                              ┌──────────────────────────────┐
                              │         CALIBER API          │
                              │  - REST endpoints            │
                              │  - WebSocket events          │
                              │  - Multi-tenant isolation    │
                              └──────────────────────────────┘
```

## Quick Start

### 1. Install Dependencies

```bash
cd examples/convex-integration
bun install
```

### 2. Configure Environment

Create a `.env.local` file with your CALIBER credentials:

```env
CALIBER_API_KEY=your-api-key
CALIBER_TENANT_ID=your-tenant-id
CALIBER_API_URL=https://api.caliber.run  # or your self-hosted URL
```

### 3. Deploy to Convex

```bash
bun run dev
```

### 4. Use in Your Code

```typescript
import { api } from "./_generated/api";

// Start a new task
const task = await ctx.runAction(api.actions.caliber.startTask, {
  name: "Help user with code review",
  description: "Reviewing PR #123 for security issues",
  tokenBudget: 8000,
});

// Add conversation turns
await ctx.runAction(api.actions.caliber.addTurn, {
  scopeId: task.scopeId,
  role: "User",
  content: "Can you review this PR for security vulnerabilities?",
});

// Extract valuable artifacts
await ctx.runAction(api.actions.caliber.extractArtifact, {
  trajectoryId: task.trajectoryId,
  scopeId: task.scopeId,
  name: "Security Findings",
  content: "Found SQL injection vulnerability in auth.ts:45...",
  artifactType: "Document",
  sourceTurn: 3,
});

// Get context for LLM prompts
const context = await ctx.runAction(api.actions.caliber.getContext, {
  trajectoryId: task.trajectoryId,
  relevanceQuery: "security vulnerabilities",
  format: "xml",
});

// Use context in your LLM call
const response = await callLLM(`
${context}

Continue the code review...
`);
```

## Available Actions

### Task Lifecycle

| Action | Description |
|--------|-------------|
| `startTask` | Create a trajectory + scope for a new task |
| `completeTask` | Mark a task as completed with outcome |

### Conversation (Turns)

| Action | Description |
|--------|-------------|
| `addTurn` | Add a message to the current scope |

### Artifacts (Extracted Values)

| Action | Description |
|--------|-------------|
| `extractArtifact` | Save valuable output that should persist |
| `listArtifacts` | Get artifacts for a trajectory |

### Notes (Cross-Trajectory Knowledge)

| Action | Description |
|--------|-------------|
| `createNote` | Create long-term knowledge |
| `listNotes` | Get all notes |

### Context Retrieval

| Action | Description |
|--------|-------------|
| `getContext` | Get formatted context for LLM prompts |
| `getContextRaw` | Get structured context data |

### Scope Management

| Action | Description |
|--------|-------------|
| `createScope` | Create a new context window |
| `closeScope` | Close a scope (triggers summarization) |

### Batch Operations

| Action | Description |
|--------|-------------|
| `batchCreateArtifacts` | Bulk create artifacts |
| `batchCreateNotes` | Bulk create notes |

## Memory Hierarchy

CALIBER organizes memory in a hierarchy that maps naturally to AI agent workflows:

```
Tenant (Organization)
└── Notes (Long-term knowledge, persists forever)
    └── Trajectory (Task/Conversation)
        └── Scope (Context window with token budget)
            └── Turns (Individual messages)
        └── Artifacts (Extracted values from this task)
```

### Best Practices

1. **Start with a Trajectory**: Every agent task should have a trajectory
2. **Use Scopes for Token Management**: Create new scopes when approaching token limits
3. **Extract Artifacts**: Save important outputs (code, decisions, plans) as artifacts
4. **Create Notes for Patterns**: When you notice recurring patterns, save them as notes

## Optional: Local Caching

The `schema.ts` file defines optional Convex tables for caching CALIBER data locally. This is useful for:

- **Real-time subscriptions**: Use Convex's reactive system on CALIBER data
- **Offline access**: Show recent memories without API calls
- **Reduced latency**: Cache frequently-accessed context

To sync data, you can:

1. **Webhooks**: Configure CALIBER to POST events to a Convex HTTP action
2. **Polling**: Periodically fetch and cache updates
3. **On-demand**: Cache data when accessed

## Self-Hosted CALIBER

CALIBER can be self-hosted. Set `CALIBER_API_URL` to your deployment:

```env
CALIBER_API_URL=https://caliber.your-company.com
```

## Resources

- [CALIBER Documentation](https://caliber.run/docs)
- [CALIBER SDK Reference](https://caliber.run/sdk)
- [Convex Documentation](https://docs.convex.dev)

## License

MIT License - See the main CALIBER repository for details.
