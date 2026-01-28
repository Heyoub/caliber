# Agent Guide (CALIBER)

This repo is designed for agent-driven iteration with CI as the source of truth.

## Ground Rules
- Prefer `rg` for search and keep context small.
- Avoid running full `cargo test` locally unless explicitly requested.
- Use CI logs for failures; local environments may not match CI.
- Keep changes minimal and surgical; no placeholder TODOs in production code.

## CI + Logs
- Trigger CI manually via `make ci-cloud` (downloads logs into `.github/.logs/`).
- CI artifacts and logs are the canonical build signal.

## Repo Structure
- API: `caliber-api/`
- Core primitives: `caliber-core/`
- PostgreSQL extension: `caliber-pg/`
- TUI: `caliber-tui/`
- Landing site: `landing/`

## Secrets
- `.env` is local-only and ignored by Git.
- Never commit real secrets; use `.env.example` for templates.

## Release
- Tags follow `vX.Y.Z` and trigger `.github/workflows/release.yml`.
- SLSA provenance is attached to release artifacts and images.
