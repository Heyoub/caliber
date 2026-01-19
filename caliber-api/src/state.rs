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

impl FromRef<AppState> for DbClient {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl FromRef<AppState> for Arc<WsState> {
    fn from_ref(state: &AppState) -> Self {
        state.ws.clone()
    }
}

impl FromRef<AppState> for Arc<PCPRuntime> {
    fn from_ref(state: &AppState) -> Self {
        state.pcp.clone()
    }
}

impl FromRef<AppState> for Arc<WebhookState> {
    fn from_ref(state: &AppState) -> Self {
        state.webhook_state.clone()
    }
}

impl FromRef<AppState> for CaliberSchema {
    fn from_ref(state: &AppState) -> Self {
        state.graphql_schema.clone()
    }
}

impl FromRef<AppState> for Arc<BillingState> {
    fn from_ref(state: &AppState) -> Self {
        state.billing_state.clone()
    }
}

impl FromRef<AppState> for Arc<McpState> {
    fn from_ref(state: &AppState) -> Self {
        state.mcp_state.clone()
    }
}

impl FromRef<AppState> for std::time::Instant {
    fn from_ref(state: &AppState) -> Self {
        state.start_time
    }
}

#[cfg(feature = "workos")]
impl FromRef<AppState> for crate::workos_auth::WorkOsConfig {
    fn from_ref(state: &AppState) -> Self {
        state
            .workos_config
            .clone()
            .expect("WorkOS config missing from AppState")
    }
}
