# CALIBER Pack Editor

Vue 3 + CodeMirror 6 editor for CALIBER agent pack configurations.

## Status

**Foundation complete, UI components pending.**

### Implemented

- [x] TypeScript types (`src/types/`)
  - `manifest.ts` - Pack manifest types (mirrors `caliber-dsl/src/pack/schema.rs`)
  - `api.ts` - API response types
  - `editor.ts` - Editor state types

- [x] Zod validation schemas (`src/schemas/`)
  - `manifest.zod.ts` - Full manifest validation with constraints from `caliber-dsl`

- [x] Pinia store (`src/composables/usePackStore.ts`)
  - Pack loading/saving
  - Validation pipeline (parse → schema → refs → ready)
  - Dependency graph computation
  - File tree building
  - Dirty state tracking

- [x] API client stubs (`src/services/api/`)
  - `client.ts` - Base HTTP client with auth
  - `pack.ts` - Pack API methods (some endpoints pending backend)

### Pending

- [ ] Vue components (`src/components/`)
- [ ] CodeMirror integration (`src/composables/useCodeMirror.ts`)
- [ ] D3 dependency graph
- [ ] Backend endpoints (deferred to avoid merge conflicts):
  - `GET /api/v1/pack/history`
  - `GET /api/v1/pack/version/:id`
  - `GET /api/v1/pack/diff`
  - `POST /api/v1/pack/revert`

## Architecture

```
pack-editor/
├── src/
│   ├── types/           # TypeScript types
│   │   ├── manifest.ts  # Pack manifest (cal.toml)
│   │   ├── api.ts       # API responses
│   │   └── editor.ts    # Editor state
│   ├── schemas/         # Zod validation
│   │   └── manifest.zod.ts
│   ├── composables/     # Pinia stores & composables
│   │   └── usePackStore.ts
│   ├── services/        # API clients
│   │   └── api/
│   │       ├── client.ts
│   │       └── pack.ts
│   └── components/      # Vue components (TODO)
├── package.json
├── vite.config.ts
└── tsconfig.json
```

## Key Design Decisions

1. **Server-side versioning** instead of client-side git (isomorphic-git)
   - Uses existing `dsl_config_*` DB functions
   - EventDag for audit trail via `SYSTEM_CONFIG_CHANGE` events

2. **Validation pipeline** mirrors `caliber-dsl` compiler
   - Stage 1: TOML parsing
   - Stage 2: Zod schema validation
   - Stage 3: Reference checking (agents → profiles, toolsets → tools, etc.)
   - Stage 4: Server-side compose (optional)

3. **Dependency graph** computed from manifest
   - Agents → profiles, adapters, toolsets, prompt files
   - Toolsets → tools
   - Routing → providers
   - Profiles → providers (embeddings)

## Development

```bash
# Install dependencies
bun install

# Start dev server (proxies /api to caliber-api on :3000)
bun run dev

# Build for production
bun run build
```

## Reference Files

- Pack manifest schema: `caliber-dsl/src/pack/schema.rs`
- Validation constraints: `caliber-dsl/src/pack/ir.rs`
- Example pack: `agents-pack/`
- API types: `caliber-api/src/types/dsl.rs`
