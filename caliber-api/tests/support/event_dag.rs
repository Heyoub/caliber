use std::sync::Arc;

use caliber_api::state::ApiEventDag;

pub fn test_event_dag() -> Arc<ApiEventDag> {
    Arc::new(ApiEventDag::new())
}
