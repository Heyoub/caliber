//! Webhook REST API Routes
//!
//! This module implements webhook registration, management, and delivery.
//! Webhooks allow external systems to receive real-time notifications
//! when CALIBER events occur.
//!
//! Features:
//! - Register webhooks with event filtering
//! - HMAC-SHA256 signed payloads
//! - Exponential backoff retry on delivery failure
//! - Per-webhook event type filtering

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{
    collections::HashMap,
    sync::Arc,
    time::Duration,
};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    db::DbClient,
    error::{ApiError, ApiResult},
    events::WsEvent,
    ws::WsState,
};

// ============================================================================
// TYPES
// ============================================================================

/// Supported webhook event types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventType {
    /// All events (wildcard)
    #[serde(rename = "*")]
    All,
    /// Trajectory created
    TrajectoryCreated,
    /// Trajectory updated
    TrajectoryUpdated,
    /// Trajectory deleted
    TrajectoryDeleted,
    /// Scope created
    ScopeCreated,
    /// Scope updated
    ScopeUpdated,
    /// Scope closed
    ScopeClosed,
    /// Artifact created
    ArtifactCreated,
    /// Artifact updated
    ArtifactUpdated,
    /// Artifact deleted
    ArtifactDeleted,
    /// Note created
    NoteCreated,
    /// Note updated
    NoteUpdated,
    /// Note deleted
    NoteDeleted,
    /// Turn created
    TurnCreated,
    /// Agent registered
    AgentRegistered,
    /// Agent status changed
    AgentStatusChanged,
    /// Agent heartbeat
    AgentHeartbeat,
    /// Agent unregistered
    AgentUnregistered,
    /// Lock acquired
    LockAcquired,
    /// Lock released
    LockReleased,
    /// Lock expired
    LockExpired,
    /// Message sent
    MessageSent,
    /// Message delivered
    MessageDelivered,
    /// Message acknowledged
    MessageAcknowledged,
    /// Delegation created
    DelegationCreated,
    /// Delegation accepted
    DelegationAccepted,
    /// Delegation rejected
    DelegationRejected,
    /// Delegation completed
    DelegationCompleted,
    /// Handoff created
    HandoffCreated,
    /// Handoff accepted
    HandoffAccepted,
    /// Handoff completed
    HandoffCompleted,
}

impl WebhookEventType {
    /// Check if this event type matches a WsEvent.
    pub fn matches(&self, event: &WsEvent) -> bool {
        match self {
            WebhookEventType::All => true,
            WebhookEventType::TrajectoryCreated => matches!(event, WsEvent::TrajectoryCreated { .. }),
            WebhookEventType::TrajectoryUpdated => matches!(event, WsEvent::TrajectoryUpdated { .. }),
            WebhookEventType::TrajectoryDeleted => matches!(event, WsEvent::TrajectoryDeleted { .. }),
            WebhookEventType::ScopeCreated => matches!(event, WsEvent::ScopeCreated { .. }),
            WebhookEventType::ScopeUpdated => matches!(event, WsEvent::ScopeUpdated { .. }),
            WebhookEventType::ScopeClosed => matches!(event, WsEvent::ScopeClosed { .. }),
            WebhookEventType::ArtifactCreated => matches!(event, WsEvent::ArtifactCreated { .. }),
            WebhookEventType::ArtifactUpdated => matches!(event, WsEvent::ArtifactUpdated { .. }),
            WebhookEventType::ArtifactDeleted => matches!(event, WsEvent::ArtifactDeleted { .. }),
            WebhookEventType::NoteCreated => matches!(event, WsEvent::NoteCreated { .. }),
            WebhookEventType::NoteUpdated => matches!(event, WsEvent::NoteUpdated { .. }),
            WebhookEventType::NoteDeleted => matches!(event, WsEvent::NoteDeleted { .. }),
            WebhookEventType::TurnCreated => matches!(event, WsEvent::TurnCreated { .. }),
            WebhookEventType::AgentRegistered => matches!(event, WsEvent::AgentRegistered { .. }),
            WebhookEventType::AgentStatusChanged => matches!(event, WsEvent::AgentStatusChanged { .. }),
            WebhookEventType::AgentHeartbeat => matches!(event, WsEvent::AgentHeartbeat { .. }),
            WebhookEventType::AgentUnregistered => matches!(event, WsEvent::AgentUnregistered { .. }),
            WebhookEventType::LockAcquired => matches!(event, WsEvent::LockAcquired { .. }),
            WebhookEventType::LockReleased => matches!(event, WsEvent::LockReleased { .. }),
            WebhookEventType::LockExpired => matches!(event, WsEvent::LockExpired { .. }),
            WebhookEventType::MessageSent => matches!(event, WsEvent::MessageSent { .. }),
            WebhookEventType::MessageDelivered => matches!(event, WsEvent::MessageDelivered { .. }),
            WebhookEventType::MessageAcknowledged => matches!(event, WsEvent::MessageAcknowledged { .. }),
            WebhookEventType::DelegationCreated => matches!(event, WsEvent::DelegationCreated { .. }),
            WebhookEventType::DelegationAccepted => matches!(event, WsEvent::DelegationAccepted { .. }),
            WebhookEventType::DelegationRejected => matches!(event, WsEvent::DelegationRejected { .. }),
            WebhookEventType::DelegationCompleted => matches!(event, WsEvent::DelegationCompleted { .. }),
            WebhookEventType::HandoffCreated => matches!(event, WsEvent::HandoffCreated { .. }),
            WebhookEventType::HandoffAccepted => matches!(event, WsEvent::HandoffAccepted { .. }),
            WebhookEventType::HandoffCompleted => matches!(event, WsEvent::HandoffCompleted { .. }),
        }
    }
}

