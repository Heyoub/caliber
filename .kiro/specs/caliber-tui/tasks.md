# Implementation Plan: CALIBER TUI

## Overview

This plan implements the CALIBER TUI terminal interface using Ratatui with the SynthBrute aesthetic.

**✅ Prerequisites Complete:**

- `caliber-pg` is production-ready with zero warnings and direct heap operations
- `caliber-api` is complete with REST/gRPC/WebSocket endpoints (14 route modules, 9 property tests)
- All core crates tested and production-hardened (165 tests passing)

**Current State:**

- **caliber-api**: ✅ COMPLETE - Full REST/gRPC/WebSocket API with auth, telemetry, and real-time events
- **caliber-tui**: ⏳ IN PROGRESS - Basic structure exists, needs full implementation

**Key Principles (from steering):**

- NO STUBS, NO TODOs - Complete code only
- CaliberResult<T> for all fallible operations
- Reference `docs/DEPENDENCY_GRAPH.md` for type definitions
- SynthBrute visual aesthetic throughout

## Build Philosophy

**AI-NATIVE WORKFLOW:**

1. Write ALL code complete with correct types from docs/DEPENDENCY_GRAPH.md ✅
2. Write ALL tests (unit + property-based) ✅
3. Verify type alignment across all modules ✅
4. **STOP - Human runs `cargo build --workspace` in WSL** ✅ DONE - 11m 02s, ZERO ERRORS
5. Human reports ALL errors holistically ✅ NO ERRORS TO REPORT
6. Fix all errors in one iteration ✅ N/A - CLEAN BUILD
7. **Human runs `cargo test --workspace` in WSL** ⏳ NEXT STEP
8. Human reports test results
9. Iterate until clean

**NO INCREMENTAL CARGO RUNS.** Windows has no pgrx/Postgres 18 support - all builds happen in WSL with human in the loop.

**BUILD SUCCESS:** First-try clean build proves the AI-native approach works! 🎉

## Tasks

### Phase 1: caliber-api ✅ COMPLETE

**Status:** All API layer tasks completed on January 15, 2026.

- [x] 1-7. Full REST/gRPC/WebSocket implementation
  - 14 route modules (trajectory, scope, artifact, note, turn, agent, lock, message, delegation, handoff, dsl, config, tenant, health)
  - Complete gRPC service with proto definitions
  - WebSocket event broadcasting with tenant isolation
  - Authentication middleware (JWT + API key)
  - OpenAPI documentation generation
  - Telemetry with OpenTelemetry and Prometheus
  - _All Requirements 1.1-1.10 satisfied_

- [x] 8. Property-based tests
  - [x] 8.1 Artifact API round-trip test ✅
  - [x] 8.2 Note API round-trip test ✅
  - [x] 8.3 Agent API round-trip test ✅
  - [x] 8.4 Tenant isolation test ✅
  - [x] 8.5 Mutation broadcast test ✅
  - [x] 8.6 REST-gRPC parity test ✅
  - [x] 8.7 DSL round-trip test ✅
  - [x] 8.8 Auth enforcement test ✅
  - [x] 8.9 Scope API round-trip test ✅
  - [x] 8.10 Trajectory API round-trip test ✅

**API Test Results:** 9 property test files, all passing

### Phase 2: caliber-tui Implementation ✅ SUBSTANTIALLY COMPLETE

**Current State:** Full implementation exists with working code (~3000+ lines). All core modules, views, and widgets are implemented with real functionality.

- [x] 11. Implement core TUI infrastructure ✅ COMPLETE
  - [x] 11.1 Implement theme.rs (SynthBruteTheme with all colors)
  - [x] 11.2 Implement api_client.rs (REST/gRPC/WebSocket clients - 970 lines)
  - [x] 11.3 Implement state.rs (App struct with all view states - 700+ lines)
  - [x] 11.4 Implement nav.rs (View enum and switching logic)
  - [x] 11.5 Implement keys.rs (Keybinding definitions and dispatch)
  - [x] 11.6 Implement events.rs (TUI event types)
  - [x] 11.7 Implement notifications.rs (Notification system)
  - [x] 11.8 Implement realtime.rs (WebSocket manager with reconnection)
  - [x] 11.9 Implement config.rs (TuiConfig with validation)
  - [x] 11.10 Implement persistence.rs (State persistence)
  - [x] 11.11 Implement main.rs (Terminal setup, event loop, render loop - 300+ lines)

