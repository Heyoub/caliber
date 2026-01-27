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