/// A registered webhook.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Webhook {
    /// Unique webhook ID
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "uuid"))]
    pub id: Uuid,
    /// Target URL for webhook delivery
    pub url: String,
    /// Event types this webhook subscribes to
    pub events: Vec<WebhookEventType>,
    /// Optional description
    pub description: Option<String>,
    /// Whether the webhook is active
    pub active: bool,
    /// Secret for HMAC signature (not exposed in responses)
    #[serde(skip_serializing)]
    pub secret: String,
    /// Creation timestamp
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = "date-time"))]
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Number of successful deliveries
    pub success_count: u64,
    /// Number of failed deliveries
    pub failure_count: u64,
    /// Last delivery attempt timestamp
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>, format = "date-time"))]
    pub last_delivery_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Request to register a new webhook.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateWebhookRequest {
    /// Target URL for webhook delivery (must be HTTPS in production)
    pub url: String,
    /// Event types to subscribe to (use ["*"] for all events)
    pub events: Vec<WebhookEventType>,
    /// Optional description
    pub description: Option<String>,
    /// Secret for HMAC signature generation
    pub secret: String,
}

/// Response containing webhook details.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct WebhookResponse {
    /// The webhook
    pub webhook: Webhook,
}

/// Response containing a list of webhooks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ListWebhooksResponse {
    /// List of webhooks
    pub webhooks: Vec<Webhook>,
    /// Total count
    pub total: i32,
}

/// Webhook delivery payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    /// Unique delivery ID
    pub delivery_id: Uuid,
    /// Event type
    pub event_type: String,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Event data
    pub data: serde_json::Value,
}

// ============================================================================
// SHARED STATE
// ============================================================================

/// In-memory webhook store. In production, this would be persisted to the database.
pub struct WebhookStore {
    webhooks: RwLock<HashMap<Uuid, Webhook>>,
}

impl WebhookStore {
    pub fn new() -> Self {
        Self {
            webhooks: RwLock::new(HashMap::new()),
        }
    }

    pub async fn insert(&self, webhook: Webhook) {
        let mut webhooks = self.webhooks.write().await;
        webhooks.insert(webhook.id, webhook);
    }

    pub async fn get(&self, id: Uuid) -> Option<Webhook> {
        let webhooks = self.webhooks.read().await;
        webhooks.get(&id).cloned()
    }

    pub async fn list(&self) -> Vec<Webhook> {
        let webhooks = self.webhooks.read().await;
        webhooks.values().cloned().collect()
    }

    pub async fn remove(&self, id: Uuid) -> Option<Webhook> {
        let mut webhooks = self.webhooks.write().await;
        webhooks.remove(&id)
    }

