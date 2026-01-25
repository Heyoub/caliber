# Changelog

All notable changes to CALIBER will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- CI/CD pipelines for automated testing
- Continuous fuzzing integration (planned)
- Stripe payment integration (alternative to LemonSqueezy)
- Mutation testing (planned)
- Test coverage reporting (planned)

## [0.4.4] - 2026-01-24

### Changed

- **BREAKING:** Removed `EntityId` type alias in favor of 15 distinct typed IDs
  - `TenantId`, `TrajectoryId`, `ScopeId`, `ArtifactId`, `NoteId`, `TurnId`
  - `AgentId`, `EdgeId`, `LockId`, `MessageId`, `DelegationId`, `HandoffId`
  - `ApiKeyId`, `WebhookId`, `SummarizationPolicyId`
- Added `EntityIdType` trait providing `new()`, `as_uuid()`, `now_v7()`, `nil()` methods
- Updated all caliber-pg heap files (12 files) to use typed IDs with `.as_uuid()` for PostgreSQL operations
- Updated all caliber-api test files (10 files) to use `Uuid` or specific typed IDs
- Replaced `new_entity_id()` function with `TypedId::now_v7()` pattern

### Removed

- Deprecated `EntityId = Uuid` type alias from caliber-core
- Deprecated `new_entity_id()` function from caliber-core

### Notes

- This is a compile-time breaking change that improves type safety
- Tenant ID, trajectory ID, agent ID etc. can no longer be accidentally swapped at compile time
- Pattern: use `.as_uuid()` when interfacing with PostgreSQL/pgrx, `TypedId::new(uuid)` when extracting

## [0.4.3] - 2026-01-24

### Added

- Change journal API operations via `caliber-events` integration

### Changed

- Domain refactors: new status enums, generic CRUD/update paths, and filtering improvements
- Removed obsolete change journal migration and folded it into distributed correctness
- Removed `caliber-context` and `caliber-events` crates from the workspace

## [0.4.2] - 2026-01-22

### Added

- OpenAPI + Swagger UI features for API deployment
- Postgres extension control file and bootstrap SQL schema for `caliber_pg`
- `caliber_config_get` and `caliber_config_update` functions
- SSR support for the dashboard landing experience
- PG18-compatible `pgrx-tests` wiring via upstream develop branch

### Fixed

- PG18 pgrx test compilation failures (heap test imports, lock mode usage)
- Duplicate extension symbols during pg_test builds (gated pg_module_magic/_PG_init)
- PG18 list access mismatches in index ops (removed list_nth_* symbols; switched to pgrx List API)
- OpenAPI `/openapi.json` duplication when swagger-ui is enabled
- Rate limiting extraction requires ConnectInfo on the Axum server
- PCPConfig defaults now aligned with `caliber-pcp` schema
- WorkOS integration restored by replacing the broken crate with direct HTTP calls
- Vercel build configuration for the landing subdirectory
- pgrx SQL schema generation and output file handling
- `pgvector` extension installation for embedding columns

### Changed

- `pg_test` setup now runs migrations to keep migration path exercised
- Disabled default pgrx features/workspace defaults to avoid feature drift
- Disabled standard Rust test/doctest harness for the extension crate during pg_test builds
- async-graphql pinned to 7.0.0 with Axum 0.7 and Swagger UI 8.0 for compatibility
- Docker builds upgraded to Rust 1.85
- `pg18` feature moved from default to `pg_test` in pgrx-tests
- Workspace dependency constraints and cleanup for Cargo.toml self-references

### Notes

- Debugging journey: initially suspected linker/runtime symbol issues from the standard test harness, then traced PG18 build failures to pgrx list helpers removed from pg_sys. Final fix: disable lib test harness, update list access to pgrx List API, and tighten pg_test setup to run migrations.

## [0.4.1] - 2026-01-20

### Added

- WSL-specific setup notes in README for Linux filesystem and tooling
- Shared `AppState` for Axum routes with `FromRef` extractors
- DB-backed property tests for API flows (explicit DB configuration)

### Fixed

- Hardened heap row-to-domain conversions and added unit tests to prevent
  storage trait type mismatches across heap modules
- Auth config production-validation tests now isolate environment variables
- Smoke test network error handling now tolerates non-throwing fetch runtimes
- Axum router state mismatch caused by `/ws` route state requirements
- Missing tenant_id plumbing in heap property tests and moved-value assertions

### Changed

