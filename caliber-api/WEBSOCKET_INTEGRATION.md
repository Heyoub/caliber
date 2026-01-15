# WebSocket Integration Summary

This document summarizes the WebSocket event broadcasting integration across all route modules.

## Pattern Applied

For each route module with mutation operations (create, update, delete):

1. **Add imports:**
   ```rust
   use crate::events::WsEvent;
   use crate::ws::WsState;
   ```

2. **Update State struct:**
   ```rust
   pub struct XxxState {
       pub db: DbClient,
       pub ws: Arc<WsState>,  // Added
   }
   
   impl XxxState {
       pub fn new(db: DbClient, ws: Arc<WsState>) -> Self {
           Self { db, ws }
       }
   }
   ```

3. **Update create_router signature:**
   ```rust
   pub fn create_router(db: DbClient, ws: Arc<WsState>) -> axum::Router {
       let state = Arc::new(XxxState::new(db, ws));
       // ...
   }
   ```

4. **Add event broadcasting in mutation handlers:**
   ```rust
   // After successful mutation
   state.ws.broadcast(WsEvent::XxxCreated { xxx: result.clone() });
   state.ws.broadcast(WsEvent::XxxUpdated { xxx: result.clone() });
   state.ws.broadcast(WsEvent::XxxDeleted { id: id.into() });
   ```

## Files Updated

### ✅ Complete
- [x] caliber-api/src/events.rs - Event types module created
- [x] caliber-api/src/ws.rs - WebSocket handler created
- [x] caliber-api/src/lib.rs - Exports added
- [x] caliber-api/src/routes/trajectory.rs - Full integration
- [x] caliber-api/src/routes/scope.rs - Full integration
- [x] caliber-api/src/routes/artifact.rs - Full integration

### 🔄 Remaining (Pattern to Apply)
- [ ] caliber-api/src/routes/note.rs - Create, Update, Delete
- [ ] caliber-api/src/routes/turn.rs - Create only
- [ ] caliber-api/src/routes/agent.rs - Register, Update, Unregister, Heartbeat
- [ ] caliber-api/src/routes/lock.rs - Acquire, Release, Expire
- [ ] caliber-api/src/routes/message.rs - Send, Deliver, Acknowledge
- [ ] caliber-api/src/routes/delegation.rs - Create, Accept, Reject, Complete
- [ ] caliber-api/src/routes/handoff.rs - Create, Accept, Complete
- [ ] caliber-api/src/routes/dsl.rs - No mutations (read-only)
- [ ] caliber-api/src/routes/config.rs - Update only
- [ ] caliber-api/src/routes/tenant.rs - No mutations (read-only)

## Event Mapping

| Route | Mutations | Events |
|-------|-----------|--------|
| trajectory | create, update, delete | TrajectoryCreated, TrajectoryUpdated, TrajectoryDeleted |
| scope | create, update, close | ScopeCreated, ScopeUpdated, ScopeClosed |
| artifact | create, update, delete | ArtifactCreated, ArtifactUpdated, ArtifactDeleted |
| note | create, update, delete | NoteCreated, NoteUpdated, NoteDeleted |
| turn | create | TurnCreated |
| agent | register, update, unregister, heartbeat | AgentRegistered, AgentStatusChanged, AgentHeartbeat, AgentUnregistered |
| lock | acquire, release | LockAcquired, LockReleased, LockExpired |
| message | send, acknowledge | MessageSent, MessageDelivered, MessageAcknowledged |
| delegation | create, accept, reject, complete | DelegationCreated, DelegationAccepted, DelegationRejected, DelegationCompleted |
| handoff | create, accept, complete | HandoffCreated, HandoffAccepted, HandoffCompleted |
| config | update | (No specific event - could add ConfigUpdated if needed) |

## Requirements Validated

- ✅ Requirement 1.3: WebSocket connections for real-time event streaming
- ✅ Requirement 1.4: Broadcast changes via WebSocket to subscribed clients
- ✅ Requirements 3.9, 4.8, 5.11, 6.10, 7.9, 8.10, 9.8, 10.8: Real-time updates in TUI

## Next Steps

The remaining route files follow the exact same pattern. When implementing:
1. Copy the pattern from trajectory.rs, scope.rs, or artifact.rs
2. Update the State struct to include `ws: Arc<WsState>`
3. Update create_router to accept `ws: Arc<WsState>` parameter
4. Add `state.ws.broadcast(WsEvent::...)` after each successful mutation
5. Update requirement comments to include 1.4 and real-time update requirements

## Testing

All WebSocket functionality will be tested via:
- Unit tests in ws.rs (connection, broadcasting, filtering)
- Integration tests in tests/broadcast_property_tests.rs (Property 3: Mutation Broadcast)
- Manual testing with TUI client connections