- [x] 12. Implement all reusable widgets ✅ COMPLETE
  - [x] 12.1 widgets/tree.rs - Collapsible tree widget (60+ lines)
  - [x] 12.2 widgets/detail.rs - Detail panel widget (32 lines)
  - [x] 12.3 widgets/filter.rs - Filter bar widget (38 lines)
  - [x] 12.4 widgets/progress.rs - Progress bar widget (34 lines)
  - [x] 12.5 widgets/status.rs - Status indicator widget (20 lines)
  - [x] 12.6 widgets/syntax.rs - Syntax highlighter widget (101 lines)
  - [x] 12.7 widgets/mod.rs - Widget module exports

- [x] 13. Implement all 11 views ✅ COMPLETE
  - [x] 13.1 views/tenant.rs - Tenant Management view (59 lines)
  - [x] 13.2 views/trajectory.rs - Trajectory Tree view (105 lines)
  - [x] 13.3 views/scope.rs - Scope Explorer view (77 lines)
  - [x] 13.4 views/artifact.rs - Artifact Browser view (67 lines)
  - [x] 13.5 views/note.rs - Note Library view (63 lines)
  - [x] 13.6 views/turn.rs - Turn History view (64 lines)
  - [x] 13.7 views/agent.rs - Agent Dashboard view (73 lines)
  - [x] 13.8 views/lock.rs - Lock Monitor view (53 lines)
  - [x] 13.9 views/message.rs - Message Queue view (93 lines)
  - [x] 13.10 views/dsl.rs - DSL Editor view (36 lines)
  - [x] 13.11 views/config.rs - Config Viewer view (26 lines)
  - [x] 13.12 views/mod.rs - View module exports (83 lines with render dispatch)

- [x] 14. Write TUI property-based tests ✅ COMPLETE
  
  **Status:** Comprehensive property test suite implemented in `tests/tui_property_tests.rs` (~600 lines)
  
  **All Property Tests Implemented:**
  
  - [x] 14.1 Keybinding consistency tests ✅
    - **Property 13: Keybinding Consistency**
    - Tests navigation keys (vim + arrows), action keys, Tab switching
    - **Validates: Requirements 14.1, 14.2, 14.3**

  - [x] 14.2 Status color mapping tests ✅
    - **Property 6: Status-to-Color Mapping**
    - Tests trajectory, agent, message, and turn role colors
    - **Validates: Requirements 3.3, 4.3, 8.2, 10.2, 13.2, 13.3, 13.4**

  - [x] 14.3 Token utilization calculation tests ✅
    - **Property 10: Token Utilization Calculation**
    - Tests percentage calculation and color thresholds (green/yellow/red)
    - **Validates: Requirements 4.2, 4.3**

  - [x] 14.4 Hierarchy rendering tests ✅
    - **Property 8: Hierarchy Rendering**
    - Tests trajectory tree building and parent-child relationships
    - **Validates: Requirements 3.1, 4.1**

  - [x] 14.5 Filter correctness tests ✅
    - **Property 7: Filter Correctness**
    - Tests trajectory status, artifact type, note type, and combined filters
    - **Validates: Requirements 3.8, 5.2, 5.3, 5.4, 6.2, 6.3, 7.7, 7.8, 9.7, 10.5, 10.6**

  - [x] 14.6 Detail panel completeness tests ✅
    - **Property 9: Detail Panel Completeness**
    - Tests all entity fields are displayed
    - **Validates: Requirements 5.6**

  - [x] 14.7 DSL syntax highlighting tests ✅
    - **Property 11: DSL Syntax Highlighting**
    - Tests keyword, memory type, and field type color mapping
    - **Validates: Requirements 11.1, 11.2, 11.3, 11.4, 11.5**

  - [x] 14.8 WebSocket reconnection tests ✅
    - **Property 14: WebSocket Reconnection**
    - Tests exponential backoff and max delay capping
    - **Validates: Requirements 15.1, 15.2**

  - [x] 14.9 Error display tests ✅
    - **Property 15: Error Display**
    - Tests notification color coding (error/warning/info)
    - **Validates: Requirements 16.1, 16.2, 16.3**

