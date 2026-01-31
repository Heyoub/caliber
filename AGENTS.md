# Agent Guide (CALIBER)

This repo is designed for agent-driven iteration with CI as the source of truth.

## Ground Rules
- Prefer `rg` for search and keep context small.
- Avoid running full `cargo test` locally unless explicitly requested.
- Use bacon for local compilation checks - never run cargo when bacon is available.
- Use CI logs for failures; local environments may not match CI.
- Keep changes minimal and surgical; no placeholder TODOs in production code.
- Production code must be complete and correct - no placeholders, no stubs, no half-implementations.

## CI + Logs
- Trigger CI manually via `make ci-cloud` (downloads logs into `.github/.logs/`).
- CI artifacts and logs are the canonical build signal.

## Repo Structure (7 Rust Crates)
- **Core:** `caliber-core/` (entities, context assembly, agent coordination, VAL traits)
- **Storage:** `caliber-storage/` (EventDag, cache invalidation, hybrid LMDB+cold storage)
- **PCP:** `caliber-pcp/` (validation, checkpoints, harm reduction)
- **DSL:** `caliber-dsl/` (Markdown+YAML config parser)
- **PostgreSQL:** `caliber-pg/` (pgrx extension for direct heap access)
- **API:** `caliber-api/` (REST/gRPC/WebSocket server with multi-tenant isolation)
- **Test Utils:** `caliber-test-utils/` (fixtures, generators, property test helpers)

## Frontend (TypeScript/Bun)
- **SDK:** `caliber-sdk/` (TypeScript client with WebSocket streaming)
- **Pack Editor:** `app/` (SvelteKit app with 45+ Svelte 5 components)
- **UI Library:** `packages/ui/` (Svelte 5 component library)
- **Landing:** `landing/` (Astro marketing site)

## Key Architecture Patterns

### Crate Consolidation (v0.4.x → v0.5.0)
Originally 8 crates (core, llm, agents, context, storage, pcp, dsl, pg).
Consolidated to 7 by absorbing llm/agents/context into caliber-core.
**Why:** Simpler dependency graph, faster builds, clearer ownership.

### Event System (caliber-core/src/event.rs)
- Events have 64-byte aligned headers + arbitrary payload
- Hash chains for tamper-evidence (Blake3 by default)
- EventDag trait for storage abstraction
- Event kinds are u16 constants organized by category (0x1xxx = trajectory, 0x2xxx = scope, etc.)

### Cache Invalidation (caliber-storage/src/cache/)
- Two strategies: InMemoryChangeJournal (single-instance) and EventDagChangeJournal (multi-instance)
- Watermark-based freshness tracking
- EventDagChangeJournal uses event log as shared invalidation journal
- No external dependencies (no Redis, no LISTEN/NOTIFY)

### Storage Layer (caliber-storage/)
- InMemoryEventDag for testing
- HybridDag with LMDB hot cache + cold storage fallback
- All storage operations use Effect<T> for algebraic error handling

### DSL Evolution (v0.4.6+)
- Replaced 3,762-line custom lexer/parser with Markdown+YAML (~100 lines)
- Config files are now Markdown with YAML fenced blocks
- Standard tooling (serde_yaml), IDE support, better error messages

### UI Evolution (v0.5.0)
- Removed caliber-tui (4,500 lines of terminal UI)
- Added SvelteKit Pack Editor (web UI with 45+ Svelte 5 components)
- Modern component patterns with Svelte 5 runes

## Secrets
- `.env` is local-only and ignored by Git.
- Never commit real secrets; use `.env.example` for templates.

## Release
- Tags follow `vX.Y.Z` and trigger `.github/workflows/release.yml`.
- SLSA provenance is attached to release artifacts and images.