    pub async fn update_stats(&self, id: Uuid, success: bool) {
        let mut webhooks = self.webhooks.write().await;
        if let Some(webhook) = webhooks.get_mut(&id) {
            webhook.last_delivery_at = Some(chrono::Utc::now());
            if success {
                webhook.success_count += 1;
            } else {
                webhook.failure_count += 1;
            }
        }
    }
}

impl Default for WebhookStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared application state for webhook routes.
#[derive(Clone)]
pub struct WebhookState {
    pub db: DbClient,
    pub ws: Arc<WsState>,
    pub store: Arc<WebhookStore>,
    pub http_client: reqwest::Client,
}

impl WebhookState {
    pub fn new(db: DbClient, ws: Arc<WsState>) -> Result<Self, String> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            db,
            ws,
            store: Arc::new(WebhookStore::new()),
            http_client,
        })
    }
}

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// POST /api/v1/webhooks - Register a new webhook
#[utoipa::path(
    post,
    path = "/api/v1/webhooks",
    tag = "Webhooks",
    request_body = CreateWebhookRequest,
    responses(
        (status = 201, description = "Webhook registered successfully", body = WebhookResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn create_webhook(
    State(state): State<Arc<WebhookState>>,
    Json(req): Json<CreateWebhookRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate URL
    if req.url.trim().is_empty() {
        return Err(ApiError::missing_field("url"));
    }

    // Parse URL to validate format
    let url = reqwest::Url::parse(&req.url)
        .map_err(|_| ApiError::invalid_input("Invalid URL format"))?;

    // Validate events
    if req.events.is_empty() {
        return Err(ApiError::missing_field("events"));
    }

    // Validate secret
    if req.secret.len() < 16 {
        return Err(ApiError::invalid_input("Secret must be at least 16 characters"));
    }

    let webhook = Webhook {
        id: Uuid::new_v4(),
        url: url.to_string(),
        events: req.events,
        description: req.description,
        active: true,
        secret: req.secret,
        created_at: chrono::Utc::now(),
        success_count: 0,
        failure_count: 0,
        last_delivery_at: None,
    };

    state.store.insert(webhook.clone()).await;

    tracing::info!(webhook_id = %webhook.id, url = %webhook.url, "Webhook registered");

    Ok((
        StatusCode::CREATED,
        Json(WebhookResponse { webhook }),
    ))
}

/// GET /api/v1/webhooks - List all webhooks
#[utoipa::path(
    get,
    path = "/api/v1/webhooks",
    tag = "Webhooks",
    responses(
        (status = 200, description = "List of webhooks", body = ListWebhooksResponse),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn list_webhooks(
    State(state): State<Arc<WebhookState>>,
) -> impl IntoResponse {
    let webhooks = state.store.list().await;
    let total = webhooks.len() as i32;

    Json(ListWebhooksResponse { webhooks, total })
}

/// GET /api/v1/webhooks/{id} - Get a specific webhook
#[utoipa::path(
    get,
    path = "/api/v1/webhooks/{id}",
    tag = "Webhooks",
    params(
        ("id" = Uuid, Path, description = "Webhook ID")
    ),
    responses(
        (status = 200, description = "Webhook details", body = WebhookResponse),
        (status = 404, description = "Webhook not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn get_webhook(
    State(state): State<Arc<WebhookState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let webhook = state
        .store
        .get(id)
        .await
        .ok_or_else(|| ApiError::entity_not_found("Webhook", id))?;

    Ok(Json(WebhookResponse { webhook }))
}

/// DELETE /api/v1/webhooks/{id} - Remove a webhook
#[utoipa::path(
    delete,
    path = "/api/v1/webhooks/{id}",
    tag = "Webhooks",
    params(
        ("id" = Uuid, Path, description = "Webhook ID")
    ),
    responses(
        (status = 204, description = "Webhook removed successfully"),
        (status = 404, description = "Webhook not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn delete_webhook(
    State(state): State<Arc<WebhookState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let removed = state.store.remove(id).await;

    if removed.is_none() {
        return Err(ApiError::entity_not_found("Webhook", id));
    }

    tracing::info!(webhook_id = %id, "Webhook removed");

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// WEBHOOK DELIVERY
// ============================================================================

/// Generate HMAC-SHA256 signature for webhook payload.
///
/// This function should never fail as HMAC-SHA256 accepts keys of any size.
/// However, we handle the error case defensively.
fn sign_payload(payload: &[u8], secret: &str) -> Result<String, String> {
    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| format!("Failed to initialize HMAC: {}", e))?;
    mac.update(payload);
    let result = mac.finalize();

    Ok(hex::encode(result.into_bytes()))
}

/// Deliver a webhook with retry logic.
pub async fn deliver_webhook(
    client: &reqwest::Client,
    webhook: &Webhook,
    event: &WsEvent,
    store: &WebhookStore,
) {
    let delivery_id = Uuid::new_v4();

    // Serialize event data, with logging if serialization fails
    let event_data = serde_json::to_value(event).unwrap_or_else(|e| {
        tracing::warn!(
            webhook_id = %webhook.id,
            delivery_id = %delivery_id,
            error = %e,
            "Failed to serialize event data, using empty object"
        );
        serde_json::json!({})
    });

    let payload = WebhookPayload {
        delivery_id,
        event_type: event.event_type().to_string(),
        timestamp: chrono::Utc::now(),
        data: event_data,
    };

    let payload_bytes = match serde_json::to_vec(&payload) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::error!(error = %e, "Failed to serialize webhook payload");
            return;
        }
    };

    let signature = match sign_payload(&payload_bytes, &webhook.secret) {
        Ok(sig) => sig,
        Err(e) => {
            tracing::error!(error = %e, webhook_id = %webhook.id, "Failed to sign webhook payload");
            return;
        }
    };

    // Retry with exponential backoff: 1s, 2s, 4s (3 attempts)
    let mut delay = Duration::from_secs(1);
    let max_attempts = 3;

    for attempt in 1..=max_attempts {
        let result = client
            .post(&webhook.url)
            .header("Content-Type", "application/json")
            .header("X-Webhook-Signature", format!("sha256={}", signature))
            .header("X-Webhook-Delivery-ID", delivery_id.to_string())
            .header("X-Webhook-Event", event.event_type())
            .header("User-Agent", "CALIBER-Webhook/1.0")
            .body(payload_bytes.clone())
            .send()
            .await;

        match result {
            Ok(response) => {
                if response.status().is_success() {
                    store.update_stats(webhook.id, true).await;
                    tracing::debug!(
                        webhook_id = %webhook.id,
                        delivery_id = %delivery_id,
                        status = %response.status(),
                        "Webhook delivered successfully"
                    );
                    return;
                } else {
                    tracing::warn!(
                        webhook_id = %webhook.id,
                        delivery_id = %delivery_id,
                        status = %response.status(),
                        attempt = attempt,
                        "Webhook delivery failed with non-2xx status"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    webhook_id = %webhook.id,
                    delivery_id = %delivery_id,
                    error = %e,
                    attempt = attempt,
                    "Webhook delivery failed"
                );
            }
        }

        if attempt < max_attempts {
            tokio::time::sleep(delay).await;
            delay *= 2; // Exponential backoff
        }
    }

    // All attempts failed
    store.update_stats(webhook.id, false).await;
    tracing::error!(
        webhook_id = %webhook.id,
        delivery_id = %delivery_id,
        "Webhook delivery failed after {} attempts", max_attempts
    );

    // Record metric
    #[cfg(feature = "openapi")]
    {
        use crate::telemetry::METRICS;
        METRICS.record_webhook_delivery(false);
    }
}

/// Start the webhook delivery background task.
/// This subscribes to WsState events and delivers matching webhooks.
pub fn start_webhook_delivery_task(state: Arc<WebhookState>) {
    tokio::spawn(async move {
        let mut rx = state.ws.subscribe();

        loop {
            match rx.recv().await {
                Ok(event) => {
                    // Get all active webhooks that match this event
                    let webhooks = state.store.list().await;

                    for webhook in webhooks {
                        if !webhook.active {
                            continue;
                        }

                        // Check if any event filter matches
                        let matches = webhook.events.iter().any(|e| e.matches(&event));

                        if matches {
                            let client = state.http_client.clone();
                            let webhook = webhook.clone();
                            let event = event.clone();
                            let store = state.store.clone();

                            // Deliver webhook in background
                            tokio::spawn(async move {
                                deliver_webhook(&client, &webhook, &event, &store).await;
                            });
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(lagged = n, "Webhook delivery lagged behind");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    tracing::info!("Webhook delivery channel closed, stopping task");
                    break;
                }
            }
        }
    });
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the webhook routes router and start the delivery task.
///
/// # Panics
///
/// Panics if the HTTP client cannot be created. This should only happen
/// if the system's TLS configuration is invalid, which is a fatal error
/// that should be caught at startup.
pub fn create_router(db: DbClient, ws: Arc<WsState>) -> Router {
    let state = Arc::new(
        WebhookState::new(db, ws)
            .unwrap_or_else(|e| panic!("Failed to initialize webhook state: {}", e))
    );

    // Start the webhook delivery background task
    start_webhook_delivery_task(state.clone());

    Router::new()
        .route("/", post(create_webhook))
        .route("/", get(list_webhooks))
        .route("/:id", get(get_webhook))
        .route("/:id", delete(delete_webhook))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_event_type_matching() {
        // Test wildcard matches all
        assert!(WebhookEventType::All.matches(&WsEvent::TrajectoryCreated {
            trajectory: crate::types::TrajectoryResponse {
                trajectory_id: Uuid::new_v4(),
                name: "test".to_string(),
                description: None,
                status: caliber_core::TrajectoryStatus::Active,
                parent_trajectory_id: None,
                root_trajectory_id: None,
                agent_id: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                completed_at: None,
                outcome: None,
                metadata: None,
            },
        }));

        // Test specific event type
        assert!(WebhookEventType::TrajectoryCreated.matches(&WsEvent::TrajectoryCreated {
            trajectory: crate::types::TrajectoryResponse {
                trajectory_id: Uuid::new_v4(),
                name: "test".to_string(),
                description: None,
                status: caliber_core::TrajectoryStatus::Active,
                parent_trajectory_id: None,
                root_trajectory_id: None,
                agent_id: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                completed_at: None,
                outcome: None,
                metadata: None,
            },
        }));

        // Test non-matching event type
        assert!(!WebhookEventType::NoteCreated.matches(&WsEvent::TrajectoryCreated {
            trajectory: crate::types::TrajectoryResponse {
                trajectory_id: Uuid::new_v4(),
                name: "test".to_string(),
                description: None,
                status: caliber_core::TrajectoryStatus::Active,
                parent_trajectory_id: None,
                root_trajectory_id: None,
                agent_id: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                completed_at: None,
                outcome: None,
                metadata: None,
            },
        }));
    }

    #[test]
    fn test_sign_payload() {
        let payload = b"test payload";
        let secret = "supersecretkey123";

        let signature = sign_payload(payload, secret).expect("Failed to sign payload");

        // Signature should be a hex string
        assert!(!signature.is_empty());
        assert!(signature.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_webhook_serialization() {
        let webhook = Webhook {
            id: Uuid::new_v4(),
            url: "https://example.com/webhook".to_string(),
            events: vec![WebhookEventType::All],
            description: Some("Test webhook".to_string()),
            active: true,
            secret: "supersecretkey123".to_string(),
            created_at: chrono::Utc::now(),
            success_count: 0,
            failure_count: 0,
            last_delivery_at: None,
        };

        let json = serde_json::to_string(&webhook).expect("Failed to serialize");

        // Secret should not be in the JSON (skip_serializing)
        assert!(!json.contains("supersecret"));

        // ID and URL should be present
        assert!(json.contains("id"));
        assert!(json.contains("https://example.com/webhook"));
    }

    #[test]
    fn test_webhook_event_type_serialization() {
        let all = WebhookEventType::All;
        let json = serde_json::to_string(&all).expect("Failed to serialize");
        assert_eq!(json, "\"*\"");

        let trajectory = WebhookEventType::TrajectoryCreated;
        let json = serde_json::to_string(&trajectory).expect("Failed to serialize");
        assert_eq!(json, "\"trajectory_created\"");
    }
}
