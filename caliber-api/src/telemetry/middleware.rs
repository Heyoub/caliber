//! Axum Middleware for HTTP Request Tracing and Metrics
//!
//! Provides automatic instrumentation of all HTTP requests with:
//! - Distributed tracing spans
//! - Prometheus metrics collection
//! - Trace context propagation (traceparent header)

use axum::http::HeaderMap;
use axum::{body::Body, middleware::Next, response::Response};
use opentelemetry::{
    global,
    propagation::Extractor,
    trace::{Status, TraceContextExt},
    Context, KeyValue,
};
use std::sync::OnceLock;
use std::time::Instant;
use tracing::{info_span, Instrument};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use super::metrics::METRICS;

/// Extract trace context from incoming request headers.
///
/// Looks for W3C traceparent header for distributed tracing.
fn extract_trace_context(headers: &HeaderMap) -> Context {
    global::get_text_map_propagator(|propagator| propagator.extract(&HeaderMapExtractor(headers)))
}

struct HeaderMapExtractor<'a>(&'a HeaderMap);

impl<'a> Extractor for HeaderMapExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|value| value.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|key| key.as_str()).collect()
    }
}

/// Normalize path for metrics/spans (replace UUIDs and IDs with placeholders).
///
/// This prevents high-cardinality label explosion in Prometheus.
fn normalize_path(path: &str) -> String {
    static UUID_REGEX: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();
    static ID_REGEX: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();

    let uuid_regex = UUID_REGEX.get_or_init(|| {
        regex::Regex::new(
            r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}",
        )
    });
    let id_regex = ID_REGEX.get_or_init(|| regex::Regex::new(r"/\d+(/|$)"));

    let mut result = path.to_string();
    match uuid_regex {
        Ok(regex) => {
            result = regex.replace_all(&result, "{id}").to_string();
        }
        Err(err) => {
            tracing::error!(error = %err, "Failed to compile UUID regex");
        }
    }

    match id_regex {
        Ok(regex) => {
            result = regex.replace_all(&result, "/{id}$1").to_string();
        }
        Err(err) => {
            tracing::error!(error = %err, "Failed to compile numeric ID regex");
        }
    }

    result
}

/// Observability middleware for Axum.
///
/// This middleware wraps every request with:
/// 1. OpenTelemetry span (with trace context propagation)
/// 2. Prometheus metrics recording
/// 3. Request/response logging
pub async fn observability_middleware(request: axum::http::Request<Body>, next: Next) -> Response {
    let start = Instant::now();

    // Extract request metadata
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = uri.path().to_string();
    let normalized_path = normalize_path(&path);

    // Extract trace context from headers (if present)
    let parent_context = extract_trace_context(request.headers());

    // Create tracing span and link extracted trace context
    let tracing_span = info_span!(
        "http_request",
        http.method = %method,
        http.target = %path,
        http.route = %normalized_path,
        otel.kind = "server",
    );
    tracing_span.set_parent(parent_context);

    // Execute the request within the tracing span
    let span = tracing_span.clone();
    let response = next.run(request).instrument(tracing_span).await;

    // Record metrics and complete span
    let duration = start.elapsed();
    let status = response.status();
    let duration_secs = duration.as_secs_f64();

    // Record Prometheus metrics
    if let Ok(metrics) = METRICS.as_ref() {
        metrics.record_http_request(
            method.as_str(),
            &normalized_path,
            status.as_u16(),
            duration_secs,
        );
    } else {
        tracing::error!("Metrics registry unavailable; skipping HTTP request metrics");
    }

    // Update span with response status
    let cx = span.context();
    cx.span()
        .set_attribute(KeyValue::new("http.method", method.to_string()));
    cx.span()
        .set_attribute(KeyValue::new("http.target", path.clone()));
    cx.span()
        .set_attribute(KeyValue::new("http.route", normalized_path.clone()));
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
