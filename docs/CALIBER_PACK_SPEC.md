# CALIBER Pack Spec (TOML + Markdown)

This document is the end-to-end spec and context scratch pad for the "pack" workflow:
`cal.toml` is the only source of truth, Markdown is prompt-only, and the existing DSL
parser/compiler remains the internal IR target. This is designed to complete the
DX refactor (server owns business logic; SDK is thin IO/sugar).

---
## Goals
- Users never see or edit the DSL directly.
- `cal.toml` is the only real configuration source.
- Markdown files contain prompts only (strict headings + fenced blocks).
- Tools are registered in TOML; Markdown only references tool IDs.
- Reuse existing DSL parser + compiler ("free batteries").
- Server owns business logic; SDK is optional and thin.

## Non-goals
- Replace or deprecate the DSL parser/compiler.
- Build a new language; this is a wrapper format only.
- Require the SDK for usage (REST/gRPC should be sufficient).

---
## Current State (codebase anchors)
- DSL parser + AST: `caliber-dsl/src/parser/*`
- DSL compiler: `caliber-dsl/src/compiler/mod.rs`
- API endpoints: `/api/v1/dsl/validate|parse|compile|deploy` in `caliber-api/src/routes/dsl.rs`
- Context assembly is server-side: `/api/v1/context/assemble` in `caliber-api/src/routes/context.rs`
- SDK still has leftover context wiring; must be completed to be thin.

### Existing primitives we can reuse ("free batteries")
- Runtime config type: `CompiledConfig` in `caliber-dsl/src/compiler/mod.rs`.
- DSL storage + deployment tables: `caliber_dsl_config` + `caliber_dsl_deployment` in
  `caliber-pg/sql/migrations/V4__dsl_config.sql`.
- API DB integration for DSL: `caliber-api/src/db.rs` has `dsl_validate`, `dsl_parse`,
  `dsl_config_next_version`, and deployment persistence.
- gRPC parity for DSL validate/parse in `caliber-api/src/grpc.rs`.
- OpenAPI models already exist for DSL in `caliber-api/src/types/dsl.rs`.
- SDK already has a `DslManager` for validate/parse (`caliber-sdk/src/managers/dsl.ts`).
- Context assembly already centralized in Rust (`caliber-core/src/context.rs` +
  `caliber-api/src/routes/context.rs`).
- MCP tool types already exist and can be reused as a shape reference for tool registry
  (`caliber-api/src/routes/mcp/types.rs`).
- DSL pretty printer exists (`caliber-dsl/src/pretty_printer.rs`) and can be used for
  diagnostics or round-trip tests.

### Minimal new code required (if we reuse the above)
- A pack parser/validator in `caliber-dsl` (TOML + markdown + tool registry -> IR).
- An IR-to-AST builder in `caliber-dsl` (build `CaliberAst` directly, then compile).
- A compose endpoint in `caliber-api` that calls the new pack module and returns
  diagnostics + compiled config JSON.
- Thin SDK `compose` method (or skip SDK entirely, since REST/gRPC already exist).

---
## Architecture Overview
User-facing pack format -> internal IR -> DSL AST -> existing compiler -> deploy

```
cal.toml + agents/*.md + tools/*
  -> strict lint + ref resolution
  -> IR (typed "pack config")
  -> build CaliberAst directly (no DSL text emission)
  -> DslCompiler::compile(&ast)
  -> deploy via existing DSL pipeline

AST storage note:
- We still store `dsl_source` for audit/debug by pretty-printing the AST
  (`caliber-dsl/src/pretty_printer.rs`). This keeps schema changes minimal
  and avoids a new parser pass.
```

---
## Design Rationale (Filesystem Analogy)
The pack is a deterministic, persistent coordination protocol:
- `cal.toml` is the single source of truth (metadata + policy).
- Markdown files are payloads (prompts), not configuration.
- Tool registry is a capability index (like a directory table).
- The compiler is the "fsck": it validates, emits canonical IR, and prevents partial truth.
This keeps the system durable, discoverable, and safe to mutate without exposing DSL.

