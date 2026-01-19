# CALIBER + PCP

> **Context Abstraction Layer Integrating Behavioral Extensible Runtime**  
> **+ Persistent Context Protocol**

A Postgres-native memory framework for AI agents, built as a multi-crate Rust workspace using pgrx.

**Version:** 0.4.0  
**Architecture:** Multi-crate ECS (Entity-Component-System)  
**Language:** Rust (pgrx)

---

## 🚀 Quick Start

### Prerequisites

- **Rust** 1.75+ (install via [rustup](https://rustup.rs/))
- **PostgreSQL** 18+ (for pgrx extension, optional for core development)
- **Cargo** (comes with Rust)

### WSL Notes (Windows)

- Clone and run the repo from the Linux filesystem (e.g. `/home/<user>/projects/...`), not `/mnt/c`, for reliable file watching and performance.
- Install build tooling and SSL headers if missing: `build-essential`, `pkg-config`, `libssl-dev` (and `clang` if you use crates that require it).
- If file watching is flaky, increase inotify limits (e.g. `fs.inotify.max_user_watches`).

### Build & Test (Without PostgreSQL)

```bash
# Clone the repository (replace with your repo URL)
git clone https://github.com/caliber-run/caliber.git
cd caliber

# Build all crates (excluding pgrx extension)
cargo build --workspace --exclude caliber-pg

# Run all tests (165 tests)
cargo test --workspace --exclude caliber-pg

# Run with verbose output
cargo test --workspace --exclude caliber-pg -- --nocapture

# Run clippy lints
cargo clippy --workspace --exclude caliber-pg -- -D warnings

# (Optional) Install JS deps for SDK/landing work
bun install
```

### Build with PostgreSQL (Full Extension)

```bash
# Install pgrx CLI
cargo install cargo-pgrx

# Initialize pgrx (downloads and configures PostgreSQL)
cargo pgrx init

# Build the extension
cargo build -p caliber-pg --release

# Package for deployment
cargo pgrx package -p caliber-pg

# Run pgrx tests
cargo pgrx test -p caliber-pg
```

Note: `pgrx-tests` is currently blocked by upstream PG18 incompatibility.
Use the ops checklist for the fallback test lane.

### Hello World (Postgres, low-level API)

```bash
psql -c "CREATE EXTENSION caliber;"
psql -c "SELECT caliber_init();"
psql -c "SELECT caliber_trajectory_get(caliber_trajectory_create('hello-world', NULL, NULL));"
psql -c "WITH t AS (SELECT caliber_trajectory_create('hello-world', NULL, NULL) AS id) SELECT caliber_scope_create(t.id, 'scope-1', NULL, 800) FROM t;"
psql -c "WITH t AS (SELECT caliber_trajectory_create('hello-world', NULL, NULL) AS id) SELECT caliber_scope_get_current(t.id) FROM t;"
```

Config is required for runtime operations; see `docs/QUICK_REFERENCE.md` for the full JSON shape.

---

## 📁 Project Structure

```
caliber/
├── caliber-core/        # Entity types (data only, no behavior)
├── caliber-storage/     # Storage trait + mock implementation
├── caliber-context/     # Context assembly logic
├── caliber-pcp/         # Validation, checkpoints, recovery
├── caliber-llm/         # VAL (Vector Abstraction Layer)
├── caliber-agents/      # Multi-agent coordination
├── caliber-dsl/         # DSL parser → CaliberConfig
├── caliber-pg/          # pgrx extension (requires PostgreSQL)
├── caliber-api/         # REST/gRPC/WebSocket API server
├── caliber-tui/         # Terminal user interface
├── caliber-test-utils/  # Test generators, fixtures, assertions
├── caliber-sdk/         # TypeScript SDK for REST/WebSocket APIs
├── examples/            # Example programs and usage patterns
├── docs/                # Specification documents
├── fuzz/                # Fuzz testing targets (requires nightly)
├── docker/              # Docker configs and compose files
├── charts/              # Helm charts for Kubernetes
├── terraform/           # Infrastructure as Code (AWS, Azure, GCP)
├── landing/             # Marketing website (Astro + Svelte)
├── .github/             # CI/CD workflows and issue templates
├── Cargo.toml           # Workspace manifest
├── DEVLOG.md            # Development timeline
├── BENCHMARKS.md        # Performance benchmarks and comparisons
├── CONTRIBUTING.md      # Contribution guidelines
├── SECURITY.md          # Security policy and vulnerability reporting
└── README.md            # This file
```

---

## 🏗️ Architecture

CALIBER uses ECS (Entity-Component-System) architecture:

- **Entities** (caliber-core): Pure data structures — Trajectory, Scope, Artifact, Note, Turn
- **Components** (caliber-*): Behavior via traits — storage, context, validation, LLM
- **System** (caliber-pg): Wires everything together in PostgreSQL

```
┌─────────────────────────────────────────────────────────────────┐
│                      CALIBER + PCP                              │
├─────────────────────────────────────────────────────────────────┤
│  CaliberConfig (user-provided, no defaults)                     │
│                              │                                  │
│  PCP Protocol Layer (validation, checkpoints, harm reduction)   │
│                              │                                  │
│  CALIBER Components (ECS)                                       │
│  caliber-core │ caliber-storage │ caliber-context               │
│  caliber-pcp  │ caliber-llm     │ caliber-agents                │
│                              │                                  │
│  pgrx Direct Storage (heap ops, no SQL in hot path)             │
│                              │                                  │
│  PostgreSQL Storage Engine                                      │
└─────────────────────────────────────────────────────────────────┘
```

---

## 🔑 Key Features

| Feature | Description |
|---------|-------------|
| **Hierarchical Memory** | Trajectory → Scope → Artifact → Note |
| **No SQL in Hot Path** | Direct pgrx heap operations for performance |
| **VAL (Vector Abstraction)** | Provider-agnostic embeddings, any dimension |
| **Multi-Agent Support** | Locks, messages, delegation, handoffs |
| **Custom DSL** | Declarative configuration language |
| **PCP Harm Reduction** | Validation, checkpoints, contradiction detection |
| **Zero Defaults** | All configuration explicit — framework, not product |

---

## 📊 Test Coverage

| Crate | Unit Tests | Property Tests | Total |
|-------|------------|----------------|-------|
| caliber-core | 7 | 10 | 17 |
| caliber-dsl | 21 | 10 | 31 |
| caliber-llm | 16 | 7 | 23 |
| caliber-context | 10 | 9 | 19 |
| caliber-pcp | 16 | 5 | 21 |
| caliber-agents | 16 | 6 | 22 |
| caliber-storage | 12 | 5 | 17 |
| caliber-test-utils | 10 | 5 | 15 |
| **Total** | **108** | **57** | **165** |

---

## 📚 Documentation

| Document | Description |
|----------|-------------|
| [CALIBER_PCP_SPEC.md](docs/CALIBER_PCP_SPEC.md) | Core specification |
| [DSL_PARSER.md](docs/DSL_PARSER.md) | Lexer, parser, AST |
| [LLM_SERVICES.md](docs/LLM_SERVICES.md) | VAL + summarization |
| [MULTI_AGENT_COORDINATION.md](docs/MULTI_AGENT_COORDINATION.md) | Agent coordination |
| [DEPENDENCY_GRAPH.md](docs/DEPENDENCY_GRAPH.md) | Type system reference |
| [QUICK_REFERENCE.md](docs/QUICK_REFERENCE.md) | Cheat sheet |
| [CONFIG_PRESETS.md](docs/CONFIG_PRESETS.md) | Preset-first config philosophy + hard-value audit |
| [BENCHMARKS.md](BENCHMARKS.md) | Performance benchmarks and comparisons |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contribution guidelines and workflow |
| [SECURITY.md](SECURITY.md) | Security policy and vulnerability reporting |
| [OPERATIONS_CHECKLIST.md](docs/OPERATIONS_CHECKLIST.md) | Production readiness checklist |
| [DEVLOG.md](DEVLOG.md) | Development timeline |
| [examples/README.md](examples/README.md) | Example programs and usage patterns |

---

## 🎯 Usage Example (Rust, high-level)

```rust
use caliber_context::{ContextAssembler, ContextPackage};
use caliber_core::{
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

---

## 🧪 Running Tests

```bash
# All workspace tests (excludes pgrx extension)
cargo test --workspace --exclude caliber-pg

# Specific crate
cargo test -p caliber-core

# Property tests only
cargo test --workspace --exclude caliber-pg -- prop_

# With output
cargo test --workspace --exclude caliber-pg -- --nocapture

# Examples (separate from workspace tests)
cargo test --examples

# Fuzz tests (requires nightly Rust)
cargo +nightly fuzz run lexer_fuzz -- -max_total_time=60
cargo +nightly fuzz run parser_fuzz -- -max_total_time=60

# pgrx extension tests (requires PostgreSQL)
cargo pgrx test -p caliber-pg
```

**Fuzz Testing Results:**
- 462,947 adversarial inputs tested
- 0 crashes (100% robust)
- Validates DSL parser production-readiness

---

## 💡 Examples

See [examples/README.md](examples/README.md) for detailed usage examples:

- **basic_trajectory.rs** - Complete workflow: Trajectory → Scope → Artifacts → Turns → Notes
- More examples coming soon: context assembly, multi-agent coordination, vector search, DSL configuration

Run examples with:
```bash
cargo run --example basic_trajectory
```

---

## 🔧 Development

### Philosophy

CALIBER is a **framework**, not a product. Every value must be explicitly configured:

```rust
// ❌ WRONG - We don't do this
const DEFAULT_TOKEN_BUDGET: i32 = 8000;

// ✅ RIGHT - User must configure
pub struct CaliberConfig {
    pub token_budget: i32,  // Required, no default
}
```

### Code Standards

- Use `CaliberResult<T>` for all fallible operations
- No `unwrap()` in production code — use `?` operator
- All public items have doc comments
- Property tests for correctness properties

---

## 📄 License

This specification is released for implementation. Build cool shit. 🚀
