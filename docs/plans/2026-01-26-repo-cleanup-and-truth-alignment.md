# Repo Cleanup and Truth Alignment Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Eliminate hidden debt by establishing a single source of truth for packaging, SQL artifacts, contracts, and documentation.

**Architecture:** This is a systematic audit and cleanup effort, not a feature build. Each phase produces a deliverable (classification, decision, or cleanup). The work is ordered to unblock downstream phases: packaging first, then contracts, then docs, then cleanup.

**Tech Stack:** Rust/pgrx (extension), PostgreSQL 18, OpenAPI/utoipa, TypeScript SDK

---

## Current State Summary (from git history + code audit)

### What's Working
- **DSL pack compose pipeline**: `caliber-dsl/src/pack/` → `compose_pack()` → `PackOutput{ast, compiled}`
- **REST endpoint**: `POST /api/v1/dsl/compose` exists in `caliber-api/src/routes/dsl.rs:393`
- **Pack storage**: `caliber_dsl_pack` table, `db.dsl_pack_create()` in `caliber-api/src/db.rs:1834`
- **Extension SQL**: versioned SQL at `caliber-pg/sql/caliber_pg--0.4.4.sql` (87KB, includes functions)
- **CI workflow**: `.github/workflows/extension-sql-check.yml` validates `caliber_agent_register` exists
- **Migrations**: V1-V8 in `caliber-pg/sql/migrations/`, run via `_PG_init()` + `run_pending_migrations()`

### What's Drifting
- **OpenAPI**: `openapi.json` (Jan 19) doesn't include `/dsl/compose` endpoint (added Jan 26)
- **SDK**: Generated from stale OpenAPI; missing compose helpers
- **TUI**: `caliber-tui/src/api_client.rs` doesn't hit compose/deploy endpoints
- **Docs**: `MENTAL_MODEL.md` references `caliber_init()` as bootstrap, but reality is `CREATE EXTENSION caliber_pg`
- **Config files**: `railway.toml`, `vercel.json`, `terraform/` - unclear if maintained

### SQL Generation Reality
From `scripts/test.sh` lines 151-169:
1. `cargo pgrx schema` generates SQL to stdout
2. Script checks for expected tables (`caliber_dsl_config`)
3. Falls back to repo-checked-in SQL if generation fails
4. Repo SQL is authoritative when CI passes

---

## Phase A: Packaging & SQL Artifact Audit (ROOT BLOCKER)

### Task A1: Document current SQL generation flow

**Files:**
- Read: `scripts/test.sh:126-173`
- Read: `.github/workflows/extension-sql-check.yml`
- Read: `caliber-pg/Makefile`
- Create: `docs/SQL_GENERATION.md`

**Step 1: Create documentation file with findings**

```markdown
# SQL Generation and Extension Install Flow

## Authoritative SQL Source
The repo-checked-in SQL files are authoritative:
- `caliber-pg/sql/caliber_pg--{version}.sql` - versioned extension SQL
- `caliber-pg/sql/caliber_init.sql` - bootstrap schema
- `caliber-pg/sql/migrations/V*` - incremental migrations

## Generation Flow
1. `cargo pgrx schema` generates SQL from `#[pg_extern]` annotations
2. CI workflow validates generated SQL contains expected functions
3. `scripts/test.sh` copies generated SQL to extension dir OR falls back to repo SQL

## Local Development
If `cargo pgrx schema` produces empty output locally:
1. Ensure `~/.pgrx` has valid pg18 install: `cargo pgrx init --pg18 download`
2. Use repo SQL as authoritative fallback
3. Only regenerate when adding/modifying `#[pg_extern]` functions

## Extension Install
```bash
sudo cargo pgrx install --package caliber-pg --pg-config "/usr/lib/postgresql/18/bin/pg_config"
sudo -u postgres psql -d caliber -c "CREATE EXTENSION IF NOT EXISTS caliber_pg;"
```
```

**Step 2: Verify documentation matches reality**

Run: `cat scripts/test.sh | grep -A 50 "CALIBER_PG_INSTALL"`
Expected: Script matches documented flow

**Step 3: Commit**

```bash
git add docs/SQL_GENERATION.md
git commit -m "docs: document SQL generation and extension install flow

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

### Task A2: Add version check to CI SQL validation

**Files:**
- Modify: `.github/workflows/extension-sql-check.yml:118-131`

**Step 1: Write the validation logic**

Add after the `caliber_agent_register` check (line 131):