---
## Pack Layout
```
my-pack/
  cal.toml
  agents/
    support.md
    research.md
  tools/
    bin/
      fetch_url
      llm_call
    prompts/
      search.md
```

Notes:
- Markdown never registers tools. TOML does.
- Tools can be exec binaries or prompt tools.
- Optional: `cal.lock` (generated, pinned hashes).

---
## cal.toml (Canonical Schema)
High-level sections:
- [meta]
- [defaults]
- [settings.matrix] (valid state matrix)
- [profiles.*] (named bindings that must satisfy matrix)
- [adapters.*]
- [formats.*]
- [policies.*]
- [injections.*]
- [routing] (provider routing hints)
- [tools.*] (registry)
- [toolsets.*]
- [agents.*]

Pack injection targeting:
- `injections.*.entity_type` can explicitly target `note` or `artifact`.
- Pack compose validates `entity_type` values at compose time.
- Pack compose validates `[routing]` hints:
  - `strategy` must be one of: first|round_robin|random|least_latency
  - provider hints must reference declared providers.

Example (abbreviated):
```toml
[meta]
version = "1.0"
project = "acme-support"

[defaults]
context_format = "xml"
token_budget = 8000
strict_markdown = true
strict_refs = true
secrets_mode = "env_only"

[settings.matrix]
allowed = [
  { name = "local_dev", retention = "session", index = "none", embeddings = "off", format = "xml" },
  { name = "prod_small", retention = "persistent", index = "embedding", embeddings = "on", format = "xml" },
]
enforce_profiles_only = true

[profiles.prod_small]
retention = "persistent"
index = "embedding"
embeddings = "on"
format = "xml"

[adapters.postgres.main]
type = "postgres"
connection = "${ENV:CALIBER_DB_URL}"

[tools.bin.fetch_url]
kind = "exec"
cmd = "tools/bin/fetch_url"
timeout_ms = 30000

[tools.prompts.search]
kind = "prompt"
prompt_md = "tools/prompts/search.md"
result_format = "json"

[toolsets.core]
tools = ["tools.bin.fetch_url", "tools.prompts.search"]

[agents.support]
enabled = true
profile = "prod_small"
adapter = "adapters.postgres.main"
format = "formats.xml"
token_budget = 8000
prompt_md = "agents/support.md"
toolsets = ["toolsets.core"]
```

Matrix rule:
- Users choose profiles only.
- Raw knob combinations are invalid if `enforce_profiles_only = true`.
- Every profile must satisfy `settings.matrix.allowed`.

Secrets rule:
- `secrets_mode = env_only` by default.
- Disallow secrets in markdown entirely.

---
## Markdown Contract (Strict)
Required headings (in order):
- `# System`
- `## PCP`
- `### User`
Heading rule:
- `### User` may repeat multiple times.

Allowed fenced blocks:
- ```tool
- ```json
- ```xml
- ```cal.context (optional)

Rules:
- Tool blocks must reference a TOML tool ID.
- If `strict_refs = true`, fenced block contents must be refs (`${...}`) or `${INPUT:*}`.
- Tool/payload pairing: a ```tool block declares the tool ID (exactly one ref). The next
  fenced block of `json|xml` is the payload for that tool invocation. If there is no
  payload block, payload is `{}`. If there are multiple payload blocks, hard error.
- Fenced blocks belong to the immediately preceding `### User` section unless marked
  as global.
- No raw secrets or raw tool registration in markdown.

Example:
```markdown
# System
You are the support agent for ${meta.project}.

## PCP
Use profile ${agents.support.profile}.

### User
${INPUT:user_message}

```tool
${tools.prompts.search}
```
```

---
## Tool Registry
Tools are registered in TOML and implemented in `tools/`.

Two classes:
- Exec tools: `kind = "exec"`, `cmd = "tools/bin/..."`
- Prompt tools: `kind = "prompt"`, `prompt_md = "tools/prompts/..."`

Optional contract for validation:
```
tools/contracts/search.schema.json
```

TOML:
```toml
[tools.prompts.search]
kind = "prompt"
prompt_md = "tools/prompts/search.md"
contract = "tools/contracts/search.schema.json"
result_format = "json"
```

