//! Message REST API Routes
//!
//! This module implements Axum route handlers for inter-agent message operations.
//! All handlers call caliber_* pg_extern functions via the DbClient.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    auth::validate_tenant_ownership,
    db::{DbClient, MessageListParams},
    error::{ApiError, ApiResult},
    events::WsEvent,
    middleware::AuthExtractor,
    state::AppState,
    types::{ListMessagesRequest, ListMessagesResponse, MessageResponse, SendMessageRequest},
    ws::WsState,
};
// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// POST /api/v1/messages - Send a message
#[utoipa::path(
    post,
    path = "/api/v1/messages",
    tag = "Messages",
    request_body = SendMessageRequest,
    responses(
        (status = 201, description = "Message sent successfully", body = MessageResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn send_message(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    Json(req): Json<SendMessageRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate that either to_agent_id or to_agent_type is specified
    if req.to_agent_id.is_none() && req.to_agent_type.is_none() {
        return Err(ApiError::invalid_input(
            "Either to_agent_id or to_agent_type must be specified",
        ));
    }

    // Validate message type
    let valid_types = [
        "TaskDelegation",
        "TaskResult",
        "ContextRequest",
        "ContextShare",
        "CoordinationSignal",
        "Handoff",
        "Interrupt",
        "Heartbeat",
    ];
    if !valid_types.contains(&req.message_type.as_str()) {
        return Err(ApiError::invalid_input(format!(
            "message_type must be one of: {}",
            valid_types.join(", ")
        )));
    }

    // Validate priority
    let valid_priorities = ["Low", "Normal", "High", "Critical"];
    if !valid_priorities.contains(&req.priority.as_str()) {
        return Err(ApiError::invalid_input(format!(
            "priority must be one of: {}",
            valid_priorities.join(", ")
        )));
    }

    // Validate payload is valid JSON
    if serde_json::from_str::<serde_json::Value>(&req.payload).is_err() {
        return Err(ApiError::invalid_input(
            "payload must be valid JSON string",
        ));
    }

    // Send message via database client with tenant_id for isolation
    let message = db.message_send(&req, auth.tenant_id).await?;

    // Broadcast MessageSent event
    ws.broadcast(WsEvent::MessageSent {
        message: message.clone(),
    });

    Ok((StatusCode::CREATED, Json(message)))
}

/// GET /api/v1/messages - List messages with filters
#[utoipa::path(
    get,
    path = "/api/v1/messages",
    tag = "Messages",
    params(
        ("message_type" = Option<String>, Query, description = "Filter by message type"),
        ("from_agent_id" = Option<String>, Query, description = "Filter by sender agent"),
        ("to_agent_id" = Option<String>, Query, description = "Filter by recipient agent"),
        ("to_agent_type" = Option<String>, Query, description = "Filter by recipient agent type"),
        ("trajectory_id" = Option<String>, Query, description = "Filter by trajectory"),
        ("priority" = Option<String>, Query, description = "Filter by priority"),
        ("undelivered_only" = Option<bool>, Query, description = "Only return undelivered messages"),
        ("unacknowledged_only" = Option<bool>, Query, description = "Only return unacknowledged messages"),
        ("limit" = Option<i32>, Query, description = "Maximum number of results"),
        ("offset" = Option<i32>, Query, description = "Offset for pagination"),
    ),
    responses(
        (status = 200, description = "List of messages", body = ListMessagesResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn list_messages(
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    Query(params): Query<ListMessagesRequest>,
) -> ApiResult<impl IntoResponse> {
    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);

    // List messages filtered by tenant for isolation
    let messages = db.message_list_by_tenant(MessageListParams {
        from_agent_id: params.from_agent_id,
        to_agent_id: params.to_agent_id,
        to_agent_type: params.to_agent_type.as_deref(),
        trajectory_id: params.trajectory_id,
        message_type: params.message_type.as_deref(),
        priority: params.priority.as_deref(),
        undelivered_only: params.undelivered_only.unwrap_or(false),
        unacknowledged_only: params.unacknowledged_only.unwrap_or(false),
        limit,
        offset,
    }, auth.tenant_id).await?;

    let total = messages.len() as i32;

    Ok(Json(ListMessagesResponse {
        messages,
        total,
    }))
}

/// GET /api/v1/messages/{id} - Get message by ID
#[utoipa::path(
    get,
    path = "/api/v1/messages/{id}",
    tag = "Messages",
    params(
        ("id" = Uuid, Path, description = "Message ID")
    ),
    responses(
        (status = 200, description = "Message details", body = MessageResponse),
        (status = 404, description = "Message not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn get_message(
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let message = db
        .message_get(id)
        .await?
        .ok_or_else(|| ApiError::message_not_found(id))?;

    // Validate tenant ownership before returning
    validate_tenant_ownership(&auth, Some(message.tenant_id))?;

    Ok(Json(message))
}

/// POST /api/v1/messages/{id}/acknowledge - Acknowledge a message
#[utoipa::path(
    post,
    path = "/api/v1/messages/{id}/acknowledge",
    tag = "Messages",
    params(
        ("id" = Uuid, Path, description = "Message ID")
    ),
    responses(
        (status = 204, description = "Message acknowledged successfully"),
        (status = 404, description = "Message not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn acknowledge_message(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    // Verify the message exists and belongs to this tenant
    let message = db
        .message_get(id)
        .await?
        .ok_or_else(|| ApiError::message_not_found(id))?;
    validate_tenant_ownership(&auth, Some(message.tenant_id))?;

    // Acknowledge message via database client
    db.message_acknowledge(id).await?;

    // Broadcast MessageAcknowledged event with tenant_id for filtering
    ws.broadcast(WsEvent::MessageAcknowledged {
        tenant_id: auth.tenant_id,
        message_id: id,
    });

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/messages/{id}/deliver - Mark a message as delivered
#[utoipa::path(
    post,
    path = "/api/v1/messages/{id}/deliver",
    tag = "Messages",
    params(
        ("id" = Uuid, Path, description = "Message ID")
    ),
    responses(
        (status = 204, description = "Message delivered successfully"),
        (status = 404, description = "Message not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn deliver_message(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    // Verify the message exists and belongs to this tenant
    let message = db
        .message_get(id)
        .await?
        .ok_or_else(|| ApiError::message_not_found(id))?;
    validate_tenant_ownership(&auth, Some(message.tenant_id))?;

    db.message_deliver(id).await?;

    ws.broadcast(WsEvent::MessageDelivered {
        tenant_id: auth.tenant_id,
        message_id: id,
    });

    Ok(StatusCode::NO_CONTENT)
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the message routes router.
pub fn create_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::post(send_message))
        .route("/", axum::routing::get(list_messages))
        .route("/:id", axum::routing::get(get_message))
        .route("/:id/acknowledge", axum::routing::post(acknowledge_message))
        .route("/:id/deliver", axum::routing::post(deliver_message))
}

#[cfg(test)]
mod tests {
    use super::*;
    use caliber_core::EntityId;

    #[test]
    fn test_send_message_request_validation() {
        let req = SendMessageRequest {
            from_agent_id: EntityId::from(Uuid::new_v4()),
            to_agent_id: None,
            to_agent_type: None,
            message_type: "TaskDelegation".to_string(),
            payload: "{}".to_string(),
            trajectory_id: None,
            scope_id: None,
            artifact_ids: vec![],
            priority: "Normal".to_string(),
            expires_at: None,
        };

        assert!(req.to_agent_id.is_none() && req.to_agent_type.is_none());
    }

    #[test]
    fn test_valid_message_types() {
        let valid_types = [
            "TaskDelegation",
            "TaskResult",
            "ContextRequest",
            "ContextShare",
            "CoordinationSignal",
            "Handoff",
            "Interrupt",
            "Heartbeat",
        ];

        assert!(valid_types.contains(&"TaskDelegation"));
        assert!(valid_types.contains(&"Heartbeat"));
        assert!(!valid_types.contains(&"InvalidType"));
    }

    #[test]
    fn test_valid_priorities() {
        let valid_priorities = ["Low", "Normal", "High", "Critical"];

        assert!(valid_priorities.contains(&"Low"));
        assert!(valid_priorities.contains(&"Critical"));
        assert!(!valid_priorities.contains(&"Invalid"));
    }

    #[test]
    fn test_payload_json_validation() {
        let valid_payload = r#"{"key": "value"}"#;
        let invalid_payload = "not json";

        assert!(serde_json::from_str::<serde_json::Value>(valid_payload).is_ok());
        assert!(serde_json::from_str::<serde_json::Value>(invalid_payload).is_err());
    }

    #[test]
    fn test_list_messages_request_filters() {
        let req = ListMessagesRequest {
            message_type: Some("TaskDelegation".to_string()),
            from_agent_id: Some(EntityId::from(Uuid::new_v4())),
            to_agent_id: None,
            to_agent_type: Some("coder".to_string()),
            trajectory_id: None,
            priority: Some("High".to_string()),
            undelivered_only: Some(true),
            unacknowledged_only: Some(false),
            limit: Some(50),
            offset: Some(0),
        };

        assert_eq!(req.message_type, Some("TaskDelegation".to_string()));
        assert_eq!(req.priority, Some("High".to_string()));
        assert_eq!(req.undelivered_only, Some(true));
    }
}
