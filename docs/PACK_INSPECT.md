# Pack Inspect Endpoint

This document describes the pack inspection endpoint and how to use it to
understand the *effective* runtime configuration for a tenant.

## Endpoint

- Method: `GET`
- Path: `/api/v1/pack/inspect`
- Auth: required (tenant-scoped)

## Purpose

`/api/v1/pack/inspect` answers the question:

- "What pack is actually deployed and active right now?"

It surfaces both:

- the active compiled config
- the active pack source (if stored)
- derived summaries (tools, toolsets, agents, injections)
- effective provider selection under routing hints

## Key Fields

- `has_active`: whether a deployed pack/config exists
- `compiled`: the full active compiled config (if present)
- `pack_source`: stored pack source payload (if present)
- `tools`: flattened list of tool IDs
- `toolsets`: map of toolset -> tool IDs
- `agents`: map of agent -> toolsets
- `injections`: normalized injection view
- `providers`: list of provider names
- `routing`: routing hints from `[routing]`
- `effective_embedding_provider`: provider chosen for embedding capability
- `effective_summarization_provider`: provider chosen for summarization capability

## Example Request (curl)

```bash
curl -s \
  -H "authorization: Bearer $CALIBER_JWT" \
  -H "x-tenant-id: $CALIBER_TENANT_ID" \
  "$CALIBER_API_BASE/api/v1/pack/inspect"
```

## Example Response (abridged)

```json
{
  "has_active": true,
  "tools": [
    "tools.prompts.search",
    "tools.bin.fetch_url"
  ],
  "toolsets": {
    "core": [
      "tools.prompts.search"
    ]
  },
  "agents": {
    "support": [
      "core"
    ]
  },
  "injections": [
    {
      "source": "notes",
      "target": "context",
      "entity_type": "note",
      "mode": "relevant(0.72)",
      "priority": 80,
      "max_tokens": 2000
    }
  ],
  "providers": [
    "openai"
  ],
  "routing": {
    "strategy": "least_latency",
    "embedding_provider": "openai",
    "summarization_provider": "openai"
  },
  "effective_embedding_provider": "openai",
  "effective_summarization_provider": "openai"
}
```

## Notes

- Effective provider selection uses routing hints when present, and otherwise
  falls back to the routing strategy (default: `first`).
- MCP tool visibility is pack-driven and toolset-scoped. Use this endpoint to
  confirm the active tool registry and agent/toolset bindings.