- Centralized API router state initialization (webhooks, GraphQL schema, billing state)
- Route modules now rely on shared app state instead of per-module routers
- Async handlers now return `ApiResult` for consistent error propagation
- JWT secret handling now type-safe with tighter auth config validation

## [0.4.0] - 2026-01-17

### BREAKING CHANGES

- **PostgreSQL 18+ Required**: Dropped support for PostgreSQL 13-17. CALIBER now requires PostgreSQL 18 or later.
- **API Router Signature**: `create_api_router()` now requires `&ApiConfig` parameter for CORS and rate limiting configuration.

### Added

- Initial public release preparation

### Added - Production Hardening

#### Tenant Auto-Provisioning (SSO)
- Automatic tenant creation on first SSO login
- Domain-based tenant association (user@acme.com → "Acme" tenant)
- Public email detection (gmail, outlook, etc. → personal tenant)
- First user becomes admin, subsequent users become members
- WorkOS organization ID mapping for enterprise SSO

#### Database Schema
- `caliber_tenant` table for multi-tenant management
- `caliber_tenant_member` table for user-tenant relationships
- `caliber_public_email_domain` table for public email detection
- `caliber_schema_version` table for migration tracking
- Built-in migration runner in `_PG_init()`

#### PostgreSQL Functions (pgrx)
- `caliber_is_public_email_domain(domain)` - Check if domain is public
- `caliber_tenant_create(name, domain, workos_org_id)` - Create tenant
- `caliber_tenant_get_by_domain(domain)` - Lookup by email domain
- `caliber_tenant_get_by_workos_org(org_id)` - Lookup by WorkOS org
- `caliber_tenant_member_upsert(...)` - Add/update member
- `caliber_tenant_member_count(tenant_id)` - Count members
- `caliber_tenant_get(tenant_id)` - Get tenant by ID

#### CORS Hardening
- Config-based CORS origins via `CALIBER_CORS_ORIGINS`
- Wildcard subdomain support (`*.caliber.run`)
- Development mode (empty = allow all) vs production mode (strict)
- Configurable credentials and preflight cache

#### Rate Limiting
- Per-IP rate limiting for unauthenticated requests (default: 100/min)
- Per-tenant rate limiting for authenticated requests (default: 1000/min)
- Configurable burst capacity (default: 10)
- `429 Too Many Requests` with `Retry-After` header
- Rate limit headers on all responses (`X-RateLimit-Limit`, etc.)

#### API Enhancements
- `TooManyRequests` error code (HTTP 429)
- `ApiConfig` struct for production settings
- Rate limiting middleware with `governor` crate

#### PGXN Publishing Support
- `META.json` for PGXN distribution metadata
- `Makefile` for build/install/package commands
- PostgreSQL 18+ version validation in Makefile

### Changed

- Docker image updated from PostgreSQL 16 to PostgreSQL 18
- SSO callback now performs tenant resolution and member upsert
- CORS layer now uses `build_cors_layer()` with config
- Updated `workos_auth.rs` for workos crate 0.8 trait-based API
  - `GetProfileAndToken` and `GetAuthorizationUrl` traits must be imported for method access
  - `GetProfileAndTokenParams` and `GetAuthorizationUrlParams` are direct structs (no builder)
  - `Profile.idp_id` changed from `Option<String>` to `String`
  - `Profile.raw_attributes` removed from struct

### Configuration

New environment variables:
```env
# CORS
CALIBER_CORS_ORIGINS=https://caliber.run,https://app.caliber.run
CALIBER_CORS_ALLOW_CREDENTIALS=false
CALIBER_CORS_MAX_AGE_SECS=86400

# Rate Limiting
CALIBER_RATE_LIMIT_ENABLED=true
CALIBER_RATE_LIMIT_UNAUTHENTICATED=100
CALIBER_RATE_LIMIT_AUTHENTICATED=1000
CALIBER_RATE_LIMIT_BURST=10
```

### Files Created

- `caliber-api/src/config.rs` - API configuration module
- `caliber-pg/META.json` - PGXN metadata
- `caliber-pg/Makefile` - PGXN build wrapper

### Files Modified

