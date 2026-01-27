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
