# Pull Request

## Description

<!-- Provide a clear and concise description of your changes -->

## Related Issue

<!-- Link to the issue this PR addresses -->
Fixes #(issue number)

## Type of Change

<!-- Check all that apply -->

- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)
- [ ] ✨ New feature (non-breaking change that adds functionality)
- [ ] 💥 Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] 📝 Documentation update
- [ ] 🎨 Code style update (formatting, renaming)
- [ ] ♻️ Refactoring (no functional changes)
- [ ] ⚡ Performance improvement
- [ ] ✅ Test update
- [ ] 🔧 Build/CI update
- [ ] 🔒 Security fix

## Affected Components

<!-- Check all components affected by this PR -->

- [ ] caliber-core (Entity types, config)
- [ ] caliber-storage (Storage trait, backends)
- [ ] caliber-context (Context assembly)
- [ ] caliber-pcp (Validation, checkpoints)
- [ ] caliber-llm (VAL, embeddings)
- [ ] caliber-agents (Locks, messages, coordination)
- [ ] caliber-dsl (DSL parser)
- [ ] caliber-pg (PostgreSQL extension)
- [ ] caliber-api (REST/gRPC/WebSocket)
- [ ] caliber-tui (Terminal UI)
- [ ] Documentation
- [ ] Build system / CI/CD

## Changes Made

<!-- Detailed list of changes -->

### Core Changes
- 
- 

### API Changes (if applicable)
- 
- 

### Database Changes (if applicable)
- 
- 

### Breaking Changes (if applicable)
- 
- 

## Testing

### Verification Gates

<!-- All gates must pass before merge -->

- [ ] ✅ **Gate 1: Build** - `cargo build --workspace` succeeds
- [ ] ✅ **Gate 2: Clippy** - `cargo clippy --workspace -- -D warnings` passes (ZERO warnings)
- [ ] ✅ **Gate 3: Tests** - `cargo test --workspace` passes (ALL tests)
- [ ] ✅ **Gate 4: Format** - `cargo fmt --all -- --check` passes

### Test Coverage

<!-- Describe the tests you've added or updated -->

- [ ] Unit tests added/updated
- [ ] Property tests added/updated (100+ iterations)
- [ ] Integration tests added/updated
- [ ] Fuzz tests added/updated (if applicable)

**Test Details:**
```bash
# Commands run and results
cargo test -p caliber-core -- test_name
# Output: test result: ok. 1 passed; 0 failed
```

### Manual Testing

<!-- Describe manual testing performed -->

- [ ] Tested with PostgreSQL 13
- [ ] Tested with PostgreSQL 14
- [ ] Tested with PostgreSQL 15
- [ ] Tested with PostgreSQL 16
- [ ] Tested with PostgreSQL 17
- [ ] Tested in WSL environment
- [ ] Tested with caliber-api running
- [ ] Tested with caliber-tui
- [ ] Tested with Docker deployment
- [ ] Tested with Helm chart

**Manual Test Results:**
<!-- Describe what you tested manually and the results -->

## Performance Impact

<!-- Required for performance-related changes -->

- [ ] No performance impact
- [ ] Performance improved (include benchmarks)
- [ ] Performance degraded (explain why acceptable)
- [ ] Performance not measured (explain why)

**Benchmarks (if applicable):**
```
Before: 
After: 
Improvement: 
```

## Documentation

- [ ] Updated relevant documentation in `docs/`
- [ ] Updated CHANGELOG.md
- [ ] Updated README.md (if needed)
- [ ] Added/updated code comments
- [ ] Added/updated rustdoc comments
- [ ] Updated API documentation (if API changed)
- [ ] Updated DSL documentation (if DSL changed)

## Framework Philosophy Compliance

<!-- CALIBER has strict principles - confirm compliance -->

- [ ] **No hard-coded defaults** - All values explicitly configured
- [ ] **No stubs or TODOs** - All code complete and working
- [ ] **Type-first design** - Types defined in docs/DEPENDENCY_GRAPH.md (if new types)
- [ ] **CaliberResult<T>** - All fallible operations return CaliberResult
- [ ] **No unwrap() in production** - All errors handled with `?` operator
- [ ] **Direct heap operations** - No SQL in hot path (if caliber-pg changes)

## Security Considerations

<!-- Required for security-sensitive changes -->

- [ ] No security implications
- [ ] Security implications documented below
- [ ] Security review requested

**Security Notes:**
<!-- Describe any security implications -->

## Breaking Changes

<!-- Required if this is a breaking change -->

- [ ] No breaking changes
- [ ] Breaking changes documented below

**Breaking Changes Details:**
<!-- Describe what breaks and how users should migrate -->

**Migration Guide:**
```rust
// Before (old API)


// After (new API)

```

## Deployment Notes

<!-- Any special deployment considerations -->

- [ ] No special deployment steps
- [ ] Requires database migration
- [ ] Requires configuration changes
- [ ] Requires dependency updates
- [ ] Requires PostgreSQL restart
- [ ] Requires API restart

**Deployment Steps:**
<!-- List any special deployment steps -->

## Checklist

<!-- All items must be checked before merge -->

### Code Quality
- [ ] Code follows CALIBER style guidelines
- [ ] No clippy warnings (ran with `-D warnings`)
- [ ] No compiler warnings
- [ ] Code is properly formatted (`cargo fmt`)
- [ ] All tests pass locally
- [ ] Property tests run 100+ iterations
- [ ] No `unwrap()` or `expect()` in production code
- [ ] All public items have doc comments

### Testing
- [ ] Added tests for new functionality
- [ ] Updated tests for changed functionality
- [ ] All verification gates pass
- [ ] Manual testing completed

### Documentation
- [ ] Updated relevant documentation
- [ ] Added/updated code comments
- [ ] Updated CHANGELOG.md
- [ ] Breaking changes documented (if applicable)

### Review
- [ ] Self-reviewed the code
- [ ] Checked for potential security issues
- [ ] Verified no sensitive data in commits
- [ ] Squashed commits (if needed)
- [ ] Descriptive commit messages

## Additional Notes

<!-- Any additional information for reviewers -->

## Screenshots/Recordings (if applicable)

<!-- For UI changes, include screenshots or recordings -->

---

**For Reviewers:**

Please verify:
1. All verification gates pass in CI
2. Code follows CALIBER philosophy (no defaults, no stubs)
3. Tests are comprehensive (unit + property + integration)
4. Documentation is updated
5. Breaking changes are clearly documented
6. Security implications are considered
