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

    // Validate required PostgreSQL extensions are installed
    validate_extensions(&db).await?;

    let config_response = db.config_get().await?;
    let pcp_config: PCPConfig = serde_json::from_value(config_response.config)?;
    let pcp = Arc::new(PCPRuntime::new(pcp_config).map_err(|e| {
        ApiError::internal_error(format!("Failed to initialize PCP runtime: {}", e))
    })?);

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

    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    );
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

/// Validates that required PostgreSQL extensions are installed.
/// Provides clear error messages if extensions are missing.
async fn validate_extensions(db: &DbClient) -> ApiResult<()> {
    let conn = db.get_conn().await?;

    // Check for caliber_pg extension
    let caliber_pg_check = conn
        .query_opt(
            "SELECT 1 FROM pg_extension WHERE extname = 'caliber_pg'",
            &[],
        )
        .await
        .map_err(|e| {
            ApiError::internal_error(format!("Failed to check for caliber_pg extension: {}", e))
        })?;

    if caliber_pg_check.is_none() {
        return Err(ApiError::internal_error(
            "Extension 'caliber_pg' is not installed. \
             Run: CREATE EXTENSION IF NOT EXISTS caliber_pg;"
                .to_string(),
        ));
    }

    // Check for pgvector extension
    let pgvector_check = conn
        .query_opt(
            "SELECT 1 FROM pg_extension WHERE extname = 'vector'",
            &[],
        )
        .await
        .map_err(|e| {
            ApiError::internal_error(format!("Failed to check for pgvector extension: {}", e))
        })?;

    if pgvector_check.is_none() {
        return Err(ApiError::internal_error(
            "Extension 'vector' (pgvector) is not installed. \
             Run: CREATE EXTENSION IF NOT EXISTS vector;"
                .to_string(),
        ));
    }

    tracing::info!("Required PostgreSQL extensions validated: caliber_pg, vector");
    Ok(())
}

fn resolve_bind_addr() -> ApiResult<SocketAddr> {
    let host = std::env::var("CALIBER_API_BIND").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port_str = std::env::var("PORT")
        .ok()
        .or_else(|| std::env::var("CALIBER_API_PORT").ok())
        .unwrap_or_else(|| "3000".to_string());
    let port = port_str
        .parse::<u16>()
        .map_err(|_| ApiError::invalid_input(format!("Invalid port value: {}", port_str)))?;

    let addr = format!("{}:{}", host, port);
    addr.parse::<SocketAddr>()
        .map_err(|e| ApiError::invalid_input(format!("Invalid bind address {}: {}", addr, e)))
}
