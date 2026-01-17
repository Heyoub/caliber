# Verification Gates: Lessons from Clippy Failure

## The Incident (January 16, 2026)

After "successful" build of caliber-api, ran `cargo clippy --workspace` and discovered:

- **31 compilation errors**
- **7 warnings**
- **14 locations** with incomplete security fix
- **17 functions** with wrong framework signatures
- **1 import path error**

**Root Cause:** Assumed "build succeeds" = "code complete"

**Impact:** 2-3 hours of rework

**Lesson:** Build success is only Phase 1 of 5.

---

## The Five Verification Gates

Every feature implementation MUST pass through these gates:

```text
Phase 1: Generate → Build
Phase 2: Build → Clippy      ← MOST CRITICAL
Phase 3: Clippy → Tests
Phase 4: Tests → Integration
Phase 5: Integration → Production
```

### Gate 1: Build

```bash
cargo build --workspace
```

**Verifies:**
- Code compiles
- Dependencies resolve
- Type system satisfied

**Does NOT verify:**
- Code quality
- Unused code
- Framework signatures
- Import correctness

### Gate 2: Clippy (CRITICAL)

```bash
cargo clippy --workspace -- -D warnings
```

**Verifies:**
- Zero warnings
- No unused imports
- No unused variables
- All code paths reachable
- Common mistakes caught

**Why critical:** Catches 90% of issues that "successful build" misses.

**Real example:** Clippy caught:
- 14 missing `tenant_id` fields (security fix incomplete)
- 17 wrong Axum handler signatures (framework mismatch)
- 1 wrong import path (assumed re-export)
- 7 unused variables (incomplete wiring)

### Gate 3: Tests

```bash
cargo test --workspace
```

**Verifies:**
- Unit tests pass
- Property tests pass (100+ iterations)
- Integration tests pass
- Behavior correctness

### Gate 4: Integration

```bash
# End-to-end testing with real dependencies
cargo test --workspace --test integration_*
```

**Verifies:**
- Components work together
- Real database operations
- Real API calls
- Performance acceptable

### Gate 5: Production

**Verifies:**
- Security audit complete
- Performance benchmarks met
- Documentation complete
- Deployment tested

---

## Common Failure Patterns

### Pattern 1: Skipping Clippy

**Symptom:** Build succeeds, mark as complete, discover issues later

**Example:**
```rust
// Builds fine, but clippy catches:
use axum::async_trait;  // ❌ Wrong import path
let unused_var = extract_value();  // ❌ Unused variable
```

**Fix:** Always run clippy before marking complete.

### Pattern 2: Partial Security Fixes

**Symptom:** Security issue identified, fix started but not completed

**Example:**
```rust
// Type definition updated
enum WsEvent {
    Created { tenant_id: Uuid, ... },  // ✅ Added field
}

// But 14 call sites not updated
broadcast(WsEvent::Created { ... });  // ❌ Missing tenant_id
```

**Fix:** Grep for ALL usage locations, update atomically.

### Pattern 3: Framework Version Mismatch

**Symptom:** Code uses API from older framework version

**Example:**
```rust
// Axum 0.7 style (AI training data)
async fn handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<...>

// Axum 0.8 requires different ordering
// Clippy error: Handler<_, _> trait not satisfied
```

**Fix:** Verify framework version, check current API docs.

### Pattern 4: Import Path Confusion

**Symptom:** Assumes re-exports that don't exist

**Example:**
```rust
use axum::async_trait;  // ❌ axum doesn't re-export
use async_trait::async_trait;  // ✅ Correct
```

**Fix:** Verify imports compile, check crate docs.

---

## Verification Workflow

### For Single Crate

```bash
# Phase 1: Build
cargo build -p caliber-{name}

# Phase 2: Clippy (REQUIRED)
cargo clippy -p caliber-{name} -- -D warnings

# Phase 3: Tests
cargo test -p caliber-{name}

# Mark complete only after all pass
```

### For Workspace

```bash
# Phase 1: Build
cargo build --workspace

# Phase 2: Clippy (REQUIRED)
cargo clippy --workspace -- -D warnings

# Phase 3: Tests
cargo test --workspace

# Mark complete only after all pass
```

### For Security Fixes

```bash
# Before implementing
rg "AffectedType::" --type rust  # Find all locations

# After implementing
cargo build --workspace
cargo clippy --workspace -- -D warnings  # Catches missed locations
cargo test --workspace  # Verify security property

# Add security-specific tests
cargo test security_*
```

---

## AI Code Smell Detection

