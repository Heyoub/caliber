//! Billing REST API Routes
//!
//! This module implements billing status, checkout, and portal endpoints
//! for LemonSqueezy integration. These routes require authentication.

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    config::EndpointsConfig,
    db::DbClient,
    error::{ApiError, ApiResult},
    middleware::AuthExtractor,
    state::AppState,
};
use caliber_core::{EntityIdType, TenantId};

// ============================================================================
// TYPES
// ============================================================================

/// Billing plan type.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum BillingPlan {
    /// Free trial period
    #[default]
    Trial,
    /// Paid professional plan
    Pro,
    /// Enterprise plan
    Enterprise,
}

/// Billing status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct BillingStatus {
    /// Tenant ID
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub tenant_id: TenantId,
    /// Current billing plan
    pub plan: BillingPlan,
    /// Trial end date (if on trial)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub trial_ends_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Storage used in bytes
    pub storage_used_bytes: i64,
    /// Storage limit in bytes
    pub storage_limit_bytes: i64,
    /// Hot cache used in bytes
    pub hot_cache_used_bytes: i64,
    /// Hot cache limit in bytes
    pub hot_cache_limit_bytes: i64,
}

/// Checkout session request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateCheckoutRequest {
    /// Plan to upgrade to
    pub plan: String,
    /// LemonSqueezy variant ID (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant_id: Option<String>,
    /// URL to redirect on success
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_url: Option<String>,
    /// URL to redirect on cancel
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_url: Option<String>,
}

/// Checkout session response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CheckoutResponse {
    /// LemonSqueezy checkout URL
    pub checkout_url: String,
}

/// Customer portal response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct PortalResponse {
    /// LemonSqueezy customer portal URL
    pub portal_url: String,
}

/// LemonSqueezy webhook event.
#[derive(Debug, Clone, Deserialize)]
pub struct LemonSqueezyWebhook {
    /// Event type
    pub event: String,
    /// Event data
    pub data: LemonSqueezyData,
}

/// LemonSqueezy webhook data.
#[derive(Debug, Clone, Deserialize)]
pub struct LemonSqueezyData {
    /// Subscription or order ID
    pub id: String,
    /// Type of resource
    #[serde(rename = "type")]
    pub resource_type: String,
    /// Attributes
    pub attributes: LemonSqueezyAttributes,
}

/// LemonSqueezy attributes.
#[derive(Debug, Clone, Deserialize)]
pub struct LemonSqueezyAttributes {
    /// Customer email
    #[serde(default)]
    pub user_email: Option<String>,
    /// Status
    #[serde(default)]
    pub status: Option<String>,
    /// Custom data (contains tenant_id)
    #[serde(default)]
    pub custom_data: Option<serde_json::Value>,
}

// ============================================================================
// SHARED STATE
// ============================================================================

/// Shared application state for billing routes.
#[derive(Clone)]
pub struct BillingState {
    pub db: DbClient,
    pub http_client: reqwest::Client,
    /// LemonSqueezy store ID
    pub store_id: Option<String>,
    /// LemonSqueezy API key
    pub api_key: Option<String>,
    /// LemonSqueezy webhook secret
    pub webhook_secret: Option<String>,
    /// Endpoints configuration for URLs
    pub endpoints: EndpointsConfig,
}

impl BillingState {
    pub fn new(db: DbClient, endpoints: EndpointsConfig) -> Self {
        let store_id = std::env::var("LEMONSQUEEZY_STORE_ID").ok();
        let api_key = std::env::var("LEMONSQUEEZY_API_KEY").ok();
        let webhook_secret = std::env::var("LEMONSQUEEZY_WEBHOOK_SECRET").ok();

        Self {
            db,
            http_client: reqwest::Client::new(),
            store_id,
            api_key,
            webhook_secret,
            endpoints,
        }
    }

    /// Get the LemonSqueezy checkouts API URL.
    fn checkouts_url(&self) -> String {
        format!("{}/v1/checkouts", self.endpoints.lemonsqueezy_api_url)
    }

