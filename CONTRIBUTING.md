# Contributing to CALIBER

Thanks for your interest in contributing to CALIBER! This document provides guidelines and instructions for contributing.

## Code of Conduct

Be respectful, constructive, and professional. We're all here to build cool shit.

## Getting Started

### Prerequisites

- **Rust** 1.75+ ([rustup](https://rustup.rs/))
- **PostgreSQL** 13-17 (optional for core development)
- **Git**

### Development Setup

```bash
# Clone the repo
git clone https://github.com/your-org/caliber.git
cd caliber

# Build everything (excluding pgrx extension)
cargo build --workspace --exclude caliber-pg

# Run tests
cargo test --workspace --exclude caliber-pg

# Run clippy
cargo clippy --workspace --exclude caliber-pg -- -D warnings

# Format code
cargo fmt --all
```

### With PostgreSQL

```bash
# Install pgrx CLI
cargo install cargo-pgrx

# Initialize pgrx
cargo pgrx init

# Build extension
cargo build -p caliber-pg

# Run pgrx tests
cargo pgrx test -p caliber-pg
```

## Development Philosophy

CALIBER follows strict principles:

### 1. No Stubs, No TODOs

Every file created must have complete, working code. No placeholders.

```rust
// ❌ WRONG
pub fn my_function() {
    todo!("implement this later")
}

// ✅ RIGHT
pub fn my_function() -> CaliberResult<T> {
    // Complete implementation
    Ok(result)
}
```

### 2. No Hard-Coded Defaults

CALIBER is a framework, not a product. Users must configure everything.

```rust
// ❌ WRONG
const DEFAULT_TOKEN_BUDGET: i32 = 8000;

// ✅ RIGHT
pub struct CaliberConfig {
    pub token_budget: i32,  // Required, no default
}
```

### 3. Type-First Design

All types are defined in `docs/DEPENDENCY_GRAPH.md` before implementation.

### 4. Property-Based Testing

Use proptest for universal correctness properties (100+ iterations).

```rust
proptest! {
    #[test]
    fn prop_round_trip(data in arb_trajectory()) {
        let serialized = serialize(&data)?;
        let deserialized = deserialize(&serialized)?;
        prop_assert_eq!(data, deserialized);
    }
}
```

### 5. No SQL in Hot Path

Use direct heap operations via pgrx, not SPI.

```rust
// ❌ WRONG - SPI in hot path
Spi::run("INSERT INTO caliber_artifact ...")?;

// ✅ RIGHT - Direct heap operations
let tuple = form_tuple(&rel, &values, &nulls)?;
insert_tuple(&rel, tuple)?;
```

## Contribution Workflow

### 1. Find or Create an Issue

- Check existing issues first
- For bugs: include reproduction steps, expected vs actual behavior
- For features: explain use case and proposed design

### 2. Fork and Branch

```bash
# Fork on GitHub, then:
git clone https://github.com/YOUR_USERNAME/caliber.git
cd caliber
git checkout -b feature/your-feature-name
```

### 3. Make Changes

- Follow the development philosophy above
- Write tests for new functionality
- Update documentation if needed
- Run verification gates before committing

### 4. Verification Gates

**CRITICAL:** All code must pass these gates:

```bash
# Gate 1: Build
cargo build --workspace --exclude caliber-pg

# Gate 2: Clippy (ZERO warnings)
cargo clippy --workspace --exclude caliber-pg -- -D warnings

# Gate 3: Tests (ALL passing)
cargo test --workspace --exclude caliber-pg

# Gate 4: Format
cargo fmt --all -- --check
```

### 5. Commit

Use conventional commits:

```
feat: add vector search optimization
fix: correct lock timeout handling
docs: update DSL parser documentation
test: add property tests for artifact round-trip
refactor: simplify context assembly logic
perf: optimize heap tuple formation
```

### 6. Push and Create PR

```bash
git push origin feature/your-feature-name
```

Then create a PR on GitHub with:
- Clear description of changes
- Link to related issue
- Screenshots/examples if applicable
- Confirmation that all verification gates passed

## Code Style

### Rust Conventions

- Use `CaliberResult<T>` for all fallible operations
- No `unwrap()` or `expect()` in production code
- All public items have doc comments
- Use `?` operator for error propagation

```rust
/// Creates a new trajectory.
///
/// # Arguments
/// * `name` - Human-readable trajectory name
/// * `agent_id` - Optional agent identifier
///
/// # Returns
/// The created trajectory ID
///
/// # Errors
/// Returns `StorageError` if insertion fails
pub fn create_trajectory(
    name: &str,
    agent_id: Option<EntityId>,
) -> CaliberResult<EntityId> {
    // Implementation
}
```

### Error Handling

```rust
// ❌ WRONG
let value = operation().unwrap();

// ✅ RIGHT
let value = operation()?;
```

### Imports

Group imports logically:

```rust
// Standard library
use std::collections::HashMap;
use std::sync::Arc;

// External crates
use uuid::Uuid;
use serde::{Deserialize, Serialize};

// Internal crates
use caliber_core::{EntityId, CaliberResult};
use caliber_storage::StorageTrait;

// Local modules
use crate::heap_ops::insert_tuple;
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trajectory_creation() {
        let trajectory = Trajectory::new("test");
        assert_eq!(trajectory.name, "test");
    }
}
```

### Property Tests

```rust
#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_config_validation(
            token_budget in 1..100000i32,
            threshold in 0.0..1.0f32,
        ) {
            let config = CaliberConfig {
                token_budget,
                contradiction_threshold: threshold,
                // ... other fields
            };
            prop_assert!(config.validate().is_ok());
        }
    }
}
```

### Integration Tests

Place in `tests/` directory:

```rust
// tests/integration_test.rs
use caliber_core::*;
use caliber_storage::*;

#[test]
fn test_full_workflow() {
    // Test complete user workflow
}
```

## Documentation

### Code Comments

- Public APIs: comprehensive doc comments
- Complex logic: inline comments explaining why
- No obvious comments (e.g., `// increment counter` for `counter += 1`)

### Documentation Files

Update relevant docs in `docs/`:
- `CALIBER_PCP_SPEC.md` - Core specification
- `DSL_PARSER.md` - DSL grammar changes
- `DEPENDENCY_GRAPH.md` - Type system changes
- `QUICK_REFERENCE.md` - API changes

## Performance Considerations

- Profile before optimizing
- Use `cargo bench` for benchmarks
- Avoid allocations in hot paths
- Use direct heap operations, not SPI
- Consider cache locality

## Security

- No secrets in code or commits
- Validate all user input
- Use prepared statements (when SPI is necessary)
- Follow principle of least privilege
- Report security issues privately to maintainers

## Questions?

- Open an issue for questions
- Check existing documentation in `docs/`
- Review `.kiro/steering/` for development philosophy

## License

By contributing, you agree that your contributions will be licensed under AGPL-3.0.

---

**Remember:** Build succeeds ≠ Complete. Clippy clean + Tests passing = Complete.