```yaml
      - name: Verify extension version matches Cargo
        run: |
          set -euo pipefail
          CARGO_VERSION="$(awk -F'\"' '
            /^\[workspace\.package\]/ { in_wp = 1; next }
            /^\[/ { in_wp = 0 }
            in_wp && /^version = / { print $2; exit }
          ' Cargo.toml)"
          SQL_FILE="caliber-pg/sql/caliber_pg--${CARGO_VERSION}.sql"
          if [[ ! -f "${SQL_FILE}" ]]; then
            echo "FAIL: No SQL file for version ${CARGO_VERSION}"
            echo "Expected: ${SQL_FILE}"
            echo "Available:"
            ls -la caliber-pg/sql/caliber_pg--*.sql || true
            exit 1
          fi
          echo "PASS: SQL file exists for version ${CARGO_VERSION}"
```

**Step 2: Run CI locally to verify**

Run: `bash -c 'CARGO_VERSION=$(awk -F"\"" "/\[workspace.package\]/{in_wp=1;next} /^\[/{in_wp=0} in_wp && /^version = /{print \$2;exit}" Cargo.toml); ls -la caliber-pg/sql/caliber_pg--${CARGO_VERSION}.sql'`
Expected: File exists

**Step 3: Commit**

```bash
git add .github/workflows/extension-sql-check.yml
git commit -m "ci: add version check for extension SQL file

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Phase B: OpenAPI Contract Alignment

### Task B1: Regenerate OpenAPI from current code

**Files:**
- Read: `caliber-api/src/routes/dsl.rs` (verify utoipa annotations)
- Read: `scripts/generate-sdk.sh:49-58`
- Run: SDK generator spec target

**Step 1: Check utoipa annotations exist for compose endpoint**

Run: `grep -A 20 "POST /api/v1/dsl/compose" caliber-api/src/routes/dsl.rs`
Expected: `#[utoipa::path(` annotation present

**Step 2: Generate OpenAPI spec**

Run: `cargo run -p caliber-api --bin generate-openapi --features openapi > openapi.json.new 2>/dev/null && mv openapi.json.new openapi.json`
Expected: File updated, `grep "/dsl/compose" openapi.json` returns match

**Step 3: Validate spec**

Run: `npx @openapitools/openapi-generator-cli validate -i openapi.json 2>&1 | head -20`
Expected: No errors (warnings OK)

**Step 4: Commit**

```bash
git add openapi.json
git commit -m "chore: regenerate OpenAPI spec with /dsl/compose endpoint

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

### Task B2: Document OpenAPI regeneration policy

**Files:**
- Modify: `docs/SQL_GENERATION.md` (add section)

**Step 1: Add OpenAPI section**

Append to `docs/SQL_GENERATION.md`:

```markdown
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
```

**Step 2: Commit**

```bash
git add docs/SQL_GENERATION.md
git commit -m "docs: add OpenAPI regeneration policy

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Phase C: Bootstrap Documentation Alignment

### Task C1: Update MENTAL_MODEL.md bootstrap section

**Files:**
- Modify: `docs/MENTAL_MODEL.md`

**Step 1: Locate bootstrap references**

Run: `grep -n "caliber_init\|bootstrap" docs/MENTAL_MODEL.md`
Expected: Any references to find and update

**Step 2: Update or add bootstrap section**

If bootstrap section exists, update it. Otherwise add before "## Quick Reference":

```markdown
## Extension Installation (Bootstrap)

CALIBER runs as a PostgreSQL extension. Installation:

```sql
-- Requires PostgreSQL 18+
CREATE EXTENSION IF NOT EXISTS vector;  -- pgvector for embeddings
CREATE EXTENSION IF NOT EXISTS caliber_pg;  -- creates all tables + functions
```

The extension:
1. Creates schema tables (`caliber_agent`, `caliber_trajectory`, etc.)
2. Creates SQL functions (`caliber_agent_register`, etc.)
3. Runs pending migrations automatically via `_PG_init()`

### Manual Schema Init (without extension)
If extension not available, use raw SQL bootstrap:
```bash
psql -d caliber -f caliber-pg/sql/caliber_init.sql
```

### caliber_init() Function
The `caliber_init()` SQL function is available for re-running bootstrap, but is NOT required for normal usage. The extension handles initialization automatically.
```

**Step 3: Verify no conflicting statements remain**

Run: `grep -n "caliber_init" docs/MENTAL_MODEL.md`
Expected: Only the new section references it

**Step 4: Commit**

```bash
git add docs/MENTAL_MODEL.md
git commit -m "docs: clarify extension bootstrap vs caliber_init()

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Phase D: Config File Classification

### Task D1: Create classification table

**Files:**
- Create: `docs/REPO_CONFIG_CLASSIFICATION.md`

**Step 1: Create classification document**

```markdown
# Repository Configuration File Classification

