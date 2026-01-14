# CALIBER + PCP

> **Context Abstraction Layer Integrating Behavioral Extensible Runtime**  
> **+ Persistent Context Protocol**

A Postgres-native memory framework for AI agents, built as a multi-crate Rust workspace using pgrx.

**Version:** 0.2.1  
**Architecture:** Multi-crate ECS (Entity-Component-System)  
**Language:** Rust (pgrx)

---

## 🚀 Quick Start

### Prerequisites

- **Rust** 1.75+ (install via [rustup](https://rustup.rs/))
- **PostgreSQL** 13-17 (for pgrx extension, optional for core development)
- **Cargo** (comes with Rust)

### Build & Test (Without PostgreSQL)

```bash
# Clone the repository
git clone <repository-url>
cd caliber

# Build all crates (excluding pgrx extension)
cargo build --workspace --exclude caliber-pg

# Run all tests (165 tests)
cargo test --workspace --exclude caliber-pg

# Run with verbose output
cargo test --workspace --exclude caliber-pg -- --nocapture

# Run clippy lints
cargo clippy --workspace --exclude caliber-pg -- -D warnings
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
├── caliber-test-utils/  # Test generators, fixtures, assertions
├── docs/                # Specification documents
├── fuzz/                # Fuzz testing targets
├── Cargo.toml           # Workspace manifest
├── DEVLOG.md            # Development timeline
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
| [DEVLOG.md](DEVLOG.md) | Development timeline |

---

## 🎯 Usage Example

```rust
use caliber_core::{CaliberConfig, Trajectory, TrajectoryStatus, EntityType};
use caliber_storage::StorageTrait;
use uuid::Uuid;

// Configuration is REQUIRED — no defaults
let config = CaliberConfig {
    token_budget: 8000,
    checkpoint_retention: 5,
    stale_threshold: std::time::Duration::from_secs(86400 * 30),
    contradiction_threshold: 0.85,
    context_persistence: ContextPersistence::Ttl(Duration::from_secs(86400)),
    validation_mode: ValidationMode::OnMutation,
    section_priorities: SectionPriorities::default_test(),
    embedding_provider: None,
    summarization_provider: None,
    llm_retry_config: RetryConfig::default_test(),
    lock_timeout: Duration::from_secs(30),
    message_retention: Duration::from_secs(86400),
    delegation_timeout: Duration::from_secs(3600),
};

// Validate configuration
config.validate()?;

// Create a trajectory
let trajectory = Trajectory {
    trajectory_id: Uuid::now_v7(),
    title: "Implement feature X".to_string(),
    status: TrajectoryStatus::Active,
    // ... other fields
};
```

---

## 🧪 Running Tests

```bash
# All tests
cargo test --workspace --exclude caliber-pg

# Specific crate
cargo test -p caliber-core

# Property tests only
cargo test --workspace --exclude caliber-pg -- prop_

# With output
cargo test --workspace --exclude caliber-pg -- --nocapture

# Fuzz tests (requires nightly)
cargo +nightly fuzz run lexer_fuzz -- -max_total_time=60
cargo +nightly fuzz run parser_fuzz -- -max_total_time=60
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