    /// Get the LemonSqueezy customer API URL.
    fn customer_url(&self, customer_id: &str) -> String {
        format!(
            "{}/v1/customers/{}",
            self.endpoints.lemonsqueezy_api_url, customer_id
        )
    }
}

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// GET /api/v1/billing/status - Get billing status for current tenant
#[utoipa::path(
    get,
    path = "/api/v1/billing/status",
    tag = "Billing",
    responses(
        (status = 200, description = "Billing status", body = BillingStatus),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn get_billing_status(
    State(state): State<Arc<BillingState>>,
    AuthExtractor(auth): AuthExtractor,
) -> ApiResult<impl IntoResponse> {
    // Get billing status from database
    // The db function handles "not found" by returning a default trial status.
    // We must NOT mask database errors - they indicate real problems.
    let billing = match state.db.billing_get_status(auth.tenant_id).await {
        Ok(status) => status,
        Err(e) => {
            // Database error - propagate, don't mask as trial
            tracing::error!(
                tenant_id = %auth.tenant_id,
                error = %e,
                "Failed to fetch billing status"
            );
            return Err(ApiError::database_error(
                "Unable to retrieve billing status",
            ));
        }
    };

    Ok(Json(billing))
}

/// POST /api/v1/billing/checkout - Create a checkout session
#[utoipa::path(
    post,
    path = "/api/v1/billing/checkout",
    tag = "Billing",
    request_body = CreateCheckoutRequest,
    responses(
        (status = 200, description = "Checkout session created", body = CheckoutResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 503, description = "Billing service unavailable", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn create_checkout(
    State(state): State<Arc<BillingState>>,
    AuthExtractor(auth): AuthExtractor,
    Json(req): Json<CreateCheckoutRequest>,
) -> ApiResult<impl IntoResponse> {
    // Check if LemonSqueezy is configured
    let store_id = state
        .store_id
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("Billing service not configured"))?;
    let api_key = state
        .api_key
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("Billing service not configured"))?;

    // Get variant ID from request or default based on plan
    let variant_id = req.variant_id.unwrap_or_else(|| {
        // Default variant IDs - in production these would be configured
        match req.plan.as_str() {
            "pro" => std::env::var("LEMONSQUEEZY_PRO_VARIANT_ID")
                .unwrap_or_else(|_| "default_pro_variant".to_string()),
            "enterprise" => std::env::var("LEMONSQUEEZY_ENTERPRISE_VARIANT_ID")
                .unwrap_or_else(|_| "default_enterprise_variant".to_string()),
            _ => "default_pro_variant".to_string(),
        }
    });

    // Build checkout URL with custom data
    let redirect_url = req
        .success_url
        .unwrap_or_else(|| state.endpoints.billing_success_redirect());
    let checkout_data = serde_json::json!({
        "data": {
            "type": "checkouts",
            "attributes": {
                "checkout_data": {
                    "custom": {
                        "tenant_id": auth.tenant_id.to_string(),
                        "user_id": auth.user_id,
                    }
                },
                "product_options": {
                    "redirect_url": redirect_url,
                }
            },
            "relationships": {
                "store": {
                    "data": {
                        "type": "stores",
                        "id": store_id
                    }
                },
                "variant": {
                    "data": {
                        "type": "variants",
                        "id": variant_id
                    }
                }
            }
        }
    });

    // Call LemonSqueezy API to create checkout
    let response = state
        .http_client
        .post(&state.checkouts_url())
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/vnd.api+json")
        .json(&checkout_data)
        .send()
        .await
        .map_err(|e| ApiError::service_unavailable(format!("Failed to create checkout: {}", e)))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        tracing::error!(error = %error_text, "LemonSqueezy checkout creation failed");
        return Err(ApiError::service_unavailable(
            "Failed to create checkout session",
        ));
    }

    let checkout_response: serde_json::Value = response
        .json()
        .await
        .map_err(|e| ApiError::internal_error(format!("Invalid checkout response: {}", e)))?;

    let checkout_url = checkout_response
        .get("data")
        .and_then(|d| d.get("attributes"))
        .and_then(|a| a.get("url"))
        .and_then(|u| u.as_str())
        .ok_or_else(|| ApiError::internal_error("No checkout URL in response"))?
        .to_string();

    tracing::info!(
        tenant_id = %auth.tenant_id,
        plan = %req.plan,
        "Checkout session created"
    );

    Ok(Json(CheckoutResponse { checkout_url }))
}

