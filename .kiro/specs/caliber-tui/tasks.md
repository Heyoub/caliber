# Implementation Plan: CALIBER TUI

## Overview

This plan implements the full CALIBER TUI system consisting of two crates:
1. **caliber-api** - REST/gRPC/WebSocket API layer (Axum + Tonic)
2. **caliber-tui** - Terminal UI (Ratatui) with SynthBrute aesthetic

**✅ Prerequisite Complete:** `caliber-pg` is production-ready with zero warnings and direct heap operations.

**Key Principles (from steering):**
- NO STUBS, NO TODOs - Complete code only
- NO SQL IN HOT PATH - API calls caliber_* pg_extern functions (already optimized)
- CaliberResult<T> for all fallible operations
- Reference `docs/DEPENDENCY_GRAPH.md` for type definitions
- SynthBrute visual aesthetic throughout

## Build Philosophy

**AI-NATIVE WORKFLOW:**
1. Write ALL code complete with correct types from docs/DEPENDENCY_GRAPH.md
2. Write ALL tests (unit + property-based)
3. Verify type alignment across all modules
4. **STOP - Human runs `cargo build --workspace` in WSL**
5. Human reports ALL errors holistically
6. Fix all errors in one iteration
7. **Human runs `cargo test --workspace` in WSL**
8. Human reports test results
9. Iterate until clean

**NO INCREMENTAL CARGO RUNS.** Windows has no pgrx/Postgres 18 support - all builds happen in WSL with human in the loop.

## Tasks

### Phase 1: caliber-api Complete Implementation (No Cargo Yet)

### Phase 1: caliber-api Complete Implementation (No Cargo Yet)

- [x] 1. Set up caliber-api crate structure
  - [x] 1.1 Create caliber-api/Cargo.toml with dependencies
  - [x] 1.2 Create API types module (types.rs)
  - [x] 1.3 Create error types module (error.rs)
  - [x] 1.4 Create database connection pool module (db.rs)

- [x] 2. Implement authentication middleware
  - [x] 2.1 Create auth module (auth.rs)
  - [x] 2.2 Create auth middleware for Axum
  - [x] 2.3 Write property test for authentication enforcement

- [x] 3. Implement Trajectory REST endpoints
  - [x] 3.1 Create trajectory routes (routes/trajectory.rs)
  - [x] 3.2 Write property test for Trajectory API round-trip

- [x] 4. Implement Scope REST endpoints
  - [x] 4.1 Create scope routes (routes/scope.rs)
  - [x] 4.2 Write property test for Scope API round-trip

- [x] 5. Implement remaining REST endpoints (complete code, no cargo)
  - [x] 5.1 Create artifact routes (routes/artifact.rs)
    - All CRUD endpoints with proper types from DEPENDENCY_GRAPH.md
    - Similarity search endpoint
    - _Requirements: 1.1, 5.7, 5.8, 5.9, 5.10_

  - [x] 5.2 Create note routes (routes/note.rs)
    - All CRUD endpoints with proper types
    - Similarity search endpoint
    - _Requirements: 1.1, 6.6, 6.7, 6.8_

  - [x] 5.3 Create turn routes (routes/turn.rs)
    - Create and get endpoints
    - _Requirements: 1.1_

  - [x] 5.4 Create agent routes (routes/agent.rs)
    - Register, list, get, update, unregister, heartbeat
    - _Requirements: 1.1, 8.6, 8.7, 8.8_

  - [x] 5.5 Create lock routes (routes/lock.rs)
    - Acquire, release, extend, list
    - _Requirements: 1.1, 9.6_

  - [x] 5.6 Create message routes (routes/message.rs)
    - Send, list, acknowledge
    - _Requirements: 1.1_

  - [x] 5.7 Create delegation routes (routes/delegation.rs)
    - Create, get, accept, reject, complete
    - _Requirements: 1.1_

  - [x] 5.8 Create handoff routes (routes/handoff.rs)
    - Create, get, accept, complete
    - _Requirements: 1.1_

  - [x] 5.9 Create DSL routes (routes/dsl.rs)
    - Validate and parse endpoints
    - _Requirements: 1.1, 11.6, 11.7_

  - [x] 5.10 Create config routes (routes/config.rs)
    - Get, update, validate endpoints
    - _Requirements: 1.1, 12.5, 12.6_

  - [x] 5.11 Create tenant routes (routes/tenant.rs)
    - List and get endpoints
    - _Requirements: 1.1, 2.4_

  - [x] 5.12 Update routes/mod.rs to export all route modules

