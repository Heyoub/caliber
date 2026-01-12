# CALIBER + PCP Specification Suite

> **Context Abstraction Layer Integrating Behavioral Extensible Runtime**  
> **+ Persistent Context Protocol**

**Version:** 0.2.1  
**Architecture:** Multi-crate ECS (Entity-Component-System)  
**Language:** Rust (pgrx)  
**SQL Usage:** Human debug interface only - NOT in hot path  
**Philosophy:** **NOTHING HARD-CODED. This is a FRAMEWORK, not a product.**

---

## 🚨 Critical Philosophy

CALIBER is a **toolkit/framework**. Users configure everything explicitly.

```rust
// WRONG - We do NOT do this
const DEFAULT_TOKEN_BUDGET: i32 = 8000;

// RIGHT - User MUST configure
pub struct CaliberConfig {
    pub token_budget: i32,  // Required, no default
}
```

**If it's not configured, it errors. Period.**

---

## 📁 Document Index

| File | Purpose |
|------|---------|
| [CALIBER_PCP_SPEC.md](./CALIBER_PCP_SPEC.md) | Core specification - types, DSL, runtime, PCP (Rust) |
| [DSL_PARSER.md](./DSL_PARSER.md) | Lexer, parser, code generator (Rust) |
| [MULTI_AGENT_COORDINATION.md](./MULTI_AGENT_COORDINATION.md) | Locks, messages, delegation, handoffs (Rust) |
| [LLM_SERVICES.md](./LLM_SERVICES.md) | VAL (Vector Abstraction Layer), summarization (Rust) |
| [QUICK_REFERENCE.md](./QUICK_REFERENCE.md) | Cheat sheets and quick lookup |

---

## 🏗️ Multi-Crate ECS Architecture

Compositional over inheritance. Each crate is a component:

```
caliber-core/        # ENTITIES: Data structures only
                     # Trajectory, Scope, Artifact, Note, Turn
                     # No behavior, just types

caliber-storage/     # COMPONENT: Storage trait + pgrx implementation
                     # Direct heap access, no SQL in hot path

caliber-context/     # COMPONENT: Context assembly logic
                     # Trait-based, composable

caliber-pcp/         # COMPONENT: Validation, checkpoints, recovery
                     # PCP harm reduction

caliber-llm/         # COMPONENT: VAL (Vector Abstraction Layer)
                     # Provider-agnostic traits for embeddings/summarization
                     # Supports any dimension (OpenAI 1536, Ollama 768, etc.)

caliber-agents/      # COMPONENT: Multi-agent coordination (full support)
                     # Locks, messages, delegation, handoffs

caliber-dsl/         # SYSTEM: DSL parser → CaliberConfig struct
                     # Separate crate, generates configuration only

caliber-pg/          # SYSTEM: The actual pgrx extension
                     # Wires all components together, runs in Postgres
```

**The runtime IS Postgres.** `caliber-pg` is the runtime. `caliber-dsl` produces config.

---

## 🔑 Key Design Decisions

| Decision | Choice |
|----------|--------|
| File organization | Multi-crate ECS |
| Multi-agent | Full support (locks, messages, delegation, handoffs) |
| DSL | Separate crate, generates config, no runtime |
| Turn buffer | `{turn_id, scope_id, role, content, created_at, token_count, model_id}` |
| Context persistence | **Configurable** (ephemeral, TTL, permanent) |
| Embedding vectors | Dynamic `Vec<f32>` via VAL, any dimension |
| Content storage | BYTEA only (flexible) |
| Error handling | Rust enums + Postgres ereport, no local bullshit |
| Validation timing | **Configurable** (on-mutation or always) |
| LLM providers | **No default**, explicit config required, VAL abstraction |
| LLM failures | Retry with configurable exponential backoff |
| Embedding cache | Postgres table (no Redis needed) |
| Token budget | **Configurable**, no default |
| Section priorities | **Configurable**, no default |

---

## 🎯 What This Solves

| Problem | CALIBER Solution |
|---------|-----------------|
| Context amnesia | Hierarchical memory: Trajectory → Scope → Artifact → Note |
| Hallucination | PCP grounding: all facts backed by stored artifacts |
| Multi-agent failures (40-80%) | Typed coordination: locks, messages, delegation |
| Token waste | Configurable context assembly with relevance scoring |
| No auditability | Full trace of assembly decisions |
| Hard-coded AI frameworks | **Zero hard-coded values, everything configurable** |

---

## 🚀 Quick Start

### 1. Build Extension

```bash
# Multi-crate workspace
cargo build -p caliber-pg --release
cargo pgrx package -p caliber-pg
```

### 2. Install & Configure

```sql
CREATE EXTENSION caliber;

-- MUST provide configuration - no defaults
SELECT caliber_init('{
    "token_budget": 8000,
    "checkpoint_retention": 5,
    "stale_threshold_days": 30,
    "contradiction_threshold": 0.85,
    "context_persistence": "ttl_24h",
    "validation_mode": "on_mutation",
    "section_priorities": {
        "user": 100,
        "system": 90,
        "artifacts": 80,
        "notes": 70,
        "history": 60
    },
    "embedding_provider": {
        "type": "openai",
        "model": "text-embedding-3-small",
        "dimensions": 1536
    },
    "retry_config": {
        "max_retries": 3,
        "initial_backoff_ms": 1000,
        "max_backoff_ms": 30000,
        "multiplier": 2.0
    }
}');
```

### 3. Use It

```rust
// Start trajectory
let trajectory_id = CaliberOrchestrator::start_trajectory(
    &config,  // Config required everywhere
    "Implement feature X",
    EntityType::Agent,
    agent_id,
    None,
)?;  // Returns CaliberResult, errors propagate to Postgres
```

---

## 🏛️ Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      CALIBER + PCP                              │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              CaliberConfig (user-provided)               │   │
│  │  • Every value explicit    • No defaults anywhere       │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                PCP Protocol Layer                        │   │
│  │  • Context validation    • Checkpoint/recovery          │   │
│  │  • Dosage control        • Contradiction detection      │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              CALIBER Components (ECS)                    │   │
│  │  caliber-core │ caliber-storage │ caliber-context       │   │
│  │  caliber-pcp  │ caliber-llm     │ caliber-agents        │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                pgrx Direct Storage                       │   │
│  │  • Heap tuple ops    • Index access    • WAL writes     │   │
│  └─────────────────────────────────────────────────────────┘   │
│                              │                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              PostgreSQL Storage Engine                   │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

---

## 📄 License

This specification is released for implementation. Build cool shit. 🚀
