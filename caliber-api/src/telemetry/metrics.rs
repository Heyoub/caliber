//! Prometheus Metrics Definitions
//!
//! Defines all CALIBER metrics with appropriate labels and types.
//! Exposes a /metrics endpoint for Prometheus scraping.

use axum::{http::StatusCode, response::IntoResponse};
use once_cell::sync::Lazy;
use prometheus::{
    register_counter_vec, register_gauge, register_histogram_vec, CounterVec, Encoder, Gauge,
    HistogramVec, TextEncoder,
};

use crate::error::{ApiError, ApiResult};

/// HTTP request latency buckets (seconds)
/// Covers: 1ms, 5ms, 10ms, 25ms, 50ms, 100ms, 250ms, 500ms, 1s, 2.5s, 5s, 10s
const HTTP_LATENCY_BUCKETS: &[f64] = &[
    0.001, 0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0, 2.5, 5.0, 10.0,
];

/// Database operation latency buckets (seconds)
const DB_LATENCY_BUCKETS: &[f64] =
    &[0.001, 0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0, 2.5, 5.0];

/// Global metrics instance - initialized once at startup
pub static METRICS: Lazy<ApiResult<CaliberMetrics>> = Lazy::new(CaliberMetrics::new);

/// Container for all CALIBER metrics.
#[derive(Clone)]
pub struct CaliberMetrics {
    /// HTTP request counter - labels: method, path, status
    pub http_requests_total: CounterVec,

    /// HTTP request duration histogram - labels: method, path
    pub http_request_duration_seconds: HistogramVec,

    /// Database operation counter - labels: operation, entity, status
    pub db_operations_total: CounterVec,

    /// Database operation duration histogram - labels: operation, entity
    pub db_operation_duration_seconds: HistogramVec,

    /// Current active WebSocket connections
    pub websocket_connections: Gauge,

    /// Webhook delivery counter - labels: status (success/failure)
    pub webhook_deliveries_total: CounterVec,

    /// MCP tool call counter - labels: tool, status
    pub mcp_tool_calls_total: CounterVec,

    /// Active agent count (gauge)
    pub active_agents: Gauge,

    /// Active trajectories count (gauge)
    pub active_trajectories: Gauge,
}

impl CaliberMetrics {
    /// Create and register all metrics with Prometheus.
    pub fn new() -> ApiResult<Self> {
        Ok(Self {
            http_requests_total: register_counter_vec!(
                "caliber_http_requests_total",
                "Total number of HTTP requests",
                &["method", "path", "status"]
            )
            .map_err(|e| ApiError::internal_error(format!("Failed to register http_requests_total: {}", e)))?,

            http_request_duration_seconds: register_histogram_vec!(
                "caliber_http_request_duration_seconds",
                "HTTP request duration in seconds",
                &["method", "path"],
                HTTP_LATENCY_BUCKETS.to_vec()
            )
            .map_err(|e| ApiError::internal_error(format!("Failed to register http_request_duration_seconds: {}", e)))?,

            db_operations_total: register_counter_vec!(
                "caliber_db_operations_total",
                "Total number of database operations",
                &["operation", "entity", "status"]
            )
            .map_err(|e| ApiError::internal_error(format!("Failed to register db_operations_total: {}", e)))?,

            db_operation_duration_seconds: register_histogram_vec!(
                "caliber_db_operation_duration_seconds",
                "Database operation duration in seconds",
                &["operation", "entity"],
                DB_LATENCY_BUCKETS.to_vec()
            )
            .map_err(|e| ApiError::internal_error(format!("Failed to register db_operation_duration_seconds: {}", e)))?,

            websocket_connections: register_gauge!(
                "caliber_websocket_connections",
                "Current number of active WebSocket connections"
            )
            .map_err(|e| ApiError::internal_error(format!("Failed to register websocket_connections: {}", e)))?,

            webhook_deliveries_total: register_counter_vec!(
                "caliber_webhook_deliveries_total",
                "Total webhook deliveries",
                &["status"]
            )
            .map_err(|e| ApiError::internal_error(format!("Failed to register webhook_deliveries_total: {}", e)))?,

            mcp_tool_calls_total: register_counter_vec!(
                "caliber_mcp_tool_calls_total",
                "Total MCP tool invocations",
                &["tool", "status"]
            )
            .map_err(|e| ApiError::internal_error(format!("Failed to register mcp_tool_calls_total: {}", e)))?,

            active_agents: register_gauge!(
                "caliber_active_agents",
                "Current number of active agents"
            )
            .map_err(|e| ApiError::internal_error(format!("Failed to register active_agents: {}", e)))?,

            active_trajectories: register_gauge!(
                "caliber_active_trajectories",
                "Current number of active trajectories"
            )
            .map_err(|e| ApiError::internal_error(format!("Failed to register active_trajectories: {}", e)))?,
        })
    }