- [x] 6. Implement WebSocket event broadcasting (complete code, no cargo)
  - [x] 6.1 Create event types module (events.rs)
    - WsEvent enum with all variants from design
    - Serialization support
    - _Requirements: 1.4_

  - [x] 6.2 Create WebSocket handler (ws.rs)
    - WebSocket upgrade endpoint
    - Broadcast channel setup
    - Tenant-specific subscriptions
    - _Requirements: 1.3, 1.4_

  - [x] 6.3 Add event broadcasting to all route handlers
    - Inject broadcast sender into route state
    - Emit events on mutations
    - _Requirements: 1.4, 3.9, 4.8, 5.11, 6.10, 7.9, 8.10, 9.8, 10.8_

- [-] 7. Implement gRPC service (complete code, no cargo)
  - [x] 7.1 Create proto/caliber.proto
    - All service definitions from design
    - Message types matching REST
    - _Requirements: 1.2_

  - [x] 7.2 Create build.rs for tonic-build
    - Configure proto compilation
    - _Requirements: 1.2_

  - [x] 7.3 Create gRPC service implementation (grpc.rs)
    - CaliberService trait implementation
    - Reuse REST handler logic
    - SubscribeEvents streaming
    - _Requirements: 1.2_

  - [x] 7.4 Update lib.rs to export gRPC module

- [ ] 8. Write all property-based tests (complete code, no cargo)
  - [x] 8.1 Artifact API round-trip test (tests/artifact_property_tests.rs)
    - **Property 1: API Completeness (Artifact)**
    - **Validates: Requirements 1.1**

  - [x] 8.2 Note API round-trip test (tests/note_property_tests.rs)
    - **Property 1: API Completeness (Note)**
    - **Validates: Requirements 1.1**

  - [x] 8.3 Agent API round-trip test (tests/agent_property_tests.rs)
    - **Property 1: API Completeness (Agent)**
    - **Validates: Requirements 1.1**

  - [ ] 8.4 Tenant isolation test (tests/tenant_property_tests.rs)
    - **Property 5: Tenant Isolation**
    - **Validates: Requirements 1.6, 2.5**

  - [ ] 8.5 Mutation broadcast test (tests/broadcast_property_tests.rs)
    - **Property 3: Mutation Broadcast**
    - **Validates: Requirements 1.4**

  - [ ] 8.6 REST-gRPC parity test (tests/grpc_parity_property_tests.rs)
    - **Property 2: REST-gRPC Parity**
    - **Validates: Requirements 1.1, 1.2**

  - [ ] 8.7 DSL round-trip test (tests/dsl_property_tests.rs)
    - **Property 12: DSL Validation Round-Trip**
    - **Validates: Requirements 11.6, 11.7**

- [ ] 9. Type verification pass (no cargo, manual review)
  - [ ] 9.1 Review all types against docs/DEPENDENCY_GRAPH.md
    - Verify EntityId, Timestamp, TTL usage
    - Check ExtractionMethod variants
    - Verify MemoryAccess struct usage
    - Confirm CaliberConfig construction
    - _Requirements: All_

  - [ ] 9.2 Review error handling patterns
    - All functions return ApiResult<T>
    - Proper error propagation with ?
    - No unwrap() in production code
    - _Requirements: 1.7, 1.8, 16.3_

  - [ ] 9.3 Review database interaction patterns
    - All DB calls go through DbClient
    - Calls caliber_* pg_extern functions only
    - No raw SQL queries
    - _Requirements: 1.9, 1.10_

### Phase 2: Human Checkpoint - Build in WSL

- [ ] 10. **HUMAN ACTION: Run cargo build**
  - [ ] 10.1 Human runs `cargo build --workspace` in WSL
  - [ ] 10.2 Human reports ALL compiler errors, warnings, type mismatches
  - [ ] 10.3 Agent fixes all issues in one iteration
  - [ ] 10.4 Repeat until clean build

### Phase 3: caliber-tui Complete Implementation (No Cargo Yet)

### Phase 3: caliber-tui Complete Implementation (No Cargo Yet)

- [ ] 11. Set up caliber-tui crate structure (complete code, no cargo)
  - [ ] 11.1 Create caliber-tui/Cargo.toml with all dependencies
    - ratatui, crossterm, tokio, reqwest, tonic
    - tokio-tungstenite, tui-textarea
    - All versions aligned with workspace
    - _Requirements: All TUI requirements_

  - [ ] 11.2 Create complete module structure
    - src/main.rs - entry point and event loop
    - src/theme.rs - SynthBrute color theme
    - src/api_client.rs - REST/gRPC/WebSocket clients
    - src/state.rs - App state and view states
    - src/nav.rs - Navigation and view switching
    - src/keys.rs - Keybinding definitions
    - src/events.rs - Event types module
    - src/notifications.rs - Notification system
    - src/views/ - All 10 view modules
    - src/widgets/ - Reusable widget components
    - _Requirements: All TUI requirements_

