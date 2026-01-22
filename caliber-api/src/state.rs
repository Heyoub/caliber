//! Shared application state for Axum routers.

use std::sync::Arc;

use axum::extract::FromRef;
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

// WorkOS configuration is accessed directly from AppState in route handlers.
// The workos_config field is an Option<WorkOsConfig> that handlers check at runtime.
