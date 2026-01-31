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

## Repo Structure
- API: `caliber-api/` (REST/gRPC/WebSocket server)
- Core primitives: `caliber-core/` (entities, events, effects)
- Storage: `caliber-storage/` (EventDag, cache, hybrid LMDB+cold storage)
- PostgreSQL extension: `caliber-pg/` (pgrx-based extension)
- DSL: `caliber-dsl/` (parser for .caliber config files)
- Test utils: `caliber-test-utils/` (fixtures, generators)
- Landing site: `landing/` (Astro + Svelte marketing site)

## Key Architecture Patterns

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

## Secrets
- `.env` is local-only and ignored by Git.
- Never commit real secrets; use `.env.example` for templates.

## Release
- Tags follow `vX.Y.Z` and trigger `.github/workflows/release.yml`.
- SLSA provenance is attached to release artifacts and images.