Markdown only references IDs (e.g., `${tools.prompts.search}`).

Tool capabilities (include now, enforce later):
- `allow_network`
- `allow_fs`
- `allow_subprocess`
- `timeout_ms`

Tool versioning:
- `cal.lock` should pin tool versions by content hash (exec, prompt, contract).

---
## Internal IR (Typed Pack Config)
Create a typed IR that captures:
- Agents (resolved profile -> concrete settings)
- Toolsets (expanded tool IDs)
- Markdown sections (System/PCP/User)
- Tool invocations (tool ID + payload template)
- Policies/injections/adapters/formats (as typed structs)

IR -> AST builder produces `CaliberAst` directly (no DSL text emit), then feeds
`DslCompiler::compile(&ast)` for runtime config.

---
## API Surface (New)
Add to DSL routes:
- `POST /api/v1/dsl/compose`
  - Input: cal.toml + markdown files + optional tool files
  - Output: AST (JSON) + compiled config (or errors). Optional: pretty-printed DSL
    string for debugging/storage.

Optional:
- `POST /api/v1/dsl/deploy` can accept `pack` as source kind (store original pack).

Transport (pick one and document):
- multipart form-data (preferred; works well for CLI/Studio)

Response should include:
- AST (JSON) + compiled config (JSON)
- Canonical DSL source string (derived from AST) as `dsl_source`
- compiled config (structured)
- diagnostics with file + line/column ranges

gRPC:
- Mirror compose in `caliber-api/src/grpc.rs` if needed.

---
## SDK (Thin IO Layer)
SDK should be optional and small:
- `client.dsl.compose(pack)` -> POST /api/v1/dsl/compose
- `client.dsl.deploy(source)` -> existing endpoint
- `client.context.assemble(...)` -> POST /api/v1/context/assemble

Remove any client-side business logic or context assembly.

Repo-specific cleanup (current smells):
- `caliber-sdk/src/client.ts` imports `ContextHelper` from a non-existent file.
- `caliber-sdk/src/index.ts` says ContextHelper is removed but client still calls it.
- Fix by removing ContextHelper usage and wiring assemble/format calls directly to
  `/api/v1/context/assemble` (server owns logic).

---
## File-Level Changes (Checklist)

### caliber-dsl
- Add pack compiler modules (no new crate):
  - `caliber-dsl/src/pack/mod.rs`
  - `caliber-dsl/src/pack/schema.rs`
  - `caliber-dsl/src/pack/markdown.rs`
  - `caliber-dsl/src/pack/ir.rs`
  - `caliber-dsl/src/pack/ast.rs`
- Export as `caliber_dsl::pack::*` from `caliber-dsl/src/lib.rs`
- Add deps to `caliber-dsl/Cargo.toml`:
  - `toml`, `pulldown-cmark` (or similar), optional `regex`

### caliber-api
- Add compose endpoint in `caliber-api/src/routes/dsl.rs`
- Add request/response types in `caliber-api/src/types/dsl.rs`
- Wire into OpenAPI: `caliber-api/src/openapi.rs`
- Optional gRPC parity: `caliber-api/src/grpc.rs`
- If `openapi.json` is committed, regenerate or update it.

### caliber-sdk
- Add `compose` method (thin IO)
- Remove any remaining context helper logic
- Document that SDK is optional
- If SDK is generated from OpenAPI, regenerate client after adding `/dsl/compose`.

---
## Tests

### caliber-dsl
- `config_manifest_tests.rs`: matrix/profile validation
- `config_markdown_tests.rs`: heading + block lint errors
- `config_ast_tests.rs`: IR -> AST -> compile

### caliber-api
- Compose endpoint tests (success + lint failures)
- Existing DSL property tests continue to pass

### Optional
- SDK smoke test for compose call

---
## Bench & Fuzz (stable, OSS-friendly)
Goal: practical coverage without nightly or enterprise CI.

