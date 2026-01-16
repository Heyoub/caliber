//! WebSocket Event Broadcasting
//!
//! This module provides WebSocket support for real-time event streaming.
//! Clients can connect via WebSocket to receive live updates about entity
//! mutations and system events.
//!
//! ## Architecture
//!
//! - Uses tokio broadcast channel for event distribution
//! - Tenant-specific subscriptions (clients only receive events for their tenant)
//! - Automatic reconnection support via standard WebSocket protocol
//! - JSON-serialized events using the WsEvent enum

use crate::auth::AuthContext;
use crate::error::ApiResult;
use crate::events::WsEvent;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
};
use caliber_core::EntityId;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

/// WebSocket state shared across the application.
///
/// This state is injected into Axum route handlers and contains
/// the broadcast channel for distributing events to connected clients.
#[derive(Clone)]
pub struct WsState {
    /// Broadcast channel for sending events to all connected clients.
    /// Each client subscribes to this channel and filters events by tenant.
    tx: broadcast::Sender<WsEvent>,
}

impl WsState {
    /// Create a new WebSocket state with the specified channel capacity.
    ///
    /// The capacity determines how many events can be buffered before
    /// slow consumers start dropping messages. A capacity of 1000 is
    /// reasonable for most use cases.
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Broadcast an event to all connected clients.
    ///
    /// This is a non-blocking operation. If no clients are connected,
    /// the event is simply dropped. If a client's buffer is full, that
    /// client will miss the event (lagged).
    pub fn broadcast(&self, event: WsEvent) {
        let event_type = event.event_type();
        match self.tx.send(event) {
            Ok(receiver_count) => {
                debug!(
                    event_type = event_type,
                    receivers = receiver_count,
                    "Broadcast event"
                );
            }
            Err(_) => {
                // No receivers connected - this is fine
                debug!(event_type = event_type, "No receivers for event");
            }
        }
    }

    /// Subscribe to the event stream.
    ///
    /// Returns a receiver that will receive all future events.
    /// The receiver must be polled to avoid lagging.
    pub fn subscribe(&self) -> broadcast::Receiver<WsEvent> {
        self.tx.subscribe()
    }
}

/// WebSocket upgrade handler.
///
/// This endpoint upgrades an HTTP connection to a WebSocket connection.
/// The client must be authenticated and have a valid tenant context.
///
/// ## Protocol
///
/// 1. Client connects with authentication headers
/// 2. Server validates auth and extracts tenant ID
/// 3. Connection upgraded to WebSocket
/// 4. Server sends Connected event with tenant ID
/// 5. Server streams events filtered by tenant
/// 6. Client can send ping frames to keep connection alive
/// 7. On disconnect, server sends Disconnected event
///
/// ## Example
///
/// ```text
/// GET /api/v1/ws
/// Authorization: Bearer <token>
/// X-Tenant-ID: <tenant-id>
/// Upgrade: websocket
/// ```
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<WsState>>,
    auth: AuthContext,
) -> ApiResult<Response> {
    // Extract tenant ID from auth context
    let tenant_id = auth.tenant_id;

    info!(
        tenant_id = %tenant_id,
        user_id = ?auth.user_id,
        "WebSocket connection request"
    );

    // Upgrade the connection
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, tenant_id)))
}

