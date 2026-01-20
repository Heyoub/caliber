//! Shared application state for Axum routers.

use std::sync::Arc;

use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use caliber_pcp::PCPRuntime;

use crate::db::DbClient;
use crate::routes::billing::BillingState;
use crate::routes::graphql::CaliberSchema;
use crate::routes::mcp::McpState;
use crate::routes::webhooks::WebhookState;
use crate::ws::WsState;

/// Application-wide state shared across all routes.
#[derive(Clone)]
pub struct AppState {
    pub db: DbClient,
    pub ws: Arc<WsState>,
    pub pcp: Arc<PCPRuntime>,
    pub webhook_state: Arc<WebhookState>,
    pub graphql_schema: CaliberSchema,
    pub billing_state: Arc<BillingState>,
    pub mcp_state: Arc<McpState>,
    pub start_time: std::time::Instant,
    #[cfg(feature = "workos")]
    pub workos_config: Option<crate::workos_auth::WorkOsConfig>,
}

// Use macro to reduce boilerplate for FromRef implementations
crate::impl_from_ref!(DbClient, db);
crate::impl_from_ref!(Arc<WsState>, ws);
crate::impl_from_ref!(Arc<PCPRuntime>, pcp);
crate::impl_from_ref!(Arc<WebhookState>, webhook_state);
crate::impl_from_ref!(CaliberSchema, graphql_schema);
crate::impl_from_ref!(Arc<BillingState>, billing_state);
crate::impl_from_ref!(Arc<McpState>, mcp_state);
crate::impl_from_ref!(std::time::Instant, start_time);

// WorkOS configuration extractor - returns error instead of panicking
#[cfg(feature = "workos")]
pub struct WorkOsConfigExtractor(pub crate::workos_auth::WorkOsConfig);

#[cfg(feature = "workos")]
impl<S> FromRequestParts<S> for WorkOsConfigExtractor
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = crate::error::ApiError;

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        app_state
            .workos_config
            .clone()
            .map(WorkOsConfigExtractor)
            .ok_or_else(|| {
                crate::error::ApiError::internal_error(
                    "WorkOS authentication enabled but not configured. \
                     Set CALIBER_WORKOS_CLIENT_ID and CALIBER_WORKOS_API_KEY environment variables.",
                )
            })
    }
}
