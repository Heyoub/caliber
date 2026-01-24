# Requirements Document

## Introduction

CALIBER (Context Abstraction Layer Integrating Behavioral Extensible Runtime) with PCP (Persistent Context Protocol) — a Postgres-native memory framework for AI agents. This spec covers the core implementation of the multi-crate Rust workspace.

## Glossary

- **CALIBER**: Context Abstraction Layer Integrating Behavioral Extensible Runtime
- **PCP**: Persistent Context Protocol — validation, checkpoints, harm reduction
- **VAL**: Vector Abstraction Layer — provider-agnostic embeddings
- **Trajectory**: Top-level task container
- **Scope**: Partitioned context window within a trajectory
- **Artifact**: Typed output preserved across scopes
- **Note**: Long-term cross-trajectory knowledge
- **Turn**: Ephemeral conversation buffer entry
- **pgrx**: Rust framework for building Postgres extensions
- **ECS**: Entity-Component-System architecture pattern
- **CaliberConfig**: Master configuration struct (all values explicit, no defaults)

---

## Requirements

### Requirement 1: Cargo Workspace Setup

**User Story:** As a developer, I want a properly structured Rust workspace, so that I can build the multi-crate CALIBER system.

#### Acceptance Criteria

1. THE Workspace SHALL contain 7 crates: caliber-core, caliber-storage, caliber-pcp, caliber-llm, caliber-agents, caliber-dsl, caliber-pg
2. WHEN `cargo build --workspace` is run, THE System SHALL compile without errors
3. THE Workspace SHALL define proper inter-crate dependencies following ECS architecture
4. THE caliber-pg crate SHALL be the only crate with pgrx runtime dependency

---

### Requirement 2: Core Entity Types (caliber-core)

**User Story:** As a developer, I want well-defined entity types, so that all crates share a common data model.

#### Acceptance Criteria

1. THE caliber-core crate SHALL define Trajectory, Scope, Artifact, Note, and Turn structs
2. THE caliber-core crate SHALL define all enums: TrajectoryStatus, TurnRole, ArtifactType, NoteType, MemoryCategory, TTL
3. WHEN an entity is created, THE System SHALL use UUIDv7 for EntityId (timestamp-sortable)
4. THE EmbeddingVector struct SHALL support dynamic dimensions via Vec<f32>
5. THE caliber-core crate SHALL define CaliberError enum with Storage, Llm, Validation, Config, Vector, and Agent variants
6. THE caliber-core crate SHALL define CaliberResult<T> type alias
7. THE caliber-core crate SHALL contain NO behavior, only data structures

---

### Requirement 3: Configuration System

**User Story:** As a user, I want explicit configuration with no hidden defaults, so that I have full control over system behavior.

#### Acceptance Criteria

1. THE CaliberConfig struct SHALL require explicit values for: token_budget, checkpoint_retention, stale_threshold, contradiction_threshold
2. WHEN a required config field is missing, THE System SHALL return ConfigError::MissingRequired
3. THE CaliberConfig::validate() method SHALL check all constraints and return CaliberResult<()>
4. IF token_budget is <= 0, THEN THE System SHALL return ConfigError::InvalidValue
5. IF contradiction_threshold is outside 0.0-1.0, THEN THE System SHALL return ConfigError::InvalidValue
6. THE System SHALL NOT contain any hard-coded default values

---

### Requirement 4: DSL Lexer (caliber-dsl)

**User Story:** As a user, I want to write CALIBER configurations in a custom DSL, so that I can define memory types and policies declaratively.

#### Acceptance Criteria

