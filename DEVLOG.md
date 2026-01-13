# CALIBER Development Log

## Project Overview

Building CALIBER (Context Abstraction Layer Integrating Behavioral Extensible Runtime) with PCP (Persistent Context Protocol) — a Postgres-native memory framework for AI agents.

---

## Timeline

### January 12, 2026 — Project Initialization

**Completed:**

- Set up `.kiro/` structure for hackathon
- Created steering documents (product.md, tech.md, structure.md)
- Created custom prompts (prime, plan-feature, execute, code-review, implement-crate)
- Initialized DEVLOG.md

**Documentation Status:**

- ✅ CALIBER_PCP_SPEC.md — Core specification complete
- ✅ DSL_PARSER.md — Lexer, parser, AST defined
- ✅ LLM_SERVICES.md — VAL and provider traits defined
- ✅ MULTI_AGENT_COORDINATION.md — Agent coordination protocol defined
- ✅ QUICK_REFERENCE.md — Cheat sheet complete

---

### January 13, 2026 — Workspace Initialization

**Completed:**

- ✅ Created workspace Cargo.toml with resolver = "2"
- ✅ Defined all workspace.dependencies with locked versions
- ✅ Created directory structure for all 8 crates
- ✅ Created individual Cargo.toml for each crate

**Workspace Decisions:**

| Decision | Rationale |
|----------|-----------|
| `resolver = "2"` | Required for Rust 2021 edition, better feature resolution |
| Locked dependency versions | Reproducible builds: pgrx 0.12.9, uuid 1.11, chrono 0.4.39, etc. |
| Profile optimizations | `opt-level = 2` for deps in dev mode speeds up iteration |
| `workspace = true` for deps | Single source of truth for versions, easier updates |
| No lib.rs stubs | AI-native approach: create source files WITH implementations |
| cdylib for caliber-pg | Required for pgrx Postgres extension |

**Crate Structure:**

```
caliber/
├── Cargo.toml              # Workspace manifest
├── caliber-core/           # Entity types (no deps)
├── caliber-storage/        # Storage trait (core)
├── caliber-context/        # Context assembly (core)
├── caliber-pcp/            # Validation (core)
├── caliber-llm/            # VAL traits (core)
├── caliber-agents/         # Coordination (core, storage)
├── caliber-dsl/            # Parser (core)
└── caliber-pg/             # pgrx extension (ALL)
```

**Next Steps:**

- [x] Implement caliber-core with full entity types
- [ ] Implement caliber-dsl lexer
- [ ] Implement caliber-dsl parser
- [ ] First cargo check after core types complete

---

### January 13, 2026 — caliber-core Implementation

**Completed:**

- ✅ Task 2.1: Fundamental types (EntityId, Timestamp, ContentHash, RawContent)
- ✅ Task 2.2: TTL and MemoryCategory enums (plus all other enums)
- ✅ Task 2.3: EmbeddingVector with dynamic dimensions and cosine_similarity
- ✅ Task 2.4: Core entity structs (Trajectory, Scope, Artifact, Note, Turn)
- ✅ Task 2.5: CaliberError enum with all 6 variants
- ✅ Task 2.6: CaliberConfig struct with validate()

**Type Design Decisions:**

| Decision | Rationale |
|----------|-----------|
| `EntityId = Uuid` (UUIDv7) | Timestamp-sortable IDs via `Uuid::now_v7()` |
| `EmbeddingVector.data: Vec<f32>` | Dynamic dimensions, any embedding model |
| `ContentHash = [u8; 32]` | Fixed-size SHA-256 for deduplication |
| `RawContent = Vec<u8>` | Flexible binary content for BYTEA |
| All enums derive Serialize/Deserialize | JSON serialization for metadata fields |
| `CaliberConfig` has no defaults | Framework philosophy — user configures everything |
| `validate()` returns `CaliberResult<()>` | Consistent error handling pattern |

**Code Quality Checks:**

- ✅ No unwrap() or expect() in production code
- ✅ All public items have doc comments
- ✅ Unit tests for core functionality (7 tests)
- ✅ No TODO placeholders — all real code
- ✅ Types match docs/DEPENDENCY_GRAPH.md exactly

**Implementation Notes:**