| File | Status | Justification | Action |
|------|--------|---------------|--------|
| `railway.toml` | ARCHIVE | Railway deploy not actively used | Move to `examples/deploy/` or delete |
| `vercel.json` | KEEP | Used for landing page deploy | Verify still works |
| `fly.api.toml` | KEEP | Fly.io API deployment | Verify still works |
| `fly.pg.toml` | KEEP | Fly.io Postgres deployment | Verify still works |
| `openapi.json` | REGENERATE | Generated from utoipa | Regenerate when routes change |
| `openapitools.json` | KEEP | SDK generator config | Used by generate-sdk.sh |
| `terraform/` | ARCHIVE | Example infra, not maintained | Move to `examples/infrastructure/` |
| `caliber-sdk/dist/` | REGENERATE | Generated SDK output | Gitignore, regenerate on release |
| `node_modules/` | IGNORE | Already gitignored | No action |
| `.env` | IGNORE | Local config, gitignored pattern | No action |
| `.env.example` | KEEP | Template for local setup | Verify current |

## Classification Rubric

- **KEEP**: Required for current supported deployment or development
- **REGENERATE**: Derived artifact from code; regenerate don't hand-edit
- **ARCHIVE**: Examples or reference; move to `examples/`
- **REMOVE**: Stale and misleading; delete
- **IGNORE**: Already gitignored or not in repo
```

**Step 2: Commit**

```bash
git add docs/REPO_CONFIG_CLASSIFICATION.md
git commit -m "docs: add repo config file classification

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

### Task D2: Archive railway.toml

**Files:**
- Move: `railway.toml` → `examples/deploy/railway.toml`
- Create: `examples/deploy/README.md`

**Step 1: Create examples/deploy directory**

Run: `mkdir -p examples/deploy`

**Step 2: Move railway.toml**

Run: `git mv railway.toml examples/deploy/railway.toml`

**Step 3: Create README**

```markdown
# Deployment Examples

Example deployment configurations for various platforms.

**These are examples only** - they may not be actively maintained.

## Files

- `railway.toml` - Railway platform deployment config (archived)

## Actively Maintained Deployment

See `fly.api.toml` and `fly.pg.toml` in repo root for Fly.io deployment.
```

**Step 4: Commit**

```bash
git add examples/deploy/
git commit -m "chore: archive railway.toml as deploy example

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

### Task D3: Archive terraform modules

**Files:**
- Move: `terraform/` → `examples/infrastructure/terraform/`

**Step 1: Create directory and move**

Run: `mkdir -p examples/infrastructure && git mv terraform examples/infrastructure/`

**Step 2: Add README**

Create `examples/infrastructure/README.md`:

```markdown
# Infrastructure Examples

Example infrastructure-as-code configurations.

**These are examples only** - they may not be actively maintained.

## Terraform

The `terraform/` directory contains example Terraform modules for cloud infrastructure.
```

**Step 3: Commit**

```bash
git add examples/infrastructure/
git commit -m "chore: archive terraform as infrastructure example

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Phase E: TUI Audit

### Task E1: Document TUI endpoint coverage

**Files:**
- Read: `caliber-tui/src/api_client.rs`
- Create: `docs/TUI_ENDPOINT_COVERAGE.md`

**Step 1: Extract endpoints from TUI**

Run: `grep -E "(get|post|put|patch|delete)\(" caliber-tui/src/api_client.rs | grep -oE '"/api/v1/[^"]+"|format!\("[^"]+' | sort -u`

**Step 2: Create coverage document**

```markdown
# TUI API Endpoint Coverage

## Currently Used Endpoints

| Endpoint | TUI Usage |
|----------|-----------|
| GET /api/v1/agents | List agents view |
| GET /api/v1/trajectories | Trajectory list |
| GET /api/v1/artifacts | Artifact browser |
| GET /api/v1/notes | Notes view |
| GET /api/v1/tenants | Tenant switcher |
| ... (fill from grep output) |

## Missing Pack/Compose Endpoints

| Endpoint | Status | Notes |
|----------|--------|-------|
| POST /api/v1/dsl/compose | NOT IMPLEMENTED | Pack compose not in TUI |
| POST /api/v1/dsl/deploy | NOT IMPLEMENTED | Pack deploy not in TUI |

## Recommendation

TUI should be marked as **experimental** until pack UX stabilizes.
Pack operations are API-first; TUI can add them later.
```

**Step 3: Commit**