- `caliber-pg/sql/caliber_init.sql` - Added tenant tables and schema version
- `caliber-pg/src/lib.rs` - Added tenant functions and migration runner
- `caliber-pg/Cargo.toml` - Removed pg13-17 features
- `caliber-api/src/error.rs` - Added TooManyRequests
- `caliber-api/src/routes/mod.rs` - Updated CORS configuration
- `caliber-api/src/middleware.rs` - Added rate limiting
- `caliber-api/src/routes/sso.rs` - Added tenant auto-creation
- `caliber-api/src/db.rs` - Added tenant DB functions
- `caliber-api/Cargo.toml` - Added governor dependency
- `docker/Dockerfile.pg` - Updated to PostgreSQL 18
- `.env.example` - Added CORS and rate limit variables

### Migration Guide (0.3.x → 0.4.0)

**1. Update PostgreSQL to 18+**
```bash
# CALIBER no longer supports PostgreSQL 13-17
pg_upgrade --old-datadir=/var/lib/postgresql/17/data \
           --new-datadir=/var/lib/postgresql/18/data
```

**2. Update API Router Calls**
```rust
// Before (0.3.x)
let router = create_api_router(db, ws, pcp);

// After (0.4.0)
let api_config = ApiConfig::from_env();
let router = create_api_router(db, ws, pcp, &api_config);
```

**3. Set Environment Variables**
```env
# Required for production
CALIBER_CORS_ORIGINS=https://yourdomain.com
CALIBER_RATE_LIMIT_ENABLED=true
```

**4. Run Schema Migration**
```sql
-- The extension auto-runs migrations on load
-- Or manually: SELECT caliber_init();
```

## [0.3.2] - 2026-01-17

### Fixed - SDK Codegen Pipeline & Lint Cleanup

- All Biome lint warnings resolved (0 errors, 0 warnings)
- `.gitignore` path for `caliber-sdk/src/generated/` (was incorrect)
- Non-null assertion in `websocket.ts` (single lookup pattern)
- Cognitive complexity in `context.ts` formatters (extracted 10 helpers)

### Changed

- Biome config excludes `**/*.astro` and `**/*.svelte` (false positives)
- Disabled import sorting in Biome (not needed)
- Removed format check from `bun check` (types + lint only)
- Refactored `formatMarkdown` and `formatXml` (complexity 21/23 → ~5)

### Added

- `is:inline` directive to Astro scripts (explicit intent)
- `tsup.config.ts` for SDK bundling configuration
- `.gitignore` for `examples/convex-integration/`
- Type helpers for Convex → SDK type bridging
- `.claude/` to gitignore (user-specific settings)

### Repository Hygiene

- Tracked `*.proptest-regressions` files (valuable test seeds)
- Tracked `caliber-sdk/tsup.config.ts` (build config)
- Removed `.claude/settings.local.json` from tracking

## [0.3.1] - 2026-01-17

### Added - Testing Infrastructure

- Comprehensive test directory structure (5 test types)
- Unit tests with mocking and async support
- Property-based tests with fast-check (100+ random cases per property)
- Fuzz tests for parser robustness
- Chaos tests for resilience and failure scenarios
- Smoke tests for quick sanity checks
- Benchmark suite for performance tracking
- Test coverage goals and CI/CD integration plan

### Added - Test Files

- `tests/unit/example.test.ts` - Unit test examples (94 lines)
- `tests/property/trajectory.property.test.ts` - Property tests (163 lines)
- `tests/fuzz/parser.fuzz.test.ts` - Fuzz tests (217 lines)
- `tests/chaos/resilience.chaos.test.ts` - Chaos tests (334 lines)
- `tests/smoke/api.test.ts` - Smoke tests (188 lines)
- `caliber-sdk/bench/index.ts` - Benchmark suite (160 lines)

### Added - Test Scripts

- `test:coverage` - Run tests with coverage reporting
- `lint` - Lint code with Biome
- `lint:fix` - Auto-fix linting issues
- `format` - Format code with Biome
- `bench` - Run performance benchmarks
- `bench:ci` - Run benchmarks with JSON output for CI

### Testing Philosophy

- **Unit tests** - Fast, isolated, run on every commit
- **Property tests** - Verify invariants, catch edge cases
- **Fuzz tests** - Find crashes, run periodically
- **Chaos tests** - Verify resilience, run before release
- **Smoke tests** - Quick sanity, run first

### Test Coverage

- Unit tests: Isolated function testing with mocks
- Property tests: 100+ random test cases per property
- Fuzz tests: Random/malformed input generation
- Chaos tests: Network failures, timeouts, rate limiting, retries
- Smoke tests: Quick sanity checks (< 10 seconds)
- Benchmarks: Performance tracking with 10,000+ iterations

## [0.3.0] - 2026-01-17