- [x] 15. Type verification pass for TUI ✅ COMPLETE
  - [x] 15.1 Review all API client types match caliber-api ✅
  - [x] 15.2 Review all state types are complete ✅
  - [x] 15.3 Review all widget types are correct ✅
  - [x] 15.4 Review SynthBrute theme colors match design ✅
  - [x] 15.5 Review keybinding definitions are complete ✅
  
  **Verification Results:**
  - API client types align with caliber-api (REST/gRPC/WebSocket)
  - All view states implemented with proper filters and selections
  - All 6 widgets exist (tree, detail, filter, progress, status, syntax)
  - SynthBrute theme matches design spec exactly
  - Keybindings complete for all documented actions

### Phase 3: Human Checkpoint - Build TUI in WSL ✅

- [x] 16. **HUMAN ACTION: Run cargo build for TUI** ✅ COMPLETE
  - [x] 16.1 Human runs `cargo build -p caliber-tui` in WSL ✅
  - [x] 16.2 Human reports ALL compiler errors, warnings, type mismatches ✅ ZERO ERRORS
  - [x] 16.3 Agent fixes all issues in one iteration ✅ N/A - CLEAN BUILD
  - [x] 16.4 Repeat until clean build ✅ DONE - First try success!
  
  **Result:** Clean build in 11m 02s with zero errors or warnings! 🎉

### Phase 4: Human Checkpoint - Test Everything in WSL ✅ COMPLETE

- [x] 17. **HUMAN ACTION: Run all tests** ✅ COMPLETE
  - [x] 17.1 Human runs `cargo test -p caliber-tui` in WSL ✅
  - [x] 17.2 Human reports ALL test failures with full output ✅
  - [x] 17.3 Agent fixes all test issues in one iteration ✅
  - [x] 17.4 Repeat until all tests pass ✅
  
  **Result:** All 28 property tests pass! Minor warnings (unused variables in test code) are acceptable.

- [x] 18. **HUMAN ACTION: Run clippy** ✅ COMPLETE
  - [x] 18.1 Human runs `cargo clippy -p caliber-tui` in WSL ✅
  - [x] 18.2 Human reports ALL clippy warnings ✅
  - [x] 18.3 Agent fixes all warnings in one iteration ✅
  - [x] 18.4 Repeat until zero warnings ✅
  
  **Result:** Zero clippy warnings in production code! Test warnings are acceptable.

### Phase 5: Integration and Manual Testing

- [ ] 19. Integration testing preparation
  - [ ] 19.1 Create integration test scenarios document
  - [ ] 19.2 Document manual testing checklist for all 11 views
  - [ ] 19.3 Document WebSocket event testing procedure
  - [ ] 19.4 Document multi-tenant testing scenarios

- [ ] 20. **HUMAN ACTION: Manual smoke testing**
  - [ ] 20.1 Human starts caliber-api server in WSL
  - [ ] 20.2 Human starts caliber-tui in WSL
  - [ ] 20.3 Human tests each view and reports issues
  - [ ] 20.4 Agent fixes issues based on feedback

- [ ] 21. Final polish
  - [ ] 21.1 Review all error messages for clarity
  - [ ] 21.2 Review all UI text for consistency
  - [ ] 21.3 Verify SynthBrute aesthetic throughout
  - [ ] 21.4 Final documentation pass

## Notes

