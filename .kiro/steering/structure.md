# CALIBER Project Structure

## Repository Layout

```text
caliber/
├── .kiro/
│   ├── steering/          # Project context for Kiro
│   └── prompts/           # Custom Kiro commands
├── docs/
│   ├── CALIBER_PCP_SPEC.md      # Core specification
│   ├── CALIBER_API_REFERENCE.md # API documentation
│   ├── MULTI_AGENT_COORDINATION.md  # Locks, messages, delegation
│   └── QUICK_REFERENCE.md       # Cheat sheet
├── caliber-core/          # Entity types, context assembly, agent coordination
│   └── src/lib.rs         # (absorbed caliber-llm, caliber-agents, caliber-context)
├── caliber-storage/       # Storage trait + event DAG
│   └── src/lib.rs
├── caliber-pcp/           # Validation, checkpoints
│   └── src/lib.rs
├── caliber-dsl/           # Markdown+YAML config parser
│   └── src/lib.rs         # (replaced custom lexer/parser with serde_yaml)
├── caliber-pg/            # pgrx PostgreSQL extension
│   └── src/lib.rs
├── caliber-api/           # REST/gRPC/WebSocket API server
│   └── src/lib.rs
├── caliber-test-utils/    # Test fixtures and generators
│   └── src/lib.rs
├── caliber-sdk/           # TypeScript SDK
│   └── src/
├── app/                   # SvelteKit Pack Editor (v0.5.0+)
│   └── src/
├── packages/ui/           # Svelte 5 component library
│   └── src/
├── landing/               # Astro marketing site
│   └── src/
├── Cargo.toml             # Workspace manifest (7 Rust crates)
├── package.json           # Bun workspace (TypeScript packages)
├── README.md
├── DEVLOG.md              # Development timeline
└── CHANGELOG.md           # Version history
```

## Memory Hierarchy

```text
Trajectory (task container)
├── Scope (context partition)
│   ├── Turn (ephemeral conversation buffer)
│   └── Artifact (preserved output)
└── Note (cross-trajectory knowledge)
```

## Memory Types

| Type        | Retention    | Use Case       |
|-------------|--------------|----------------|
| Ephemeral   | Session/scope| Turn buffer    |
| Working     | Bounded      | Active scope   |
| Episodic    | Configurable | Artifacts      |
| Semantic    | Long-lived   | Notes          |
| Procedural  | Persistent   | Procedures     |
| Meta        | Persistent   | Trajectories   |

## Architecture Evolution (v0.1 → v0.5)

### Crate Consolidation
**Original Design (8 crates):**
- caliber-core (entities only)
- caliber-llm (VAL traits)
- caliber-agents (coordination)
- caliber-context (assembly)
- caliber-storage, caliber-pcp, caliber-dsl, caliber-pg

**Current Design (7 crates):**
- **caliber-core** (absorbed llm, agents, context) - All domain logic
- caliber-storage, caliber-pcp, caliber-dsl, caliber-pg - Infrastructure
- caliber-api - HTTP/gRPC/WebSocket server
- caliber-test-utils - Test fixtures

**Rationale:** Reduced inter-crate dependencies, simpler build graph, clearer ownership.

### DSL Evolution
**v0.1-0.4.5:** Custom lexer/parser (3,762 lines)
**v0.4.6+:** Markdown + YAML fenced blocks (~100 lines)

**Rationale:** Standard tooling, IDE support, simpler maintenance.

### UI Evolution
**v0.1-0.4.5:** caliber-tui (ratatui terminal UI, 4,500 lines)
**v0.4.6:** TUI removed (TypeScript SDK provides all functionality)
**v0.5.0:** SvelteKit Pack Editor (web UI, 45+ components)

**Rationale:** Web > terminal for memory visualization, modern component patterns with Svelte 5 runes.