### Added - Managed Service Infrastructure

- WorkOS SSO integration with OAuth callback flow
- JWT-based authentication with Svelte stores
- Dashboard layout with sidebar navigation and mobile menu
- User profile management and API key generation
- LemonSqueezy payments integration (checkout, portal, webhooks)
- Billing status tracking and subscription management
- Overview dashboard with stats and quick actions
- Trajectory list page with pagination
- Settings page with API key management

### Added - Convex Integration

- CORS middleware for cross-origin requests (tower_http)
- WebSocket client with auto-reconnection and event subscriptions
- Context assembly helper for LLM prompts (XML/Markdown/JSON formats)
- Batch operations manager for bulk creates/deletes
- Complete Convex integration example with 17 actions
- Support for all 35+ WebSocket event types
- Relevance filtering via semantic search
- Token budget awareness in context assembly

### Added - SDK Enhancements

- `caliber-sdk/src/websocket.ts` - WebSocket client (350 lines)
- `caliber-sdk/src/context.ts` - Context assembly (400 lines)
- `caliber-sdk/src/managers/batch.ts` - Batch operations (250 lines)
- Subpath exports for websocket and context modules
- assembleContext(), formatContext(), getFormattedContext() methods

### Added - Development Tooling

- Bun workspace configuration for all TypeScript packages
- Global typecheck command across all packages
- npm compatibility for publishing to registry
- Updated documentation with bun/npm/pnpm installation options

### Changed

- Migrated all TypeScript packages to bun for development
- Updated landing page to use bun scripts internally
- Enhanced caliber-sdk with batch operations and context assembly
- Extended AuthContext with profile fields (email, first_name, last_name)
- Modified SSO route to support web client redirects (302 with token)

### Files Created

- `landing/src/stores/auth.ts` - Auth state management
- `landing/src/lib/api.ts` - Authenticated API client
- `landing/src/pages/login.astro` - Login page
- `landing/src/pages/auth/callback.astro` - OAuth callback
- `landing/src/layouts/DashboardLayout.astro` - Dashboard layout
- `landing/src/components/svelte/UserMenu.svelte` - User menu
- `landing/src/components/svelte/TrajectoryList.svelte` - Trajectory table
- `landing/src/components/svelte/PricingCTA.svelte` - Checkout button
- `landing/src/pages/dashboard/` - Dashboard pages (index, trajectories, settings)
- `caliber-api/src/routes/user.rs` - User management
- `caliber-api/src/routes/billing.rs` - Billing integration
- `examples/convex-integration/` - Complete Convex example
- `package.json` (root) - Workspace configuration

### Configuration

- Added WorkOS environment variables (client ID, API key, redirect URI)
- Added LemonSqueezy environment variables (store ID, API key, webhook secret)
- Updated railway.toml with workos feature flag
- Added packageManager: bun@1.1.0 to all TypeScript packages

### Database Schema

- caliber_users table for user management
- caliber_billing table for subscription tracking

### Documentation

- Updated README files with bun commands
- Added Convex integration documentation
- Created deployment checklist
- Added testing instructions for managed service

## [0.2.2] - 2026-01-17

### Added

- Professional repository documentation suite
- Custom GitHub issue templates (bug report, feature request, performance issue)
- Custom GitHub PR template with CALIBER-specific verification checklist
- Dependabot configuration with CALIBER-specific dependency groups
- Working examples directory with basic_trajectory.rs (400+ lines)
- BENCHMARKS.md with real performance data and comparisons
- CONTRIBUTING.md with development workflow and verification gates
- SECURITY.md with vulnerability reporting and security concerns
- CODE_OF_CONDUCT.md with community guidelines
- SUPPORT.md with help resources and response times
- Comprehensive .gitignore for Rust, Node, Python, OS files
- Repository quality checklist and assessment (Grade: A+)

### Changed

- Updated README.md with accurate project structure (12 crates)
- Enhanced Kiro steering documentation with verification gates
- Added multi-phase verification workflow to dev-philosophy.md
- Updated tech.md with code quality standards and framework integration guidelines

### Documentation

- Created verification-gates.md documenting the clippy failure incident
- Added AI code smell detection patterns
- Documented the Five Verification Gates (Build → Clippy → Tests → Integration → Production)
- Added 9 planned examples (1 complete, 8 planned)
- Real performance benchmarks vs alternatives (ORM, Redis, Pinecone)

### Testing

