# Implementation Plan: CALIBER TUI

## Overview

This plan implements the full CALIBER TUI system consisting of two crates:
1. **caliber-api** - REST/gRPC/WebSocket API layer (Axum + Tonic)
2. **caliber-tui** - Terminal UI (Ratatui) with SynthBrute aesthetic

**Prerequisite:** `caliber-pg-hot-path` spec must be completed first.

**Key Principles (from steering):**
- NO STUBS, NO TODOs - Complete code only
- NO SQL IN HOT PATH - API calls caliber_* pg_extern functions
- CaliberResult<T> for all fallible operations
- Reference `docs/DEPENDENCY_GRAPH.md` for type definitions
- SynthBrute visual aesthetic throughout

## Tasks

### Phase 1: caliber-api Crate Foundation

- [ ] 1. Set up caliber-api crate structure
  - [ ] 1.1 Create caliber-api/Cargo.toml with dependencies
    - axum, tonic, tokio, tower, deadpool-postgres
    - serde, serde_json for serialization
    - jsonwebtoken for JWT auth
    - tokio-tungstenite for WebSocket
    - _Requirements: 1.1, 1.2, 1.3_

  - [ ] 1.2 Create API types module (types.rs)
    - Define all request/response structs from design
    - CreateTrajectoryRequest, UpdateTrajectoryRequest, etc.
    - SearchRequest, SearchResult
    - TenantInfo, TenantStatus
    - _Requirements: 1.1, 1.2_

  - [ ] 1.3 Create error types module (error.rs)
    - ApiError struct with code, message, details
    - ErrorCode enum (Unauthorized, Forbidden, ValidationFailed, etc.)
    - Implement IntoResponse for Axum
    - _Requirements: 1.7, 1.8, 16.3_

  - [ ] 1.4 Create database connection pool module (db.rs)
    - deadpool-postgres pool configuration
    - Connection wrapper that calls caliber_* functions
    - NOT raw SQL - call pg_extern functions only
    - _Requirements: 1.9, 1.10_

- [ ] 2. Implement authentication middleware
  - [ ] 2.1 Create auth module (auth.rs)
    - API key validation
    - JWT token validation
    - Extract tenant context from headers
    - _Requirements: 1.5, 1.6_

  - [ ] 2.2 Create auth middleware for Axum
    - Reject unauthenticated requests with 401
    - Reject unauthorized tenant access with 403
    - Inject TenantContext into request extensions
    - _Requirements: 1.7, 1.8_

  - [ ] 2.3 Write property test for authentication enforcement
    - **Property 4: Authentication Enforcement**
    - Generate requests with/without auth, verify responses
    - **Validates: Requirements 1.5, 1.7, 1.8**

- [ ] 3. Implement Trajectory REST endpoints
  - [ ] 3.1 Create trajectory routes (routes/trajectory.rs)
    - POST /api/v1/trajectories - create
    - GET /api/v1/trajectories - list with filters
    - GET /api/v1/trajectories/{id} - get by ID
    - PATCH /api/v1/trajectories/{id} - update
    - DELETE /api/v1/trajectories/{id} - delete
    - GET /api/v1/trajectories/{id}/scopes - list scopes
    - GET /api/v1/trajectories/{id}/children - list children
    - _Requirements: 1.1, 3.5, 3.6, 3.7_

  - [ ] 3.2 Write property test for Trajectory API round-trip
    - **Property 1: API Completeness (Trajectory)**
    - Create, get, update, delete cycle
    - **Validates: Requirements 1.1**

- [ ] 4. Implement Scope REST endpoints
  - [ ] 4.1 Create scope routes (routes/scope.rs)
    - POST /api/v1/scopes - create
    - GET /api/v1/scopes/{id} - get by ID
    - PATCH /api/v1/scopes/{id} - update
    - POST /api/v1/scopes/{id}/checkpoint - create checkpoint
    - POST /api/v1/scopes/{id}/close - close scope
    - GET /api/v1/scopes/{id}/turns - list turns
    - GET /api/v1/scopes/{id}/artifacts - list artifacts
    - _Requirements: 1.1, 4.5, 4.6_

  - [ ] 4.2 Write property test for Scope API round-trip
    - **Property 1: API Completeness (Scope)**
    - **Validates: Requirements 1.1**

