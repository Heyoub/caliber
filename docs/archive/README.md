# Archived Documentation

These documents are preserved for historical reference but are **no longer accurate** for the current codebase.

## Why Archived

### LLM_SERVICES.md
- References deleted `caliber-llm` crate (absorbed into caliber-core/caliber-api in v0.4.6)
- VAL traits now in `caliber-core/src/llm.rs`
- Provider orchestration now in `caliber-api/src/providers/`

### MULTI_AGENT_COORDINATION.md
- References deleted `caliber-agents` crate (absorbed into caliber-core in v0.4.6)
- Agent types now in `caliber-core/src/agent.rs` and `caliber-core/src/entities.rs`
- API routes in `caliber-api/src/routes/agent.rs`, `message.rs`, `delegation.rs`, `handoff.rs`

## Current Architecture (v0.4.7)

The workspace has **7 crates**:
- `caliber-core` - Entity types, VAL traits, agent types
- `caliber-dsl` - Markdown+YAML DSL parser
- `caliber-pcp` - Validation, checkpoints, recovery
- `caliber-storage` - Storage traits, mock implementation
- `caliber-pg` - PostgreSQL extension (pgrx)
- `caliber-api` - REST/gRPC/WebSocket API server
- `caliber-test-utils` - Test generators and fixtures

## Deleted Crates (v0.4.6)

- `caliber-tui` (~4,500 LOC) - Terminal UI, replaced by TypeScript SDK
- `caliber-agents` (~1,654 LOC) - Absorbed into caliber-core
- `caliber-llm` (~1,145 LOC) - Absorbed into caliber-core/caliber-api
- `caliber-context` - Absorbed into caliber-core
- `caliber-events` - Absorbed into caliber-storage

See `docs/SPEC_CRATE_ABSORPTION_FINAL.md` for the full absorption specification.
