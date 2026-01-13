# CALIBER Development Philosophy

## AI-Native Development Model

We're building with AI assistance, which changes how we approach code generation.

### The Problem with Stubs

Traditional scaffolding creates empty files with TODO comments:
```rust
// TODO: Implement this
pub struct SomeType;
```

**This is bad because:**
1. **Forgotten work** — TODOs get lost, modules stay empty
2. **Duplicate effort** — You plan twice (stub + real implementation)
3. **False progress** — Files exist but contain nothing useful
4. **Context loss** — By the time you fill the stub, you've forgotten the design

### Our Approach: Plan Complete, Generate Complete

```text
Traditional:  scaffold stubs → forget → rediscover → implement → fix
AI-Native:    plan everything → generate complete code → verify once
```

**Rules:**
1. **No empty files** — Every file created has real, working code
2. **No TODO placeholders** — If you don't know what goes there, don't create the file yet
3. **Structure follows implementation** — Create directories when you have code to put in them
4. **Cargo.toml before lib.rs** — Manifest files can exist without source (cargo doesn't care)

### Workspace Initialization Pattern

**DO:**
```text
1. Create workspace Cargo.toml (complete)
2. Create crate directories with Cargo.toml only
3. Implement caliber-core with FULL lib.rs
4. Implement next crate with FULL lib.rs
5. ... continue until all crates have real code
6. Run cargo check ONCE at the end
```

**DON'T:**
```text
1. Create all directories
2. Create stub lib.rs files with "// TODO"
3. Forget what half of them were for
4. Implement some, miss others
5. Run cargo check, get 47 errors
6. Cry
```

### Why This Works for AI-Assisted Development

1. **Full context available** — docs/DEPENDENCY_GRAPH.md has ALL types defined
2. **No incremental compilation pain** — Generate everything, compile once
3. **Clear completion criteria** — A crate is done when lib.rs has real code
4. **No orphaned work** — Every file serves a purpose from creation

### The "Can I Create This File?" Test

Before creating any source file, ask:
1. Do I know EXACTLY what goes in it? → Yes → Create with full content
2. Am I just making a placeholder? → Yes → DON'T CREATE IT YET

### Reference Documents

All types are pre-defined in:
- `docs/DEPENDENCY_GRAPH.md` — Complete type system for all 8 crates
- `docs/CALIBER_PCP_SPEC.md` — Core specification
- `docs/DSL_PARSER.md` — Lexer/parser types
- `docs/MULTI_AGENT_COORDINATION.md` — Agent types

When implementing a crate, COPY types from these docs. Don't reinvent.