    /// Record an HTTP request.
    pub fn record_http_request(&self, method: &str, path: &str, status: u16, duration_secs: f64) {
        let status_str = status.to_string();
        self.http_requests_total
            .with_label_values(&[method, path, &status_str])
            .inc();
        self.http_request_duration_seconds
            .with_label_values(&[method, path])
            .observe(duration_secs);
    }

    /// Record a database operation.
    pub fn record_db_operation(
        &self,
        operation: &str,
        entity: &str,
        success: bool,
        duration_secs: f64,
    ) {
        let status = if success { "success" } else { "error" };
        self.db_operations_total
            .with_label_values(&[operation, entity, status])
            .inc();
        self.db_operation_duration_seconds
            .with_label_values(&[operation, entity])
            .observe(duration_secs);
    }

    /// Increment WebSocket connection count.
    pub fn ws_connected(&self) {
        self.websocket_connections.inc();
    }

    /// Decrement WebSocket connection count.
    pub fn ws_disconnected(&self) {
        self.websocket_connections.dec();
    }

    /// Record a webhook delivery.
    pub fn record_webhook_delivery(&self, success: bool) {
        let status = if success { "success" } else { "failure" };
        self.webhook_deliveries_total
            .with_label_values(&[status])
            .inc();
    }

    /// Record an MCP tool call.
    pub fn record_mcp_tool_call(&self, tool: &str, success: bool) {
        let status = if success { "success" } else { "error" };
        self.mcp_tool_calls_total
            .with_label_values(&[tool, status])
            .inc();
    }

    /// Set active agent count.
    pub fn set_active_agents(&self, count: i64) {
        self.active_agents.set(count as f64);
    }

    /// Set active trajectory count.
    pub fn set_active_trajectories(&self, count: i64) {
        self.active_trajectories.set(count as f64);
    }
}

impl Default for CaliberMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Handler for GET /metrics endpoint.
///
/// Returns Prometheus text format metrics.
#[utoipa::path(
    get,
    path = "/metrics",
    tag = "Observability",
    responses(
        (status = 200, description = "Prometheus metrics in text format", content_type = "text/plain"),
        (status = 500, description = "Failed to encode metrics"),
    ),
)]
pub async fn metrics_handler() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();

    match encoder.encode(&metric_families, &mut buffer) {
        Ok(_) => (
            StatusCode::OK,
            [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
            buffer,
        ),
        Err(e) => {
            tracing::error!(error = %e, "Failed to encode metrics");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [("content-type", "text/plain")],
                format!("Failed to encode metrics: {}", e).into_bytes(),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prometheus::core::Collector;

    #[test]
    fn test_metrics_creation() -> Result<(), String> {
        // Force initialization
        let metrics = METRICS
            .as_ref()
            .map_err(|e| format!("Metrics init failed: {}", e.message))?;
        assert!(metrics.http_requests_total.desc().len() > 0);
        Ok(())
    }

    #[test]
    fn test_record_http_request() -> Result<(), String> {
        let metrics = METRICS
            .as_ref()
            .map_err(|e| format!("Metrics init failed: {}", e.message))?;
        metrics.record_http_request("GET", "/api/v1/trajectories", 200, 0.015);
        // Metric should be recorded without panicking
        Ok(())
    }

    #[test]
    fn test_record_db_operation() -> Result<(), String> {
        let metrics = METRICS
            .as_ref()
            .map_err(|e| format!("Metrics init failed: {}", e.message))?;
        metrics.record_db_operation("create", "trajectory", true, 0.005);
        metrics.record_db_operation("get", "artifact", false, 0.010);
        Ok(())
    }

    #[test]
    fn test_websocket_metrics() -> Result<(), String> {
        let metrics = METRICS
            .as_ref()
            .map_err(|e| format!("Metrics init failed: {}", e.message))?;
        metrics.ws_connected();
        metrics.ws_connected();
        metrics.ws_disconnected();
        // Connection count should be 1
        Ok(())
    }

    #[test]
    fn test_webhook_metrics() -> Result<(), String> {
        let metrics = METRICS
            .as_ref()
            .map_err(|e| format!("Metrics init failed: {}", e.message))?;
        metrics.record_webhook_delivery(true);
        metrics.record_webhook_delivery(false);
        Ok(())
    }

    #[test]
    fn test_mcp_metrics() -> Result<(), String> {
        let metrics = METRICS
            .as_ref()
            .map_err(|e| format!("Metrics init failed: {}", e.message))?;
        metrics.record_mcp_tool_call("trajectory_create", true);
        metrics.record_mcp_tool_call("note_search", false);
        Ok(())
    }
}