1. THE Lexer SHALL tokenize all keywords: caliber, memory, policy, adapter, inject, into, on, context
2. THE Lexer SHALL tokenize memory types: ephemeral, working, episodic, semantic, procedural, meta
3. THE Lexer SHALL tokenize field types: uuid, text, int, float, bool, timestamp, json, embedding, enum
4. THE Lexer SHALL tokenize operators: =, !=, >, <, >=, <=, ~, contains, and, or, not
5. THE Lexer SHALL handle string literals with escape sequences (\n, \t, \\, \")
6. THE Lexer SHALL handle duration literals (e.g., 30s, 5m, 1h, 7d)
7. THE Lexer SHALL skip whitespace and comments (// line and /*block*/)
8. WHEN an invalid character is encountered, THE Lexer SHALL produce TokenKind::Error

---

### Requirement 5: DSL Parser (caliber-dsl)

**User Story:** As a user, I want my DSL configurations parsed into an AST, so that they can be validated and compiled.

#### Acceptance Criteria

1. THE Parser SHALL parse the top-level config structure: `caliber: "version" { definitions... }`
2. THE Parser SHALL parse adapter definitions with type, connection, and options
3. THE Parser SHALL parse memory definitions with type, schema, retention, lifecycle, index, and inject_on
4. THE Parser SHALL parse policy definitions with trigger-action rules
5. THE Parser SHALL parse injection rules with mode, priority, max_tokens, and filter
6. THE Parser SHALL parse filter expressions with comparisons and boolean operators
7. WHEN a syntax error is encountered, THE Parser SHALL return ParseError with line/column info
8. FOR ALL valid DSL input, parsing then pretty-printing then parsing SHALL produce an equivalent AST (round-trip property)

---

### Requirement 6: VAL - Vector Abstraction Layer (caliber-llm)

**User Story:** As a developer, I want provider-agnostic embedding support, so that I can use any LLM provider.

#### Acceptance Criteria

1. THE EmbeddingProvider trait SHALL define embed(), embed_batch(), dimensions(), and model_id() methods
2. THE SummarizationProvider trait SHALL define summarize(), extract_artifacts(), and detect_contradiction() methods
3. THE ProviderRegistry SHALL allow explicit registration of providers (no auto-discovery)
4. WHEN no provider is registered and an LLM operation is attempted, THE System SHALL return LlmError::ProviderNotConfigured
5. THE EmbeddingVector SHALL track model_id and dimensions explicitly
6. WHEN comparing vectors with mismatched dimensions, THE System SHALL return VectorError::DimensionMismatch

---

### Requirement 7: Multi-Agent Coordination (caliber-agents)

**User Story:** As a multi-agent system builder, I want coordination primitives, so that agents can work together without conflicts.

#### Acceptance Criteria

1. THE Agent struct SHALL track agent_id, agent_type, capabilities, status, and memory_access
2. THE System SHALL support distributed locks via Postgres advisory locks
3. WHEN a lock is acquired, THE System SHALL record holder_agent_id and expires_at
4. THE System SHALL support message passing between agents via AgentMessage
5. WHEN a message is sent, THE System SHALL use Postgres NOTIFY for real-time delivery
6. THE System SHALL support task delegation with DelegatedTask tracking
7. THE System SHALL support agent handoffs with context transfer
8. WHEN a conflict is detected, THE System SHALL create a Conflict record with resolution options

---

### Requirement 8: Storage Layer (caliber-storage)

**User Story:** As a developer, I want direct Postgres storage access, so that I avoid SQL parsing overhead in the hot path.

#### Acceptance Criteria

1. THE storage layer SHALL NOT use SQL for hot-path operations
2. THE storage layer SHALL use pgrx for direct heap tuple operations
3. THE storage layer SHALL support index operations for btree, hash, and hnsw
4. WHEN an entity is not found, THE System SHALL return StorageError::NotFound
5. WHEN an insert fails, THE System SHALL return StorageError::InsertFailed with reason

---

### Requirement 9: Context Assembly (caliber-core::context)

**User Story:** As an AI agent, I want intelligent context assembly, so that I get relevant information within my token budget.

#### Acceptance Criteria

1. THE ContextWindow struct SHALL track max_tokens, used_tokens, and sections
2. THE System SHALL assemble context sections ordered by priority
3. WHEN token budget is exceeded, THE System SHALL truncate lower-priority sections
4. THE System SHALL track assembly decisions in assembly_trace for auditability
5. THE System SHALL support injection modes: full, summary, top_k, and relevant

---

### Requirement 10: PCP - Persistent Context Protocol (caliber-pcp)

**User Story:** As a developer, I want validation and checkpointing, so that I can ensure context integrity and recover from failures.

#### Acceptance Criteria

1. THE System SHALL validate context integrity on configurable triggers (on_mutation or always)
2. THE System SHALL create checkpoints at scope close
3. THE System SHALL detect contradictions between artifacts using embedding similarity
4. WHEN contradiction_threshold is exceeded and content differs, THE System SHALL flag a potential contradiction
5. THE System SHALL support checkpoint recovery for failed trajectories

---

## 🎯 HACKATHON REMINDERS

**Hey future me! Don't forget these for maximum points:**

### Documentation (20 pts)

- [ ] Update DEVLOG.md after each major milestone
- [ ] Document decisions and rationale
- [ ] Keep README.md current with setup instructions

### Kiro Usage (20 pts)

- [ ] Use @prime at session start
- [ ] Use @plan-feature before implementing
- [ ] Use @code-review after implementations
- [ ] Customize prompts as you learn the workflow

### Process Transparency

- [ ] Record yourself working (you already started!)
- [ ] Note time spent on each feature
- [ ] Document challenges and how you solved them

### Before Submission

- [ ] Run @code-review-hackathon for final evaluation
- [ ] Record 2-5 minute demo video
- [ ] Verify judges can run the project with clear setup instructions