- [ ] 5. Implement Artifact REST endpoints
  - [ ] 5.1 Create artifact routes (routes/artifact.rs)
    - POST /api/v1/artifacts - create
    - GET /api/v1/artifacts - list with filters
    - GET /api/v1/artifacts/{id} - get by ID
    - PATCH /api/v1/artifacts/{id} - update
    - DELETE /api/v1/artifacts/{id} - delete
    - POST /api/v1/artifacts/search - similarity search
    - _Requirements: 1.1, 5.7, 5.8, 5.9, 5.10_

  - [ ] 5.2 Write property test for Artifact API round-trip
    - **Property 1: API Completeness (Artifact)**
    - **Validates: Requirements 1.1**

- [ ] 6. Implement Note REST endpoints
  - [ ] 6.1 Create note routes (routes/note.rs)
    - POST /api/v1/notes - create
    - GET /api/v1/notes - list with filters
    - GET /api/v1/notes/{id} - get by ID
    - PATCH /api/v1/notes/{id} - update
    - DELETE /api/v1/notes/{id} - delete
    - POST /api/v1/notes/search - similarity search
    - _Requirements: 1.1, 6.6, 6.7, 6.8_

  - [ ] 6.2 Write property test for Note API round-trip
    - **Property 1: API Completeness (Note)**
    - **Validates: Requirements 1.1**

- [ ] 7. Implement Turn REST endpoints
  - [ ] 7.1 Create turn routes (routes/turn.rs)
    - POST /api/v1/turns - create
    - GET /api/v1/turns/{id} - get by ID
    - _Requirements: 1.1_

- [ ] 8. Implement Agent REST endpoints
  - [ ] 8.1 Create agent routes (routes/agent.rs)
    - POST /api/v1/agents - register
    - GET /api/v1/agents - list
    - GET /api/v1/agents/{id} - get by ID
    - PATCH /api/v1/agents/{id} - update
    - DELETE /api/v1/agents/{id} - unregister
    - POST /api/v1/agents/{id}/heartbeat - heartbeat
    - _Requirements: 1.1, 8.6, 8.7, 8.8_

  - [ ] 8.2 Write property test for Agent API round-trip
    - **Property 1: API Completeness (Agent)**
    - **Validates: Requirements 1.1**

- [ ] 9. Implement Lock REST endpoints
  - [ ] 9.1 Create lock routes (routes/lock.rs)
    - POST /api/v1/locks/acquire - acquire lock
    - POST /api/v1/locks/{id}/release - release lock
    - POST /api/v1/locks/{id}/extend - extend lock
    - GET /api/v1/locks - list active locks
    - _Requirements: 1.1, 9.6_

- [ ] 10. Implement Message REST endpoints
  - [ ] 10.1 Create message routes (routes/message.rs)
    - POST /api/v1/messages - send message
    - GET /api/v1/messages - list messages
    - POST /api/v1/messages/{id}/acknowledge - acknowledge
    - _Requirements: 1.1_

- [ ] 11. Implement Delegation and Handoff REST endpoints
  - [ ] 11.1 Create delegation routes (routes/delegation.rs)
    - POST /api/v1/delegations - create
    - GET /api/v1/delegations/{id} - get
    - POST /api/v1/delegations/{id}/accept - accept
    - POST /api/v1/delegations/{id}/reject - reject
    - POST /api/v1/delegations/{id}/complete - complete
    - _Requirements: 1.1_

  - [ ] 11.2 Create handoff routes (routes/handoff.rs)
    - POST /api/v1/handoffs - create
    - GET /api/v1/handoffs/{id} - get
    - POST /api/v1/handoffs/{id}/accept - accept
    - POST /api/v1/handoffs/{id}/complete - complete
    - _Requirements: 1.1_

- [ ] 12. Implement DSL and Config REST endpoints
  - [ ] 12.1 Create DSL routes (routes/dsl.rs)
    - POST /api/v1/dsl/validate - validate DSL source
    - POST /api/v1/dsl/parse - parse and return AST
    - _Requirements: 1.1, 11.6, 11.7_

  - [ ] 12.2 Create config routes (routes/config.rs)
    - GET /api/v1/config - get current config
    - PATCH /api/v1/config - update config
    - POST /api/v1/config/validate - validate config
    - _Requirements: 1.1, 12.5, 12.6_

  - [ ] 12.3 Write property test for DSL round-trip
    - **Property 12: DSL Validation Round-Trip**
    - Parse, print, parse again, compare ASTs
    - **Validates: Requirements 11.6, 11.7**