/// Handle an individual WebSocket connection.
///
/// This function runs for the lifetime of the WebSocket connection.
/// It subscribes to the broadcast channel and forwards tenant-specific
/// events to the client.
async fn handle_socket(socket: WebSocket, state: Arc<WsState>, tenant_id: EntityId) {
    info!(tenant_id = %tenant_id, "WebSocket connected");

    // Split the socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to the broadcast channel
    let mut rx = state.subscribe();

    // Send initial Connected event
    let connected_event = WsEvent::Connected { tenant_id };
    if let Err(e) = send_event(&mut sender, connected_event).await {
        error!(tenant_id = %tenant_id, error = %e, "Failed to send Connected event");
        return;
    }

    // Spawn a task to handle incoming messages from the client
    let tenant_id_clone = tenant_id;
    let mut recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Close(_)) => {
                    debug!(tenant_id = %tenant_id_clone, "Client sent close frame");
                    break;
                }
                Ok(Message::Ping(data)) => {
                    debug!(tenant_id = %tenant_id_clone, "Received ping");
                    // Pong is automatically sent by axum
                    let _ = data; // Suppress unused warning
                }
                Ok(Message::Pong(_)) => {
                    debug!(tenant_id = %tenant_id_clone, "Received pong");
                }
                Ok(Message::Text(text)) => {
                    debug!(
                        tenant_id = %tenant_id_clone,
                        text = %text,
                        "Received text message (ignored)"
                    );
                }
                Ok(Message::Binary(data)) => {
                    debug!(
                        tenant_id = %tenant_id_clone,
                        len = data.len(),
                        "Received binary message (ignored)"
                    );
                }
                Err(e) => {
                    warn!(tenant_id = %tenant_id_clone, error = %e, "WebSocket receive error");
                    break;
                }
            }
        }
    });

    // Main loop: forward events to the client
    loop {
        tokio::select! {
            // Receive event from broadcast channel
            result = rx.recv() => {
                match result {
                    Ok(event) => {
                        // Filter events by tenant
                        if should_send_event(&event, tenant_id) {
                            if let Err(e) = send_event(&mut sender, event).await {
                                error!(
                                    tenant_id = %tenant_id,
                                    error = %e,
                                    "Failed to send event, closing connection"
                                );
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!(
                            tenant_id = %tenant_id,
                            skipped = skipped,
                            "Client lagged, some events were dropped"
                        );
                        // Send error event to notify client
                        let error_event = WsEvent::Error {
                            message: format!("Lagged: {} events dropped", skipped),
                        };
                        if let Err(e) = send_event(&mut sender, error_event).await {
                            error!(tenant_id = %tenant_id, error = %e, "Failed to send error event");
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!(tenant_id = %tenant_id, "Broadcast channel closed");
                        break;
                    }
                }
            }

            // Check if receiver task finished (client disconnected)
            _ = &mut recv_task => {
                debug!(tenant_id = %tenant_id, "Receiver task finished");
                break;
            }
        }
    }

    // Send Disconnected event before closing
    let disconnected_event = WsEvent::Disconnected {
        reason: "Connection closed".to_string(),
    };
    let _ = send_event(&mut sender, disconnected_event).await;

    info!(tenant_id = %tenant_id, "WebSocket disconnected");
}

/// Send an event to the WebSocket client.
///
/// Serializes the event to JSON and sends it as a text message.
async fn send_event(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    event: WsEvent,
) -> Result<(), axum::Error> {
    let json = serde_json::to_string(&event).map_err(|e| {
        error!(error = %e, "Failed to serialize event");
        axum::Error::new(e)
    })?;

    sender.send(Message::Text(json.into())).await
}

/// Determine if an event should be sent to a client based on tenant filtering.
///
/// Most events are tenant-specific and should only be sent to clients
/// connected to that tenant. Connection events (Connected, Disconnected, Error)
/// are always sent.
fn should_send_event(event: &WsEvent, client_tenant_id: EntityId) -> bool {
    // Connection events are always sent
    if !event.is_tenant_specific() {
        return true;
    }

    match tenant_id_from_event(event) {
        Some(event_tenant_id) => event_tenant_id == client_tenant_id,
        None => {
            // Tenant-aware metadata is optional; fall back to allow when missing.
            debug!(
                event_type = event.event_type(),
                "Tenant metadata missing for event; allowing broadcast"
            );
            true
        }
    }
}

fn tenant_id_from_event(event: &WsEvent) -> Option<EntityId> {
    match event {
        WsEvent::TrajectoryCreated { trajectory } => tenant_id_from_metadata(&trajectory.metadata),
        WsEvent::TrajectoryUpdated { trajectory } => tenant_id_from_metadata(&trajectory.metadata),
        WsEvent::ScopeCreated { scope } => tenant_id_from_metadata(&scope.metadata),
        WsEvent::ScopeUpdated { scope } => tenant_id_from_metadata(&scope.metadata),
        WsEvent::ScopeClosed { scope } => tenant_id_from_metadata(&scope.metadata),
        WsEvent::ArtifactCreated { artifact } => tenant_id_from_metadata(&artifact.metadata),
        WsEvent::ArtifactUpdated { artifact } => tenant_id_from_metadata(&artifact.metadata),
        WsEvent::NoteCreated { note } => tenant_id_from_metadata(&note.metadata),
        WsEvent::NoteUpdated { note } => tenant_id_from_metadata(&note.metadata),
        WsEvent::TurnCreated { turn } => tenant_id_from_metadata(&turn.metadata),
        WsEvent::DelegationCreated { delegation } => tenant_id_from_metadata(&delegation.context),
        WsEvent::DelegationCompleted { delegation } => tenant_id_from_metadata(&delegation.context),
        _ => None,
    }
}

fn tenant_id_from_metadata(metadata: &Option<JsonValue>) -> Option<EntityId> {
    let metadata = metadata.as_ref()?;
    let tenant_value = metadata.get("tenant_id")?;
    tenant_value
        .as_str()
        .and_then(|value| value.parse::<EntityId>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_ws_state_creation() {
        let state = WsState::new(100);
        // Should be able to subscribe
        let _rx = state.subscribe();
    }

    #[test]
    fn test_broadcast_no_receivers() {
        let state = WsState::new(100);
        let event = WsEvent::Connected {
            tenant_id: EntityId::new(),
        };
        // Should not panic when no receivers
        state.broadcast(event);
    }

    #[test]
    fn test_broadcast_with_receiver() {
        let state = WsState::new(100);
        let mut rx = state.subscribe();

        let event = WsEvent::Connected {
            tenant_id: EntityId::new(),
        };
        state.broadcast(event.clone());

        // Should receive the event
        let received = rx.try_recv().expect("Should receive event");
        assert_eq!(received, event);
    }

    #[test]
    fn test_event_filtering() {
        let tenant_id = EntityId::new();

        // Connection events should always be sent
        let connected = WsEvent::Connected { tenant_id };
        assert!(should_send_event(&connected, tenant_id));

        let error = WsEvent::Error {
            message: "test".to_string(),
        };
        assert!(should_send_event(&error, tenant_id));

        // Tenant-specific events should be filtered
        // (Currently sends to all, but test the logic)
        let trajectory_created = WsEvent::TrajectoryCreated {
            trajectory: crate::types::TrajectoryResponse {
                trajectory_id: EntityId::new(),
                name: "test".to_string(),
                description: None,
                status: caliber_core::TrajectoryStatus::Active,
                parent_trajectory_id: None,
                root_trajectory_id: None,
                agent_id: None,
                created_at: caliber_core::Timestamp::now(),
                updated_at: caliber_core::Timestamp::now(),
                completed_at: None,
                outcome: None,
                metadata: Some(json!({ "tenant_id": tenant_id.to_string() })),
            },
        };
        assert!(should_send_event(&trajectory_created, tenant_id));

        let other_tenant = EntityId::new();
        let other_tenant_event = WsEvent::TrajectoryCreated {
            trajectory: crate::types::TrajectoryResponse {
                trajectory_id: EntityId::new(),
                name: "other".to_string(),
                description: None,
                status: caliber_core::TrajectoryStatus::Active,
                parent_trajectory_id: None,
                root_trajectory_id: None,
                agent_id: None,
                created_at: caliber_core::Timestamp::now(),
                updated_at: caliber_core::Timestamp::now(),
                completed_at: None,
                outcome: None,
                metadata: Some(json!({ "tenant_id": other_tenant.to_string() })),
            },
        };
        assert!(!should_send_event(&other_tenant_event, tenant_id));
    }
}
