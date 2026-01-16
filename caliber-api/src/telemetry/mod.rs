//! CALIBER Telemetry - Observability Infrastructure
//!
//! Provides OpenTelemetry tracing and Prometheus metrics for the API layer.
//! All features work standalone without external dependencies.

pub mod metrics;
pub mod middleware;
pub mod tracer;

pub use metrics::{metrics_handler, CaliberMetrics, METRICS};
pub use middleware::observability_middleware;
pub use tracer::{init_tracer, shutdown_tracer, TelemetryConfig};
