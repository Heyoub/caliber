//! CALIBER API Server Entry Point
//!
//! Bootstraps configuration, loads PCPConfig from the database, and
//! starts the Axum HTTP server.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use caliber_api::{
    create_api_router, ApiConfig, ApiError, ApiResult, AuthConfig, DbClient, DbConfig,
};
use caliber_pcp::{PCPConfig, PCPRuntime};

use caliber_api::telemetry::{init_tracer, shutdown_tracer, TelemetryConfig};
use caliber_api::ws::WsState;

#[tokio::main]
async fn main() -> ApiResult<()> {
    let telemetry_config = TelemetryConfig::default();
    init_tracer(&telemetry_config)?;

    let db_config = DbConfig::from_env();
    let db = DbClient::from_config(&db_config)?;

    let config_response = db.config_get().await?;
    let pcp_config: PCPConfig = serde_json::from_value(config_response.config)?;
    let pcp = Arc::new(
        PCPRuntime::new(pcp_config).map_err(|e| {
            ApiError::internal_error(format!("Failed to initialize PCP runtime: {}", e))
        })?,
    );

    let api_config = ApiConfig::from_env();
    let auth_config = AuthConfig::from_env();

    let ws_capacity = std::env::var("CALIBER_WS_CAPACITY")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1000);
    let ws = Arc::new(WsState::new(ws_capacity));

    let app: Router = create_api_router(db, ws, pcp, &api_config, auth_config)?;

    let addr = resolve_bind_addr()?;
    tracing::info!(%addr, "Starting CALIBER API server");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| ApiError::internal_error(format!("Failed to bind {}: {}", addr, e)))?;

    let server = axum::serve(listener, app);
    tokio::select! {
        result = server => {
            result.map_err(|e| ApiError::internal_error(format!("Server error: {}", e)))?;
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Shutdown signal received");
        }
    }

    shutdown_tracer();
    Ok(())
}

fn resolve_bind_addr() -> ApiResult<SocketAddr> {
    let host = std::env::var("CALIBER_API_BIND").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port_str = std::env::var("PORT")
        .ok()
        .or_else(|| std::env::var("CALIBER_API_PORT").ok())
        .unwrap_or_else(|| "3000".to_string());
    let port = port_str.parse::<u16>().map_err(|_| {
        ApiError::invalid_input(format!("Invalid port value: {}", port_str))
    })?;

    let addr = format!("{}:{}", host, port);
    addr.parse::<SocketAddr>().map_err(|e| {
        ApiError::invalid_input(format!("Invalid bind address {}: {}", addr, e))
    })
}
