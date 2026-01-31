# CALIBER + PCP

> **Context Abstraction Layer Integrating Behavioral Extensible Runtime**  
> **+ Persistent Context Protocol**

A Postgres-native memory framework for AI agents, built as a multi-crate Rust workspace using pgrx.

**Version:** 0.5.0  
**Status:** Production-ready  
**Architecture:** Multi-crate ECS (Entity-Component-System)  
**Language:** Rust + TypeScript

---

## 🚀 Quick Start

### Prerequisites

- **Rust** 1.75+ ([install via rustup](https://rustup.rs/))
- **PostgreSQL** 18+ (required - this is a Postgres-native framework)
- **Bun** (for frontend development)
- **Cargo** (comes with Rust)

### 1. Clone & Install

```bash
git clone https://github.com/caliber-run/caliber.git
cd caliber

# Install Rust dependencies
cargo build --workspace

# Install JavaScript dependencies
bun install
```

### 2. Setup PostgreSQL

```bash
# Install pgrx CLI (first time only)
cargo install cargo-pgrx

# Initialize pgrx (downloads and configures PostgreSQL)
cargo pgrx init

# Install the caliber_pg extension
cargo pgrx install --package caliber-pg

# Or use your existing PostgreSQL 18+ instance
psql -c "CREATE EXTENSION vector;"
psql -c "CREATE EXTENSION caliber_pg;"
psql -c "SELECT caliber_init();"
```

### 3. Configure Environment

```bash
# Copy example environment file
cp .env.example .env

# Edit with your settings
# Required variables:
# - DATABASE_URL=postgresql://user:pass@localhost/caliber
# - JWT_SECRET=your-secret-key-here
```

### 4. Start Development Servers

```bash
# Terminal 1: Start API server
make dev
# Or: cargo run -p caliber-api

# Terminal 2: Start Pack Editor (frontend)
bun run dev:app

# Terminal 3 (optional): Start landing page
bun run dev:landing
```

### 5. Verify

- **API:** http://localhost:3000
- **Pack Editor:** http://localhost:5173
- **Landing:** http://localhost:4321

---

## 🏗️ Architecture

### 7 Rust Crates

| Crate | Description |
|-------|-------------|
| **caliber-core** | Entities, context assembly, agent coordination, VAL traits |
| **caliber-storage** | EventDag, cache invalidation, hybrid LMDB+cold storage |
| **caliber-pcp** | Validation, checkpoints, harm reduction |
| **caliber-dsl** | Markdown+YAML configuration parser |
| **caliber-pg** | PostgreSQL extension (direct pgrx heap access) |
| **caliber-api** | REST/gRPC/WebSocket server with multi-tenant isolation |
| **caliber-test-utils** | Test fixtures, generators, property test helpers |

### Frontend (TypeScript/Bun)

| Package | Description |
|---------|-------------|
| **caliber-sdk** | TypeScript client with WebSocket streaming |
| **app/** | SvelteKit Pack Editor (45+ Svelte 5 components) |
| **packages/ui/** | Svelte 5 component library |
| **landing/** | Astro marketing site |

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                      CALIBER + PCP                              │
├─────────────────────────────────────────────────────────────────┤
│  CaliberConfig (user-provided, no defaults)                     │
│                              │                                  │
│  PCP Protocol Layer (validation, checkpoints, harm reduction)   │
│                              │                                  │
│  CALIBER Core (ECS)                                             │
│  Entities + Context + Agents + VAL                              │
│                              │                                  │
│  Storage Layer (EventDag + Cache)                               │
│                              │                                  │
│  PostgreSQL Extension (pgrx direct heap access)                 │
│                              │                                  │
│  PostgreSQL 18+ Storage Engine                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 🔑 Key Features

| Feature | Description |
|---------|-------------|
| **Postgres-Native** | Direct pgrx heap access, no SQL in hot path |
| **Hierarchical Memory** | Trajectory → Scope → Artifact → Note |
| **Multi-Agent Coordination** | Locks, messages, delegation, handoffs |
| **Vector Abstraction Layer** | Provider-agnostic embeddings, any dimension |
| **Markdown Configuration** | YAML-based config in Markdown fences |
| **Multi-Tenant Isolation** | Row-level security, tenant-scoped JWTs |
| **Horizontal Scaling** | Event DAG-based cache coordination |
| **Zero Defaults** | All configuration explicit — framework, not product |

---

## 📁 Project Structure

```
caliber/
├── caliber-core/        # Entities, context, agents, VAL (consolidated)
├── caliber-storage/     # EventDag, cache invalidation
├── caliber-pcp/         # Validation, checkpoints
├── caliber-dsl/         # Markdown+YAML parser
├── caliber-pg/          # PostgreSQL extension (pgrx)
├── caliber-api/         # REST/gRPC/WebSocket server
├── caliber-test-utils/  # Test fixtures and generators
├── caliber-sdk/         # TypeScript SDK
├── app/                 # SvelteKit Pack Editor
├── packages/ui/         # Svelte 5 component library
├── landing/             # Astro marketing site
├── examples/            # Example programs
├── docs/                # Specification documents
├── tests/               # Integration and E2E tests
├── scripts/             # Build and test scripts
├── docker/              # Docker configs
├── .github/             # CI/CD workflows
├── Cargo.toml           # Workspace manifest
├── package.json         # Bun workspace
└── README.md            # This file
```

---

## 🧪 Development

### Build Commands

```bash
# Build all Rust crates (fast - excludes heavy caliber-pg)
cargo build --workspace --exclude caliber-pg

# Build everything including caliber-pg (slower)
cargo build --workspace

# Build release binary
cargo build --release -p caliber-api

# Build frontend
bun run build:app
```

### Test Commands

```bash
# Run all Rust tests (fast)
cargo test --workspace --exclude caliber-pg

# Run all tests including pgrx
./scripts/test.sh

# Run frontend tests
bun test

# Run E2E tests
bun run test:e2e

# Run property tests
cargo test --workspace -- prop_

# Run with coverage
make coverage
```

### Dev Server Commands

```bash
# Start API server
make dev
cargo run -p caliber-api

# Start API with auto-reload
make dev-watch

# Start Pack Editor
bun run dev:app

# Start all frontend servers
bun run dev:all

# Start landing page
bun run dev:landing
```

### Linting & Formatting

```bash
# Run all linters
make lint

# Fix linting issues
make lint-fix

# Format code
cargo fmt --all
bunx biome format --write .
```

---

## 🧪 Testing

### Test Matrix

| Test Type | Command | Description |
|-----------|---------|-------------|
| Unit | `cargo test --workspace` | Fast unit tests |
| Property | `cargo test -- prop_` | Property-based tests (100+ iterations) |
| Integration | `make test-integration` | DB-backed integration tests |
| E2E | `bun run test:e2e` | End-to-end Playwright tests |
| Fuzz | `make test-fuzz` | Fuzz testing |
| Smoke | `make test-smoke` | Quick sanity checks |
| Load | `make test-load` | k6 load tests |
| Security | `make test-security` | OWASP security tests |

### Test Coverage

- **Rust:** 150+ unit tests, 15+ property tests
- **TypeScript:** 80% coverage threshold (Vitest)
- **Fuzz:** 462,947 inputs tested, 0 crashes
- **E2E:** Playwright tests for critical paths

Run full test suite:
```bash
make test-all
```

---

## 📚 Documentation

| Document | Description |
|----------|-------------|
| [CALIBER_PCP_SPEC.md](docs/CALIBER_PCP_SPEC.md) | Core specification |
| [CALIBER_API_REFERENCE.md](docs/CALIBER_API_REFERENCE.md) | API documentation |
| [QUICK_REFERENCE.md](docs/QUICK_REFERENCE.md) | Quick reference guide |
| [DEPENDENCY_GRAPH.md](docs/DEPENDENCY_GRAPH.md) | Type system reference |
| [OPERATIONS_CHECKLIST.md](docs/OPERATIONS_CHECKLIST.md) | Production readiness |
| [BENCHMARKS.md](BENCHMARKS.md) | Performance benchmarks |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contribution guidelines |
| [SECURITY.md](SECURITY.md) | Security policy |
| [DEVLOG.md](DEVLOG.md) | Development timeline |
| [AGENTS.md](AGENTS.md) | Agent development guide |

---

## 🎯 Usage Example

```rust
use caliber_core::{
    ContextAssembler,
    ContextPackage,
    CaliberConfig,
    CaliberResult,
    ContextPersistence,
    RetryConfig,
    SectionPriorities,
    ValidationMode,
};
use std::time::Duration;
use uuid::Uuid;

fn main() -> CaliberResult<()> {
    // Configuration is REQUIRED — no defaults
    let config = CaliberConfig {
        token_budget: 8000,
        checkpoint_retention: 5,
        stale_threshold: Duration::from_secs(86400 * 30),
        contradiction_threshold: 0.85,
        context_window_persistence: ContextPersistence::Ttl(Duration::from_secs(86400)),
        validation_mode: ValidationMode::OnMutation,
        section_priorities: SectionPriorities {
            user: 100,
            system: 90,
            artifacts: 80,
            notes: 70,
            history: 60,
            custom: vec![],
        },
        embedding_provider: None,
        summarization_provider: None,
        llm_retry_config: RetryConfig {
            max_retries: 3,
            initial_backoff: Duration::from_millis(200),
            max_backoff: Duration::from_secs(2),
            backoff_multiplier: 2.0,
        },
        lock_timeout: Duration::from_secs(30),
        message_retention: Duration::from_secs(86400),
        delegation_timeout: Duration::from_secs(3600),
    };

    let assembler = ContextAssembler::new(config)?;
    let trajectory_id = Uuid::now_v7();
    let scope_id = Uuid::now_v7();
    let pkg = ContextPackage::new(trajectory_id, scope_id)
        .with_user_input("Summarize the last scope.".to_string());

    let window = assembler.assemble(pkg)?;
    println!("Assembled {} sections", window.sections.len());
    Ok(())
}
```

See [examples/](examples/) for more usage patterns.

---

## 🚀 Production Deployment

### Cache Invalidation Strategies

**Single-Instance (Default):**
- Uses `InMemoryChangeJournal`
- Zero external dependencies
- Microsecond latency
- Recommended for most deployments

**Multi-Instance (Horizontal Scaling):**
- Uses `EventDagChangeJournal`
- Event DAG-based coordination
- ~100ms invalidation propagation
- No Redis/LISTEN/NOTIFY required

See `caliber-storage/src/cache/watermark.rs` for configuration.

### Environment Variables

```bash
# Database
DATABASE_URL=postgresql://user:pass@localhost/caliber

# Authentication
JWT_SECRET=your-secret-key-here
WORKOS_API_KEY=your-workos-key
WORKOS_CLIENT_ID=your-client-id

# API Configuration
CALIBER_API_BASE_URL=https://api.caliber.run
CALIBER_CORS_ORIGINS=https://caliber.run,https://app.caliber.run

# Rate Limiting
CALIBER_RATE_LIMIT_ENABLED=true
CALIBER_RATE_LIMIT_UNAUTHENTICATED=100
CALIBER_RATE_LIMIT_AUTHENTICATED=1000

# Multi-Tenant
CALIBER_REQUIRE_TENANT_HEADER=true
```

### Docker Deployment

```bash
# Build images
docker-compose -f docker/docker-compose.yml build

# Start services
docker-compose -f docker/docker-compose.yml up -d

# View logs
docker-compose -f docker/docker-compose.yml logs -f
```

### Kubernetes Deployment

```bash
# Install with Helm
helm install caliber ./charts/caliber \
  --set postgresql.enabled=true \
  --set api.replicas=3

# Upgrade
helm upgrade caliber ./charts/caliber
```

---

## 🛡️ Security & Compliance

- **SBOM Generation** - SPDX JSON artifacts
- **CodeQL Analysis** - JavaScript/TypeScript scanning
- **Semgrep** - Multi-language security scanning
- **Gitleaks** - Secret scanning
- **OSV Scanner** - Vulnerability scanning
- **OpenSSF Scorecard** - Security best practices
- **SLSA Provenance** - Build attestations

See [SECURITY.md](SECURITY.md) for vulnerability reporting.

---

## 🤖 Agent Development

See [AGENTS.md](AGENTS.md) for:
- Repository structure
- CI/CD workflow
- Test patterns
- Architecture patterns

---

## 🔧 Troubleshooting

### Build Issues

**Problem:** `cargo build` fails with PostgreSQL errors

**Solution:** Build without caliber-pg for faster iteration:
```bash
cargo build --workspace --exclude caliber-pg
```

**Problem:** Cross-device link errors during tests

**Solution:** Set TMPDIR:
```bash
TMPDIR=$PWD/target/tmp cargo test --workspace
```

### Database Issues

**Problem:** Extension not found

**Solution:** Install the extension:
```bash
cargo pgrx install --package caliber-pg
psql -c "CREATE EXTENSION caliber_pg;"
```

**Problem:** Schema not initialized

**Solution:** Run initialization:
```bash
psql -c "SELECT caliber_init();"
```

### Frontend Issues

**Problem:** Port already in use

**Solution:** Kill the process or use a different port:
```bash
lsof -ti:5173 | xargs kill -9
# Or set PORT environment variable
PORT=5174 bun run dev:app
```

---

## 📊 Performance

- **Latency:** < 10ms p95 for context assembly
- **Throughput:** 1000+ req/s per instance
- **Memory:** ~200MB baseline per API instance
- **Storage:** Direct pgrx heap access (no SQL parsing overhead)

See [BENCHMARKS.md](BENCHMARKS.md) for detailed performance analysis.

---

## 🤝 Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for:
- Code of conduct
- Development workflow
- Coding standards
- Pull request process

---

## 📄 License

AGPL-3.0-or-later

---

## 🙏 Acknowledgments

Built with:
- [pgrx](https://github.com/pgcentralfoundation/pgrx) - PostgreSQL extension framework
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [SvelteKit](https://kit.svelte.dev/) - Frontend framework
- [Astro](https://astro.build/) - Static site generator

---

## 📞 Support

- **Documentation:** [docs/](docs/)
- **Issues:** [GitHub Issues](https://github.com/caliber-run/caliber/issues)
- **Discussions:** [GitHub Discussions](https://github.com/caliber-run/caliber/discussions)
- **Security:** See [SECURITY.md](SECURITY.md)

---

**Built with ❤️ for the AI agent community**
