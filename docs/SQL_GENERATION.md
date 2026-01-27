# SQL Generation and Extension Install Flow

This document describes how CALIBER's PostgreSQL extension SQL is generated, validated, and installed.

## Authoritative SQL Source

The **repo-checked-in SQL files are authoritative**. Generated SQL from `cargo pgrx schema` is validated against these files, but the repo files take precedence.

### SQL Files

| File | Purpose |
|------|---------|
| `caliber-pg/sql/caliber_pg--{version}.sql` | Versioned extension SQL (e.g., `caliber_pg--0.4.4.sql`). Contains the complete schema for `CREATE EXTENSION caliber_pg`. |
| `caliber-pg/sql/caliber_init.sql` | Bootstrap schema. Creates core tables (`caliber_config`, `caliber_schema_version`) and default configuration. |
| `caliber-pg/sql/migrations/V*` | Incremental migrations (e.g., `V2__tenant_isolation.sql`, `V4__dsl_config.sql`). Applied in version order. |

### Version Management

The extension version comes from `Cargo.toml`:

```toml
[workspace.package]
version = "0.4.4"
```

The control file (`caliber-pg/caliber_pg.control`) uses `@CARGO_VERSION@` placeholder:

```
default_version = '@CARGO_VERSION@'
```

When a new version is released, create a matching `caliber_pg--{version}.sql` file.

## Generation Flow

### How `cargo pgrx schema` Works

The `cargo pgrx schema` command generates SQL from Rust `#[pg_extern]` functions and pgrx macros:

```bash
cargo pgrx schema \
  --package caliber-pg \
  --pg-config /path/to/pg_config \
  --features pg18 \
  --no-default-features
```

This produces SQL for all PostgreSQL functions defined in Rust code. The output is captured to a file or stdout.

### CI Validation (extension-sql-check.yml)

The CI workflow validates that schema generation works and produces expected SQL:

1. **Setup**: Install `cargo-pgrx`, download PostgreSQL 18 via `cargo pgrx init --pg18 download`
2. **Generate**: Run `cargo pgrx schema` and capture output
3. **Validate**: Check that the generated SQL includes key functions (e.g., `caliber_agent_register`)
4. **Artifact**: Upload generated SQL and logs for debugging

If validation fails, CI provides deterministic next steps:
- Ensure pgrx can download PostgreSQL
- Re-run schema with explicit directories
- Inspect pgrx log for errors

### test.sh Fallback Logic

The `scripts/test.sh` script (lines 126-173) handles unreliable pgrx schema generation:

1. **Run schema generation**: Capture stdout to temp file
2. **Validate output**: Check for expected table (`caliber_dsl_config`)
3. **Fallback**: If generation fails or produces empty output, use repo SQL:
   ```bash
   # Prefer generated SQL if valid
   if rg -q "CREATE TABLE IF NOT EXISTS caliber_dsl_config" "${PG_SQL_GEN}"; then
     sudo cp -f "${PG_SQL_GEN}" "${PG_SQL_DST}"
   # Fall back to repo SQL
   elif rg -q "CREATE TABLE IF NOT EXISTS caliber_dsl_config" "${PG_SQL_SRC}"; then
     sudo cp -f "${PG_SQL_SRC}" "${PG_SQL_DST}"
   fi
   ```

## Local Development

### If pgrx Schema Produces Empty Output

This is a known issue. Use the repo SQL as fallback:

```bash
# Check if repo SQL exists
ls -la caliber-pg/sql/caliber_pg--*.sql

# Copy repo SQL to PostgreSQL extension directory
PG_VERSION=18
VERSION=$(awk -F'"' '/\[workspace\.package\]/,/version/ {if ($2 ~ /^[0-9]/) print $2}' Cargo.toml)
sudo cp caliber-pg/sql/caliber_pg--${VERSION}.sql \
  /usr/share/postgresql/${PG_VERSION}/extension/
```

### Running Schema Generation Manually

