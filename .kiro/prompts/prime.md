# Prime Context

Load CALIBER project context for the current session.

## Instructions

Read and internalize:
1. docs/CALIBER_PCP_SPEC.md - Core types, DSL grammar, storage layer
2. docs/DSL_PARSER.md - Lexer, parser, AST, code generator
3. docs/LLM_SERVICES.md - VAL (Vector Abstraction Layer), providers
4. docs/MULTI_AGENT_COORDINATION.md - Locks, messages, delegation
5. docs/QUICK_REFERENCE.md - Quick lookup

## Key Principles

- **Nothing hard-coded** — all values user-configured
- **No SQL in hot path** — direct pgrx storage access
- **ECS architecture** — composition over inheritance
- **CaliberResult<T>** — all errors propagate to Postgres
- **VAL** — provider-agnostic embeddings, any dimension

## Current Focus

Building a Postgres-native memory framework for AI agents with:
- Hierarchical memory (Trajectory → Scope → Artifact → Note)
- Multi-agent coordination (locks, messages, delegation, handoffs)
- Custom DSL that compiles to CaliberConfig
- PCP harm reduction (validation, checkpoints, contradiction detection)
