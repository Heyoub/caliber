//! Axum Middleware for HTTP Request Tracing and Metrics
//!
//! Provides automatic instrumentation of all HTTP requests with:
//! - Distributed tracing spans
//! - Prometheus metrics collection
//! - Trace context propagation (traceparent header)

use axum::{extract::Request, http::HeaderMap, middleware::Next, response::Response};
use opentelemetry::{
    global,
    trace::{SpanKind, Status, TraceContextExt, Tracer},
    Context, KeyValue,
};
use opentelemetry_http::HeaderExtractor;
use std::time::Instant;
use tracing::{info_span, Instrument};

use super::metrics::METRICS;

/// Extract trace context from incoming request headers.
///
/// Looks for W3C traceparent header for distributed tracing.
fn extract_trace_context(headers: &HeaderMap) -> Context {
    global::get_text_map_propagator(|propagator| propagator.extract(&HeaderExtractor(headers)))
}

/// Normalize path for metrics/spans (replace UUIDs and IDs with placeholders).
///
/// This prevents high-cardinality label explosion in Prometheus.
fn normalize_path(path: &str) -> String {
    // UUID pattern: 8-4-4-4-12 hex chars
    let uuid_pattern = regex::Regex::new(
        r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}",
    )
    .expect("Invalid UUID regex");

    // Numeric ID pattern
    let id_pattern = regex::Regex::new(r"/\d+(/|$)").expect("Invalid ID regex");

    let result = uuid_pattern.replace_all(path, "{id}");
    let result = id_pattern.replace_all(&result, "/{id}$1");
    result.to_string()
}

/// Observability middleware for Axum.
///
/// This middleware wraps every request with:
/// 1. OpenTelemetry span (with trace context propagation)
/// 2. Prometheus metrics recording
/// 3. Request/response logging
pub async fn observability_middleware(request: Request, next: Next) -> Response {
    let start = Instant::now();

    // Extract request metadata
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = uri.path().to_string();
    let normalized_path = normalize_path(&path);

    // Extract trace context from headers (if present)
    let parent_context = extract_trace_context(request.headers());

    // Create OpenTelemetry span
    let tracer = global::tracer("caliber-api");
    let span = tracer
        .span_builder(format!("{} {}", method, normalized_path))
        .with_kind(SpanKind::Server)
        .with_attributes(vec![
            KeyValue::new("http.method", method.to_string()),
            KeyValue::new("http.target", path.clone()),
            KeyValue::new("http.route", normalized_path.clone()),
        ])
        .start_with_context(&tracer, &parent_context);

    let cx = Context::current_with_span(span);

    // Create tracing span for compatibility with existing tracing ecosystem
    let tracing_span = info_span!(
        "http_request",
        http.method = %method,
        http.target = %path,
        http.route = %normalized_path,
        otel.kind = "server",
    );

    // Execute the request with context
    let _guard = cx.clone().attach();
    let response = next.run(request).instrument(tracing_span).await;

    // Record metrics and complete span
    let duration = start.elapsed();
    let status = response.status();
    let duration_secs = duration.as_secs_f64();

    // Record Prometheus metrics
    METRICS.record_http_request(method.as_str(), &normalized_path, status.as_u16(), duration_secs);

    // Update span with response status
    cx.span()
        .set_attribute(KeyValue::new("http.status_code", status.as_u16() as i64));

    if status.is_server_error() {
        cx.span().set_status(Status::error("Server error"));
    } else if status.is_client_error() {
        cx.span().set_status(Status::error("Client error"));
    } else {
        cx.span().set_status(Status::Ok);
    }

    cx.span().end();

    // Log request completion
    tracing::info!(
        method = %method,
        path = %path,
        status = status.as_u16(),
        duration_ms = duration.as_millis(),
        "Request completed"
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path_uuid() {
        let path = "/api/v1/trajectories/550e8400-e29b-41d4-a716-446655440000";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "/api/v1/trajectories/{id}");
    }

    #[test]
    fn test_normalize_path_numeric_id() {
        let path = "/api/v1/items/12345";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "/api/v1/items/{id}");
    }

    #[test]
    fn test_normalize_path_mixed() {
        let path = "/api/v1/trajectories/550e8400-e29b-41d4-a716-446655440000/scopes/123";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "/api/v1/trajectories/{id}/scopes/{id}");
    }

    #[test]
    fn test_normalize_path_no_ids() {
        let path = "/api/v1/trajectories";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "/api/v1/trajectories");
    }

    #[test]
    fn test_normalize_path_health() {
        let path = "/health/ready";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "/health/ready");
    }
}