- [ ] 12. Implement core TUI infrastructure (complete code, no cargo)
  - [ ] 12.1 Implement theme.rs
    - SynthBruteTheme with all colors from design
    - Color mapping functions
    - _Requirements: 13.1, 13.2, 13.3, 13.4_

  - [ ] 12.2 Implement api_client.rs
    - REST client with reqwest
    - gRPC client with tonic
    - WebSocket client with reconnection
    - _Requirements: 1.1, 1.2, 1.3_

  - [ ] 12.3 Implement state.rs
    - App struct with all view states
    - TenantContext
    - Event queue
    - _Requirements: 2.1, 2.2, 15.1_

  - [ ] 12.4 Implement nav.rs
    - View enum
    - View switching logic
    - _Requirements: 14.2, 14.3_

  - [ ] 12.5 Implement keys.rs
    - Keybinding definitions
    - Key handler dispatch
    - _Requirements: 14.1, 14.4, 14.5, 14.6, 14.7, 14.8_

  - [ ] 12.6 Implement events.rs
    - Event types for TUI
    - Event processing
    - _Requirements: 15.3, 15.4_

  - [ ] 12.7 Implement notifications.rs
    - Notification system
    - Error display
    - _Requirements: 16.1, 16.2, 16.3_

  - [ ] 12.8 Implement main.rs
    - Terminal setup
    - Main event loop
    - Render loop
    - _Requirements: 14.1, 14.9, 14.10_

- [ ] 13. Implement all reusable widgets (complete code, no cargo)
  - [ ] 13.1 widgets/tree.rs - Collapsible tree widget
  - [ ] 13.2 widgets/detail.rs - Detail panel widget
  - [ ] 13.3 widgets/filter.rs - Filter bar widget
  - [ ] 13.4 widgets/progress.rs - Progress bar widget
  - [ ] 13.5 widgets/status.rs - Status indicator widget
  - [ ] 13.6 widgets/syntax.rs - Syntax highlighter widget
  - [ ] 13.7 widgets/mod.rs - Widget module exports
  - _Requirements: 3.1, 3.2, 3.4, 3.8, 4.3, 5.2, 5.3, 5.5, 6.4, 13.5_

- [ ] 14. Implement all 10 views (complete code, no cargo)
  - [ ] 14.1 views/tenant.rs - Tenant Management view
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.6, 2.7_

  - [ ] 14.2 views/trajectory.rs - Trajectory Tree view
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 3.9_

  - [ ] 14.3 views/scope.rs - Scope Explorer view
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8_

  - [ ] 14.4 views/artifact.rs - Artifact Browser view
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8, 5.9, 5.10, 5.11_

  - [ ] 14.5 views/note.rs - Note Library view
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 6.8, 6.9, 6.10_

  - [ ] 14.6 views/turn.rs - Turn History view
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7, 7.8, 7.9_

  - [ ] 14.7 views/agent.rs - Agent Dashboard view
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, 8.7, 8.8, 8.9, 8.10_

  - [ ] 14.8 views/lock.rs - Lock Monitor view
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7, 9.8_

  - [ ] 14.9 views/message.rs - Message Queue view
    - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 10.6, 10.7, 10.8_

  - [ ] 14.10 views/dsl.rs - DSL Editor view
    - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5, 11.6, 11.7, 11.8, 11.9, 11.10_

  - [ ] 14.11 views/config.rs - Config Viewer view
    - _Requirements: 12.1, 12.2, 12.3, 12.4, 12.5, 12.6, 12.7_

  - [ ] 14.12 views/mod.rs - View module exports

- [ ] 15. Implement real-time update infrastructure (complete code, no cargo)
  - [ ] 15.1 WebSocket connection manager
    - Persistent connection
    - Reconnection with exponential backoff
    - _Requirements: 15.1, 15.2_

  - [ ] 15.2 Event processor
    - Process incoming events
    - Update view states
    - _Requirements: 15.3, 15.4_

  - [ ] 15.3 Pause/resume functionality
    - Toggle with 'p'
    - Queue events while paused
    - _Requirements: 15.5, 15.6, 15.7_