- [ ] 13. Implement Tenant REST endpoints
  - [ ] 13.1 Create tenant routes (routes/tenant.rs)
    - GET /api/v1/tenants - list accessible tenants
    - GET /api/v1/tenants/{id} - get tenant info
    - _Requirements: 1.1, 2.4_

  - [ ] 13.2 Write property test for tenant isolation
    - **Property 5: Tenant Isolation**
    - Cross-tenant requests return only own data
    - **Validates: Requirements 1.6, 2.5**

- [ ] 14. Implement WebSocket event broadcasting
  - [ ] 14.1 Create WebSocket handler (ws.rs)
    - GET /api/v1/ws - WebSocket upgrade
    - tokio broadcast channel for events
    - Subscribe clients to tenant-specific events
    - _Requirements: 1.3, 1.4_

  - [ ] 14.2 Create event types module (events.rs)
    - WsEvent enum with all event variants
    - TrajectoryCreated, ScopeUpdated, etc.
    - Serialize to JSON for WebSocket
    - _Requirements: 1.4_

  - [ ] 14.3 Integrate event broadcasting into routes
    - Broadcast on create/update/delete operations
    - Filter by tenant for isolation
    - _Requirements: 1.4, 3.9, 4.8, 5.11, 6.10, 7.9, 8.10, 9.8, 10.8_

  - [ ] 14.4 Write property test for mutation broadcast
    - **Property 3: Mutation Broadcast**
    - Perform mutation, verify WebSocket event received
    - **Validates: Requirements 1.4**

- [ ] 15. Implement gRPC service
  - [ ] 15.1 Create proto definitions (proto/caliber.proto)
    - Define all service methods from design
    - Message types matching REST request/response
    - _Requirements: 1.2_

  - [ ] 15.2 Generate Rust code from proto
    - Use tonic-build in build.rs
    - _Requirements: 1.2_

  - [ ] 15.3 Implement CaliberService (grpc.rs)
    - All RPC methods calling same handlers as REST
    - SubscribeEvents streaming method
    - _Requirements: 1.2_

  - [ ] 15.4 Write property test for REST-gRPC parity
    - **Property 2: REST-gRPC Parity**
    - Same input to REST and gRPC returns same output
    - **Validates: Requirements 1.1, 1.2**

- [ ] 16. Checkpoint - API layer complete
  - Ensure all tests pass, ask the user if questions arise.
  - Run `cargo test -p caliber-api`
  - Verify all endpoints respond correctly

### Phase 2: caliber-tui Crate Foundation

- [ ] 17. Set up caliber-tui crate structure
  - [ ] 17.1 Create caliber-tui/Cargo.toml with dependencies
    - ratatui for TUI framework
    - crossterm for terminal backend
    - tokio for async runtime
    - reqwest for REST client
    - tonic for gRPC client
    - tokio-tungstenite for WebSocket client
    - tui-textarea for DSL editor
    - _Requirements: All TUI requirements_

  - [ ] 17.2 Create SynthBrute theme module (theme.rs)
    - SynthBruteTheme struct with all colors
    - Background: #0a0a0a, #1a1a1a, #2a2a2a
    - Primary: cyan #00ffff
    - Secondary: magenta #ff00ff
    - Tertiary: yellow #ffff00
    - Status colors: green, yellow, red
    - _Requirements: 13.1, 13.2, 13.3, 13.4_

  - [ ] 17.3 Create API client module (api_client.rs)
    - REST client with reqwest
    - gRPC client with tonic
    - WebSocket client for real-time events
    - _Requirements: 1.1, 1.2, 1.3_

  - [ ] 17.4 Create app state module (state.rs)
    - App struct with all view states
    - TenantContext for multi-tenant
    - Event queue for real-time updates
    - _Requirements: 2.1, 2.2, 15.1_