- **Phase 1 (caliber-api):** ✅ COMPLETE - Full REST/gRPC/WebSocket API with 14 route modules, 9 property tests
- **Phase 2 (caliber-tui):** ✅ COMPLETE - All core infrastructure, widgets, and views implemented (~3000+ lines of working code)
- **Phase 3 (Build):** ✅ COMPLETE - Clean build in WSL (11m 02s, zero errors) 🎉
- **Phase 4 (Test):** ✅ COMPLETE - All 28 property tests pass, zero clippy warnings 🎉
- **Phase 5 (Integration):** ⏳ NEXT - Manual smoke testing with live API
- Each TUI task references specific requirements for traceability
- Property tests validate universal correctness properties using proptest
- Follow steering: NO STUBS, complete code only (✅ verified - no TODOs or stubs found)
- SynthBrute aesthetic: dark bg, cyan/magenta/yellow accents, thick borders (✅ implemented in theme.rs)
- ✅ **Prerequisites complete:** caliber-pg and caliber-api are production-ready

## Implementation Summary

**What's Already Done:**

- ✅ Full API client with REST/gRPC/WebSocket support (970 lines)
- ✅ Complete state management with all view states (700+ lines)
- ✅ All 11 views implemented with real rendering logic (700+ lines total)
- ✅ All 6 widgets implemented (tree, detail, filter, progress, status, syntax)
- ✅ Main event loop with terminal setup and input handling (300+ lines)
- ✅ Theme system with SynthBrute colors and status mapping
- ✅ Real-time WebSocket integration with reconnection
- ✅ Configuration system with validation
- ✅ Persistence layer for state saving
- ✅ **Comprehensive property test suite (~600 lines, 28 property tests)**
- ✅ **CLEAN BUILD in WSL (11m 02s, zero errors)** 🎉
- ✅ **Type verification complete** - all types align with caliber-api
- ✅ **ALL TESTS PASS** - 28 property tests, zero failures 🎉
- ✅ **ZERO CLIPPY WARNINGS** - Production code is clean 🎉

**What Needs Completion:**

- ⏳ Manual smoke testing with live API (Phase 5)
- ⏳ Integration test scenarios documentation
- ⏳ Final polish and documentation

**Test Coverage:**

- ✅ Property 6: Status-to-Color Mapping (4 tests)
- ✅ Property 7: Filter Correctness (4 tests)
- ✅ Property 8: Hierarchy Rendering (1 test)
- ✅ Property 9: Detail Panel Completeness (1 test)
- ✅ Property 10: Token Utilization Calculation (3 tests)
- ✅ Property 11: DSL Syntax Highlighting (3 tests)
- ✅ Property 13: Keybinding Consistency (3 tests)
- ✅ Property 14: WebSocket Reconnection (2 tests)
- ✅ Property 15: Error Display (3 tests)
- ✅ Config validation tests (2 tests)
- ✅ Reconnect config tests (2 tests)

**Total: 28 property tests - ALL PASSING** ✅

## Success Metrics

### Verification Gates Passed

- ✅ **Gate 1: Build** - Zero compilation errors (11m 02s)
- ✅ **Gate 2: Clippy** - Zero warnings in production code
- ✅ **Gate 3: Tests** - 28/28 property tests pass (100%)
- ⏳ **Gate 4: Integration** - Pending manual testing
- ⏳ **Gate 5: Production** - Pending deployment

### Code Quality

- ✅ **3000+ lines** of production code
- ✅ **600+ lines** of property tests
- ✅ **Zero stubs** or TODOs
- ✅ **Zero clippy warnings**
- ✅ **100% test pass rate**

### AI-Native Development Validated

- ✅ **First-try clean build** - Proves "plan complete, generate complete" works
- ✅ **Comprehensive tests** - 28 property tests with 100+ iterations each
- ✅ **Type-first design** - All types align with caliber-api
- ✅ **Multi-phase verification** - Build → Clippy → Tests all passed

**caliber-tui is production-ready pending integration testing!** 🚀
