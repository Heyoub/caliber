# TUI API Endpoint Coverage

This document tracks which API endpoints the TUI (`caliber-tui`) currently implements and identifies gaps.

## Currently Used Endpoints

The TUI uses REST API endpoints via `RestClient` in `caliber-tui/src/api_client.rs`.

| Endpoint | Method | TUI Usage |
|----------|--------|-----------|
| `/api/v1/tenants` | GET | List all tenants (`list_tenants`) |
| `/api/v1/trajectories` | GET | List trajectories with filtering (`list_trajectories`) |
| `/api/v1/trajectories/{id}/scopes` | GET | List scopes for a trajectory (`list_scopes`) |
| `/api/v1/scopes/{id}/turns` | GET | List turns for a scope (`list_turns`) |
| `/api/v1/artifacts` | GET | List artifacts with filtering (`list_artifacts`) |
| `/api/v1/notes` | GET | List notes with filtering (`list_notes`) |
| `/api/v1/agents` | GET | List agents with filtering (`list_agents`) |
| `/api/v1/agents/{id}` | GET | Get single agent details (`get_agent`) |
| `/api/v1/locks` | GET | List all locks (`list_locks`) |
| `/api/v1/locks/{id}` | GET | Get single lock details (`get_lock`) |
| `/api/v1/messages` | GET | List messages with filtering (`list_messages`) |
| `/api/v1/messages/{id}` | GET | Get single message details (`get_message`) |

### Additional Protocols

| Protocol | Status | Notes |
|----------|--------|-------|
| WebSocket | Implemented | Real-time event streaming via `WsClient` |
| gRPC | Stub only | `GrpcClient` creates channels but no RPC methods implemented |

## Missing Pack/Compose Endpoints

The DSL/Pack endpoints are NOT implemented in the TUI:

| Endpoint | Method | Status | Notes |
|----------|--------|--------|-------|
| `/api/v1/dsl/validate` | POST | NOT IMPLEMENTED | DSL validation not in TUI |
| `/api/v1/dsl/parse` | POST | NOT IMPLEMENTED | DSL parsing not in TUI |
| `/api/v1/dsl/compile` | POST | NOT IMPLEMENTED | DSL compilation not in TUI |
| `/api/v1/dsl/compose` | POST | NOT IMPLEMENTED | Pack compose not in TUI |
| `/api/v1/dsl/deploy` | POST | NOT IMPLEMENTED | Pack deploy not in TUI |

## Summary

- **Total TUI endpoints**: 12 (all GET, read-only operations)
- **Missing DSL endpoints**: 5 (all POST, write operations)
- **Write operations**: None implemented in TUI

## Recommendation

The TUI should be marked as **experimental** until pack UX stabilizes.

**Rationale:**
1. Pack operations (compose, deploy, validate) are complex workflows better suited to CLI or programmatic access
2. The TUI currently provides read-only monitoring capabilities
3. Adding pack editing UX requires significant design work for multi-file editing
4. API-first approach allows pack operations via SDK/CLI while TUI focuses on observability

**Future considerations:**
- TUI could add pack operations once the API surface stabilizes
- Consider a simpler "deploy from file path" UI rather than full editing
- Real-time validation feedback would require WebSocket integration with DSL endpoints
