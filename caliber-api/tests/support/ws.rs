use std::sync::Arc;

use caliber_api::ws::WsState;

pub fn test_ws_state(capacity: usize) -> Arc<WsState> {
    Arc::new(WsState::new(capacity))
}