- [ ] 18. Implement core TUI infrastructure
  - [ ] 18.1 Create main event loop (main.rs)
    - Terminal setup with crossterm
    - Event handling (keyboard, mouse, resize)
    - Render loop with ratatui
    - _Requirements: 14.1, 14.9, 14.10_

  - [ ] 18.2 Create navigation module (nav.rs)
    - View enum with all 10 views
    - View switching with number keys 1-0
    - Tab cycling between panels
    - _Requirements: 14.2, 14.3_

  - [ ] 18.3 Create keybinding module (keys.rs)
    - Vim-style navigation (h/j/k/l)
    - Global keys (q, ?, /, :)
    - View-specific keys (n, e, d, etc.)
    - _Requirements: 14.1, 14.4, 14.5, 14.6, 14.7, 14.8_

  - [ ] 18.4 Write property test for keybinding consistency
    - **Property 13: Keybinding Consistency**
    - Standard keys work in all views
    - **Validates: Requirements 14.1, 14.2, 14.3**

- [ ] 19. Implement reusable widget components
  - [ ] 19.1 Create tree widget (widgets/tree.rs)
    - Collapsible tree structure
    - Selection highlighting
    - SynthBrute styling with thick borders
    - _Requirements: 3.1, 3.4, 13.5_

  - [ ] 19.2 Create detail panel widget (widgets/detail.rs)
    - Key-value field display
    - Syntax highlighting for content
    - _Requirements: 3.2, 5.5, 6.4_

  - [ ] 19.3 Create filter bar widget (widgets/filter.rs)
    - Filter chips with active state
    - Search input field
    - _Requirements: 3.8, 5.2, 5.3_

  - [ ] 19.4 Create progress bar widget (widgets/progress.rs)
    - Token utilization display
    - Color thresholds (green/yellow/red)
    - _Requirements: 4.3_

  - [ ] 19.5 Create status indicator widget (widgets/status.rs)
    - Status-to-color mapping
    - Icon + label display
    - _Requirements: 3.3, 8.2, 10.2_

  - [ ] 19.6 Write property test for status-to-color mapping
    - **Property 6: Status-to-Color Mapping**
    - All status values map to correct colors
    - **Validates: Requirements 3.3, 4.3, 8.2, 10.2, 13.2, 13.3, 13.4**

  - [ ] 19.7 Write property test for token utilization
    - **Property 10: Token Utilization Calculation**
    - Verify percentage and color thresholds
    - **Validates: Requirements 4.2, 4.3**

- [ ] 20. Implement Tenant Management view
  - [ ] 20.1 Create tenant selector modal (views/tenant.rs)
    - List available tenants
    - Status indicators
    - Selection and switching
    - _Requirements: 2.1, 2.3, 2.4_

  - [ ] 20.2 Create tenant status bar component
    - Display current tenant name
    - Connection status indicator
    - _Requirements: 2.2, 15.2_

  - [ ] 20.3 Implement tenant persistence
    - Save last-used tenant locally
    - Auto-connect on startup
    - _Requirements: 2.6, 2.7_

- [ ] 21. Implement Trajectory Tree view
  - [ ] 21.1 Create trajectory tree view (views/trajectory.rs)
    - Hierarchical tree with parent-child
    - Color-coded by status
    - Expand/collapse with Enter
    - _Requirements: 3.1, 3.3, 3.4_

  - [ ] 21.2 Create trajectory detail panel
    - Name, description, status, agent, timestamps
    - _Requirements: 3.2_

  - [ ] 21.3 Implement trajectory CRUD operations
    - 'n' to create new
    - 'e' to edit selected
    - 'd' to delete with confirmation
    - _Requirements: 3.5, 3.6, 3.7_

  - [ ] 21.4 Implement trajectory filtering
    - Filter by status, agent, date range
    - _Requirements: 3.8_

  - [ ] 21.5 Implement real-time updates
    - WebSocket subscription
    - Update tree on events
    - _Requirements: 3.9_

  - [ ] 21.6 Write property test for hierarchy rendering
    - **Property 8: Hierarchy Rendering**
    - Children nested under parents
    - **Validates: Requirements 3.1, 4.1**

  - [ ] 21.7 Write property test for filter correctness
    - **Property 7: Filter Correctness (Trajectory)**
    - Filtered results match criteria
    - **Validates: Requirements 3.8**

- [ ] 22. Implement Scope Explorer view
  - [ ] 22.1 Create scope explorer view (views/scope.rs)
    - Scopes nested under trajectory
    - Token utilization progress bars
    - Checkpoint indicators
    - _Requirements: 4.1, 4.3, 4.4_

  - [ ] 22.2 Create scope detail panel
    - Token budget, used, percentage
    - Purpose and metadata
    - _Requirements: 4.2, 4.7_

  - [ ] 22.3 Implement scope operations
    - 'c' to create checkpoint
    - 'x' to close scope
    - _Requirements: 4.5, 4.6_

  - [ ] 22.4 Implement real-time updates
    - _Requirements: 4.8_

