# @caliber-run/sdk

TypeScript SDK for **CALIBER** - Cognitive Agent Long-term Intelligence, Behavioral Episodic Recall.

## Installation

```bash
npm install @caliber-run/sdk
```

## Quick Start

```typescript
import { CalibrClient } from '@caliber-run/sdk';

const client = new CalibrClient({
  apiKey: process.env.CALIBER_API_KEY!,
  tenantId: 'your-tenant-id',
  // Optional: baseUrl defaults to https://api.caliber.run
});

// Create a trajectory (task container)
const trajectory = await client.trajectories.create({
  name: 'Build feature X',
  description: 'Implement new user authentication',
});

// Create a scope (context window)
const scope = await client.scopes.create({
  trajectory_id: trajectory.trajectory_id,
  name: 'Implementation phase',
  token_budget: 8000,
});

// Store an artifact (extracted value)
await client.artifacts.create({
  trajectory_id: trajectory.trajectory_id,
  scope_id: scope.scope_id,
  artifact_type: 'Code',
  name: 'auth-handler.ts',
  content: 'export function authenticate() {...}',
  source_turn: 1,
  extraction_method: 'Explicit',
  ttl: 'Persistent',
});

// Create a note (cross-trajectory knowledge)
await client.notes.create({
  note_type: 'Convention',
  title: 'Always use async/await',
  content: 'Prefer async/await over .then() chains for readability.',
  source_trajectory_ids: [trajectory.trajectory_id],
  source_artifact_ids: [],
  ttl: 'Persistent',
});
```

## Core Concepts

### Hierarchy

```
TENANT (Isolation boundary)
  └── TRAJECTORY (Task/Goal)
        ├── SCOPE (Context window)
        │     └── TURN (Ephemeral message)
        ├── ARTIFACT (Persists - valuable outputs)
        └── NOTE (Cross-trajectory knowledge)
```

### Memory Types

| Type | Lifespan | Example |
|------|----------|---------|
| Ephemeral | Scope only | Current messages (Turns) |
| Episodic | Configurable | Saved outputs (Artifacts) |
| Semantic | Long-term | Learned knowledge (Notes) |

### What Survives?

When a **Scope** closes:
- ❌ Turns are deleted (ephemeral)
- ✅ Artifacts persist
- ✅ Notes persist

## API Reference

### Managers

| Manager | Resource | Description |
|---------|----------|-------------|
| `client.trajectories` | Trajectory | Task containers |
| `client.scopes` | Scope | Context windows with token budgets |
| `client.artifacts` | Artifact | Extracted valuable outputs |
| `client.notes` | Note | Cross-trajectory knowledge |
| `client.turns` | Turn | Ephemeral conversation messages |
| `client.agents` | Agent | Agent registration and lifecycle |
| `client.locks` | Lock | Distributed locking |
| `client.messages` | Message | Inter-agent messaging |
| `client.delegations` | Delegation | Task delegation |
| `client.handoffs` | Handoff | Context handoffs |
| `client.search` | - | Global search |
| `client.dsl` | - | DSL validation/parsing |

### Common Operations

```typescript
// CRUD operations
const item = await client.trajectories.create({ name: 'Task' });
const item = await client.trajectories.get(id);
const items = await client.trajectories.list({ status: 'Active' });
const item = await client.trajectories.update(id, { name: 'Updated' });
await client.trajectories.delete(id);

// Search
const results = await client.search.search({
  query: 'authentication',
  entity_types: ['Artifact', 'Note'],
});

// Multi-agent coordination
await client.agents.register({ agent_type: 'coder', capabilities: ['write_code'] });
await client.delegations.create({ from_agent_id, to_agent_id, task_description: '...' });
await client.locks.acquire({ resource_type: 'trajectory', resource_id: id, mode: 'Exclusive' });
```

## Error Handling

```typescript
import { CaliberError, NotFoundError, ValidationError } from '@caliber-run/sdk';

try {
  await client.trajectories.get('nonexistent');
} catch (error) {
  if (error instanceof NotFoundError) {
    console.log('Trajectory not found');
  } else if (error instanceof ValidationError) {
    console.log('Validation errors:', error.validationErrors);
  } else if (error instanceof CaliberError) {
    console.log('API error:', error.code, error.message);
  }
}
```

## Configuration

```typescript
const client = new CalibrClient({
  baseUrl: 'https://api.caliber.run',  // API endpoint
  apiKey: 'your-api-key',               // Required
  tenantId: 'your-tenant-id',           // Required
  timeout: 30000,                        // Request timeout (ms)
  headers: { 'X-Custom': 'value' },     // Custom headers
});
```

## License

AGPL-3.0-or-later
