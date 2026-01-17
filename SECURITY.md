# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.2.x   | :white_check_mark: |
| < 0.2   | :x:                |

## Reporting a Vulnerability

**DO NOT** open a public issue for security vulnerabilities.

Instead, please report security issues privately:

1. **Email:** security@heyoub.dev (if available)
2. **GitHub:** Use GitHub's private vulnerability reporting feature
3. **Direct Contact:** Reach out via the contact info in the repository

### What to Include

Please provide:

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if you have one)
- Your contact information for follow-up

### Response Timeline

- **Initial Response:** Within 48 hours
- **Status Update:** Within 7 days
- **Fix Timeline:** Depends on severity
  - Critical: 1-7 days
  - High: 7-14 days
  - Medium: 14-30 days
  - Low: 30-90 days

### Disclosure Policy

- We follow coordinated disclosure
- We'll work with you to understand and fix the issue
- We'll credit you in the security advisory (unless you prefer anonymity)
- We'll publish a security advisory after the fix is released

## Security Considerations

### CALIBER-Specific Concerns

#### 1. SQL Injection

CALIBER uses direct heap operations to avoid SQL injection in hot paths. However:

- Cold-path operations (schema init) use SPI
- User-provided DSL is parsed, not executed as SQL
- Always validate input before passing to any SQL operation

#### 2. Access Control

- Memory regions have read/write permissions
- Agents must be authenticated before operations
- Collaborative regions require lock acquisition
- Validate tenant isolation in multi-tenant deployments

#### 3. Resource Exhaustion

- Token budgets prevent unbounded context growth
- Lock timeouts prevent deadlocks
- Message retention limits prevent queue overflow
- Checkpoint retention limits prevent storage exhaustion

#### 4. Data Integrity

- Content hashing prevents tampering
- Checksums verify artifact integrity
- Contradiction detection catches inconsistencies
- Validation mode enforces data quality

#### 5. Secrets Management

- Never hard-code secrets
- Use environment variables or secret managers
- Rotate credentials regularly
- Use least-privilege access

### PostgreSQL Extension Security

#### pgrx Safety

- All heap operations use safe wrappers
- No raw pointer dereference without safety comments
- Lock modes prevent race conditions
- Transaction isolation prevents dirty reads

#### Extension Privileges

- CALIBER extension requires `CREATE EXTENSION` privilege
- Functions respect PostgreSQL's role-based access control
- Debug functions are feature-gated
- Production deployments should disable debug features

### Network Security

#### API Security

- Use TLS for all network communication
- Implement rate limiting
- Validate JWT tokens
- Sanitize all input

#### WebSocket Security

- Authenticate before subscription
- Validate tenant isolation
- Implement connection limits
- Use secure WebSocket (wss://)

### Deployment Security

#### Docker

- Use minimal base images
- Run as non-root user
- Scan images for vulnerabilities
- Keep dependencies updated

#### Kubernetes

- Use network policies
- Implement pod security policies
- Use secrets for sensitive data
- Enable audit logging

## Known Security Limitations

### 1. In-Memory Metrics

Current implementation uses in-memory storage for operation metrics. This is session-local and not suitable for production monitoring.

**Mitigation:** Use external monitoring (Prometheus, Grafana)

### 2. Advisory Locks

Advisory locks are cooperative and can be bypassed by malicious code with direct database access.

**Mitigation:** Use PostgreSQL's role-based access control

### 3. Embedding Vectors

Embedding vectors are stored as-is without encryption.

**Mitigation:** Use PostgreSQL's transparent data encryption (TDE) if needed

### 4. Debug Functions

Debug functions (`caliber_debug_clear`, `caliber_debug_dump_*`) can expose sensitive data.

**Mitigation:** Disable debug feature in production builds

## Security Best Practices

### For Users

1. **Authentication:** Always use strong authentication (JWT, API keys)
2. **Authorization:** Implement proper access control
3. **Encryption:** Use TLS for all network traffic
4. **Monitoring:** Enable audit logging and monitoring
5. **Updates:** Keep CALIBER and dependencies updated
6. **Backups:** Regular backups with encryption
7. **Secrets:** Use secret managers, not environment variables in production

### For Contributors

1. **Input Validation:** Validate all user input
2. **Error Handling:** Never expose internal details in errors
3. **Dependencies:** Keep dependencies minimal and updated
4. **Testing:** Include security-focused tests
5. **Code Review:** All changes require review
6. **Static Analysis:** Run clippy with security lints
7. **Fuzzing:** Use cargo-fuzz for critical parsers

## Security Tooling

### Recommended Tools

```bash
# Security audit
cargo install cargo-audit
cargo audit

# Dependency checking
cargo install cargo-deny
cargo deny check

# Fuzzing
cargo install cargo-fuzz
cargo +nightly fuzz run lexer_fuzz

# Static analysis
cargo clippy -- -W clippy::all -W clippy::pedantic
```

### CI/CD Integration

Our CI pipeline includes:
- `cargo audit` for known vulnerabilities
- `cargo deny` for dependency policies
- `cargo clippy` with security lints
- Automated dependency updates via Dependabot

## Acknowledgments

We appreciate responsible disclosure and will acknowledge security researchers who help improve CALIBER's security.

---

**Remember:** Security is everyone's responsibility. If you see something, say something.