```bash
git add docs/TUI_ENDPOINT_COVERAGE.md
git commit -m "docs: document TUI endpoint coverage and gaps

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Phase F: MCP + Tools Model

### Task F1: Document cal.toml vs mcp.json relationship

**Files:**
- Read: `tests/fixtures/pack_min/cal.toml`
- Read: `.kiro/settings/mcp.json`
- Create: `docs/CAL_MCP_RELATIONSHIP.md`

**Step 1: Create relationship document**

```markdown
# cal.toml vs mcp.json Relationship

## cal.toml (Compile-Time Contract)

`cal.toml` is the **capability registry** for a CALIBER pack:
- Declares adapters (database connections)
- Declares agents and their profiles
- Declares toolsets and their tools
- Declares policies (summarization, lifecycle)

Example:
```toml
[tools.prompts.search]
kind = "prompt"
prompt_md = "tools/prompts/search.md"
```

**cal.toml is authoritative for what capabilities exist.**

## mcp.json (Runtime IO Wiring)

`mcp.json` is the **runtime tool configuration** for MCP-compatible clients:
- Maps tool IDs to transport endpoints
- Configures authentication
- Wires external tools to agent runtimes

**mcp.json SHOULD reference tool IDs declared in cal.toml.**
No new capabilities at runtime - only wiring.

## Relationship

```
cal.toml (compile-time)
    ↓ declares tools
pack compose
    ↓ validates tool references
CompiledConfig
    ↓ runtime deploys
mcp.json (runtime)
    ↓ wires declared tools to endpoints
Agent Runtime
```

## Enforcement

Pack compose validates that all tool references resolve.
Tools referenced in mcp.json but not declared in cal.toml are invalid.
```

**Step 2: Commit**

```bash
git add docs/CAL_MCP_RELATIONSHIP.md
git commit -m "docs: document cal.toml vs mcp.json relationship

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Phase G: Git History Cleanup (DESTRUCTIVE - LAST)

### Task G1: Pre-cleanup verification

**Files:**
- Check: No secrets in files to purge
- Check: No dependencies on purged files

**Step 1: List files to purge**

Run: `ls -la scripts/rust_workspace_analyzer.py scripts/repo_audit_rust.py scripts/repo_features_matrix.py 2>&1 | cat`
Expected: Files don't exist OR are marked for deletion

**Step 2: Verify no secrets**

Run: `git log --all --oneline -- scripts/rust_workspace_analyzer.py scripts/repo_audit_rust.py scripts/repo_features_matrix.py 2>&1 | head -10`
Expected: Only commits shown, no sensitive data visible

**Step 3: Document decision**

This task requires explicit user approval before execution.
The git filter-repo command will rewrite history.

**DO NOT EXECUTE** until user confirms:
1. All team members are aware
2. Force push is acceptable
3. Any secrets in those files have been rotated

---

### Task G2: Execute git filter-repo (REQUIRES EXPLICIT APPROVAL)

**Files:**
- Purge: `scripts/rust_workspace_analyzer.py`
- Purge: `scripts/repo_audit_rust.py`
- Purge: `scripts/repo_features_matrix.py`

**Step 1: Install git-filter-repo if needed**

Run: `pip install git-filter-repo`

**Step 2: Create backup**

Run: `git clone --mirror . ../caliber-backup-$(date +%Y%m%d)`

**Step 3: Execute purge**

Run: `git filter-repo --path scripts/rust_workspace_analyzer.py --path scripts/repo_audit_rust.py --path scripts/repo_features_matrix.py --invert-paths`

**Step 4: Force push**

Run: `git push origin --force --all && git push origin --force --tags`

**Step 5: Notify team**

All clones must be re-cloned or `git fetch --all && git reset --hard origin/main`.

---

## Summary Checklist

- [ ] Phase A: SQL generation documented, CI version check added
- [ ] Phase B: OpenAPI regenerated with compose endpoint
- [ ] Phase C: Bootstrap docs updated
- [ ] Phase D: Config files classified, railway.toml + terraform archived
- [ ] Phase E: TUI endpoint coverage documented
- [ ] Phase F: cal.toml/mcp.json relationship documented
- [ ] Phase G: Git history cleanup (pending approval)

## Do-Not-Touch List

- `caliber-pg/sql/caliber_pg--0.4.4.sql` - authoritative extension SQL
- `caliber-pg/sql/caliber_init.sql` - bootstrap schema
- `caliber-dsl/src/pack/` - working pack pipeline
- `caliber-api/src/routes/dsl.rs` - working compose endpoint
- `caliber-api/src/db.rs` - working pack storage
