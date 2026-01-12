# CALIBER Project Structure

## Repository Layout

```text
caliber/
├── .kiro/
│   ├── steering/          # Project context for Kiro
│   └── prompts/           # Custom Kiro commands
├── docs/
│   ├── CALIBER_PCP_SPEC.md      # Core specification
│   ├── DSL_PARSER.md            # Lexer, parser, codegen
│   ├── LLM_SERVICES.md          # VAL + summarization
│   ├── MULTI_AGENT_COORDINATION.md  # Locks, messages, delegation
│   └── QUICK_REFERENCE.md       # Cheat sheet
├── caliber-core/          # Entity types (data only)
│   └── src/lib.rs
├── caliber-storage/       # Storage trait + pgrx impl
│   └── src/lib.rs
├── caliber-context/       # Context assembly
│   └── src/lib.rs
├── caliber-pcp/           # Validation, checkpoints
│   └── src/lib.rs
├── caliber-llm/           # VAL + summarization traits
│   └── src/lib.rs
├── caliber-agents/        # Multi-agent coordination
│   └── src/lib.rs
├── caliber-dsl/           # DSL parser → config
│   └── src/lib.rs
├── caliber-pg/            # pgrx extension
│   └── src/lib.rs
├── Cargo.toml             # Workspace manifest
├── README.md
└── DEVLOG.md              # Development timeline
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