- [ ] 16. Write all TUI property-based tests (complete code, no cargo)
  - [ ] 16.1 tests/keybinding_property_tests.rs
    - **Property 13: Keybinding Consistency**
    - **Validates: Requirements 14.1, 14.2, 14.3**

  - [ ] 16.2 tests/status_color_property_tests.rs
    - **Property 6: Status-to-Color Mapping**
    - **Validates: Requirements 3.3, 4.3, 8.2, 10.2, 13.2, 13.3, 13.4**

  - [ ] 16.3 tests/token_utilization_property_tests.rs
    - **Property 10: Token Utilization Calculation**
    - **Validates: Requirements 4.2, 4.3**

  - [ ] 16.4 tests/hierarchy_property_tests.rs
    - **Property 8: Hierarchy Rendering**
    - **Validates: Requirements 3.1, 4.1**

  - [ ] 16.5 tests/filter_property_tests.rs
    - **Property 7: Filter Correctness**
    - **Validates: Requirements 3.8, 5.2, 5.3, 5.4, 6.2, 6.3, 7.7, 7.8, 9.7, 10.5, 10.6**

  - [ ] 16.6 tests/detail_panel_property_tests.rs
    - **Property 9: Detail Panel Completeness**
    - **Validates: Requirements 5.6**

  - [ ] 16.7 tests/dsl_syntax_property_tests.rs
    - **Property 11: DSL Syntax Highlighting**
    - **Validates: Requirements 11.1, 11.2, 11.3, 11.4, 11.5**

  - [ ] 16.8 tests/websocket_property_tests.rs
    - **Property 14: WebSocket Reconnection**
    - **Validates: Requirements 15.1, 15.2**

  - [ ] 16.9 tests/error_display_property_tests.rs
    - **Property 15: Error Display**
    - **Validates: Requirements 16.1, 16.2, 16.3**

- [ ] 17. Type verification pass for TUI (no cargo, manual review)
  - [ ] 17.1 Review all API client types match caliber-api
  - [ ] 17.2 Review all state types are complete
  - [ ] 17.3 Review all widget types are correct
  - [ ] 17.4 Review SynthBrute theme colors match design
  - [ ] 17.5 Review keybinding definitions are complete

### Phase 4: Human Checkpoint - Build TUI in WSL

- [ ] 18. **HUMAN ACTION: Run cargo build for TUI**
  - [ ] 18.1 Human runs `cargo build -p caliber-tui` in WSL
  - [ ] 18.2 Human reports ALL compiler errors, warnings, type mismatches
  - [ ] 18.3 Agent fixes all issues in one iteration
  - [ ] 18.4 Repeat until clean build

### Phase 5: Human Checkpoint - Test Everything in WSL

- [ ] 19. **HUMAN ACTION: Run all tests**
  - [ ] 19.1 Human runs `cargo test --workspace` in WSL
  - [ ] 19.2 Human reports ALL test failures with full output
  - [ ] 19.3 Agent fixes all test issues in one iteration
  - [ ] 19.4 Repeat until all tests pass

- [ ] 20. **HUMAN ACTION: Run clippy**
  - [ ] 20.1 Human runs `cargo clippy --workspace` in WSL
  - [ ] 20.2 Human reports ALL clippy warnings
  - [ ] 20.3 Agent fixes all warnings in one iteration
  - [ ] 20.4 Repeat until zero warnings

### Phase 6: Integration and Manual Testing

- [ ] 21. Integration testing preparation
  - [ ] 21.1 Create integration test scenarios document
  - [ ] 21.2 Document manual testing checklist for all 10 views
  - [ ] 21.3 Document WebSocket event testing procedure
  - [ ] 21.4 Document multi-tenant testing scenarios

- [ ] 22. **HUMAN ACTION: Manual smoke testing**
  - [ ] 22.1 Human starts caliber-api server in WSL
  - [ ] 22.2 Human starts caliber-tui in WSL
  - [ ] 22.3 Human tests each view and reports issues
  - [ ] 22.4 Agent fixes issues based on feedback

- [ ] 23. Final polish
  - [ ] 23.1 Review all error messages for clarity
  - [ ] 23.2 Review all UI text for consistency
  - [ ] 23.3 Verify SynthBrute aesthetic throughout
  - [ ] 23.4 Final documentation pass

## Notes

- All tasks are required - no optional markers
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties using proptest
- Follow steering: NO STUBS, complete code only
- SynthBrute aesthetic: dark bg, cyan/magenta/yellow accents, thick borders
- ✅ **Prerequisite complete:** caliber-pg is production-ready - proceed with confidence!
