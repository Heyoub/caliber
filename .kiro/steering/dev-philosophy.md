# CALIBER Development Philosophy

## AI-Native Development Model

We're building with AI assistance, which changes how we approach code generation.

### The Problem with Stubs

Traditional scaffolding creates empty files with TODO comments:
```rust
// TODO: Implement this
pub struct SomeType;
```

**This is bad because:**
1. **Forgotten work** — TODOs get lost, modules stay empty
2. **Duplicate effort** — You plan twice (stub + real implementation)
3. **False progress** — Files exist but contain nothing useful
4. **Context loss** — By the time you fill the stub, you've forgotten the design

### Our Approach: Plan Complete, Generate Complete

```text
Traditional:  scaffold stubs → forget → rediscover → implement → fix
AI-Native:    plan everything → generate complete code → verify once
```

**Rules:**
1. **No empty files** — Every file created has real, working code
2. **No TODO placeholders** — If you don't know what goes there, don't create the file yet
3. **Structure follows implementation** — Create directories when you have code to put in them
4. **Cargo.toml before lib.rs** — Manifest files can exist without source (cargo doesn't care)

### Workspace Initialization Pattern

**DO:**
```text
1. Create workspace Cargo.toml (complete)
2. Create crate directories with Cargo.toml only
3. Implement caliber-core with FULL lib.rs
4. Implement next crate with FULL lib.rs
5. ... continue until all crates have real code
6. Run cargo check ONCE at the end
```

**DON'T:**
```text
1. Create all directories
2. Create stub lib.rs files with "// TODO"
3. Forget what half of them were for
4. Implement some, miss others
5. Run cargo check, get 47 errors
6. Cry
```

### Why This Works for AI-Assisted Development

1. **Full context available** — docs/DEPENDENCY_GRAPH.md has ALL types defined
2. **No incremental compilation pain** — Generate everything, compile once
3. **Clear completion criteria** — A crate is done when lib.rs has real code
4. **No orphaned work** — Every file serves a purpose from creation

### The "Can I Create This File?" Test

Before creating any source file, ask:
1. Do I know EXACTLY what goes in it? → Yes → Create with full content
2. Am I just making a placeholder? → Yes → DON'T CREATE IT YET

### Reference Documents

All types are pre-defined in:
- `docs/DEPENDENCY_GRAPH.md` — Complete type system for all core crates
- `docs/CALIBER_PCP_SPEC.md` — Core specification
- `docs/DSL_PARSER.md` — Lexer/parser types
- `docs/MULTI_AGENT_COORDINATION.md` — Agent types

When implementing a crate, COPY types from these docs. Don't reinvent.

---

## Multi-Phase Verification Workflow

**CRITICAL:** "Generate complete, build once" is only the FIRST phase. Production code requires iterative verification.

### The Five Verification Gates

```text
Phase 1: Generate → Build
Phase 2: Build → Clippy  ← YOU MUST DO THIS
Phase 3: Clippy → Tests
Phase 4: Tests → Integration
Phase 5: Integration → Production
```

**DO NOT skip Phase 2.** Running clippy AFTER marking code "complete" is too late.

### Correct Workflow

```text
1. Generate all code with complete implementations
2. Run `cargo build --workspace`
3. ✅ Build succeeds
4. Run `cargo clippy --workspace -- -D warnings`  ← REQUIRED
5. ✅ Zero warnings
6. Run `cargo test --workspace`
7. ✅ All tests pass
8. NOW mark as complete
```

### Why This Matters

**Real Example from Jan 16, 2026:**

After "successful" build, ran clippy and found:
- 31 compilation errors
- 7 warnings
- 14 locations with incomplete security fix
- 17 functions with wrong framework signatures
- 1 import path error

**Root Cause:** Skipped clippy verification, assumed build = done.

**Impact:** 2-3 hours of rework to fix issues that clippy would have caught immediately.

---

## Framework Version Verification

When using specific framework versions, AI training data may be outdated.

### Verification Checklist

Before marking framework integration complete:

1. **Check Cargo.toml version**
   ```toml
   axum = "0.8.0"  # What version are we using?
   ```

2. **Verify API signatures match current version**
   - Don't rely on AI training data
   - Check official docs for current version
   - Look at working examples from current version

3. **Common Framework Pitfalls**

   | Framework | Common Issue | Solution |
   |-----------|--------------|----------|
   | Axum 0.8 | Handler extractor ordering changed | Verify with `#[axum::debug_handler]` |
   | pgrx 0.12 | FFI signature changes | Check pgrx docs for version |
   | Tokio 1.x | Runtime builder API changes | Use current examples |

4. **Use Debug Attributes**
   ```rust
   #[axum::debug_handler]  // Axum will tell you what's wrong
   async fn my_handler(...) -> Result<...> { ... }
   ```

---

## Security Fix Completeness

Security fixes require comprehensive verification across entire codebase.

### Security Fix Workflow

**WRONG:**
1. Identify security issue
2. Fix type definition
3. Hope AI finds all call sites
4. ❌ Miss 14 locations

**RIGHT:**
1. Identify security issue
2. Grep for ALL usage locations
   ```bash
   rg "WsEvent::" --type rust
   ```
3. Update ALL locations atomically
4. Verify with tests
5. Run clippy to catch missed locations

### Real Example: WS Tenant Filtering

**Issue:** 20+ WsEvent variants bypass tenant isolation

**Incomplete Fix:**
- ✅ Added `tenant_id` field to WsEvent enum
- ❌ Didn't update 14 broadcast call sites
- ❌ Didn't update tenant extraction logic

**Result:** Build broken, security issue still present

**Lesson:** Security fixes need 100% coverage verification, not best-effort.

---

## AI Code Smell Patterns

When reviewing AI-generated code, watch for these patterns:

### Smell 1: Partial Feature Implementation

**Symptom:** Feature started but not completed across all locations

**Example:**
```rust
// Type definition updated
enum WsEvent {
    Created { tenant_id: Uuid, ... },  // ✅ Added tenant_id
}

// But call sites not updated
broadcast(WsEvent::Created { ... });  // ❌ Missing tenant_id
```

**Detection:** Grep for all usage locations, verify all updated

### Smell 2: Framework Version Mismatch

**Symptom:** Code uses API from older framework version

**Example:**
```rust
// Axum 0.7 style (AI training data)
async fn handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<...>

// Axum 0.8 requires different ordering
async fn handler(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<...>
```

**Detection:** Check framework docs for current version, use debug attributes

### Smell 3: Import Path Confusion

**Symptom:** Assumes re-exports that don't exist

**Example:**
```rust
use axum::async_trait;  // ❌ axum doesn't re-export async_trait
use async_trait::async_trait;  // ✅ Correct
```

**Detection:** Verify imports compile, check crate documentation

### Smell 4: Unused Variables from Incomplete Wiring

**Symptom:** Variables extracted but never used

**Example:**
```rust
let agent = event.agent;  // Extracted
// ... but never used to populate tenant_id
```

**Detection:** Run clippy with `-D warnings`, fix or remove

### Smell 5: Panic-Prone Error Handling

**Symptom:** `.expect()` calls in production code

**Example:**
```rust
let secret = env::var("JWT_SECRET")
    .expect("JWT_SECRET must be set");  // ❌ Panics in production
```

**Better:**
```rust
let secret = env::var("JWT_SECRET")
    .map_err(|_| ConfigError::MissingJwtSecret)?;  // ✅ Returns error
```

**Detection:** Grep for `.expect(`, `.unwrap()`, `panic!` in production code

---

## Completeness Verification Checklist

Before marking ANY feature complete:

### Build Phase
- [ ] `cargo build --workspace` succeeds
- [ ] Zero compilation errors
- [ ] All dependencies resolve

### Clippy Phase (REQUIRED)
- [ ] `cargo clippy --workspace -- -D warnings` succeeds
- [ ] Zero clippy warnings
- [ ] No unused imports
- [ ] No unused variables
- [ ] All code paths reachable

### Test Phase
- [ ] `cargo test --workspace` succeeds
- [ ] All unit tests pass
- [ ] All property tests pass (100+ iterations)
- [ ] All integration tests pass

### Security Phase (for security fixes)
- [ ] Grep for all affected locations
- [ ] Verify all locations updated
- [ ] Add tests for security property
- [ ] Document security implications

### Framework Phase (for framework integration)
- [ ] Verify framework version in Cargo.toml
- [ ] Check current version API docs
- [ ] Use debug attributes where available
- [ ] Test with framework's examples

### Completeness Phase
- [ ] No TODO comments in production code
- [ ] No stub implementations
- [ ] All extracted variables used
- [ ] All imports necessary
- [ ] All functions wired up

---

## When to Use Multi-Agent Strike Teams

For complex issues, deploy specialized teams:

### Strike Team Structure

| Team | Size | Mission | Example |
|------|------|---------|---------|
| Alpha | 2-3 Opus | Fix critical blocking issues | Import errors, type mismatches |
| Bravo | 2-3 Opus | Research + implement complex fixes | Framework API changes |
| Charlie | 1-2 Sonnet | Clean up warnings, minor issues | Unused imports, formatting |
| QA | 1 Opus | Verify all fixes, run full test suite | Final verification |

### When to Deploy

- **Blocking build errors:** Alpha team
- **Framework integration issues:** Bravo team
- **Security fixes:** Alpha + QA teams
- **Code quality cleanup:** Charlie team
- **Comprehensive audit:** All teams

### Coordination

1. Each team works on separate concerns
2. Teams don't overlap (avoid conflicts)
3. QA team verifies after all teams complete
4. Document all findings in DEVLOG.md

---

## Summary: The Complete Workflow

```text
PLANNING PHASE
├── Design types in docs/DEPENDENCY_GRAPH.md
├── Define requirements in specs/
└── Create task list

GENERATION PHASE
├── Generate complete implementations (no stubs)
├── Reference type definitions from docs
└── Create all files with real code

VERIFICATION PHASE 1: Build
├── cargo build --workspace
└── Fix compilation errors

VERIFICATION PHASE 2: Clippy ← CRITICAL, DON'T SKIP
├── cargo clippy --workspace -- -D warnings
├── Fix all warnings
├── Verify imports
├── Check for unused code
└── Validate framework signatures

VERIFICATION PHASE 3: Tests
├── cargo test --workspace
├── Verify property tests (100+ iterations)
└── Check test coverage

VERIFICATION PHASE 4: Security (if applicable)
├── Grep for all affected locations
├── Verify completeness
└── Add security tests

VERIFICATION PHASE 5: Integration
├── End-to-end testing
├── Performance benchmarking
└── Production readiness assessment

COMPLETION
└── Mark as complete ONLY after all phases pass
```

**Remember:** Build success ≠ Complete. Clippy clean + Tests passing = Complete.