- [ ] 23. Implement Artifact Browser view
  - [ ] 23.1 Create artifact browser view (views/artifact.rs)
    - Filterable, sortable list
    - Type filtering (ErrorLog, CodePatch, etc.)
    - Full-text search
    - _Requirements: 5.1, 5.2, 5.3, 5.4_

  - [ ] 23.2 Create artifact detail panel
    - Full content with syntax highlighting
    - Provenance information
    - _Requirements: 5.5, 5.6_

  - [ ] 23.3 Implement artifact CRUD operations
    - 'n' to create, 'e' to edit, 'd' to delete
    - _Requirements: 5.7, 5.8, 5.9_

  - [ ] 23.4 Implement similarity search
    - Search by embedding if available
    - _Requirements: 5.10_

  - [ ] 23.5 Implement real-time updates
    - _Requirements: 5.11_

  - [ ] 23.6 Write property test for detail panel completeness
    - **Property 9: Detail Panel Completeness**
    - All non-null fields displayed
    - **Validates: Requirements 5.6**

- [ ] 24. Implement Note Library view
  - [ ] 24.1 Create note library view (views/note.rs)
    - Grouped by type (Convention, Strategy, etc.)
    - Type and date filtering
    - Full-text search
    - _Requirements: 6.1, 6.2, 6.3_

  - [ ] 24.2 Create note detail panel
    - Full content and source references
    - Access statistics
    - Superseded indicator
    - _Requirements: 6.4, 6.5, 6.9_

  - [ ] 24.3 Implement note CRUD operations
    - _Requirements: 6.6, 6.7, 6.8_

  - [ ] 24.4 Implement real-time updates
    - _Requirements: 6.10_

- [ ] 25. Implement Turn History view
  - [ ] 25.1 Create turn history view (views/turn.rs)
    - Chronological order within scope
    - Color-coded by role
    - Token count display
    - _Requirements: 7.1, 7.2, 7.4_

  - [ ] 25.2 Create turn detail panel
    - Full content with formatting
    - Tool calls/results collapsible
    - _Requirements: 7.3, 7.5, 7.6_

  - [ ] 25.3 Implement turn filtering and search
    - Filter by role
    - Search content
    - _Requirements: 7.7, 7.8_

  - [ ] 25.4 Implement real-time updates with auto-scroll
    - _Requirements: 7.9_

- [ ] 26. Implement Agent Dashboard view
  - [ ] 26.1 Create agent dashboard view (views/agent.rs)
    - All agents with status indicators
    - Color-coded status (Idle, Active, Blocked, Failed)
    - _Requirements: 8.1, 8.2_

  - [ ] 26.2 Create agent detail panel
    - Type, capabilities, memory access
    - Heartbeat with staleness warnings
    - Current trajectory/scope
    - _Requirements: 8.3, 8.4, 8.5_

  - [ ] 26.3 Implement agent CRUD operations
    - _Requirements: 8.6, 8.7, 8.8_

  - [ ] 26.4 Implement delegation graph/list
    - _Requirements: 8.9_

  - [ ] 26.5 Implement real-time updates
    - _Requirements: 8.10_

- [ ] 27. Implement Lock Monitor view
  - [ ] 27.1 Create lock monitor view (views/lock.rs)
    - Active locks with holder, resource, mode
    - Expiration countdown
    - Near-expiration highlighting
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_

  - [ ] 27.2 Implement force-release operation
    - 'r' to release with confirmation
    - _Requirements: 9.6_

  - [ ] 27.3 Implement lock filtering
    - By resource type or holder
    - _Requirements: 9.7_

  - [ ] 27.4 Implement real-time updates
    - _Requirements: 9.8_

- [ ] 28. Implement Message Queue view
  - [ ] 28.1 Create message queue view (views/message.rs)
    - Queue with sender, recipient, type, priority
    - Color-coded by priority
    - Delivery/ack status
    - _Requirements: 10.1, 10.2, 10.3_

  - [ ] 28.2 Create message detail panel
    - Full payload display
    - _Requirements: 10.4_

  - [ ] 28.3 Implement message filtering
    - By type, sender, recipient, trajectory
    - Highlight expired undelivered
    - _Requirements: 10.5, 10.6, 10.7_

  - [ ] 28.4 Implement real-time updates
    - _Requirements: 10.8_

