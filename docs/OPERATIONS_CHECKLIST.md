# Operations and Production Checklist

This checklist is the production readiness baseline for CALIBER deployments.
It is designed for operators who want safe defaults, explicit configuration,
and a predictable rollout path.

## 1) Build and Release Readiness

- Verify `cargo build --workspace --exclude caliber-pg` passes.
- Verify `TMPDIR=$PWD/target/tmp cargo test --workspace --exclude caliber-pg` passes.
- Verify `TMPDIR=$PWD/target/tmp cargo clippy --workspace --exclude caliber-pg -- -D warnings` passes.
- If running DB-backed API property tests, set `CALIBER_DB_*` env vars and run with `--features db-tests`.
- Verify formatting (add `rustfmt.toml` if you want repo-wide enforcement).
- Verify dependency policy (add `deny.toml` if you want supply-chain gates).
- Record the build matrix (Rust version, PostgreSQL version, pgrx version).

## 2) PostgreSQL and pgrx

- PostgreSQL 18+ is required.
- Install pgvector into the server before enabling `caliber_pg`.
- `cargo pgrx init` must run on the deployment host.
- Install the extension into the target Postgres (`cargo pgrx install --package caliber-pg --pg-config "/usr/lib/postgresql/18/bin/pg_config"`).
- Run `cargo pgrx test pg18 --package caliber-pg` for extension tests.
- Extension smoke test: `CREATE EXTENSION caliber_pg;`

## 3) Migrations and Schema

- Validate `_PG_init()` runs migrations on startup for new clusters.
- For upgrades, verify `caliber_schema_version` reflects expected migration level.
- For rollback planning, document schema changes per release.

## 4) Authentication and Tenancy

- Set `CALIBER_JWT_SECRET` (minimum 32 chars in production).
- Set `CALIBER_API_KEYS` for service-to-service access.
- Enforce `X-Tenant-ID` when multi-tenant isolation is required.
- If WorkOS is enabled, ensure WorkOS env vars are set and validated at startup.

## 5) CORS and Rate Limiting

- Set `CALIBER_CORS_ORIGINS` in production.
- Set `CALIBER_RATE_LIMIT_ENABLED=true` for public deployments.
- Confirm rate limit configuration matches your intended traffic profile.

## 6) Observability

- Enable structured logs at INFO in production.
- Verify `/metrics` is reachable and scraped by Prometheus.
- Verify request tracing is enabled in your tracing backend.
- Consider adding DB-level timing for pgrx heap operations if you need perf SLAs.

## 7) External Interfaces

- REST API is under `/api/v1/*`.
- MCP endpoint is under `/mcp/*` and not versioned with REST.
- WebSocket endpoint is `/api/v1/ws` and requires auth and tenant context.
- OpenAPI JSON is `/openapi.json` and Swagger UI is `/swagger-ui` (feature-gated).

## 8) Config Profiles and Defaults

- Prefer explicit presets for numeric knobs and document overrides.
- Avoid untracked defaults in production; document all overrides.

## 9) Backup and Recovery

- Snapshot the Postgres cluster at least daily in production.
- Define restore runbooks for tenant-level recovery scenarios.

## 10) Security Hygiene

- Keep secrets out of source control.
- Rotate API keys on schedule.
- Review auth logs for tenant boundary violations.