/// GET /api/v1/billing/portal - Get customer portal URL
#[utoipa::path(
    get,
    path = "/api/v1/billing/portal",
    tag = "Billing",
    responses(
        (status = 200, description = "Portal URL", body = PortalResponse),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 503, description = "Billing service unavailable", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn get_portal_url(
    AuthExtractor(auth): AuthExtractor,
    State(state): State<Arc<BillingState>>,
) -> ApiResult<impl IntoResponse> {
    // Get customer ID for tenant from database
    let customer_id = state
        .db
        .billing_get_customer_id(auth.tenant_id)
        .await
        .map_err(|_| ApiError::not_found("No subscription found for this tenant"))?;

    let api_key = state
        .api_key
        .as_ref()
        .ok_or_else(|| ApiError::service_unavailable("Billing service not configured"))?;

    // Get customer portal URL from LemonSqueezy
    let response = state
        .http_client
        .get(&state.customer_url(&customer_id))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| ApiError::service_unavailable(format!("Failed to get portal URL: {}", e)))?;

    if !response.status().is_success() {
        return Err(ApiError::service_unavailable(
            "Failed to get customer portal",
        ));
    }

    let customer_data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| ApiError::internal_error(format!("Invalid customer response: {}", e)))?;

    let portal_url = customer_data
        .get("data")
        .and_then(|d| d.get("attributes"))
        .and_then(|a| a.get("urls"))
        .and_then(|u| u.get("customer_portal"))
        .and_then(|p| p.as_str())
        .ok_or_else(|| ApiError::internal_error("No portal URL in response"))?
        .to_string();

    Ok(Json(PortalResponse { portal_url }))
}

/// POST /api/v1/billing/webhooks/lemonsqueezy - Handle LemonSqueezy webhooks
pub async fn handle_lemonsqueezy_webhook(
    State(state): State<Arc<BillingState>>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<StatusCode> {
    // Verify webhook signature
    if let Some(secret) = &state.webhook_secret {
        let signature = headers
            .get("X-Signature")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| ApiError::unauthorized("Missing webhook signature"))?;

        if !verify_webhook_signature(&body, signature, secret) {
            return Err(ApiError::unauthorized("Invalid webhook signature"));
        }
    }

    // Parse webhook payload
    let webhook: LemonSqueezyWebhook = serde_json::from_slice(&body)
        .map_err(|e| ApiError::invalid_input(format!("Invalid webhook payload: {}", e)))?;

    tracing::info!(
        event = %webhook.event,
        resource_type = %webhook.data.resource_type,
        "Received LemonSqueezy webhook"
    );

    // Handle different event types
    match webhook.event.as_str() {
        "subscription_created" | "subscription_updated" => {
            handle_subscription_event(&state, &webhook).await?;
        }
        "subscription_cancelled" | "subscription_expired" => {
            handle_subscription_cancelled(&state, &webhook).await?;
        }
        "order_created" => {
            // One-time payment - could be used for credits or addons
            tracing::info!("Order created event received");
        }
        _ => {
            tracing::debug!(event = %webhook.event, "Ignoring webhook event");
        }
    }

    Ok(StatusCode::OK)
}

// ============================================================================
// HELPERS
// ============================================================================

/// Verify LemonSqueezy webhook signature.
fn verify_webhook_signature(payload: &[u8], signature: &str, secret: &str) -> bool {
    type HmacSha256 = Hmac<Sha256>;

    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        return false;
    };

    mac.update(payload);
    let expected = hex::encode(mac.finalize().into_bytes());

    // Compare in constant time
    expected == signature
}

