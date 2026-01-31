# CALIBER Examples

This directory contains real-world examples of using CALIBER in different scenarios.

## Examples Overview

| Example | Description | Components Used | Difficulty |
|---------|-------------|-----------------|------------|
| [agents-pack](agents-pack/) | Complete pack manifest with agents & tools | dsl, api | Reference |
| [basic_trajectory](basic_trajectory.rs) | Create trajectory, scope, and artifacts | core, storage | Beginner |
| [context_assembly](context_assembly.rs) | Assemble context with token budgets | core, context | Intermediate |
| [multi_agent_coordination](multi_agent_coordination.rs) | Agents with locks and messages | core, agents | Advanced |
| [vector_search](vector_search.rs) | Semantic search with embeddings | core, llm, storage | Intermediate |
| [dsl_configuration](dsl_configuration.rs) | Configure CALIBER via DSL | core, dsl | Intermediate |
| [pcp_validation](pcp_validation.rs) | Validation and checkpoints | core, pcp | Advanced |
| [rest_api_client](rest_api_client/) | Use CALIBER via REST API | Python, TypeScript, Rust | Beginner |
| [grpc_client](grpc_client/) | Use CALIBER via gRPC | Go, Rust | Intermediate |
| [websocket_realtime](websocket_realtime/) | Real-time updates via WebSocket | JavaScript, Rust | Intermediate |

## Running Examples

### Rust Examples

```bash
# Run a specific example
cargo run --example basic_trajectory

# Run with PostgreSQL (requires caliber-pg installed)
cargo run --example basic_trajectory --features pg

# Run all examples
./scripts/run_examples.sh
```

### API Client Examples

```bash
# Python
cd examples/rest_api_client/python
pip install -r requirements.txt
python client.py

# TypeScript (bun recommended)
cd examples/rest_api_client/typescript
bun install
bun start

# TypeScript (npm compatible)
cd examples/rest_api_client/typescript
npm install
npm start

# Go
cd examples/grpc_client/go
go run main.go
```

## Prerequisites

### For Rust Examples
- Rust 1.75+
- PostgreSQL 13-17 (for caliber-pg examples)

### For API Examples
- Running caliber-api server
- Python 3.12+ / Node.js 20+ / Go 1.22+

## Example Categories

### 0. Pack Development
- **[agents-pack](agents-pack/)** - Complete reference pack with:
  - `cal.toml` manifest with all supported sections
  - Agent definitions (`.agent.md` files)
  - Prompt-based tools (`.md` prompt files)
  - Tool contracts (JSON schemas)
  - Toolsets for grouping tools