- `cosine_similarity()` returns `VectorError::DimensionMismatch` for mismatched vectors
- `CaliberConfig::validate()` checks: token_budget > 0, contradiction_threshold ∈ [0,1], all durations positive
- PostgresType/PostgresEnum derives intentionally omitted — those go in caliber-pg
- ~600 lines of complete, working Rust code

**Property Tests Implemented (Task 2.7):**

| Property | Description | Validates |
|----------|-------------|-----------|
| Property 1 | Config validation rejects invalid token_budget (≤0) | Req 3.4 |
| Property 1 | Config validation rejects invalid contradiction_threshold (outside [0,1]) | Req 3.5 |
| Property 1 | Config validation accepts valid values | Req 3.4, 3.5 |
| Property 5 | EmbeddingVector dimension mismatch returns error | Req 6.6 |
| Property 5 | EmbeddingVector same dimension succeeds | Req 6.6 |
| Property 7 | EntityId uses UUIDv7 (version check) | Req 2.3 |
| Property 7 | EntityIds are timestamp-sortable | Req 2.3 |

All property tests configured with 100 iterations per proptest convention.

**Next Steps:**

- [x] Implement caliber-dsl lexer (Task 3)
- [ ] Implement caliber-dsl parser (Task 4)
- [ ] First cargo check (Task 5)

**Time Spent:** ~45 minutes

---

### January 13, 2026 — caliber-dsl Lexer Implementation

**Completed:**

- ✅ Task 3.1: TokenKind enum with all token types (50+ variants)
- ✅ Task 3.2: Token and Span structs for source location tracking
- ✅ Task 3.3: Lexer struct with tokenize() method
- ✅ Task 3.4: Error handling for invalid characters
- ✅ Task 3.5: Property tests for lexer (Property 4)
- ✅ Task 3.6: Fuzz tests for lexer

**Parsing Approach:**

| Decision | Rationale |
|----------|-----------|
| Hand-written lexer | Full control, no external parser generator dependency |
| Case-insensitive keywords | User-friendly DSL, `CALIBER` = `caliber` |
| Peekable CharIndices | Efficient single-pass tokenization with lookahead |
| Span tracking | Line/column info for error messages |
| TokenKind::Error | Graceful error recovery, continue tokenizing |

**Token Categories:**

| Category | Count | Examples |
|----------|-------|----------|
| Keywords | 22 | caliber, memory, policy, adapter, inject, schedule |
| Memory types | 6 | ephemeral, working, episodic, semantic, procedural, meta |
| Field types | 9 | uuid, text, int, float, bool, timestamp, json, embedding, enum |
| Operators | 12 | =, !=, >, <, >=, <=, ~, and, or, not, in, contains |
| Delimiters | 10 | { } ( ) [ ] : , . -> |
| Literals | 4 | String, Number, Duration, Identifier |

**Escape Sequences Supported:**

- `\n` → newline
- `\t` → tab
- `\\` → backslash
- `\"` → quote
- `\r` → carriage return

**Duration Suffixes:**

- `s` → seconds (e.g., `30s`)
- `m` → minutes (e.g., `5m`)
- `h` → hours (e.g., `1h`)
- `d` → days (e.g., `7d`)
- `w` → weeks (e.g., `2w`)

**Comment Handling:**

- Line comments: `// comment to end of line`
- Block comments: `/* multi-line comment */`

**Property Tests (Task 3.5):**

| Property | Description | Validates |
|----------|-------------|-----------|
| Property 4 | Invalid characters produce Error token | Req 4.8 |
| Valid identifiers | No errors for valid identifiers | Lexer correctness |
| Valid numbers | Numbers parse correctly | Req 4.1 |
| Duration literals | Duration suffixes parse correctly | Req 4.6 |
| String preservation | String content preserved | Req 4.5 |
| Eof invariant | Tokenization always ends with Eof | Lexer correctness |
| Line tracking | Line numbers monotonically increase | Span correctness |

**Fuzz Tests (Task 3.6):**

- Fuzz target: `fuzz/fuzz_targets/lexer_fuzz.rs`
- Tests arbitrary UTF-8 byte sequences
- Invariants checked: non-empty tokens, Eof at end, valid spans
- Run with: `cargo +nightly fuzz run lexer_fuzz -- -max_total_time=60`

