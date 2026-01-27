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

Current enforcement behavior:

- Pack compose validates that all tool references resolve.
- MCP is strict pack-only by default.
- MCP tools are sourced from the active deployed pack (`CompiledConfig.tools`).
- Toolset scoping is supported:
  - `POST /mcp/tools/list` accepts `x-agent-name` or `x-agent-id`.
  - `POST /mcp/tools/call` accepts `arguments.agent_name` or `arguments.agent_id`.
  - When agent IDs are provided, MCP resolves the runtime agent record and uses
    `agent_type` as the pack agent name for toolset scoping.
- Active pack inspection is available at:
  - `GET /api/v1/pack/inspect`
  - Includes effective provider selection fields for embeddings/summarization.

Operational notes:

- If no pack is deployed, MCP tool listing is empty and tool calls fail in strict mode.
- Strictness can be relaxed via `CALIBER_MCP_STRICT_PACK=0` (fallbacks re-enabled).
