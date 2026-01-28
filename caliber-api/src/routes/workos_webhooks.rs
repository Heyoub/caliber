//! WorkOS webhook endpoints.
//!
//! Validates WorkOS signatures and logs event metadata for auditability.

use axum::{
    body::Bytes,
    http::HeaderMap,
    response::IntoResponse,
    routing::post,
    Router,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

type HmacSha256 = Hmac<Sha256>;

const SIGNATURE_HEADER: &str = "workos-signature";
const DEFAULT_TOLERANCE_SECS: i64 = 300;

#[cfg(feature = "workos")]
pub fn create_router() -> Router<AppState> {
    Router::new().route("/webhooks", post(handle_workos_webhook))
}

#[cfg(feature = "workos")]
async fn handle_workos_webhook(headers: HeaderMap, body: Bytes) -> ApiResult<impl IntoResponse> {
    let secret = std::env::var("CALIBER_WORKOS_WEBHOOK_SECRET").ok().ok_or_else(|| {
        ApiError::unauthorized("WorkOS webhook secret is not configured")
    })?;

    let signature = headers
        .get(SIGNATURE_HEADER)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ApiError::unauthorized("Missing WorkOS signature"))?;

    let tolerance_secs = std::env::var("CALIBER_WORKOS_WEBHOOK_TOLERANCE_SECS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(DEFAULT_TOLERANCE_SECS);

    verify_signature(&body, signature, &secret, tolerance_secs)?;

    let event: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|e| ApiError::invalid_input(format!("Invalid webhook payload: {}", e)))?;

    let event_id = event.get("id").and_then(|v| v.as_str());
    let event_type = event.get("type").and_then(|v| v.as_str());

    tracing::info!(
        workos_event_id = event_id.unwrap_or("unknown"),
        workos_event_type = event_type.unwrap_or("unknown"),
        "Received WorkOS webhook"
    );

    Ok(axum::http::StatusCode::OK)
}

fn verify_signature(
    body: &Bytes,
    signature_header: &str,
    secret: &str,
    tolerance_secs: i64,
) -> ApiResult<()> {
    let (timestamp_ms, signature_hex) = parse_signature_header(signature_header)?;

    let now_ms = chrono::Utc::now().timestamp_millis();
    let age_secs = (now_ms - timestamp_ms).abs() / 1000;
    if age_secs > tolerance_secs {
        return Err(ApiError::unauthorized("Stale WorkOS signature"));
    }

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| ApiError::internal_error("Failed to initialize HMAC"))?;
    mac.update(timestamp_ms.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);

    let expected = hex::decode(signature_hex)
        .map_err(|_| ApiError::unauthorized("Invalid WorkOS signature encoding"))?;

    mac.verify_slice(&expected)
        .map_err(|_| ApiError::unauthorized("Invalid WorkOS signature"))?;

    Ok(())
}

fn parse_signature_header(header: &str) -> ApiResult<(i64, &str)> {
    let mut timestamp: Option<i64> = None;
    let mut signature: Option<&str> = None;

    for part in header.split(',') {
        let part = part.trim();
        if let Some(value) = part.strip_prefix("t=") {
            timestamp = value.parse::<i64>().ok();
        } else if let Some(value) = part.strip_prefix("v1=") {
            signature = Some(value);
        }
    }

    let timestamp = timestamp.ok_or_else(|| ApiError::unauthorized("Missing WorkOS timestamp"))?;
    let signature = signature.ok_or_else(|| ApiError::unauthorized("Missing WorkOS signature"))?;

    Ok((timestamp, signature))
}