When reviewing AI-generated code, watch for:

### Smell 1: Partial Feature Implementation

**Detection:**
```bash
# Grep for all usage locations
rg "TypeName::" --type rust

# Verify all locations updated
git diff | grep "TypeName::"
```

### Smell 2: Framework Version Mismatch

**Detection:**
```bash
# Check Cargo.toml version
grep "axum =" Cargo.toml

# Verify against current docs
# Use debug attributes
#[axum::debug_handler]  # Framework validates
```

### Smell 3: Unused Variables

**Detection:**
```bash
# Clippy catches these
cargo clippy -- -D warnings

# Look for pattern:
let var = extract();  # Extracted
// ... but never used
```

### Smell 4: Panic-Prone Code

**Detection:**
```bash
# Grep for dangerous patterns
rg "\.expect\(" --type rust
rg "\.unwrap\(" --type rust
rg "panic!\(" --type rust
```

---

## Completeness Checklist

Before marking ANY code complete:

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

### Security Phase (if applicable)
- [ ] Grep for all affected locations
- [ ] Verify all locations updated
- [ ] Add tests for security property
- [ ] Document security implications

### Framework Phase (if applicable)
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

## When to Deploy Strike Teams

For complex issues requiring parallel work:

### Strike Team Structure

| Team | Size | Mission | Example |
|------|------|---------|---------|
| Alpha | 2-3 Opus | Critical blocking issues | Import errors, type mismatches |
| Bravo | 2-3 Opus | Research + complex fixes | Framework API changes |
| Charlie | 1-2 Sonnet | Cleanup + minor issues | Unused imports, formatting |
| QA | 1 Opus | Final verification | Run all gates |

### Deployment Triggers

- **Blocking build errors:** Alpha team
- **Framework integration issues:** Bravo team
- **Security fixes:** Alpha + QA teams
- **Code quality cleanup:** Charlie team
- **Comprehensive audit:** All teams

### Coordination Protocol

1. Each team works on separate concerns (no overlap)
2. Teams document findings in DEVLOG.md
3. QA team verifies after all teams complete
4. Final clippy + test run before marking complete

---

## Summary: The Complete Workflow

```text
PLANNING
├── Design types in docs/
├── Define requirements
└── Create task list

GENERATION
├── Generate complete code (no stubs)
├── Reference type definitions
└── Create files with real implementations

VERIFICATION GATE 1: Build
├── cargo build --workspace
└── Fix compilation errors

VERIFICATION GATE 2: Clippy ← CRITICAL
├── cargo clippy --workspace -- -D warnings
├── Fix ALL warnings
├── Verify imports
├── Check unused code
└── Validate framework signatures

VERIFICATION GATE 3: Tests
├── cargo test --workspace
├── Property tests (100+ iterations)
└── Integration tests

VERIFICATION GATE 4: Security (if applicable)
├── Grep for all locations
├── Verify completeness
└── Add security tests

VERIFICATION GATE 5: Integration
├── End-to-end testing
├── Performance benchmarking
└── Production readiness

COMPLETION
└── Mark complete ONLY after ALL gates pass
```

**Remember:** Build success ≠ Complete. Clippy clean + Tests passing = Complete.

---

## Real-World Example: The Clippy Failure

### What Happened

1. ✅ Generated caliber-api code
2. ✅ `cargo build --workspace` succeeded
3. ❌ Marked as "complete"
4. ❌ Skipped clippy verification
5. ❌ Discovered 31 errors + 7 warnings later

### What Should Have Happened

1. ✅ Generated caliber-api code
2. ✅ `cargo build --workspace` succeeded
3. ✅ `cargo clippy --workspace -- -D warnings` run
4. ❌ 31 errors + 7 warnings discovered
5. ✅ Fixed all issues
6. ✅ Clippy clean
7. ✅ Tests pass
8. ✅ NOW mark as complete

### Time Impact

- **Skipping clippy:** 2-3 hours of rework
- **Running clippy:** 15 minutes of fixes

**Lesson:** Clippy saves time, doesn't waste it.

---

## Integration with Existing Workflow

This verification gate system integrates with existing CALIBER development philosophy:

- **"No Stubs" philosophy:** Still applies - generate complete code
- **"Plan Complete, Generate Complete":** Still applies - but add verification
- **Type-first design:** Still applies - docs/DEPENDENCY_GRAPH.md
- **Property-based testing:** Still applies - 100+ iterations

**What's new:** Multi-phase verification instead of single-pass generation.

**Why:** AI can generate 95% correct code, but final 5% needs verification gates.
