//! Health Check Endpoints
//!
//! Provides Kubernetes-compatible health check endpoints:
//! - /health/ping - Simple liveness check
//! - /health/ready - Database connectivity check
//! - /health/live - Process alive check
//!
//! No authentication required for health endpoints.

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use serde::{Deserialize, Serialize};

use crate::{db::DbClient, state::AppState};

// ============================================================================
// TYPES
// ============================================================================

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct HealthResponse {
    pub status: HealthStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<HealthDetails>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Degraded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct HealthDetails {
    pub database: ComponentHealth,
    pub version: String,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ComponentHealth {
    pub status: HealthStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ============================================================================
// HANDLERS
// ============================================================================

/// GET /health/ping - Simple pong response
#[utoipa::path(
    get,
    path = "/health/ping",
    tag = "Health",
    responses(
        (status = 200, description = "Service is responding", body = String),
    ),
)]
pub async fn ping() -> impl IntoResponse {
    (StatusCode::OK, "pong")
}

/// GET /health/live - Process liveness check
#[utoipa::path(
    get,
    path = "/health/live",
    tag = "Health",
    responses(
        (status = 200, description = "Process is alive", body = HealthResponse),
    ),
)]
pub async fn liveness() -> impl IntoResponse {
    let response = HealthResponse {
        status: HealthStatus::Healthy,
        message: Some("Process is alive".to_string()),
        details: None,
    };
    (StatusCode::OK, Json(response))
}

/// GET /health/ready - Readiness check (database connectivity)
#[utoipa::path(
    get,
    path = "/health/ready",
    tag = "Health",
    responses(
        (status = 200, description = "Service is ready", body = HealthResponse),
        (status = 503, description = "Service is not ready", body = HealthResponse),
    ),
)]
pub async fn readiness(
    State(db): State<DbClient>,
    State(start_time): State<std::time::Instant>,
) -> impl IntoResponse {
    // Check database connectivity
    let db_health = match check_database(&db).await {
        Ok(latency) => ComponentHealth {
            status: HealthStatus::Healthy,
            latency_ms: Some(latency),
            error: None,
        },
        Err(e) => ComponentHealth {
            status: HealthStatus::Unhealthy,
            latency_ms: None,
            error: Some(e),
        },
    };

    let overall_status = if db_health.status == HealthStatus::Healthy {
        HealthStatus::Healthy
    } else {
        HealthStatus::Unhealthy
    };

    let response = HealthResponse {
        status: overall_status,
        message: None,
        details: Some(HealthDetails {
            database: db_health,
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: start_time.elapsed().as_secs(),
        }),
    };

    let status_code = if overall_status == HealthStatus::Healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status_code, Json(response))
}

async fn check_database(db: &DbClient) -> Result<u64, String> {
    let start = std::time::Instant::now();

    // Try to get a connection - this validates pool connectivity
    match db.health_check().await {
        Ok(_) => Ok(start.elapsed().as_millis() as u64),
        Err(e) => Err(format!("Database check failed: {}", e.message)),
    }
}

// ============================================================================
// ROUTER
// ============================================================================

/// Create health check router (no auth required)
pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/ping", get(ping))
        .route("/live", get(liveness))
        .route("/ready", get(readiness))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_serialization() -> Result<(), serde_json::Error> {
        let response = HealthResponse {
            status: HealthStatus::Healthy,
            message: Some("All systems operational".to_string()),
            details: None,
        };

        let json = serde_json::to_string(&response)?;
        assert!(json.contains("\"status\":\"healthy\""));
        Ok(())
    }

    #[test]
    fn test_health_status_variants() {
        assert_ne!(HealthStatus::Healthy, HealthStatus::Unhealthy);
        assert_ne!(HealthStatus::Healthy, HealthStatus::Degraded);
        assert_ne!(HealthStatus::Unhealthy, HealthStatus::Degraded);
    }

    #[test]
    fn test_health_details_structure() -> Result<(), serde_json::Error> {
        let details = HealthDetails {
            database: ComponentHealth {
                status: HealthStatus::Healthy,
                latency_ms: Some(5),
                error: None,
            },
            version: "0.1.0".to_string(),
            uptime_seconds: 3600,
        };

        let json = serde_json::to_string(&details)?;
        assert!(json.contains("\"version\":\"0.1.0\""));
        assert!(json.contains("\"uptime_seconds\":3600"));
        Ok(())
    }

    #[test]
    fn test_component_health_with_error() -> Result<(), serde_json::Error> {
        let component = ComponentHealth {
            status: HealthStatus::Unhealthy,
            latency_ms: None,
            error: Some("Connection refused".to_string()),
        };

        let json = serde_json::to_string(&component)?;
        assert!(json.contains("\"status\":\"unhealthy\""));
        assert!(json.contains("Connection refused"));
        Ok(())
    }
}