See [Creating Your Own Pack](#creating-your-own-pack) below.

### 1. Core Concepts
- **basic_trajectory.rs** - Trajectory → Scope → Artifact hierarchy
- **memory_types.rs** - Different memory types (ephemeral, working, episodic, semantic)
- **ttl_management.rs** - Time-to-live and retention policies

### 2. Context Assembly
- **context_assembly.rs** - Token budgets and section priorities
- **relevance_scoring.rs** - Artifact relevance and ranking
- **scope_summaries.rs** - Compressing scope history

### 3. Multi-Agent
- **multi_agent_coordination.rs** - Locks, messages, delegation
- **agent_handoff.rs** - Transferring work between agents
- **conflict_resolution.rs** - Detecting and resolving conflicts

### 4. Vector Operations
- **vector_search.rs** - Semantic similarity search
- **embedding_providers.rs** - Using different embedding providers
- **hybrid_search.rs** - Combining vector and keyword search

### 5. DSL Configuration
- **dsl_configuration.rs** - Parsing and applying DSL
- **memory_definitions.rs** - Defining memory structures
- **policy_rules.rs** - Automated policies and triggers

### 6. PCP (Persistent Context Protocol)
- **pcp_validation.rs** - Validation modes and rules
- **checkpoints.rs** - Creating and recovering from checkpoints
- **contradiction_detection.rs** - Detecting inconsistencies

### 7. API Integration
- **rest_api_client/** - REST API usage in multiple languages
- **grpc_client/** - gRPC usage with streaming
- **websocket_realtime/** - Real-time event subscriptions

## Common Patterns

### Pattern 1: Basic Workflow

```rust
// 1. Create trajectory
let trajectory_id = create_trajectory("my-task")?;

// 2. Create scope
let scope_id = create_scope(trajectory_id, "initial-scope")?;

// 3. Add artifacts
let artifact_id = create_artifact(
    scope_id,
    ArtifactType::Code,
    content,
)?;

// 4. Assemble context
let context = assemble_context(trajectory_id, scope_id)?;
```

### Pattern 2: Multi-Agent Coordination

```rust
// 1. Register agents
let agent1 = register_agent("agent-1", AgentType::Executor)?;
let agent2 = register_agent("agent-2", AgentType::Reviewer)?;

// 2. Acquire lock
let lock = acquire_lock(resource_id, agent1, timeout)?;

// 3. Send message
send_message(agent1, agent2, "task-complete", payload)?;

// 4. Delegate task
delegate_task(agent1, agent2, task_description)?;
```

### Pattern 3: Vector Search

```rust
// 1. Create embedding
let embedding = provider.embed("search query")?;

// 2. Search artifacts
let results = search_artifacts(
    scope_id,
    embedding,
    similarity_threshold: 0.8,
    limit: 10,
)?;

// 3. Rank by relevance
let ranked = rank_by_relevance(results, context)?;
```

## Configuration

All examples use explicit configuration (no defaults):

```rust
let config = CaliberConfig {
    token_budget: 8000,
    checkpoint_retention: 5,
    stale_threshold: Duration::from_secs(86400 * 30),
    contradiction_threshold: 0.85,
    // ... all fields required
};
```

## Error Handling

All examples use `CaliberResult<T>`:

```rust
fn example() -> CaliberResult<()> {
    let trajectory = create_trajectory("test")?;
    let scope = create_scope(trajectory, "scope")?;
    Ok(())
}
```

## Testing Examples

Each example includes tests:

```bash
# Test a specific example
cargo test --example basic_trajectory

# Test all examples
cargo test --examples
```

## Contributing Examples

When adding new examples:

1. Follow the existing structure
2. Include comprehensive comments
3. Add error handling
4. Include tests
5. Update this README
6. Add to the table above

## Creating Your Own Pack

A pack is a directory containing your CALIBER configuration, agents, and tools.

### Minimal Pack Structure

```
my-pack/
├── cal.toml           # Required: Pack manifest
└── agents/
    └── support.md     # At least one agent
```

### Quick Start

1. **Copy the minimal fixture** (or use `caliber pack init`):
   ```bash
   cp -r tests/fixtures/pack_min my-pack
   ```

2. **Or copy the full example**:
   ```bash
   cp -r examples/agents-pack my-pack
   ```

3. **Configure the TUI to use your pack**:
   ```toml
   # In your TUI config file
   pack_root = "./my-pack"
   ```

4. **Compose your pack**:
   - Run the TUI and press `n` to compose
   - Or use the REST API: `POST /v1/pack/compose`

### Pack Manifest (cal.toml)

The `cal.toml` file defines your pack structure:

```toml
[meta]
version = "1.0"
project = "my-project"

[defaults]
strict_markdown = true
strict_refs = true

[adapters.main]
type = "postgres"
connection = "env:DATABASE_URL"

[profiles.default]
retention = "persistent"
index = "vector"
embeddings = "openai"
format = "markdown"

[agents.support]
enabled = true
profile = "default"
adapter = "main"
prompt_md = "agents/support.md"
toolsets = ["core"]

[toolsets.core]
tools = ["tools.prompts.search"]

[tools.prompts.search]
kind = "prompt"
prompt_md = "tools/prompts/search.md"
```

See [examples/agents-pack/cal.toml](agents-pack/cal.toml) for a comprehensive reference.

### Agent Definition (*.agent.md or *.md)

Agents are Markdown files with frontmatter:

```markdown
---
role: support
capabilities:
  - memory_read
  - memory_write
---

# Support Agent

You are a helpful support agent...
```

### Prompt Tool (tools/prompts/*.md)

Prompt-based tools are Markdown files:

```markdown
---
name: search
description: Search the knowledge base
---

# Search Tool

Given a query, search for relevant information...
```

### Tool Contract (tools/contracts/*.json)

Optional JSON Schema for tool validation:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "query": { "type": "string" }
  },
  "required": ["query"]
}
```

## Questions?

- Check [CONTRIBUTING.md](../CONTRIBUTING.md)
- Review [docs/QUICK_REFERENCE.md](../docs/QUICK_REFERENCE.md)
- Open an issue for clarification
