# Config Presets and Hard-Value Audit

This document defines the preset-first philosophy for numeric knobs and
lists current hard-coded values that should be moved to explicit profiles.

## Preset-First Philosophy

- Presets are explicit configuration choices, not implicit defaults.
- Presets are versioned and validated; invalid combinations are rejected.
- Overrides are opt-in and must be explicitly declared by the user.
- Presets exist to prevent unsafe or inconsistent configs in production.

## Preset Catalog (Draft)

These presets represent the intended configuration surface for numeric knobs.
They are not implicit defaults; users must select a preset or provide overrides.

### Deduplication TTL (Bitemporal)

- strict: wall 6h, event 24h
- balanced: wall 24h, event 7d
- relaxed: wall 72h, event 30d

### Ack Timeout Mapping (Backoff Profiles)

- fast: base 200ms, max 30s, attempts 4, ack timeout 30s
- safe: base 1s, max 2m, attempts 4, ack timeout 2m
- conservative: base 2s, max 5m, attempts 5, ack timeout 5m

### Compatibility Window (Protocol Versions)

- short: 90d
- balanced: 180d
- long: 365d

### Vector Clock TTL (Tiered)

- lean: 256=6h, 1024=48h, 4096=7d
- balanced: 256=24h, 1024=7d, 4096=30d
- safe: 256=7d, 1024=30d, 4096=90d

### Error Redaction Templates

- core: redact secrets, credentials, tokens, and keys
- extended: core + financial, location, biometric
- strict: extended + tag-required exposure (untagged fields redacted)

## Current Hard-Value Audit (Candidates for Presets)

These are the current hard-coded values in the codebase that should migrate
into explicit configuration profiles over time.

### API and Gateway Defaults

- API rate limits, CORS, and timeouts
  - `caliber-api/src/config.rs:60` defaults for rate limiting and CORS.
  - `caliber-api/src/db.rs:52` default DB timeout is 30s.
  - `caliber-api/src/db.rs:72` default pool size is 16.
  - `caliber-api/src/db.rs:65` default port is 5432.

- REST list and search limits
  - `caliber-api/src/routes/artifact.rs:158` default list limit 100.
  - `caliber-api/src/routes/note.rs:193` default list limit 100.
  - `caliber-api/src/routes/mcp.rs:798` default MCP limit 10.
  - `caliber-api/src/routes/mcp.rs:905` default token_budget 4096.

- Billing defaults
  - `caliber-api/src/routes/billing.rs:242` default LemonSqueezy variants.
  - `caliber-api/src/db.rs:2817` default storage limit 1 GB.
  - `caliber-api/src/db.rs:2819` default hot cache limit 100 MB.

### LLM Defaults

- `caliber-llm/src/lib.rs:313` default timeout 30s.
- `caliber-llm/src/lib.rs:449` default health cache TTL 60s.

### Core Config Examples (Test-Only)

- `caliber-core/src/lib.rs:1565` token_budget 8000 (test samples).
- `caliber-core/src/lib.rs:1575` checkpoint_retention 10 (test samples).
- `caliber-core/src/lib.rs:1588` lock_timeout 30s (test samples).
- `caliber-core/src/lib.rs:1589` message_retention 86400s (test samples).
- `caliber-core/src/lib.rs:1590` delegation_timeout 300s (test samples).

### PCP Spec Test Config (Spec-Only)

- `docs/CALIBER_PCP_SPEC.md:2156` test_config values for PCPConfig.

## Migration Plan (Recommended)

1) Introduce explicit preset names in config structs.
2) Migrate hard-coded defaults into preset definitions.
3) Require preset selection in production deployments.
4) Validate preset combinations and reject invalid mixes.