### Fuzz (stable)
Triggered via `FUZZ=1 ./scripts/test.sh`:
- JS/Bun fuzz tests: `tests/fuzz/parser.fuzz.test.ts`
  - Driven by `FUZZ_RUNS` (default: 10,000; runner flag: `--fuzz-runs N`)
- Rust property tests with higher case counts (`PROPTEST_CASES`, shrink caps)

Nightly-only cargo-fuzz targets still live in `fuzz/`, but are not part of the
default workflow until nightly is available.

### Bench (stable)
Triggered via `BENCH=1 ./scripts/test.sh`:
- SDK micro-bench (`caliber-sdk/bench/index.ts`) if bun is installed.
- Rust hotpath benches (criterion, short measurement windows):
  - `caliber-dsl/benches/dsl_hotpath.rs` (parse/compile + pack compose)
  - `caliber-core/benches/context_hotpath.rs` (context assembly)
- Postgres IO micro-bench (pgbench) if `pgbench` exists and schema is ready:
  `scripts/bench/caliber_pgbench.sql` (read-only probes on core tables).
- API smoke bench (curl loop) via `scripts/bench/api_smoke.sh` (skips if preflight
  health check is not `200`).

Tuning knobs (keep benches wide + deep but short):
- `BENCH_SAMPLE_SIZE` (default 10)
- `BENCH_MEAS_TIME` seconds (default 2)
- `CALIBER_API_URL`, `CALIBER_API_HEALTH_PATH`, `CALIBER_API_BENCH_REQUESTS`

Implementation notes:
- Criterion benches are explicitly wired in:
  - `caliber-dsl/Cargo.toml` -> `dsl_hotpath`
  - `caliber-core/Cargo.toml` -> `context_hotpath`
- `scripts/test.sh` runs targeted benches (not full workspace benches) to
  avoid long runs and surprise failures.

---
## CLI Entry Points (Test Harness)
Even if the SDK is optional, the compiler needs a runner:
- `caliber pack validate`
- `caliber pack compose`
- `caliber pack deploy`

Suggested home (repo):
- `scripts/caliber_pack.py` or `scripts/caliber_pack.rs`
- Keep CLI thin: call into `caliber_dsl::pack::*`

Repo runner (current):
- `scripts/run.sh` is a smart wrapper over `scripts/test.sh`.
- It auto-detects DB env readiness, pgbench, bun, and cargo-pgrx, prints a small
  decision matrix, and sets flags for you.
  - Examples: `scripts/run.sh --db --pg-install`, `scripts/run.sh --all --reset`
  - Fuzz runs are tunable: `scripts/run.sh --fuzz --fuzz-runs 100000`

---
## Pack Fixtures (Suggested)
Add a minimal pack fixture for deterministic testing:
```
tests/fixtures/pack_min/
  cal.toml
  agents/support.md
  tools/prompts/search.md
```
Use this fixture in both DSL and API tests.

---
## Storage Model
Two artifacts should be stored together:
- Pack (source artifact): TOML + markdown + tool metadata
- DSL (compiled artifact): pretty-printed from AST (for storage/audit), plus compiled JSON

Keep them linked by hash or deployment id. This supports audit/debug and reproducible
deployments.

Implementation options (prefer minimal changes):
- Reuse `caliber_dsl_config` as the primary record (already stores `dsl_source`, `ast`,
  `compiled`, status, versioning).
- Store pack source as an additional JSON artifact:
  - Option A: add `pack_source` JSONB column to `caliber_dsl_config`.
  - Option B: store in `caliber_dsl_deployment.metadata` (less ideal for retrieval).
  - Option C: add a small `caliber_dsl_pack` table keyed by `config_id` (most explicit).

---
## Migration Notes
- Existing DSL flows remain valid.
- Pack flow is additive.
- For hosted usage, "pack" becomes the recommended path.

---
## Open Questions (Scratch Pad)
- Should pack deployments be stored alongside DSL configs or in a separate table?
- Do we allow includes in `cal.toml` (e.g., `agents/*.toml`) or keep a single file?
- Should we enforce `strict_refs = true` always, or allow a per-pack override?
- How far do we take "profiles only" vs. "profiles + overrides"?