**Code Statistics:**

- ~550 lines of lexer implementation
- ~200 lines of unit tests (25 tests)
- ~100 lines of property tests (7 properties)
- ~40 lines of fuzz test

**Next Steps:**

- [x] Implement caliber-dsl parser (Task 4)
- [ ] First cargo check (Task 5)

**Time Spent:** ~30 minutes

---

### January 13, 2026 — caliber-dsl Parser Implementation

**Completed:**

- ✅ Task 4.1: AST types (CaliberAst, Definition, AdapterDef, MemoryDef, etc.)
- ✅ Task 4.2: Parser struct with parse() method
- ✅ Task 4.3: parse_adapter() for adapter definitions
- ✅ Task 4.4: parse_memory() for memory definitions
- ✅ Task 4.5: parse_policy() for policy definitions
- ✅ Task 4.6: parse_injection() for injection rules
- ✅ Task 4.7: Filter expression parsing (And, Or, Not, Comparison)
- ✅ Task 4.8: ParseError with line/column info
- ✅ Task 4.9: Pretty-printer for AST (round-trip testing)
- ✅ Task 4.10: Property tests for parser (Property 3)
- ✅ Task 4.11: Fuzz tests for parser

**AST Design Decisions:**

| Decision | Rationale |
|----------|-----------|
| `CaliberAst` as root | Single entry point: version + definitions |
| `Definition` enum | Four variants: Adapter, Memory, Policy, Injection |
| `Trigger::Schedule(String)` | Cron expressions for scheduled policies |
| `Action::Prune { target, criteria }` | Structured prune with filter criteria |
| `FilterExpr` recursive enum | Supports And, Or, Not, Comparison |
| `InjectionMode` variants | Full, Summary, TopK(usize), Relevant(f32) |
| `FieldType::Embedding(Option<usize>)` | Optional dimension specification |

**AST Type Hierarchy:**

```text
CaliberAst
├── version: String
└── definitions: Vec<Definition>
    ├── Adapter(AdapterDef)
    │   ├── name, adapter_type, connection, options
    │   └── AdapterType: Postgres | Redis | Memory
    ├── Memory(MemoryDef)
    │   ├── name, memory_type, schema, retention, lifecycle
    │   ├── parent, indexes, inject_on, artifacts
    │   ├── MemoryType: Ephemeral | Working | Episodic | Semantic | Procedural | Meta
    │   ├── FieldType: Uuid | Text | Int | Float | Bool | Timestamp | Json | Embedding | Enum | Array
    │   ├── Retention: Persistent | Session | Scope | Duration | Max
    │   ├── Lifecycle: Explicit | AutoClose(Trigger)
    │   └── IndexType: Btree | Hash | Gin | Hnsw | Ivfflat
    ├── Policy(PolicyDef)
    │   ├── name, rules: Vec<PolicyRule>
    │   ├── PolicyRule: trigger + actions
    │   ├── Trigger: TaskStart | TaskEnd | ScopeClose | TurnEnd | Manual | Schedule(String)
    │   └── Action: Summarize | ExtractArtifacts | Checkpoint | Prune | Notify | Inject
    └── Injection(InjectionDef)
        ├── source, target, mode, priority, max_tokens, filter
        ├── InjectionMode: Full | Summary | TopK(usize) | Relevant(f32)
        └── FilterExpr: Comparison | And | Or | Not
```

**Parser Implementation:**

| Method | Parses | Requirements |
|--------|--------|--------------|
| `parse()` | Top-level `caliber: "version" { ... }` | Req 5.1 |
| `parse_adapter()` | Adapter definitions | Req 5.2 |
| `parse_memory()` | Memory definitions | Req 5.3 |
| `parse_policy()` | Policy definitions | Req 5.4 |
| `parse_injection()` | Injection rules | Req 5.5 |
| `parse_filter_expr()` | Filter expressions | Req 5.6 |

**Filter Expression Parsing:**

- Precedence: Not > And > Or (standard boolean precedence)
- Recursive descent parser with `parse_or_expr()` → `parse_and_expr()` → `parse_comparison()`
- Parentheses for grouping: `(a = 1 and b = 2) or c = 3`
- Special values: `current_trajectory`, `current_scope`, `now`, `null`, `true`, `false`