- [ ] 29. Implement DSL Editor view
  - [ ] 29.1 Create DSL editor view (views/dsl.rs)
    - Text editor with tui-textarea
    - CALIBER DSL syntax highlighting
    - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5_

  - [ ] 29.2 Implement DSL validation
    - Validate on save via API
    - Inline error highlighting
    - _Requirements: 11.6, 11.7_

  - [ ] 29.3 Implement file operations
    - Load from filesystem
    - Save to filesystem
    - _Requirements: 11.8, 11.9_

  - [ ] 29.4 Implement AST preview panel (optional toggle)
    - _Requirements: 11.10_

  - [ ] 29.5 Write property test for DSL syntax highlighting
    - **Property 11: DSL Syntax Highlighting**
    - Keywords, types, strings, numbers colored correctly
    - **Validates: Requirements 11.1, 11.2, 11.3, 11.4, 11.5**

- [ ] 30. Implement Config Viewer view
  - [ ] 30.1 Create config viewer view (views/config.rs)
    - Formatted JSON/TOML display
    - Grouped sections
    - Validation status per section
    - _Requirements: 12.1, 12.2, 12.3, 12.4_

  - [ ] 30.2 Implement inline editing
    - Edit values in place
    - Validate before applying
    - _Requirements: 12.5, 12.6_

  - [ ] 30.3 Display provider connection status
    - _Requirements: 12.7_

- [ ] 31. Implement global features
  - [ ] 31.1 Create notification system (notifications.rs)
    - Error display in notification area
    - Color-coded by level
    - Actionable messages (retry, reconnect)
    - _Requirements: 16.1, 16.2, 16.3, 16.4, 16.5_

  - [ ] 31.2 Create error log panel
    - View recent errors
    - Log to local file
    - _Requirements: 16.6, 16.7_

  - [ ] 31.3 Create command palette
    - ':' to open
    - Command search and execution
    - _Requirements: 14.7_

  - [ ] 31.4 Create global search
    - '/' to open
    - Search across entities
    - _Requirements: 14.6_

  - [ ] 31.5 Create help modal
    - '?' to open
    - Full keybinding reference
    - _Requirements: 14.5_

  - [ ] 31.6 Write property test for error display
    - **Property 15: Error Display**
    - API errors shown in notification area
    - **Validates: Requirements 16.1, 16.2, 16.3**

- [ ] 32. Implement real-time update infrastructure
  - [ ] 32.1 Create WebSocket connection manager
    - Persistent connection to API
    - Reconnection with exponential backoff
    - Connection status indicator
    - _Requirements: 15.1, 15.2_

  - [ ] 32.2 Create event processor
    - Process incoming events
    - Update relevant view states
    - New data indicator animation
    - _Requirements: 15.3, 15.4_

  - [ ] 32.3 Implement pause/resume functionality
    - 'p' to toggle pause
    - PAUSED indicator
    - Queue events while paused
    - _Requirements: 15.5, 15.6, 15.7_

  - [ ] 32.4 Write property test for WebSocket reconnection
    - **Property 14: WebSocket Reconnection**
    - Reconnect on disconnect with backoff
    - **Validates: Requirements 15.1, 15.2**

- [ ] 33. Checkpoint - TUI complete
  - Ensure all tests pass, ask the user if questions arise.
  - Run `cargo test -p caliber-tui`
  - Manual testing of all 10 views

- [ ] 34. Final integration testing
  - [ ] 34.1 Write integration tests for API→TUI flow
    - Full CRUD cycles through TUI
    - Real-time updates verified
    - Multi-tenant scenarios

  - [ ] 34.2 Write integration tests for visual consistency
    - SynthBrute theme applied throughout
    - Color fallback for 16-color terminals
    - _Requirements: 13.10_

- [ ] 35. Final checkpoint - All tests pass
  - Ensure all tests pass, ask the user if questions arise.
  - Run full test suite
  - Run `cargo clippy --workspace` - no warnings
  - Manual smoke test of complete system

## Notes

- All tasks are required - no optional markers
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties using proptest
- Follow steering: NO STUBS, complete code only
- SynthBrute aesthetic: dark bg, cyan/magenta/yellow accents, thick borders
- Prerequisite: caliber-pg-hot-path must be completed first