- Comprehensive fuzz testing validation (462,947 adversarial inputs, 0 crashes)
- lexer_fuzz: 119,847 runs in 61s (1,965 runs/sec)
- parser_fuzz: 343,100 runs in 62s (5,534 runs/sec)
- Dictionary accumulated 138 entries for future fuzzing
- Validated DSL robustness against malformed UTF-8, partial keywords, invalid characters

### Lessons Learned

- Build success ≠ Complete (must run clippy before marking done)
- Clippy catches 90% of issues that "successful build" misses
- Security fixes need 100% coverage verification (grep all locations)
- Framework version mismatches require explicit verification
- Fuzz testing validates property test coverage (no crashes = comprehensive tests)

## [0.2.1] - 2026-01-17

### Added

- caliber-tui: Terminal user interface with SynthBrute aesthetic
- Property-based testing across all crates (165 tests total)
- Direct heap operations for all entity types (no SQL in hot path)
- Production hardening: SQL persistence, access control, strict validation
- Landing page with marketing site
- Comprehensive steering documentation for AI-native development

### Changed

- Migrated from in-memory storage to SQL persistence
- Improved advisory lock semantics (session + transaction level)
- Enhanced error handling (no silent failures)
- Removed all hard-coded defaults (framework philosophy)

### Fixed

- WSL file sync issues with incremental compilation
- Lock timeout handling
- Unicode-safe string truncation
- RwLock poisoning recovery

### Security

- Added access control enforcement for memory regions
- Implemented tenant isolation for multi-tenant deployments
- Feature-gated debug functions
- Added security audit tooling

## [0.2.0] - 2026-01-15

### Added

- caliber-api: Full REST/gRPC/WebSocket API
- 14 route modules with comprehensive endpoints
- OpenAPI documentation generation
- Telemetry with OpenTelemetry and Prometheus
- WebSocket event broadcasting with tenant isolation
- Authentication middleware (JWT + API key)

### Changed

- Improved context assembly performance
- Enhanced DSL parser error messages
- Better property test coverage

## [0.1.0] - 2026-01-13

### Added

- Initial implementation of 8 core crates:
  - caliber-core: Entity types and configuration
  - caliber-storage: Storage trait and mock implementation
  - caliber-context: Context assembly logic
  - caliber-pcp: Validation, checkpoints, recovery
  - caliber-llm: VAL (Vector Abstraction Layer)
  - caliber-agents: Multi-agent coordination
  - caliber-dsl: Custom DSL parser
  - caliber-pg: pgrx PostgreSQL extension
- Property-based testing framework
- Fuzz testing for lexer and parser
- Comprehensive documentation suite
- Bootstrap SQL schema
- Multi-agent coordination primitives (locks, messages, delegation, handoffs)

### Design Decisions

- ECS (Entity-Component-System) architecture
- No hard-coded defaults (framework, not product)
- Direct heap operations for performance
- Dynamic embedding dimensions (provider-agnostic)
- UUIDv7 for timestamp-sortable IDs

## [0.0.1] - 2026-01-12

### Added

- Project initialization
- Workspace structure
- Documentation framework
- Development philosophy documents

---

## Version History Summary

- **0.2.x**: Production-ready with API, TUI, and hardening
- **0.1.x**: Core implementation with all 8 crates
- **0.0.x**: Project setup and planning

## Upgrade Guides

### 0.1.x → 0.2.x

**Breaking Changes:**

- `CaliberConfig` no longer has `Default` impl - all fields must be explicitly provided
- DSL parser now rejects unknown fields instead of ignoring them
- Storage operations return explicit errors instead of silent failures

**Migration Steps:**

1. Update config construction:

```rust
// Before (0.1.x)
let config = CaliberConfig::default();

// After (0.2.x)
let config = CaliberConfig {
    token_budget: 8000,
    checkpoint_retention: 5,
    // ... all fields required
};
```

1. Handle new error types:

```rust
// Before (0.1.x)
let result = operation(); // might silently fail

// After (0.2.x)
let result = operation()?; // explicit error handling
```

1. Update DSL files:

```dsl
// Before (0.1.x) - unknown fields ignored
memory my_memory {
    type: episodic
    typo_field: value  // silently ignored
}

// After (0.2.x) - unknown fields rejected
memory my_memory {
    type: episodic
    // typo_field: value  // ParseError!
}
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.

## Security

See [SECURITY.md](SECURITY.md) for security policy and vulnerability reporting.

## License

AGPL-3.0 - See [LICENSE](LICENSE) for details.