/// Handle subscription created/updated event.
async fn handle_subscription_event(
    state: &BillingState,
    webhook: &LemonSqueezyWebhook,
) -> ApiResult<()> {
    // Extract tenant ID from custom data
    let tenant_uuid = webhook
        .data
        .attributes
        .custom_data
        .as_ref()
        .and_then(|d| d.get("tenant_id"))
        .and_then(|t| t.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| ApiError::invalid_input("Missing tenant_id in webhook custom data"))?;
    let tenant_id = TenantId::new(tenant_uuid);

    // Determine plan from subscription status/variant
    let plan = match webhook.data.attributes.status.as_deref() {
        Some("active") | Some("on_trial") => BillingPlan::Pro,
        _ => BillingPlan::Trial,
    };

    // Update tenant billing status
    state.db.billing_update_plan(tenant_id, plan).await?;

    // Store LemonSqueezy customer ID for future portal access
    state
        .db
        .billing_set_customer_id(tenant_id, &webhook.data.id)
        .await?;

    tracing::info!(
        tenant_id = %tenant_id,
        subscription_id = %webhook.data.id,
        "Subscription updated"
    );

    Ok(())
}

/// Handle subscription cancelled/expired event.
async fn handle_subscription_cancelled(
    state: &BillingState,
    webhook: &LemonSqueezyWebhook,
) -> ApiResult<()> {
    // Extract tenant ID from custom data
    let tenant_uuid = webhook
        .data
        .attributes
        .custom_data
        .as_ref()
        .and_then(|d| d.get("tenant_id"))
        .and_then(|t| t.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| ApiError::invalid_input("Missing tenant_id in webhook custom data"))?;
    let tenant_id = TenantId::new(tenant_uuid);

    // Downgrade to trial (or free tier)
    state
        .db
        .billing_update_plan(tenant_id, BillingPlan::Trial)
        .await?;

    tracing::info!(
        tenant_id = %tenant_id,
        subscription_id = %webhook.data.id,
        "Subscription cancelled/expired"
    );

    Ok(())
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the billing routes router.
pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/status", get(get_billing_status))
        .route("/checkout", post(create_checkout))
        .route("/portal", get(get_portal_url))
        .route("/webhooks/lemonsqueezy", post(handle_lemonsqueezy_webhook))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_billing_plan_serialization() -> Result<(), serde_json::Error> {
        let trial = BillingPlan::Trial;
        let json = serde_json::to_string(&trial)?;
        assert_eq!(json, "\"trial\"");

        let pro = BillingPlan::Pro;
        let json = serde_json::to_string(&pro)?;
        assert_eq!(json, "\"pro\"");
        Ok(())
    }

    #[test]
    fn test_billing_status_serialization() -> Result<(), serde_json::Error> {
        let status = BillingStatus {
            tenant_id: TenantId::new(Uuid::new_v4()),
            plan: BillingPlan::Trial,
            trial_ends_at: Some(chrono::Utc::now()),
            storage_used_bytes: 1024,
            storage_limit_bytes: 1024 * 1024,
            hot_cache_used_bytes: 512,
            hot_cache_limit_bytes: 1024 * 100,
        };

        let json = serde_json::to_string(&status)?;
        assert!(json.contains("\"plan\":\"trial\""));
        assert!(json.contains("storage_used_bytes"));
        Ok(())
    }

    #[test]
    fn test_checkout_request_deserialization() -> Result<(), serde_json::Error> {
        let json = r#"{
            "plan": "pro",
            "success_url": "https://example.com/success"
        }"#;

        let req: CreateCheckoutRequest = serde_json::from_str(json)?;
        assert_eq!(req.plan, "pro");
        assert_eq!(
            req.success_url,
            Some("https://example.com/success".to_string())
        );
        assert!(req.variant_id.is_none());
        Ok(())
    }

    #[test]
    fn test_webhook_signature_verification() -> Result<(), Box<dyn std::error::Error>> {
        let payload = b"test payload";
        let secret = "supersecret123";

        // Generate expected signature
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())?;
        mac.update(payload);
        let expected = hex::encode(mac.finalize().into_bytes());

        assert!(verify_webhook_signature(payload, &expected, secret));
        assert!(!verify_webhook_signature(payload, "invalid", secret));
        Ok(())
    }
}
