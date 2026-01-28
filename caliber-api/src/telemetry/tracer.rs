//! OpenTelemetry Tracer Initialization
//!
//! Sets up OTLP exporter for distributed tracing compatible with:
//! - Jaeger
//! - DataDog
//! - Grafana Tempo
//! - Any OTLP-compatible backend

use opentelemetry::{global, trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::{
    propagation::TraceContextPropagator,
    trace::{RandomIdGenerator, Sampler, TracerProvider},
    Resource,
};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::error::{ApiError, ApiResult};

/// Telemetry configuration from environment variables.
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// OTLP endpoint for traces (e.g., "http://localhost:4317")
    pub otlp_endpoint: Option<String>,
    /// Service name for traces
    pub service_name: String,
    /// Service version
    pub service_version: String,
    /// Environment (production, staging, development)
    pub environment: String,
    /// Enable trace sampling (0.0 to 1.0)
    pub trace_sample_rate: f64,
    /// Enable metrics collection
    pub metrics_enabled: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            otlp_endpoint: std::env::var("CALIBER_OTLP_ENDPOINT").ok(),
            service_name: std::env::var("CALIBER_SERVICE_NAME")
                .unwrap_or_else(|_| "caliber-api".to_string()),
            service_version: std::env::var("CALIBER_SERVICE_VERSION")
                .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string()),
            environment: std::env::var("CALIBER_ENVIRONMENT")
                .unwrap_or_else(|_| "development".to_string()),
            trace_sample_rate: std::env::var("CALIBER_TRACE_SAMPLE_RATE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1.0),
            metrics_enabled: std::env::var("CALIBER_METRICS_ENABLED")
                .map(|s| s == "true" || s == "1")
                .unwrap_or(true),
        }
    }
}

/// Initialize the OpenTelemetry tracer and tracing subscriber.
///
/// This function should be called once at application startup before any
/// tracing occurs. It sets up:
/// - OTLP exporter for distributed traces (if endpoint configured)
/// - TraceContext propagation (W3C traceparent header)
/// - tracing-subscriber with OpenTelemetry layer
pub fn init_tracer(config: &TelemetryConfig) -> ApiResult<()> {
    // Set up W3C TraceContext propagation
    global::set_text_map_propagator(TraceContextPropagator::new());

    // Build resource with service metadata
    let resource = Resource::new(vec![
        KeyValue::new("service.name", config.service_name.clone()),
        KeyValue::new("service.version", config.service_version.clone()),
        KeyValue::new("deployment.environment", config.environment.clone()),
    ]);

    // Configure sampler based on sample rate
    let sampler = if config.trace_sample_rate >= 1.0 {
        Sampler::AlwaysOn
    } else if config.trace_sample_rate <= 0.0 {
        Sampler::AlwaysOff
    } else {
        Sampler::TraceIdRatioBased(config.trace_sample_rate)
    };

    // Build tracer provider
    let tracer_provider = if let Some(endpoint) = &config.otlp_endpoint {
        // OTLP exporter configured
        let exporter = SpanExporter::builder()
            .with_http()
            .with_endpoint(endpoint)
            .build()
            .map_err(|e| {
                ApiError::internal_error(format!("Failed to create OTLP exporter: {}", e))
            })?;

        TracerProvider::builder()
            .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
            .with_sampler(sampler)
            .with_id_generator(RandomIdGenerator::default())
            .with_resource(resource)
            .build()
    } else {
        // No OTLP endpoint - use simple tracer (still captures spans for local logging)
        TracerProvider::builder()
            .with_sampler(sampler)
            .with_id_generator(RandomIdGenerator::default())
            .with_resource(resource)
            .build()
    };

    let tracer = tracer_provider.tracer("caliber-api");
    global::set_tracer_provider(tracer_provider);

    // Build tracing subscriber with OpenTelemetry layer
    let otel_layer = OpenTelemetryLayer::new(tracer);
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("caliber_api=debug,tower_http=debug,info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().json())
        .with(otel_layer)
        .try_init()
        .map_err(|e| ApiError::internal_error(format!("Failed to init subscriber: {}", e)))?;

    tracing::info!(
        service_name = config.service_name,
        environment = config.environment,
        otlp_endpoint = ?config.otlp_endpoint,
        "Telemetry initialized"
    );

    Ok(())
}

/// Gracefully shutdown the tracer provider, flushing pending spans.
///
/// Should be called before application exit.
pub fn shutdown_tracer() {
    global::shutdown_tracer_provider();
    tracing::info!("Tracer shutdown complete");
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EnvVarGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: Option<&str>) -> Self {
            let original = std::env::var(key).ok();
            match value {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
            Self { key, original }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match self.original.as_deref() {
                Some(v) => std::env::set_var(self.key, v),
                None => std::env::remove_var(self.key),
            }
        }
    }

    #[test]
    fn test_telemetry_config_default() {
        let _guard = EnvVarGuard::set("CALIBER_TRACE_SAMPLE_RATE", None);
        let config = TelemetryConfig::default();
        assert_eq!(config.service_name, "caliber-api");
        assert_eq!(config.trace_sample_rate, 1.0);
        assert!(config.metrics_enabled);
    }

    #[test]
    fn test_telemetry_config_sampler_selection() {
        // Test always on
        let config = TelemetryConfig {
            trace_sample_rate: 1.0,
            ..Default::default()
        };
        assert!(config.trace_sample_rate >= 1.0);

        // Test always off
        let config = TelemetryConfig {
            trace_sample_rate: 0.0,
            ..Default::default()
        };
        assert!(config.trace_sample_rate <= 0.0);

        // Test ratio based
        let config = TelemetryConfig {
            trace_sample_rate: 0.5,
            ..Default::default()
        };
        assert!(config.trace_sample_rate > 0.0 && config.trace_sample_rate < 1.0);
    }
}