---
## CLI Entry Points (Test Harness)
Even if the SDK is optional, the compiler needs a runner:
- `caliber pack validate`
- `caliber pack compose`
- `caliber pack deploy`

These commands are the primary integration test harness and keep Studio from being
the only test path.

---
## Refactor Completion (Smells to Close)
- SDK still references context helpers (needs final cleanup).
- Any remaining client-side context assembly logic must be removed.
- Server remains the single source of business logic.

### Repo inconsistencies to fix while implementing pack
- `caliber-sdk/src/client.ts` imports a non-existent `./context` helper; `index.ts` says it
  was removed. Decide: remove or rewire once `/context/assemble` is the only path.
- `caliber-api/WEBSOCKET_INTEGRATION.md` claims `routes/dsl.rs` is read-only; it now
  has deploy/compile mutations. Update the doc to avoid misleading contributors.
- No Rust markdown parser is currently used in core crates (pulldown-cmark is in
  `Cargo.lock` but not wired). Pack implementation must add a real parser dependency.
- SDK generated OpenAPI (`caliber-sdk/src/generated`) won’t include `/dsl/compose` until
  OpenAPI is regenerated after adding the endpoint.

---
## Done Criteria
- `cal.toml` + `agents/*.md` can be composed -> DSL -> compiled config.
- Markdown lint errors are precise (file + line).
- `/api/v1/dsl/compose` returns valid output or structured errors.
- SDK is thin and optional.

---
## Context Bomb (Zero-Context Execution Plan)
If you open this repo cold, do **not** invent new infra. Use what exists.

### Non-negotiables
- No new crate. Pack lives in `caliber-dsl` under `caliber_dsl::pack::*`.
- No DSL text emission step for compilation. Build `CaliberAst` directly.
- Store `dsl_source` by pretty-printing AST (for audit only).
- REST + gRPC are canonical. SDK is deferred until pack stabilizes.

### Existing primitives to reuse (avoid duplication)
- Parser + AST: `caliber-dsl/src/parser/*`
- Compiler + `CompiledConfig`: `caliber-dsl/src/compiler/mod.rs`
- Pretty printer: `caliber-dsl/src/pretty_printer.rs`
- DSL storage + deploy tables: `caliber-pg/sql/migrations/V4__dsl_config.sql`
- API DSL routes: `caliber-api/src/routes/dsl.rs`
- API DB DSL integration: `caliber-api/src/db.rs`
- gRPC DSL service: `caliber-api/src/grpc.rs`
- Context assembly: `caliber-core/src/context.rs` + `caliber-api/src/routes/context.rs`
- SDK DSL manager: `caliber-sdk/src/managers/dsl.ts` (optional)

### Pack → AST mapping rule
- Pack TOML + Markdown are **facades** over existing IR. Do not extend the DSL.
- Pack IR must map deterministically to `CaliberAst` using only existing AST nodes.
- If the pack needs a concept not present in AST, it’s out-of-scope for this phase.

### Storage plan (Option B)
- Add new table `caliber_dsl_pack` keyed by `config_id` (FK to `caliber_dsl_config`).
- Store pack source + tool metadata there; keep compiled artifacts in `caliber_dsl_config`.

### REST + gRPC endpoints
- Add `POST /api/v1/dsl/compose` (multipart).
- Add gRPC `ComposeDsl` method mirroring REST shape.
- Wire to OpenAPI after endpoint stabilizes.

### Refactor/cleanup list (must close)
- Fix SDK context helper mismatch (client.ts imports non-existent `./context`).
- Update `caliber-api/WEBSOCKET_INTEGRATION.md` to reflect DSL mutations.
- Regenerate OpenAPI client only after `/dsl/compose` is stable.

### Build order (reduces churn)
1) Pack schema + validator in `caliber-dsl`.
2) Markdown lint + tool/payload pairing.
3) Pack IR -> `CaliberAst`.
4) Compose endpoint in REST + gRPC.
5) Pack storage table + DB wiring.
6) Tests + fixtures.
7) (Optional) SDK compose wrapper.
