# Implementation Plan: CALIBER Core Implementation

## Overview

Implement the CALIBER multi-crate Rust workspace following ECS architecture.

**AI-Native Approach:** We front-load ALL planning before touching cargo. Step 0 builds the complete semantic dependency graph, type system, and Cargo.toml with optimized settings. Then we generate all code, THEN run cargo once at the end. No incremental build pain.

## Philosophy

```text
Traditional (human): code → cargo check → fix → repeat 1000x
AI-Native (us): plan everything → generate all code → cargo check once → fix → done
```

---

## Tasks

- [x] 0. **STEP ZERO: Complete Semantic Planning (DO THIS FIRST)**
  - [x] 0.1 Build Full Crate Dependency Graph
    - Map all core crates with exact inter-crate dependencies
    - Verify DAG has no cycles
    - Document which types flow between crates
  - [x] 0.2 Lock External Dependencies with Exact Versions
    - Research and lock versions for pgrx, uuid, chrono, serde, thiserror, sha2, regex, proptest, once_cell
    - Check pgvector/pgrx compatibility
    - Document any version conflicts or constraints
  - [x] 0.3 Design Complete Type System (ALL types, ALL crates)
    - All caliber-core types documented
    - All caliber-dsl types documented
    - All caliber-llm types documented
    - All context module types documented (caliber-core::context)
    - All caliber-pcp types documented
    - All caliber-agents types documented
    - All caliber-storage types documented
    - All caliber-pg types documented
  - [x] 0.4 Identify Free Batteries (Don't Reinvent)
    - pgrx derives and features documented
    - Postgres features documented
    - Rust std features documented
    - External crate features documented
  - [x] 0.5 Create Optimized Cargo.toml Template
    - Workspace-level optimizations documented
    - Profile configurations documented
  - [x] 0.6 Output: Create `docs/DEPENDENCY_GRAPH.md`
    - Complete reference document created
  - **🎯 HACKATHON: Step 0 complete — docs/DEPENDENCY_GRAPH.md created**

---

- [x] 1. Initialize Cargo Workspace (after Step 0 approval)
  - [x] 1.1 Create workspace Cargo.toml from Step 0.5 template
    - Use resolver = "2"
    - Define all workspace.dependencies
    - Configure profile optimizations
    - _Requirements: 1.1, 1.2_
  - [x] 1.2 Create directory structure for all core crates
    - caliber-core (incl. context), caliber-storage, caliber-pcp
    - caliber-llm, caliber-agents, caliber-dsl, caliber-pg
    - Create `src/` folders only — NO lib.rs stubs (those come with actual code in Tasks 2-12)
    - _Requirements: 1.1_
  - [x] 1.3 Create each crate's Cargo.toml with locked deps from Step 0.2
    - Use workspace = true for shared dependencies
    - _Requirements: 1.3_
  - **DO NOT run cargo yet — just create Cargo.toml files and directories**
  - **lib.rs files are created WITH their actual implementations in Tasks 2-12**
  - **🎯 HACKATHON: Update DEVLOG.md with workspace decisions**

- [x] 2. Implement caliber-core (Entity Types) — CODE GEN ONLY
  - **Generate all code, do NOT run cargo check yet**
  - [x] 2.1 Create fundamental types (EntityId, Timestamp, ContentHash, RawContent)
    - Use UUIDv7 for timestamp-sortable IDs
    - _Requirements: 2.3_
  - [x] 2.2 Create TTL and MemoryCategory enums
    - _Requirements: 2.2_
  - [x] 2.3 Create EmbeddingVector with dynamic dimensions
    - Implement cosine_similarity with dimension checking
    - _Requirements: 2.4, 6.5, 6.6_
  - [x] 2.4 Create core entity structs (Trajectory, Scope, Artifact, Note, Turn)
    - _Requirements: 2.1_
  - [x] 2.5 Create CaliberError enum with all variants
    - Storage, Llm, Validation, Config, Vector, Agent
    - _Requirements: 2.5, 2.6_
  - [x] 2.6 Create CaliberConfig struct with validation
    - All fields required, no defaults
    - validate() returns CaliberResult<()>
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_
  - [x] 2.7 Write property tests for caliber-core
    - **Property 1: Config validation rejects invalid values**
    - **Property 5: EmbeddingVector dimension mismatch detection**
    - **Property 7: EntityId uses UUIDv7 (timestamp-sortable)**
    - **Validates: Requirements 2.3, 3.4, 3.5, 6.6**
  - **🎯 HACKATHON: Run @code-review, update DEVLOG.md**

- [x] 3. Implement caliber-dsl Lexer
  - [x] 3.1 Create TokenKind enum with all token types
    - Keywords, memory types, field types, operators, delimiters, literals
    - Include Schedule keyword for cron-based triggers
    - _Requirements: 4.1, 4.2, 4.3, 4.4_
  - [x] 3.2 Create Token and Span structs
    - _Requirements: 4.1_
  - [x] 3.3 Implement Lexer struct with tokenize()
    - Handle whitespace and comments
    - Handle string literals with escapes
    - Handle duration literals
    - _Requirements: 4.5, 4.6, 4.7_
  - [x] 3.4 Implement error handling for invalid characters
    - _Requirements: 4.8_
  - [x] 3.5 Write property tests for lexer
    - **Property 4: Lexer produces Error token for invalid characters**
    - **Validates: Requirements 4.8**
  - [x] 3.6 Write fuzz tests for lexer
    - Fuzz with arbitrary byte sequences
  - **🎯 HACKATHON: Update DEVLOG.md with parsing approach**

- [x] 4. Implement caliber-dsl Parser
  - [x] 4.1 Create AST types (CaliberAst, Definition, AdapterDef, MemoryDef, etc.)
    - Include Trigger::Schedule(String) for cron expressions
    - Include Action::Prune with criteria
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_
  - [x] 4.2 Implement Parser struct with parse()
    - _Requirements: 5.1_
  - [x] 4.3 Implement parse_adapter()
    - _Requirements: 5.2_
  - [x] 4.4 Implement parse_memory()
    - _Requirements: 5.3_
  - [x] 4.5 Implement parse_policy()
    - _Requirements: 5.4_
  - [x] 4.6 Implement parse_injection()
    - _Requirements: 5.5_
  - [x] 4.7 Implement filter expression parsing
    - _Requirements: 5.6_
  - [x] 4.8 Implement ParseError with line/column info
    - _Requirements: 5.7_
  - [x] 4.9 Implement pretty-printer for AST
    - For round-trip testing
    - _Requirements: 5.8_
  - [x] 4.10 Write property tests for parser
    - **Property 3: DSL round-trip parsing preserves semantics**
    - **Validates: Requirements 5.8**
  - [x] 4.11 Write fuzz tests for parser
    - Fuzz with random token streams
  - **🎯 HACKATHON: Update DEVLOG.md with AST design**

- [x] 5. **FIRST CARGO RUN** - Core Types Complete
  - **NOW run cargo for the first time:**
  - [x] 5.1 Run `cargo check --workspace`
    - Fix any compilation errors
  - [x] 5.2 Run `cargo test --workspace` for property tests
  - [x] 5.3 Run `cargo clippy --workspace -- -D warnings`
  - **🎯 HACKATHON: Commit with meaningful message, update DEVLOG.md**

- [x] 6. Implement caliber-llm (VAL)
  - [x] 6.1 Create EmbeddingProvider trait
    - embed(), embed_batch(), dimensions(), model_id()
    - _Requirements: 6.1_
  - [x] 6.2 Create SummarizationProvider trait
    - summarize(), extract_artifacts(), detect_contradiction()
    - _Requirements: 6.2_
  - [x] 6.3 Create ProviderRegistry with explicit registration
    - _Requirements: 6.3, 6.4_
  - [x] 6.4 Create mock providers for testing
    - MockEmbeddingProvider, MockSummarizationProvider
  - [x] 6.5 Write property tests for VAL
    - **Property 6: Provider registry returns error when not configured**
    - **Validates: Requirements 6.4**
  - **🎯 HACKATHON: Document VAL design decisions in DEVLOG.md**

- [x] 7. Implement context module in caliber-core (Context Assembly)
  - [x] 7.1 Create ContextPackage struct
    - user_input, relevant_notes, recent_artifacts, session_markers, kernel_config
    - _Requirements: 9.1_
  - [x] 7.2 Create ContextWindow and ContextSection structs
    - _Requirements: 9.1_
  - [x] 7.3 Implement token estimation (estimate_tokens)
    - ~0.75 tokens per character
    - _Requirements: 9.3_
  - [x] 7.4 Implement smart truncation (truncate_to_token_budget)
    - Prefer sentence boundaries, fall back to word boundaries
    - _Requirements: 9.3_
  - [x] 7.5 Implement ContextAssembler with assemble()
    - Add sections by priority until budget exhausted
    - Track assembly decisions
    - _Requirements: 9.2, 9.3, 9.4, 9.5_
  - [x] 7.6 Write property tests for context assembly
    - **Property 8: Context assembly respects token budget**
    - **Property 11: Context sections ordered by priority**
    - **Property 12: Token estimation consistency**
    - **Property 13: Truncation respects budget**
    - **Validates: Requirements 9.2, 9.3**

- [x] 8. Implement caliber-pcp (Validation & Memory Commit)
  - [x] 8.1 Create MemoryCommit struct
    - query, response, mode, reasoning_trace, tools_invoked, tokens, cost
    - _Requirements: 10.1_
  - [x] 8.2 Create RecallService with recall methods
    - recall_previous(), search_interactions(), recall_decisions()
    - _Requirements: 10.1_
  - [x] 8.3 Implement decision extraction from responses
    - Look for recommend/should/decision/conclude patterns
  - [x] 8.4 Create PCPConfig with sub-configs
    - ContextDagConfig, RecoveryConfig, DosageConfig, AntiSprawlConfig, GroundingConfig
  - [x] 8.5 Create PCPValidator/PCPRuntime struct
    - _Requirements: 10.1_
  - [x] 8.6 Implement validate_context_integrity()
    - Returns ValidationResult with issues list
    - _Requirements: 10.1_
  - [x] 8.7 Implement detect_contradictions()
    - Use embedding similarity
    - _Requirements: 10.3, 10.4_
  - [x] 8.8 Implement apply_dosage_limits()
    - Enforce token/scope/artifact limits
  - [x] 8.9 Implement lint_artifact()
    - Size check, duplicate detection
  - [x] 8.10 Implement checkpoint creation and recovery
    - create_checkpoint(), recover_from_checkpoint()
    - _Requirements: 10.2, 10.5_
  - [x] 8.11 Write property tests for PCP
    - **Property 14: Memory commit preserves query/response**
    - **Property 15: Recall decisions filters correctly**
    - **Validates: Requirements 10.1, 10.2**

- [x] 9. Checkpoint - Component Crates Complete
  - [x] 9.1 Ensure `cargo build --workspace` succeeds
  - [x] 9.2 Ensure all property tests pass
  - **🎯 HACKATHON: Commit, update DEVLOG.md with progress**

- [x] 10. Implement caliber-agents (Multi-Agent Coordination)
  - [x] 10.1 Create Agent struct with status and memory_access
    - _Requirements: 7.1_
  - [x] 10.2 Create MemoryRegion enum and MemoryRegionConfig
    - Private, Team, Public, Collaborative regions
    - Access control with caliber_check_access()
  - [x] 10.3 Create DistributedLock struct
    - _Requirements: 7.2, 7.3_
  - [x] 10.4 Create AgentMessage struct and MessageType enum
    - _Requirements: 7.4, 7.5_
  - [x] 10.5 Create DelegatedTask struct
    - _Requirements: 7.6_
  - [x] 10.6 Create AgentHandoff struct
    - _Requirements: 7.7_
  - [x] 10.7 Create Conflict struct and resolution types
    - ConflictType, ConflictStatus, ResolutionStrategy
    - _Requirements: 7.8_
  - [x] 10.8 Write property tests for agents
    - **Property 9: Lock acquisition records holder**
    - **Validates: Requirements 7.3**
  - **🎯 HACKATHON: Document coordination protocol in DEVLOG.md**

- [x] 11. Implement caliber-storage (Storage Traits)
  - [x] 11.1 Create StorageTrait with CRUD operations
    - trajectory_insert/get, scope_insert/get, artifact_insert/get, note_insert/get
    - _Requirements: 8.1, 8.2_
  - [x] 11.2 Create vector_search method signature
    - _Requirements: 8.3_
  - [x] 11.3 Define error mapping to StorageError
    - _Requirements: 8.4, 8.5_
  - [x] 11.4 Create MockStorageTrait for testing
  - [x] 11.5 Write property tests for storage
    - **Property 10: Storage not-found returns correct error**
    - **Validates: Requirements 8.4**

- [x] 12. Implement caliber-pg (pgrx Extension)
  - [x] 12.1 Set up pgrx extension boilerplate
    - _Requirements: 1.4_
  - [x] 12.2 Create bootstrap SQL schema (caliber_init)
    - Tables: caliber_trajectory, caliber_scope, caliber_artifact, caliber_note, caliber_turn
    - Agent tables: caliber_agent, caliber_lock, caliber_message, caliber_delegation, caliber_conflict, caliber_handoff
    - Indexes for btree, hash, and hnsw
    - This SQL runs ONCE at extension install, NOT in hot path
  - [x] 12.3 Implement StorageTrait via direct heap operations
    - _Requirements: 8.1, 8.2_
  - [x] 12.4 Implement advisory lock functions
    - caliber_lock_acquire, caliber_lock_release
    - _Requirements: 7.2_
  - [x] 12.5 Implement NOTIFY-based message passing
    - _Requirements: 7.5_
  - [x] 12.6 Wire up pg_extern functions
  - [x] 12.7 Create debug SQL views (human interface only)
  - [x] 12.8 Write pgrx integration tests

- [x] 13. Implement Test Infrastructure
  - [x] 13.1 Create caliber-test-utils crate
  - [x] 13.2 Implement proptest generators for all entity types
  - [x] 13.3 Implement mock providers
  - [x] 13.4 Implement test fixtures
  - [x] 13.5 Implement custom assertions

- [x] 14. Final Checkpoint - All Tests Pass
  - [x] 14.1 Run `cargo test --workspace`
  - [x] 14.2 Run `cargo clippy --workspace -- -D warnings`
  - [x] 14.3 Run property tests with 100+ iterations
  - **🎯 HACKATHON: Run @code-review-hackathon for final evaluation**

- [x] 15. Documentation & Submission Prep
  - [x] 15.1 Update README.md with clear setup instructions
    - Prerequisites, build steps, usage examples
  - [x] 15.2 Finalize DEVLOG.md
    - Complete timeline, all decisions documented
  - [x] 15.3 Record 2-5 minute demo video
  - [x] 15.4 Verify judges can run the project
  - **🎯 HACKATHON: Final submission checklist**

## Notes

- 🎯 HACKATHON markers remind you to update documentation for judging
- Each checkpoint is a good commit point
- Property tests validate universal correctness properties
- Unit tests validate specific examples and edge cases
- Fuzz tests catch edge cases the property tests might miss
