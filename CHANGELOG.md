# Changelog

All notable changes to CALIBER will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial public release preparation
- CI/CD pipelines for automated testing
- Continuous fuzzing integration (planned)
- Stripe payment integration (alternative to LemonSqueezy)

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