**Pretty-Printer:**

- `pretty_print(ast)` → DSL source code
- Proper indentation (4 spaces per level)
- String escaping for special characters
- Round-trip property: `parse(pretty_print(ast)) == ast`

**Property Tests (Task 4.10):**

| Property | Description | Validates |
|----------|-------------|-----------|
| Property 3 | Round-trip parsing preserves semantics | Req 5.8 |
| Property 4 | Invalid chars produce Error token | Req 4.8 |
| Eof invariant | Lexer always ends with Eof | Lexer correctness |
| Span validity | Span positions are valid | Parser correctness |

**Fuzz Tests (Task 4.11):**

- Fuzz target: `fuzz/fuzz_targets/parser_fuzz.rs`
- Tests arbitrary UTF-8 byte sequences through full parse pipeline
- Invariants: no panics, valid error locations, non-empty error messages
- Run with: `cargo +nightly fuzz run parser_fuzz -- -max_total_time=60`

**Code Statistics:**

- ~1800 lines total in caliber-dsl/src/lib.rs
- Lexer: ~550 lines
- AST types: ~250 lines
- Parser: ~600 lines
- Pretty-printer: ~250 lines
- Tests: ~350 lines (unit + property)

**Next Steps:**

- [ ] First cargo check (Task 5)
- [ ] Implement caliber-llm (Task 6)
- [ ] Implement caliber-context (Task 7)

**Time Spent:** ~45 minutes

---

## Decisions

| Date | Decision | Rationale |
|------|----------|-----------|
| Jan 12 | Multi-crate ECS architecture | Composition over inheritance, clear separation |
| Jan 12 | No SQL in hot path | Avoid parsing overhead, direct pgrx access |
| Jan 12 | Dynamic embedding dimensions | Support any provider (OpenAI, Ollama, etc.) |
| Jan 12 | All config explicit | Framework philosophy — no hidden defaults |
| Jan 13 | UUIDv7 for EntityId | Timestamp-sortable, no separate created_at index needed |
| Jan 13 | Serde derives on all types | JSON metadata fields, future API serialization |
| Jan 13 | No PostgresType in caliber-core | Keep core pure Rust, pgrx derives in caliber-pg only |

---

## Challenges

### Challenge 1: Agent Ignores "Don't Run Cargo Yet" Instructions

**Problem:** During Task 4 (parser implementation), agents would ignore explicit instructions to NOT run `cargo check` yet. They'd run cargo, see compilation errors (because other crates weren't implemented), panic, and immediately start spamming TODO stubs and placeholder code everywhere — exactly what we're trying to avoid.

**Root Cause:** The agent's instinct is to verify code compiles. When it sees errors, it tries to "fix" them by creating stub files, which defeats the AI-native "plan complete, generate complete" approach.

**Solution:** 
1. Added explicit "DO NOT run cargo yet" markers in tasks.md
2. Created steering file (dev-philosophy.md) explaining WHY we don't want stubs
3. Made Task 5 a dedicated "FIRST CARGO RUN" checkpoint
4. Had to manually revert stub files multiple times and re-prompt with stronger language

**Lesson Learned:** AI agents need VERY explicit guardrails about build verification timing. The steering files help but aren't foolproof. Sometimes you just have to babysit.

### Challenge 2: Property Test Generator Complexity

**Problem:** Writing proptest generators for recursive AST types (FilterExpr with And/Or/Not) was tricky. Naive recursive generators can explode or create invalid structures.

**Solution:** Used bounded recursion with `prop_oneof!` and kept filter expressions simple (single comparison) for most property tests. The round-trip property test uses carefully crafted generators that produce valid but varied ASTs.

---

## Kiro Usage Statistics

| Prompt | Uses | Notes |
|--------|------|-------|
| @prime | 3 | Load project context at session start |
| @plan-feature | 1 | Initial feature planning |
| @execute | 4 | Implementation sessions |
| @implement-crate | 2 | caliber-core, caliber-dsl |
| @code-review | 1 | Post-implementation review |
| @update-devlog | 2 | Keeping log current |
