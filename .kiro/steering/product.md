# CALIBER + PCP Product Context

## What We're Building

CALIBER (Context Abstraction Layer Integrating Behavioral Extensible Runtime) with PCP (Persistent Context Protocol) — a Postgres-native memory framework for AI agents.

## Core Philosophy

**NOTHING HARD-CODED. This is a FRAMEWORK, not a product.**

- Every value is user-configured
- Missing config = error, not silent default
- No SQL in hot path — direct pgrx storage access
- Compositional ECS architecture over inheritance

## Problem Space

| Problem | CALIBER Solution |
|---------|-----------------|
| Context amnesia | Hierarchical memory: Trajectory → Scope → Artifact → Note |
| Hallucination | PCP grounding: all facts backed by stored artifacts |
| Multi-agent failures (40-80%) | Typed coordination: locks, messages, delegation |
| Token waste | Configurable context assembly with relevance scoring |
| No auditability | Full trace of assembly decisions |
| Hard-coded AI frameworks | Zero hard-coded values, everything configurable |

## Target Users

- AI agent developers needing persistent, structured memory
- Multi-agent system builders requiring coordination primitives
- Teams building LLM applications that need context management

## Key Differentiators

1. **Direct Postgres storage** — bypasses SQL parsing overhead via pgrx
2. **VAL (Vector Abstraction Layer)** — provider-agnostic embeddings, any dimension
3. **Full multi-agent support** — locks, messages, delegation, handoffs
4. **Custom DSL** — compiles to CaliberConfig, not SQL
5. **PCP harm reduction** — validation, checkpoints, contradiction detection