```bash
# Create temp directories
mkdir -p target/tmp target/schema-target

# Generate schema with explicit dirs
TMPDIR=target/tmp CARGO_TARGET_DIR=target/schema-target \
  cargo pgrx schema \
    --package caliber-pg \
    --pg-config /usr/lib/postgresql/18/bin/pg_config \
    --features pg18 \
    --no-default-features > target/tmp/schema.sql

# Check if valid
rg "CREATE FUNCTION caliber" target/tmp/schema.sql
```

## Extension Install

### Using test.sh (Recommended)

```bash
# Full install with extension
CALIBER_PG_INSTALL=1 ./scripts/test.sh
```

This:
1. Runs `cargo pgrx install`
2. Generates SQL schema
3. Copies versioned SQL to PostgreSQL extension directory
4. Falls back to repo SQL if generation fails

### Using Makefile

```bash
cd caliber-pg

# Build and install
make install PG_CONFIG=/usr/lib/postgresql/18/bin/pg_config

# Or with explicit DESTDIR
sudo make install DESTDIR=/
```

### Manual Install

```bash
# Install shared library and control file
sudo cargo pgrx install --package caliber-pg \
  --pg-config /usr/lib/postgresql/18/bin/pg_config

# Copy versioned SQL (if not copied by pgrx)
VERSION=0.4.4
sudo cp caliber-pg/sql/caliber_pg--${VERSION}.sql \
  /usr/share/postgresql/18/extension/
```

### Verify Installation

```sql
-- In psql
CREATE EXTENSION caliber_pg;
SELECT caliber_init();

-- Check version
SELECT caliber_schema_version();
```

## Migrations

Migrations are in `caliber-pg/sql/migrations/V*` and follow Flyway naming:

| Migration | Description |
|-----------|-------------|
| V2__tenant_isolation.sql | Add tenant_id columns for multi-tenant isolation |
| V3__distributed_correctness.sql | Distributed systems correctness features |
| V4__dsl_config.sql | DSL configuration storage tables |
| V5__rls_policies.sql | Row-level security policies |
| V6__tenant_not_null.sql | Make tenant_id columns NOT NULL |
| V7__fix_shared_locks.sql | Fix shared lock handling |
| V8__dsl_pack_source.sql | DSL pack source tracking |

Apply migrations after extension install:

```bash
psql -d your_database -f caliber-pg/sql/migrations/V2__tenant_isolation.sql
# ... continue in order
```

## Troubleshooting

### "No valid extension SQL script found"

The repo SQL file is missing or doesn't contain expected tables. Ensure:
```bash
ls caliber-pg/sql/caliber_pg--*.sql
rg "caliber_dsl_config" caliber-pg/sql/caliber_pg--*.sql
```

### "cargo pgrx schema did not produce a non-empty SQL file"

1. Ensure pgrx can find PostgreSQL: `cargo pgrx init --pg18 download`
2. Check pgrx version: `cargo install cargo-pgrx --version 0.16.1`
3. Use explicit directories in schema command

### Extension Install Fails

Check that the shared library was built:
```bash
find target -name "caliber_pg.so" -o -name "libcaliber_pg.so" 2>/dev/null
```

Verify PostgreSQL version (must be 18+):
```bash
/usr/lib/postgresql/18/bin/pg_config --version
```

---

## OpenAPI Specification

### Regeneration
OpenAPI is generated from utoipa annotations in `caliber-api/src/routes/*.rs`.

```bash
cargo run -p caliber-api --bin generate-openapi --features openapi > openapi.json
```

### Policy
- Regenerate after adding/modifying route handlers with `#[utoipa::path]`
- Commit updated `openapi.json` with the route changes
- SDK is downstream of OpenAPI; SDK regeneration is separate (see SDK section below)

### SDK Regeneration
SDK is a derived artifact from OpenAPI:
```bash
./scripts/generate-sdk.sh typescript
```

SDK is NOT regenerated automatically. Wait until API contracts stabilize.
