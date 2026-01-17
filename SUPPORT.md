# Support

## Getting Help

### Documentation

Start here:
- **[README.md](README.md)** - Quick start and overview
- **[docs/QUICK_REFERENCE.md](docs/QUICK_REFERENCE.md)** - API cheat sheet
- **[docs/CALIBER_PCP_SPEC.md](docs/CALIBER_PCP_SPEC.md)** - Core specification
- **[CONTRIBUTING.md](CONTRIBUTING.md)** - Development guide

### Common Issues

#### Build Errors

```bash
# Clean build
cargo clean
cargo build --workspace --exclude caliber-pg

# WSL cache issues
cargo clean && cargo build --workspace
```

#### Test Failures

```bash
# Run tests with output
cargo test --workspace --exclude caliber-pg -- --nocapture

# Run specific test
cargo test -p caliber-core test_name
```

#### pgrx Issues

```bash
# Reinitialize pgrx
cargo pgrx init --pg16

# Check PostgreSQL version
psql --version
```

### Community Support

- **GitHub Issues:** [Open an issue](https://github.com/caliber-run/caliber/issues/new)
- **GitHub Discussions:** [Ask a question](https://github.com/caliber-run/caliber/discussions)
- **Documentation:** Check the `docs/` directory

### Bug Reports

When reporting bugs, include:
- CALIBER version
- Rust version (`rustc --version`)
- Operating system
- Steps to reproduce
- Expected vs actual behavior
- Error messages (full output)

### Feature Requests

When requesting features:
- Describe the use case
- Explain why existing features don't work
- Propose a solution (optional)
- Consider contributing it yourself!

## Commercial Support

For commercial support, consulting, or custom development:
- **Website:** [heyoub.dev](https://heyoub.dev)
- **Email:** Contact via website

## Security Issues

**DO NOT** open public issues for security vulnerabilities.

See [SECURITY.md](SECURITY.md) for reporting instructions.

## Contributing

Want to help? See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Response Times

- **Bug reports:** 1-3 days
- **Feature requests:** 1-7 days
- **Security issues:** 24-48 hours
- **Pull requests:** 1-5 days

*Note: These are best-effort timelines for open-source contributions.*

---

**Remember:** Check existing issues and documentation before asking. Someone may have already solved your problem!
