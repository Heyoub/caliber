//! gRPC Service Implementation
//!
//! This module implements all gRPC services defined in proto/caliber.proto.
//! The implementation reuses the REST handler logic from the routes modules,
//! ensuring REST-gRPC parity.
//!
//! All services call caliber_* pg_extern functions via the DbClient, maintaining
//! the hot-path optimization (no SQL parsing overhead).

use std::pin::Pin;
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use tokio_stream::{wrappers::BroadcastStream, Stream, StreamExt};
use tonic::{Request, Response, Status};

use crate::{
    db::DbClient,
    error::ApiError,
    events::WsEvent,
    ws::WsState,
};
use uuid::Uuid;

// ============================================================================
// TENANT EXTRACTION HELPER
// ============================================================================

/// Extract tenant_id from gRPC request metadata or extensions.
///
/// This bridges the architectural gap between REST handlers (which use AuthContext)
/// and gRPC handlers (which receive Request<T>).
///
/// Priority:
/// 1. Try to get from metadata first (x-tenant-id header)
/// 2. Fallback: try extensions (if set by interceptor)
fn extract_tenant_id<T>(request: &Request<T>) -> Result<caliber_core::EntityId, Status> {
    // Try to get from metadata first
    if let Some(tenant) = request.metadata().get("x-tenant-id") {
        let tenant_str = tenant.to_str()
            .map_err(|_| Status::invalid_argument("Invalid tenant ID header"))?;
        let tenant_id = tenant_str.parse::<Uuid>()
            .map_err(|_| Status::invalid_argument("Invalid tenant ID format"))?;
        return Ok(caliber_core::EntityId::from(tenant_id));
    }

    // Fallback: try extensions (if set by interceptor)
    if let Some(tenant_id) = request.extensions().get::<caliber_core::EntityId>() {
        return Ok(*tenant_id);
    }

    Err(Status::unauthenticated("Missing tenant ID"))
}

fn parse_filter_operator(value: &str) -> Result<caliber_core::FilterOperator, Status> {
    match value.trim().to_ascii_lowercase().as_str() {
        "eq" => Ok(caliber_core::FilterOperator::Eq),
        "ne" => Ok(caliber_core::FilterOperator::Ne),
        "gt" => Ok(caliber_core::FilterOperator::Gt),
        "lt" => Ok(caliber_core::FilterOperator::Lt),
        "gte" => Ok(caliber_core::FilterOperator::Gte),
        "lte" => Ok(caliber_core::FilterOperator::Lte),
        "contains" => Ok(caliber_core::FilterOperator::Contains),
        "in" => Ok(caliber_core::FilterOperator::In),
        "regex" => Ok(caliber_core::FilterOperator::Regex),
        "and" => Ok(caliber_core::FilterOperator::And),
        "or" => Ok(caliber_core::FilterOperator::Or),
        "not" => Ok(caliber_core::FilterOperator::Not),
        _ => Err(Status::invalid_argument("Invalid filter operator")),
    }
}

// Include the generated protobuf code
pub mod proto {
    tonic::include_proto!("caliber");
}

// Use proto types - crate::types are referenced explicitly throughout
use proto::*;

// ============================================================================
// CONVERSION HELPERS
// ============================================================================

/// Convert ApiError to tonic Status
impl From<ApiError> for Status {
    fn from(err: ApiError) -> Self {
        match err.code {
            crate::error::ErrorCode::Unauthorized => Status::unauthenticated(err.message),
            crate::error::ErrorCode::Forbidden => Status::permission_denied(err.message),
            crate::error::ErrorCode::InvalidToken => Status::unauthenticated(err.message),
            crate::error::ErrorCode::TokenExpired => Status::unauthenticated(err.message),
            crate::error::ErrorCode::ValidationFailed => Status::invalid_argument(err.message),
            crate::error::ErrorCode::InvalidInput => Status::invalid_argument(err.message),
            crate::error::ErrorCode::MissingField => Status::invalid_argument(err.message),
            crate::error::ErrorCode::InvalidRange => Status::invalid_argument(err.message),
            crate::error::ErrorCode::InvalidFormat => Status::invalid_argument(err.message),
            crate::error::ErrorCode::EntityNotFound => Status::not_found(err.message),
            crate::error::ErrorCode::TenantNotFound => Status::not_found(err.message),
            crate::error::ErrorCode::TrajectoryNotFound => Status::not_found(err.message),
            crate::error::ErrorCode::ScopeNotFound => Status::not_found(err.message),
            crate::error::ErrorCode::ArtifactNotFound => Status::not_found(err.message),
            crate::error::ErrorCode::NoteNotFound => Status::not_found(err.message),
            crate::error::ErrorCode::AgentNotFound => Status::not_found(err.message),
            crate::error::ErrorCode::LockNotFound => Status::not_found(err.message),
            crate::error::ErrorCode::MessageNotFound => Status::not_found(err.message),
            crate::error::ErrorCode::EntityAlreadyExists => Status::already_exists(err.message),
            crate::error::ErrorCode::ConcurrentModification => Status::aborted(err.message),
            crate::error::ErrorCode::LockConflict => Status::resource_exhausted(err.message),
            crate::error::ErrorCode::LockExpired => Status::failed_precondition(err.message),
            crate::error::ErrorCode::StateConflict => Status::failed_precondition(err.message),
            crate::error::ErrorCode::ConnectionPoolExhausted => Status::resource_exhausted(err.message),
            crate::error::ErrorCode::Timeout => Status::deadline_exceeded(err.message),
            crate::error::ErrorCode::TooManyRequests => Status::resource_exhausted(err.message),
            crate::error::ErrorCode::InternalError => Status::internal(err.message),
            crate::error::ErrorCode::DatabaseError => Status::internal(err.message),
            crate::error::ErrorCode::ServiceUnavailable => Status::unavailable(err.message),
        }
    }
}

fn timestamp_to_millis(ts: &caliber_core::Timestamp) -> i64 {
    ts.timestamp_millis()
}

fn opt_timestamp_to_millis(ts: Option<caliber_core::Timestamp>) -> Option<i64> {
    ts.map(|t| t.timestamp_millis())
}

fn parse_timestamp_millis(value: i64, field: &'static str) -> Result<caliber_core::Timestamp, Status> {
    Utc.timestamp_millis_opt(value)
        .single()
        .ok_or_else(|| Status::invalid_argument(format!("Invalid {}", field)))
}

fn parse_optional_timestamp_millis(
    value: Option<i64>,
    field: &'static str,
) -> Result<Option<caliber_core::Timestamp>, Status> {
    value.map(|v| parse_timestamp_millis(v, field)).transpose()
}

// ============================================================================
// TRAJECTORY SERVICE IMPLEMENTATION
// ============================================================================

pub struct TrajectoryServiceImpl {
    db: DbClient,
    ws: Arc<WsState>,
}

impl TrajectoryServiceImpl {
    pub fn new(db: DbClient, ws: Arc<WsState>) -> Self {
        Self { db, ws }
    }
}

#[tonic::async_trait]
impl trajectory_service_server::TrajectoryService for TrajectoryServiceImpl {
    async fn create_trajectory(
        &self,
        request: Request<CreateTrajectoryRequest>,
    ) -> Result<Response<TrajectoryResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();

        // Convert proto request to internal type
        let create_req = crate::types::CreateTrajectoryRequest {
            name: req.name,
            description: req.description,
            parent_trajectory_id: req.parent_trajectory_id.and_then(|id| id.parse().ok()),
            agent_id: req.agent_id.and_then(|id| id.parse().ok()),
            metadata: req.metadata.and_then(|s| serde_json::from_str(&s).ok()),
        };

        // Reuse REST handler logic
        let trajectory = self.db.trajectory_create(&create_req, tenant_id).await?;
        
        // Broadcast event
        self.ws.broadcast(WsEvent::TrajectoryCreated {
            trajectory: trajectory.clone(),
        });
        
        // Convert to proto response
        let response = trajectory_to_proto(&trajectory);
        Ok(Response::new(response))
    }

    async fn get_trajectory(
        &self,
        request: Request<GetTrajectoryRequest>,
    ) -> Result<Response<TrajectoryResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        let id = req.trajectory_id.parse().map_err(|_| Status::invalid_argument("Invalid trajectory_id"))?;
        
        let trajectory = self.db.trajectory_get(id, tenant_id).await?
            .ok_or_else(|| Status::not_found("Trajectory not found"))?;
        
        let response = trajectory_to_proto(&trajectory);
        Ok(Response::new(response))
    }

    async fn list_trajectories(
        &self,
        request: Request<ListTrajectoriesRequest>,
    ) -> Result<Response<ListTrajectoriesResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        
        // Convert proto request to internal type
        let list_req = crate::types::ListTrajectoriesRequest {
            status: req.status.and_then(|s| s.parse().ok()),
            agent_id: req.agent_id.and_then(|id| id.parse().ok()),
            parent_id: req.parent_id.and_then(|id| id.parse().ok()),
            limit: req.limit,
            offset: req.offset,
        };
        
        // For now, require status filter
        let status = list_req.status.ok_or_else(|| Status::invalid_argument("status filter required"))?;
        let mut trajectories = self.db.trajectory_list_by_status(status, tenant_id).await?;
        
        // Apply filters
        if let Some(agent_id) = list_req.agent_id {
            trajectories.retain(|t| t.agent_id == Some(agent_id));
        }
        if let Some(parent_id) = list_req.parent_id {
            trajectories.retain(|t| t.parent_trajectory_id == Some(parent_id));
        }
        
        // Apply pagination
        let total = trajectories.len() as i32;
        let offset = list_req.offset.unwrap_or(0) as usize;
        let limit = list_req.limit.unwrap_or(100) as usize;
        
        let paginated: Vec<_> = trajectories.into_iter().skip(offset).take(limit).collect();
        
        let response = ListTrajectoriesResponse {
            trajectories: paginated.into_iter().map(|t| trajectory_to_proto(&t)).collect(),
            total,
        };
        
        Ok(Response::new(response))
    }

    async fn update_trajectory(
        &self,
        request: Request<UpdateTrajectoryRequest>,
    ) -> Result<Response<TrajectoryResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        let id = req.trajectory_id.parse().map_err(|_| Status::invalid_argument("Invalid trajectory_id"))?;
        
        let update_req = crate::types::UpdateTrajectoryRequest {
            name: req.name,
            description: req.description,
            status: req.status.and_then(|s| s.parse().ok()),
            metadata: req.metadata.and_then(|s| serde_json::from_str(&s).ok()),
        };
        
        let trajectory = self.db.trajectory_update(id, &update_req, tenant_id).await?;
        
        self.ws.broadcast(WsEvent::TrajectoryUpdated {
            trajectory: trajectory.clone(),
        });
        
        let response = trajectory_to_proto(&trajectory);
        Ok(Response::new(response))
    }

    async fn delete_trajectory(
        &self,
        request: Request<DeleteTrajectoryRequest>,
    ) -> Result<Response<Empty>, Status> {
        let req = request.into_inner();
        let id: caliber_core::EntityId = req.trajectory_id.parse().map_err(|_| Status::invalid_argument("Invalid trajectory_id"))?;

        self.db.trajectory_delete(id).await?;
        Ok(Response::new(Empty {}))
    }

    async fn list_trajectory_scopes(
        &self,
        request: Request<ListTrajectoryScopesRequest>,
    ) -> Result<Response<ListScopesResponse>, Status> {
        let req = request.into_inner();
        let id: caliber_core::EntityId = req.trajectory_id.parse().map_err(|_| Status::invalid_argument("Invalid trajectory_id"))?;

        let scopes = self.db.scope_list_by_trajectory(id).await?;
        let response = ListScopesResponse {
            scopes: scopes.into_iter().map(|s| scope_to_proto(&s)).collect()
        };
        Ok(Response::new(response))
    }

    async fn list_trajectory_children(
        &self,
        request: Request<ListTrajectoryChildrenRequest>,
    ) -> Result<Response<ListTrajectoriesResponse>, Status> {
        let req = request.into_inner();
        let id: caliber_core::EntityId = req.trajectory_id.parse().map_err(|_| Status::invalid_argument("Invalid trajectory_id"))?;

        let children = self.db.trajectory_list_children(id).await?;
        let response = ListTrajectoriesResponse {
            trajectories: children.iter().map(trajectory_to_proto).collect(),
            total: children.len() as i32,
        };
        Ok(Response::new(response))
    }
}

// ============================================================================
// SCOPE SERVICE IMPLEMENTATION
// ============================================================================

pub struct ScopeServiceImpl {
    db: DbClient,
    ws: Arc<WsState>,
}

impl ScopeServiceImpl {
    pub fn new(db: DbClient, ws: Arc<WsState>) -> Self {
        Self { db, ws }
    }
}

#[tonic::async_trait]
impl scope_service_server::ScopeService for ScopeServiceImpl {
    async fn create_scope(
        &self,
        request: Request<CreateScopeRequest>,
    ) -> Result<Response<ScopeResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();

        let create_req = crate::types::CreateScopeRequest {
            trajectory_id: req.trajectory_id.parse().map_err(|_| Status::invalid_argument("Invalid trajectory_id"))?,
            parent_scope_id: req.parent_scope_id.and_then(|id| id.parse().ok()),
            name: req.name,
            purpose: req.purpose,
            token_budget: req.token_budget,
            metadata: req.metadata.and_then(|s| serde_json::from_str(&s).ok()),
        };

        let scope = self.db.scope_create(&create_req, tenant_id).await?;
        
        self.ws.broadcast(WsEvent::ScopeCreated {
            scope: scope.clone(),
        });
        
        let response = scope_to_proto(&scope);
        Ok(Response::new(response))
    }

    async fn get_scope(
        &self,
        request: Request<GetScopeRequest>,
    ) -> Result<Response<ScopeResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        let id = req.scope_id.parse().map_err(|_| Status::invalid_argument("Invalid scope_id"))?;
        
        let scope = self.db.scope_get(id, tenant_id).await?
            .ok_or_else(|| Status::not_found("Scope not found"))?;
        
        let response = scope_to_proto(&scope);
        Ok(Response::new(response))
    }

    async fn update_scope(
        &self,
        request: Request<UpdateScopeRequest>,
    ) -> Result<Response<ScopeResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        let id = req.scope_id.parse().map_err(|_| Status::invalid_argument("Invalid scope_id"))?;
        
        let update_req = crate::types::UpdateScopeRequest {
            name: req.name,
            purpose: req.purpose,
            token_budget: req.token_budget,
            metadata: req.metadata.and_then(|s| serde_json::from_str(&s).ok()),
        };
        
        let scope = self.db.scope_update(id, &update_req, tenant_id).await?;
        
        self.ws.broadcast(WsEvent::ScopeUpdated {
            scope: scope.clone(),
        });
        
        let response = scope_to_proto(&scope);
        Ok(Response::new(response))
    }

    async fn create_checkpoint(
        &self,
        request: Request<CreateCheckpointRequest>,
    ) -> Result<Response<CheckpointResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        let id = req.scope_id.parse().map_err(|_| Status::invalid_argument("Invalid scope_id"))?;
        
        let checkpoint_req = crate::types::CreateCheckpointRequest {
            context_state: req.context_state,
            recoverable: req.recoverable,
        };
        
        let checkpoint = self.db.scope_create_checkpoint(id, &checkpoint_req, tenant_id).await?;
        
        let response = CheckpointResponse {
            context_state: checkpoint.context_state,
            recoverable: checkpoint.recoverable,
        };
        
        Ok(Response::new(response))
    }

    async fn close_scope(
        &self,
        request: Request<CloseScopeRequest>,
    ) -> Result<Response<ScopeResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        let id = req.scope_id.parse().map_err(|_| Status::invalid_argument("Invalid scope_id"))?;
        
        let scope = self.db.scope_close(id, tenant_id).await?;
        
        self.ws.broadcast(WsEvent::ScopeClosed {
            scope: scope.clone(),
        });
        
        let response = scope_to_proto(&scope);
        Ok(Response::new(response))
    }

    async fn list_scope_turns(
        &self,
        request: Request<ListScopeTurnsRequest>,
    ) -> Result<Response<ListTurnsResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        let id = req.scope_id.parse().map_err(|_| Status::invalid_argument("Invalid scope_id"))?;
        
        let turns = self.db.turn_list_by_scope(id, tenant_id).await?;
        
        let response = ListTurnsResponse {
            turns: turns.into_iter().map(|t| turn_to_proto(&t)).collect(),
        };
        
        Ok(Response::new(response))
    }

    async fn list_scope_artifacts(
        &self,
        request: Request<ListScopeArtifactsRequest>,
    ) -> Result<Response<ListArtifactsResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        let id = req.scope_id.parse().map_err(|_| Status::invalid_argument("Invalid scope_id"))?;
        
        let artifacts = self.db.artifact_list_by_scope(id, tenant_id).await?;
        let total = artifacts.len() as i32;
        
        let response = ListArtifactsResponse {
            artifacts: artifacts.into_iter().map(|a| artifact_to_proto(&a)).collect(),
            total,
        };
        
        Ok(Response::new(response))
    }
}

// ============================================================================
// EVENT SERVICE IMPLEMENTATION (Streaming)
// ============================================================================

pub struct EventServiceImpl {
    ws: Arc<WsState>,
}

impl EventServiceImpl {
    pub fn new(ws: Arc<WsState>) -> Self {
        Self { ws }
    }
}

#[tonic::async_trait]
impl event_service_server::EventService for EventServiceImpl {
    type SubscribeEventsStream = Pin<Box<dyn Stream<Item = Result<Event, Status>> + Send>>;

    async fn subscribe_events(
        &self,
        request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeEventsStream>, Status> {
        let req = request.into_inner();
        let event_types = req.event_types;
        
        // Subscribe to broadcast channel
        let rx = self.ws.subscribe();
        
        // Convert broadcast stream to gRPC stream
        let stream = BroadcastStream::new(rx).filter_map(move |result| {
            let event_types = event_types.clone();
            match result {
                Ok(ws_event) => {
                    // Convert WsEvent to proto Event
                    let event_type = ws_event_type(&ws_event);
                    
                    // Filter by event types if specified
                    if !event_types.is_empty() && !event_types.contains(&event_type) {
                        return None;
                    }
                    
                    let payload = serde_json::to_string(&ws_event).ok()?;
                    let timestamp = chrono::Utc::now().timestamp_millis();
                    
                    Some(Ok(Event {
                        event_type,
                        timestamp,
                        payload,
                    }))
                }
                Err(_) => None, // Lagged, skip
            }
        });
        
        Ok(Response::new(Box::pin(stream)))
    }
}

/// Extract event type string from WsEvent
fn ws_event_type(event: &WsEvent) -> String {
    event.event_type().to_string()
}

// ============================================================================
// CONVERSION FUNCTIONS (Internal Types → Proto Types)
// ============================================================================

fn trajectory_to_proto(t: &crate::types::TrajectoryResponse) -> TrajectoryResponse {
    TrajectoryResponse {
        trajectory_id: t.trajectory_id.to_string(),
        name: t.name.clone(),
        description: t.description.clone(),
        status: t.status.to_string(),
        parent_trajectory_id: t.parent_trajectory_id.map(|id| id.to_string()),
        root_trajectory_id: t.root_trajectory_id.map(|id| id.to_string()),
        agent_id: t.agent_id.map(|id| id.to_string()),
        created_at: timestamp_to_millis(&t.created_at),
        updated_at: timestamp_to_millis(&t.updated_at),
        completed_at: opt_timestamp_to_millis(t.completed_at),
        outcome: t.outcome.as_ref().map(|o| TrajectoryOutcome {
            status: o.status.to_string(),
            summary: o.summary.clone(),
            produced_artifacts: o.produced_artifacts.iter().map(|id| id.to_string()).collect(),
            produced_notes: o.produced_notes.iter().map(|id| id.to_string()).collect(),
            error: o.error.clone(),
        }),
        metadata: t.metadata.as_ref().map(|m| serde_json::to_string(m).unwrap_or_default()),
    }
}

fn scope_to_proto(s: &crate::types::ScopeResponse) -> ScopeResponse {
    ScopeResponse {
        scope_id: s.scope_id.to_string(),
        trajectory_id: s.trajectory_id.to_string(),
        parent_scope_id: s.parent_scope_id.map(|id| id.to_string()),
        name: s.name.clone(),
        purpose: s.purpose.clone(),
        is_active: s.is_active,
        created_at: timestamp_to_millis(&s.created_at),
        closed_at: opt_timestamp_to_millis(s.closed_at),
        checkpoint: s.checkpoint.as_ref().map(|c| CheckpointResponse {
            context_state: c.context_state.clone(),
            recoverable: c.recoverable,
        }),
        token_budget: s.token_budget,
        tokens_used: s.tokens_used,
        metadata: s.metadata.as_ref().map(|m| serde_json::to_string(m).unwrap_or_default()),
    }
}

fn artifact_to_proto(a: &crate::types::ArtifactResponse) -> ArtifactResponse {
    ArtifactResponse {
        artifact_id: a.artifact_id.to_string(),
        trajectory_id: a.trajectory_id.to_string(),
        scope_id: a.scope_id.to_string(),
        artifact_type: a.artifact_type.to_string(),
        name: a.name.clone(),
        content: a.content.clone(),
        content_hash: a.content_hash.to_vec(),
        embedding: a.embedding.as_ref().map(|e| Embedding {
            data: e.data.clone(),
            model_id: e.model_id.clone(),
            dimensions: e.dimensions,
        }),
        provenance: Some(Provenance {
            source_turn: a.provenance.source_turn,
            extraction_method: a.provenance.extraction_method.to_string(),
            confidence: a.provenance.confidence,
        }),
        ttl: serde_json::to_string(&a.ttl).unwrap_or_default(),
        created_at: timestamp_to_millis(&a.created_at),
        updated_at: timestamp_to_millis(&a.updated_at),
        superseded_by: a.superseded_by.map(|id| id.to_string()),
        metadata: a.metadata.as_ref().map(|m| serde_json::to_string(m).unwrap_or_default()),
    }
}

fn note_to_proto(n: &crate::types::NoteResponse) -> NoteResponse {
    NoteResponse {
        note_id: n.note_id.to_string(),
        note_type: n.note_type.to_string(),
        title: n.title.clone(),
        content: n.content.clone(),
        content_hash: n.content_hash.to_vec(),
        embedding: n.embedding.as_ref().map(|e| Embedding {
            data: e.data.clone(),
            model_id: e.model_id.clone(),
            dimensions: e.dimensions,
        }),
        source_trajectory_ids: n.source_trajectory_ids.iter().map(|id| id.to_string()).collect(),
        source_artifact_ids: n.source_artifact_ids.iter().map(|id| id.to_string()).collect(),
        ttl: serde_json::to_string(&n.ttl).unwrap_or_default(),
        created_at: timestamp_to_millis(&n.created_at),
        updated_at: timestamp_to_millis(&n.updated_at),
        accessed_at: timestamp_to_millis(&n.accessed_at),
        access_count: n.access_count,
        superseded_by: n.superseded_by.map(|id| id.to_string()),
        metadata: n.metadata.as_ref().map(|m| serde_json::to_string(m).unwrap_or_default()),
    }
}

fn turn_to_proto(t: &crate::types::TurnResponse) -> TurnResponse {
    TurnResponse {
        turn_id: t.turn_id.to_string(),
        scope_id: t.scope_id.to_string(),
        sequence: t.sequence,
        role: t.role.to_string(),
        content: t.content.clone(),
        token_count: t.token_count,
        created_at: timestamp_to_millis(&t.created_at),
        tool_calls: t.tool_calls.as_ref().map(|tc| serde_json::to_string(tc).unwrap_or_default()),
        tool_results: t.tool_results.as_ref().map(|tr| serde_json::to_string(tr).unwrap_or_default()),
        metadata: t.metadata.as_ref().map(|m| serde_json::to_string(m).unwrap_or_default()),
    }
}

fn agent_to_proto(a: &crate::types::AgentResponse) -> AgentResponse {
    AgentResponse {
        tenant_id: a.tenant_id.to_string(),
        agent_id: a.agent_id.to_string(),
        agent_type: a.agent_type.to_string(),
        capabilities: a.capabilities.clone(),
        memory_access: Some(MemoryAccess {
            read: a.memory_access.read.iter().map(|p| MemoryPermission {
                memory_type: p.memory_type.to_string(),
                scope: p.scope.clone(),
                filter: p.filter.clone(),
            }).collect(),
            write: a.memory_access.write.iter().map(|p| MemoryPermission {
                memory_type: p.memory_type.to_string(),
                scope: p.scope.clone(),
                filter: p.filter.clone(),
            }).collect(),
        }),
        status: a.status.to_string(),
        current_trajectory_id: a.current_trajectory_id.map(|id| id.to_string()),
        current_scope_id: a.current_scope_id.map(|id| id.to_string()),
        can_delegate_to: a.can_delegate_to.iter().map(|id| id.to_string()).collect(),
        reports_to: a.reports_to.map(|id| id.to_string()),
        created_at: timestamp_to_millis(&a.created_at),
        last_heartbeat: timestamp_to_millis(&a.last_heartbeat),
    }
}

fn lock_to_proto(l: &crate::types::LockResponse) -> LockResponse {
    LockResponse {
        tenant_id: l.tenant_id.to_string(),
        lock_id: l.lock_id.to_string(),
        resource_type: l.resource_type.to_string(),
        resource_id: l.resource_id.to_string(),
        holder_agent_id: l.holder_agent_id.to_string(),
        acquired_at: timestamp_to_millis(&l.acquired_at),
        expires_at: timestamp_to_millis(&l.expires_at),
        mode: l.mode.to_string(),
    }
}

fn message_to_proto(m: &crate::types::MessageResponse) -> MessageResponse {
    MessageResponse {
        tenant_id: m.tenant_id.to_string(),
        message_id: m.message_id.to_string(),
        from_agent_id: m.from_agent_id.to_string(),
        to_agent_id: m.to_agent_id.map(|id| id.to_string()),
        to_agent_type: m.to_agent_type.clone(),
        message_type: m.message_type.to_string(),
        payload: m.payload.clone(),
        trajectory_id: m.trajectory_id.map(|id| id.to_string()),
        scope_id: m.scope_id.map(|id| id.to_string()),
        artifact_ids: m.artifact_ids.iter().map(|id| id.to_string()).collect(),
        created_at: timestamp_to_millis(&m.created_at),
        delivered_at: opt_timestamp_to_millis(m.delivered_at),
        acknowledged_at: opt_timestamp_to_millis(m.acknowledged_at),
        priority: m.priority.to_string(),
        expires_at: opt_timestamp_to_millis(m.expires_at),
    }
}

fn delegation_to_proto(d: &crate::types::DelegationResponse) -> DelegationResponse {
    DelegationResponse {
        delegation_id: d.delegation_id.to_string(),
        from_agent_id: d.from_agent_id.to_string(),
        to_agent_id: d.to_agent_id.to_string(),
        trajectory_id: d.trajectory_id.to_string(),
        scope_id: d.scope_id.to_string(),
        task_description: d.task_description.clone(),
        status: d.status.to_string(),
        created_at: timestamp_to_millis(&d.created_at),
        accepted_at: opt_timestamp_to_millis(d.accepted_at),
        completed_at: opt_timestamp_to_millis(d.completed_at),
        expected_completion: opt_timestamp_to_millis(d.expected_completion),
        result: d.result.as_ref().map(|r| DelegationResult {
            status: r.status.to_string(),
            output: r.output.clone(),
            artifacts: r.artifacts.iter().map(|id| id.to_string()).collect(),
            error: r.error.clone(),
        }),
        context: d.context.as_ref().map(|c| serde_json::to_string(c).unwrap_or_default()),
    }
}

fn handoff_to_proto(h: &crate::types::HandoffResponse) -> HandoffResponse {
    HandoffResponse {
        tenant_id: h.tenant_id.to_string(),
        handoff_id: h.handoff_id.to_string(),
        from_agent_id: h.from_agent_id.to_string(),
        to_agent_id: h.to_agent_id.to_string(),
        trajectory_id: h.trajectory_id.to_string(),
        scope_id: h.scope_id.to_string(),
        reason: h.reason.clone(),
        status: h.status.to_string(),
        created_at: timestamp_to_millis(&h.created_at),
        accepted_at: opt_timestamp_to_millis(h.accepted_at),
        completed_at: opt_timestamp_to_millis(h.completed_at),
        context_snapshot: h.context_snapshot.clone(),
    }
}

// ============================================================================
// REMAINING SERVICE STUBS (Following REST Handler Pattern)
// ============================================================================
// Note: These services follow the same pattern as Trajectory/Scope services.
// They reuse REST handler logic and convert between proto and internal types.
// Full implementations will be added as caliber-pg functions become available.

// Artifact Service - mirrors routes/artifact.rs
pub struct ArtifactServiceImpl {
    db: DbClient,
    ws: Arc<WsState>,
}

impl ArtifactServiceImpl {
    pub fn new(db: DbClient, ws: Arc<WsState>) -> Self {
        Self { db, ws }
    }
}

#[tonic::async_trait]
impl artifact_service_server::ArtifactService for ArtifactServiceImpl {
    async fn create_artifact(&self, request: Request<CreateArtifactRequest>) -> Result<Response<ArtifactResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        let create_req = crate::types::CreateArtifactRequest {
            trajectory_id: req.trajectory_id.parse().map_err(|_| Status::invalid_argument("Invalid trajectory_id"))?,
            scope_id: req.scope_id.parse().map_err(|_| Status::invalid_argument("Invalid scope_id"))?,
            artifact_type: req.artifact_type.parse().map_err(|_| Status::invalid_argument("Invalid artifact_type"))?,
            name: req.name,
            content: req.content,
            source_turn: req.source_turn,
            extraction_method: req.extraction_method.parse().map_err(|_| Status::invalid_argument("Invalid extraction_method"))?,
            confidence: req.confidence,
            ttl: serde_json::from_str(&req.ttl).map_err(|_| Status::invalid_argument("Invalid TTL"))?,
            metadata: req.metadata.and_then(|s| serde_json::from_str(&s).ok()),
        };
        let artifact = self.db.artifact_create(&create_req, tenant_id).await?;
        self.ws.broadcast(WsEvent::ArtifactCreated { artifact: artifact.clone() });
        Ok(Response::new(artifact_to_proto(&artifact)))
    }

    async fn get_artifact(&self, request: Request<GetArtifactRequest>) -> Result<Response<ArtifactResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let id = request.into_inner().artifact_id.parse().map_err(|_| Status::invalid_argument("Invalid artifact_id"))?;
        let artifact = self.db.artifact_get(id, tenant_id).await?.ok_or_else(|| Status::not_found("Artifact not found"))?;
        Ok(Response::new(artifact_to_proto(&artifact)))
    }

    async fn list_artifacts(&self, request: Request<ListArtifactsRequest>) -> Result<Response<ListArtifactsResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        let scope_id = req.scope_id.ok_or_else(|| Status::invalid_argument("scope_id required"))?.parse().map_err(|_| Status::invalid_argument("Invalid scope_id"))?;
        let artifacts = self.db.artifact_list_by_scope(scope_id, tenant_id).await?;
        let total = artifacts.len() as i32;
        Ok(Response::new(ListArtifactsResponse {
            artifacts: artifacts.into_iter().map(|a| artifact_to_proto(&a)).collect(),
            total,
        }))
    }

    async fn update_artifact(&self, request: Request<UpdateArtifactRequest>) -> Result<Response<ArtifactResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        let id: caliber_core::EntityId = req.artifact_id.parse().map_err(|_| Status::invalid_argument("Invalid artifact_id"))?;
        let update_req = crate::types::UpdateArtifactRequest {
            name: req.name,
            content: req.content,
            artifact_type: req.artifact_type.and_then(|s| s.parse().ok()),
            ttl: req.ttl.and_then(|s| serde_json::from_str(&s).ok()),
            metadata: req.metadata.and_then(|s| serde_json::from_str(&s).ok()),
        };
        let artifact = self.db.artifact_update(id, &update_req, tenant_id).await?;
        self.ws.broadcast(WsEvent::ArtifactUpdated { artifact: artifact.clone() });
        Ok(Response::new(artifact_to_proto(&artifact)))
    }

    async fn delete_artifact(&self, request: Request<DeleteArtifactRequest>) -> Result<Response<Empty>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let id: caliber_core::EntityId = request.into_inner().artifact_id.parse().map_err(|_| Status::invalid_argument("Invalid artifact_id"))?;
        self.db.artifact_delete(id).await?;
        self.ws.broadcast(WsEvent::ArtifactDeleted { tenant_id, id });
        Ok(Response::new(Empty {}))
    }

    async fn search_artifacts(&self, request: Request<SearchRequest>) -> Result<Response<SearchResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();

        // Validate search query
        if req.query.trim().is_empty() {
            return Err(Status::invalid_argument("query cannot be empty"));
        }

        // Convert proto request to internal type
        let filters = req.filters
            .into_iter()
            .map(|f| {
                let operator = parse_filter_operator(&f.operator)?;
                Ok(crate::types::FilterExpr {
                    field: f.field,
                    operator,
                    value: serde_json::from_str(&f.value)
                        .unwrap_or(serde_json::Value::Null),
                })
            })
            .collect::<Result<Vec<_>, Status>>()?;

        let search_req = crate::types::SearchRequest {
            query: req.query,
            entity_types: req.entity_types.into_iter().filter_map(|s| s.parse().ok()).collect(),
            filters,
            limit: req.limit,
        };

        // Validate entity types include Artifact
        if !search_req.entity_types.contains(&caliber_core::EntityType::Artifact) {
            return Err(Status::invalid_argument(
                "entity_types must include Artifact for artifact search",
            ));
        }

        // Perform tenant-isolated search
        let result = self.db.search(&search_req, tenant_id).await?;

        Ok(Response::new(SearchResponse {
            results: result.results.into_iter().map(|r| SearchResult {
                entity_type: r.entity_type.to_string(),
                id: r.id.to_string(),
                name: r.name,
                snippet: r.snippet,
                score: r.score,
            }).collect(),
            total: result.total,
        }))
    }
}

// Note Service - mirrors routes/note.rs
pub struct NoteServiceImpl {
    db: DbClient,
    ws: Arc<WsState>,
}

impl NoteServiceImpl {
    pub fn new(db: DbClient, ws: Arc<WsState>) -> Self {
        Self { db, ws }
    }
}

#[tonic::async_trait]
impl note_service_server::NoteService for NoteServiceImpl {
    async fn create_note(&self, request: Request<CreateNoteRequest>) -> Result<Response<NoteResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        let create_req = crate::types::CreateNoteRequest {
            note_type: req.note_type.parse().map_err(|_| Status::invalid_argument("Invalid note_type"))?,
            title: req.title,
            content: req.content,
            source_trajectory_ids: req.source_trajectory_ids.into_iter().filter_map(|s| s.parse().ok()).collect(),
            source_artifact_ids: req.source_artifact_ids.into_iter().filter_map(|s| s.parse().ok()).collect(),
            ttl: serde_json::from_str(&req.ttl).map_err(|_| Status::invalid_argument("Invalid TTL"))?,
            metadata: req.metadata.and_then(|s| serde_json::from_str(&s).ok()),
        };
        let note = self.db.note_create(&create_req, tenant_id).await?;
        self.ws.broadcast(WsEvent::NoteCreated { note: note.clone() });
        Ok(Response::new(note_to_proto(&note)))
    }

    async fn get_note(&self, request: Request<GetNoteRequest>) -> Result<Response<NoteResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let id = request.into_inner().note_id.parse().map_err(|_| Status::invalid_argument("Invalid note_id"))?;
        let note = self.db.note_get(id, tenant_id).await?.ok_or_else(|| Status::not_found("Note not found"))?;
        Ok(Response::new(note_to_proto(&note)))
    }

    async fn list_notes(&self, request: Request<ListNotesRequest>) -> Result<Response<ListNotesResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();

        if let Some(source_trajectory_id) = req.source_trajectory_id {
            // Filter by source trajectory
            let trajectory_id = source_trajectory_id.parse().map_err(|_| Status::invalid_argument("Invalid source_trajectory_id"))?;
            let notes = self.db.note_list_by_trajectory(trajectory_id, tenant_id).await?;

            // Apply additional filters if needed
            let mut filtered = notes;

            if let Some(note_type) = req.note_type {
                if let Ok(parsed_type) = note_type.parse::<caliber_core::NoteType>() {
                    filtered.retain(|n| n.note_type == parsed_type);
                }
            }

            if let Some(created_after) = req.created_after {
                let timestamp = parse_timestamp_millis(created_after, "created_after")?;
                filtered.retain(|n| n.created_at >= timestamp);
            }

            if let Some(created_before) = req.created_before {
                let timestamp = parse_timestamp_millis(created_before, "created_before")?;
                filtered.retain(|n| n.created_at <= timestamp);
            }

            // Apply pagination
            let total = filtered.len() as i32;
            let offset = req.offset.unwrap_or(0) as usize;
            let limit = req.limit.unwrap_or(100) as usize;

            let paginated: Vec<_> = filtered.into_iter().skip(offset).take(limit).collect();

            Ok(Response::new(ListNotesResponse {
                notes: paginated.into_iter().map(|n| note_to_proto(&n)).collect(),
                total,
            }))
        } else {
            // No trajectory filter - return all notes with pagination
            let limit = req.limit.unwrap_or(100);
            let offset = req.offset.unwrap_or(0);

            let notes = self.db.note_list_all_by_tenant(limit, offset, tenant_id).await?;

            // Apply additional filters if needed
            let mut filtered = notes;

            if let Some(note_type) = req.note_type {
                if let Ok(parsed_type) = note_type.parse::<caliber_core::NoteType>() {
                    filtered.retain(|n| n.note_type == parsed_type);
                }
            }

            if let Some(created_after) = req.created_after {
                let timestamp = parse_timestamp_millis(created_after, "created_after")?;
                filtered.retain(|n| n.created_at >= timestamp);
            }

            if let Some(created_before) = req.created_before {
                let timestamp = parse_timestamp_millis(created_before, "created_before")?;
                filtered.retain(|n| n.created_at <= timestamp);
            }

            let total = filtered.len() as i32;

            Ok(Response::new(ListNotesResponse {
                notes: filtered.into_iter().map(|n| note_to_proto(&n)).collect(),
                total,
            }))
        }
    }

    async fn update_note(&self, request: Request<UpdateNoteRequest>) -> Result<Response<NoteResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        let id: caliber_core::EntityId = req.note_id.parse().map_err(|_| Status::invalid_argument("Invalid note_id"))?;
        let update_req = crate::types::UpdateNoteRequest {
            title: req.title,
            content: req.content,
            note_type: req.note_type.and_then(|s| s.parse().ok()),
            ttl: req.ttl.and_then(|s| serde_json::from_str(&s).ok()),
            metadata: req.metadata.and_then(|s| serde_json::from_str(&s).ok()),
        };
        let note = self.db.note_update(id, &update_req, tenant_id).await?;
        self.ws.broadcast(WsEvent::NoteUpdated { note: note.clone() });
        Ok(Response::new(note_to_proto(&note)))
    }

    async fn delete_note(&self, request: Request<DeleteNoteRequest>) -> Result<Response<Empty>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let id: caliber_core::EntityId = request.into_inner().note_id.parse().map_err(|_| Status::invalid_argument("Invalid note_id"))?;
        self.db.note_delete(id).await?;
        self.ws.broadcast(WsEvent::NoteDeleted { tenant_id, id });
        Ok(Response::new(Empty {}))
    }

    async fn search_notes(&self, request: Request<SearchRequest>) -> Result<Response<SearchResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();

        // Validate search query
        if req.query.trim().is_empty() {
            return Err(Status::invalid_argument("query cannot be empty"));
        }

        // Convert proto request to internal type
        let filters = req.filters
            .into_iter()
            .map(|f| {
                let operator = parse_filter_operator(&f.operator)?;
                Ok(crate::types::FilterExpr {
                    field: f.field,
                    operator,
                    value: serde_json::from_str(&f.value)
                        .unwrap_or(serde_json::Value::Null),
                })
            })
            .collect::<Result<Vec<_>, Status>>()?;

        let search_req = crate::types::SearchRequest {
            query: req.query,
            entity_types: req.entity_types.into_iter().filter_map(|s| s.parse().ok()).collect(),
            filters,
            limit: req.limit,
        };

        // Validate entity types include Note
        if !search_req.entity_types.contains(&caliber_core::EntityType::Note) {
            return Err(Status::invalid_argument(
                "entity_types must include Note for note search",
            ));
        }

        // Perform tenant-isolated search
        let result = self.db.search(&search_req, tenant_id).await?;

        Ok(Response::new(SearchResponse {
            results: result.results.into_iter().map(|r| SearchResult {
                entity_type: r.entity_type.to_string(),
                id: r.id.to_string(),
                name: r.name,
                snippet: r.snippet,
                score: r.score,
            }).collect(),
            total: result.total,
        }))
    }
}

// Turn Service - mirrors routes/turn.rs
pub struct TurnServiceImpl {
    db: DbClient,
    ws: Arc<WsState>,
}

impl TurnServiceImpl {
    pub fn new(db: DbClient, ws: Arc<WsState>) -> Self {
        Self { db, ws }
    }
}

#[tonic::async_trait]
impl turn_service_server::TurnService for TurnServiceImpl {
    async fn create_turn(&self, request: Request<CreateTurnRequest>) -> Result<Response<TurnResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        let create_req = crate::types::CreateTurnRequest {
            scope_id: req.scope_id.parse().map_err(|_| Status::invalid_argument("Invalid scope_id"))?,
            sequence: req.sequence,
            role: req.role.parse().map_err(|_| Status::invalid_argument("Invalid role"))?,
            content: req.content,
            token_count: req.token_count,
            tool_calls: req.tool_calls.and_then(|s| serde_json::from_str(&s).ok()),
            tool_results: req.tool_results.and_then(|s| serde_json::from_str(&s).ok()),
            metadata: req.metadata.and_then(|s| serde_json::from_str(&s).ok()),
        };
        let turn = self.db.turn_create(&create_req, tenant_id).await?;
        self.ws.broadcast(WsEvent::TurnCreated { turn: turn.clone() });
        Ok(Response::new(turn_to_proto(&turn)))
    }

    async fn get_turn(&self, request: Request<GetTurnRequest>) -> Result<Response<TurnResponse>, Status> {
        let id = request.into_inner().turn_id.parse().map_err(|_| Status::invalid_argument("Invalid turn_id"))?;
        let turn = self.db.turn_get(id).await?.ok_or_else(|| Status::not_found("Turn not found"))?;
        Ok(Response::new(turn_to_proto(&turn)))
    }
}

// Agent Service - mirrors routes/agent.rs
pub struct AgentServiceImpl {
    db: DbClient,
    ws: Arc<WsState>,
}

impl AgentServiceImpl {
    pub fn new(db: DbClient, ws: Arc<WsState>) -> Self {
        Self { db, ws }
    }
}

#[tonic::async_trait]
impl agent_service_server::AgentService for AgentServiceImpl {
    async fn register_agent(&self, request: Request<RegisterAgentRequest>) -> Result<Response<AgentResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        let memory_access = req.memory_access.ok_or_else(|| Status::invalid_argument("memory_access required"))?;
        let register_req = crate::types::RegisterAgentRequest {
            agent_type: req.agent_type.parse().map_err(|_| Status::invalid_argument("Invalid agent_type"))?,
            capabilities: req.capabilities,
            memory_access: crate::types::MemoryAccessRequest {
                read: memory_access.read.into_iter().map(|p| crate::types::MemoryPermissionRequest {
                    memory_type: p.memory_type,
                    scope: p.scope,
                    filter: p.filter,
                }).collect(),
                write: memory_access.write.into_iter().map(|p| crate::types::MemoryPermissionRequest {
                    memory_type: p.memory_type,
                    scope: p.scope,
                    filter: p.filter,
                }).collect(),
            },
            can_delegate_to: req.can_delegate_to.into_iter().filter_map(|s| s.parse().ok()).collect(),
            reports_to: req.reports_to.and_then(|s| s.parse().ok()),
        };
        let agent = self.db.agent_register(&register_req, tenant_id).await?;
        self.ws.broadcast(WsEvent::AgentRegistered { agent: agent.clone() });
        Ok(Response::new(agent_to_proto(&agent)))
    }

    async fn get_agent(&self, request: Request<GetAgentRequest>) -> Result<Response<AgentResponse>, Status> {
        let id = request.into_inner().agent_id.parse().map_err(|_| Status::invalid_argument("Invalid agent_id"))?;
        let agent = self.db.agent_get(id).await?.ok_or_else(|| Status::not_found("Agent not found"))?;
        Ok(Response::new(agent_to_proto(&agent)))
    }

    async fn list_agents(&self, request: Request<ListAgentsRequest>) -> Result<Response<ListAgentsResponse>, Status> {
        let req = request.into_inner();

        let agents = if let Some(agent_type) = req.agent_type {
            // Filter by agent type
            self.db.agent_list_by_type(&agent_type).await?
        } else if let Some(status) = req.status {
            // If status filter is "active", use agent_list_active
            if status.to_lowercase() == "active" {
                self.db.agent_list_active().await?
            } else {
                // Otherwise, list all and filter by status
                let all_agents = self.db.agent_list_all().await?;
                all_agents.into_iter().filter(|a| a.status == status).collect()
            }
        } else {
            // List all agents
            self.db.agent_list_all().await?
        };

        Ok(Response::new(ListAgentsResponse {
            agents: agents.into_iter().map(|a| agent_to_proto(&a)).collect(),
        }))
    }

    async fn update_agent(&self, request: Request<UpdateAgentRequest>) -> Result<Response<AgentResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        let id: caliber_core::EntityId = req.agent_id.parse().map_err(|_| Status::invalid_argument("Invalid agent_id"))?;
        let memory_access = req.memory_access.map(|access| crate::types::MemoryAccessRequest {
            read: access.read.into_iter().map(|p| crate::types::MemoryPermissionRequest {
                memory_type: p.memory_type,
                scope: p.scope,
                filter: p.filter,
            }).collect(),
            write: access.write.into_iter().map(|p| crate::types::MemoryPermissionRequest {
                memory_type: p.memory_type,
                scope: p.scope,
                filter: p.filter,
            }).collect(),
        });
        let update_req = crate::types::UpdateAgentRequest {
            status: req.status.and_then(|s| s.parse().ok()),
            capabilities: if req.capabilities.is_empty() { None } else { Some(req.capabilities) },
            current_trajectory_id: req.current_trajectory_id.and_then(|s| s.parse().ok()),
            current_scope_id: req.current_scope_id.and_then(|s| s.parse().ok()),
            memory_access,
        };
        let agent = self.db.agent_update(id, &update_req).await?;
        let status = agent.status.clone();
        self.ws.broadcast(WsEvent::AgentStatusChanged { tenant_id, agent_id: id, status });
        Ok(Response::new(agent_to_proto(&agent)))
    }

    async fn unregister_agent(&self, request: Request<UnregisterAgentRequest>) -> Result<Response<Empty>, Status> {
        let id: caliber_core::EntityId = request.into_inner().agent_id.parse().map_err(|_| Status::invalid_argument("Invalid agent_id"))?;
        self.db.agent_unregister(id).await?;
        Ok(Response::new(Empty {}))
    }

    async fn heartbeat(&self, request: Request<HeartbeatRequest>) -> Result<Response<HeartbeatResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let id = request.into_inner().agent_id.parse().map_err(|_| Status::invalid_argument("Invalid agent_id"))?;
        self.db.agent_heartbeat(id).await?;
        let now = Utc::now();
        self.ws.broadcast(WsEvent::AgentHeartbeat { tenant_id, agent_id: id, timestamp: now });
        Ok(Response::new(HeartbeatResponse { timestamp: timestamp_to_millis(&now) }))
    }
}

// Lock Service - mirrors routes/lock.rs
pub struct LockServiceImpl {
    db: DbClient,
    ws: Arc<WsState>,
}

impl LockServiceImpl {
    pub fn new(db: DbClient, ws: Arc<WsState>) -> Self {
        Self { db, ws }
    }
}

#[tonic::async_trait]
impl lock_service_server::LockService for LockServiceImpl {
    async fn acquire_lock(&self, request: Request<AcquireLockRequest>) -> Result<Response<LockResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        let acquire_req = crate::types::AcquireLockRequest {
            resource_type: req.resource_type.parse().map_err(|_| Status::invalid_argument("Invalid resource_type"))?,
            resource_id: req.resource_id.parse().map_err(|_| Status::invalid_argument("Invalid resource_id"))?,
            holder_agent_id: req.holder_agent_id.parse().map_err(|_| Status::invalid_argument("Invalid holder_agent_id"))?,
            timeout_ms: req.timeout_ms,
            mode: req.mode.parse().map_err(|_| Status::invalid_argument("Invalid mode"))?,
        };
        let lock = self.db.lock_acquire(&acquire_req, tenant_id).await?;
        self.ws.broadcast(WsEvent::LockAcquired { lock: lock.clone() });
        Ok(Response::new(lock_to_proto(&lock)))
    }

    async fn release_lock(&self, request: Request<ReleaseLockRequest>) -> Result<Response<Empty>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let id = request.into_inner().lock_id.parse().map_err(|_| Status::invalid_argument("Invalid lock_id"))?;
        self.db.lock_release(id).await?;
        self.ws.broadcast(WsEvent::LockReleased { tenant_id, lock_id: id });
        Ok(Response::new(Empty {}))
    }

    async fn extend_lock(&self, request: Request<ExtendLockRequest>) -> Result<Response<LockResponse>, Status> {
        let req = request.into_inner();
        let id: caliber_core::EntityId = req.lock_id.parse().map_err(|_| Status::invalid_argument("Invalid lock_id"))?;
        if req.additional_ms <= 0 {
            return Err(Status::invalid_argument("additional_ms must be positive"));
        }
        let duration = std::time::Duration::from_millis(req.additional_ms as u64);
        let lock = self.db.lock_extend(id, duration).await?;
        Ok(Response::new(lock_to_proto(&lock)))
    }

    async fn list_locks(&self, request: Request<ListLocksRequest>) -> Result<Response<ListLocksResponse>, Status> {
        let req = request.into_inner();
        let locks = self.db.lock_list_active().await?;
        let filtered = locks.into_iter().filter(|lock| {
            if let Some(ref resource_type) = req.resource_type {
                if &lock.resource_type != resource_type {
                    return false;
                }
            }
            if let Some(ref holder_agent_id) = req.holder_agent_id {
                if lock.holder_agent_id.to_string() != *holder_agent_id {
                    return false;
                }
            }
            true
        })
        .map(|lock| lock_to_proto(&lock))
        .collect();
        Ok(Response::new(ListLocksResponse { locks: filtered }))
    }
}

// Message Service - mirrors routes/message.rs
pub struct MessageServiceImpl {
    db: DbClient,
    ws: Arc<WsState>,
}

impl MessageServiceImpl {
    pub fn new(db: DbClient, ws: Arc<WsState>) -> Self {
        Self { db, ws }
    }
}

#[tonic::async_trait]
impl message_service_server::MessageService for MessageServiceImpl {
    async fn send_message(&self, request: Request<SendMessageRequest>) -> Result<Response<MessageResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        let send_req = crate::types::SendMessageRequest {
            from_agent_id: req.from_agent_id.parse().map_err(|_| Status::invalid_argument("Invalid from_agent_id"))?,
            to_agent_id: req.to_agent_id.and_then(|s| s.parse().ok()),
            to_agent_type: req.to_agent_type,
            message_type: req.message_type.parse().map_err(|_| Status::invalid_argument("Invalid message_type"))?,
            payload: req.payload,
            trajectory_id: req.trajectory_id.and_then(|s| s.parse().ok()),
            scope_id: req.scope_id.and_then(|s| s.parse().ok()),
            artifact_ids: req.artifact_ids.into_iter().filter_map(|s| s.parse().ok()).collect(),
            priority: req.priority.parse().map_err(|_| Status::invalid_argument("Invalid priority"))?,
            expires_at: parse_optional_timestamp_millis(req.expires_at, "expires_at")?,
        };
        let message = self.db.message_send(&send_req, tenant_id).await?;
        self.ws.broadcast(WsEvent::MessageSent { message: message.clone() });
        Ok(Response::new(message_to_proto(&message)))
    }

    async fn get_message(&self, request: Request<GetMessageRequest>) -> Result<Response<MessageResponse>, Status> {
        let id = request.into_inner().message_id.parse().map_err(|_| Status::invalid_argument("Invalid message_id"))?;
        let message = self.db.message_get(id).await?.ok_or_else(|| Status::not_found("Message not found"))?;
        Ok(Response::new(message_to_proto(&message)))
    }

    async fn list_messages(&self, request: Request<ListMessagesRequest>) -> Result<Response<ListMessagesResponse>, Status> {
        let req = request.into_inner();

        let limit = 100;
        let offset = 0;

        let messages = self.db.message_list(crate::db::MessageListParams {
            from_agent_id: req.from_agent_id.and_then(|s| s.parse().ok()),
            to_agent_id: req.to_agent_id.and_then(|s| s.parse().ok()),
            to_agent_type: None,
            trajectory_id: req.trajectory_id.and_then(|s| s.parse().ok()),
            message_type: req.message_type.as_deref(),
            priority: None,
            undelivered_only: false,
            unacknowledged_only: false,
            limit,
            offset,
        }).await?;

        Ok(Response::new(ListMessagesResponse {
            messages: messages.into_iter().map(|m| message_to_proto(&m)).collect(),
        }))
    }

    async fn deliver_message(&self, request: Request<DeliverMessageRequest>) -> Result<Response<MessageResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let id = request.into_inner().message_id.parse().map_err(|_| Status::invalid_argument("Invalid message_id"))?;
        self.db.message_deliver(id).await?;
        let message = self
            .db
            .message_get(id)
            .await
            ?
            .ok_or_else(|| Status::not_found("Message not found"))?;
        self.ws.broadcast(WsEvent::MessageDelivered { tenant_id, message_id: id });
        Ok(Response::new(message_to_proto(&message)))
    }

    async fn acknowledge_message(&self, request: Request<AcknowledgeMessageRequest>) -> Result<Response<MessageResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let id = request.into_inner().message_id.parse().map_err(|_| Status::invalid_argument("Invalid message_id"))?;
        self.db.message_acknowledge(id).await?;
        let message = self
            .db
            .message_get(id)
            .await
            ?
            .ok_or_else(|| Status::not_found("Message not found"))?;
        self.ws.broadcast(WsEvent::MessageAcknowledged { tenant_id, message_id: id });
        Ok(Response::new(message_to_proto(&message)))
    }
}

// Delegation Service - mirrors routes/delegation.rs
pub struct DelegationServiceImpl {
    db: DbClient,
    ws: Arc<WsState>,
}

impl DelegationServiceImpl {
    pub fn new(db: DbClient, ws: Arc<WsState>) -> Self {
        Self { db, ws }
    }
}

#[tonic::async_trait]
impl delegation_service_server::DelegationService for DelegationServiceImpl {
    async fn create_delegation(&self, request: Request<CreateDelegationRequest>) -> Result<Response<DelegationResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        let create_req = crate::types::CreateDelegationRequest {
            from_agent_id: req.from_agent_id.parse().map_err(|_| Status::invalid_argument("Invalid from_agent_id"))?,
            to_agent_id: req.to_agent_id.parse().map_err(|_| Status::invalid_argument("Invalid to_agent_id"))?,
            trajectory_id: req.trajectory_id.parse().map_err(|_| Status::invalid_argument("Invalid trajectory_id"))?,
            scope_id: req.scope_id.parse().map_err(|_| Status::invalid_argument("Invalid scope_id"))?,
            task_description: req.task_description,
            expected_completion: parse_optional_timestamp_millis(req.expected_completion, "expected_completion")?,
            context: req.context.and_then(|s| serde_json::from_str(&s).ok()),
        };
        let delegation = self.db.delegation_create(&create_req, tenant_id).await?;
        self.ws.broadcast(WsEvent::DelegationCreated {
            delegation: delegation.clone(),
        });
        Ok(Response::new(delegation_to_proto(&delegation)))
    }

    async fn get_delegation(&self, request: Request<GetDelegationRequest>) -> Result<Response<DelegationResponse>, Status> {
        let id = request.into_inner().delegation_id.parse().map_err(|_| Status::invalid_argument("Invalid delegation_id"))?;
        let delegation = self.db.delegation_get(id).await?.ok_or_else(|| Status::not_found("Delegation not found"))?;
        Ok(Response::new(delegation_to_proto(&delegation)))
    }

    async fn accept_delegation(&self, request: Request<AcceptDelegationRequest>) -> Result<Response<DelegationResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        let id = req.delegation_id.parse().map_err(|_| Status::invalid_argument("Invalid delegation_id"))?;
        let accepting_agent_id = req.accepting_agent_id.parse().map_err(|_| Status::invalid_argument("Invalid accepting_agent_id"))?;
        let delegation = self.db.delegation_accept(id, accepting_agent_id).await?;
        self.ws.broadcast(WsEvent::DelegationAccepted {
            tenant_id,
            delegation_id: id,
        });
        Ok(Response::new(delegation_to_proto(&delegation)))
    }

    async fn reject_delegation(&self, request: Request<RejectDelegationRequest>) -> Result<Response<DelegationResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        let id = req.delegation_id.parse().map_err(|_| Status::invalid_argument("Invalid delegation_id"))?;
        let delegation = self.db.delegation_reject(id, req.reason).await?;
        self.ws.broadcast(WsEvent::DelegationRejected {
            tenant_id,
            delegation_id: id,
        });
        Ok(Response::new(delegation_to_proto(&delegation)))
    }

    async fn complete_delegation(&self, request: Request<CompleteDelegationRequest>) -> Result<Response<DelegationResponse>, Status> {
        let req = request.into_inner();
        let id = req.delegation_id.parse().map_err(|_| Status::invalid_argument("Invalid delegation_id"))?;
        let result = req.result.ok_or_else(|| Status::invalid_argument("result required"))?;
        let result_req = crate::types::DelegationResultRequest {
            status: result.status,
            output: result.output,
            artifacts: result.artifacts.into_iter().filter_map(|s| s.parse().ok()).collect(),
            error: result.error,
        };
        let result_json = serde_json::to_value(&result_req).map_err(|_| Status::internal("Failed to serialize delegation result"))?;
        let delegation = self.db.delegation_complete(id, result_json).await?;
        self.ws.broadcast(WsEvent::DelegationCompleted {
            delegation: delegation.clone(),
        });
        Ok(Response::new(delegation_to_proto(&delegation)))
    }
}

// Handoff Service - mirrors routes/handoff.rs
pub struct HandoffServiceImpl {
    db: DbClient,
    ws: Arc<WsState>,
}

impl HandoffServiceImpl {
    pub fn new(db: DbClient, ws: Arc<WsState>) -> Self {
        Self { db, ws }
    }
}

#[tonic::async_trait]
impl handoff_service_server::HandoffService for HandoffServiceImpl {
    async fn create_handoff(&self, request: Request<CreateHandoffRequest>) -> Result<Response<HandoffResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        let create_req = crate::types::CreateHandoffRequest {
            from_agent_id: req.from_agent_id.parse().map_err(|_| Status::invalid_argument("Invalid from_agent_id"))?,
            to_agent_id: req.to_agent_id.parse().map_err(|_| Status::invalid_argument("Invalid to_agent_id"))?,
            trajectory_id: req.trajectory_id.parse().map_err(|_| Status::invalid_argument("Invalid trajectory_id"))?,
            scope_id: req.scope_id.parse().map_err(|_| Status::invalid_argument("Invalid scope_id"))?,
            reason: req.reason,
            context_snapshot: req.context_snapshot,
        };
        let handoff = self.db.handoff_create(&create_req, tenant_id).await?;
        self.ws.broadcast(WsEvent::HandoffCreated {
            handoff: handoff.clone(),
        });
        Ok(Response::new(handoff_to_proto(&handoff)))
    }

    async fn get_handoff(&self, request: Request<GetHandoffRequest>) -> Result<Response<HandoffResponse>, Status> {
        let id = request.into_inner().handoff_id.parse().map_err(|_| Status::invalid_argument("Invalid handoff_id"))?;
        let handoff = self.db.handoff_get(id).await?.ok_or_else(|| Status::not_found("Handoff not found"))?;
        Ok(Response::new(handoff_to_proto(&handoff)))
    }

    async fn accept_handoff(&self, request: Request<AcceptHandoffRequest>) -> Result<Response<HandoffResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        let id = req.handoff_id.parse().map_err(|_| Status::invalid_argument("Invalid handoff_id"))?;
        let accepting_agent_id = req.accepting_agent_id.parse().map_err(|_| Status::invalid_argument("Invalid accepting_agent_id"))?;
        let handoff = self.db.handoff_accept(id, accepting_agent_id).await?;
        self.ws.broadcast(WsEvent::HandoffAccepted { tenant_id, handoff_id: id });
        Ok(Response::new(handoff_to_proto(&handoff)))
    }

    async fn complete_handoff(&self, request: Request<CompleteHandoffRequest>) -> Result<Response<HandoffResponse>, Status> {
        let id = request.into_inner().handoff_id.parse().map_err(|_| Status::invalid_argument("Invalid handoff_id"))?;
        let handoff = self.db.handoff_complete(id).await?;
        self.ws.broadcast(WsEvent::HandoffCompleted {
            handoff: handoff.clone(),
        });
        Ok(Response::new(handoff_to_proto(&handoff)))
    }
}

// DSL Service - mirrors routes/dsl.rs
pub struct DslServiceImpl {
    db: DbClient,
}

impl DslServiceImpl {
    pub fn new(db: DbClient) -> Self {
        Self { db }
    }
}

#[tonic::async_trait]
impl dsl_service_server::DslService for DslServiceImpl {
    async fn validate_dsl(&self, request: Request<ValidateDslRequest>) -> Result<Response<ValidateDslResponse>, Status> {
        let req = request.into_inner();
        let validate_req = crate::types::ValidateDslRequest {
            source: req.source,
        };
        let result = self.db.dsl_validate(&validate_req).await?;
        Ok(Response::new(ValidateDslResponse {
            valid: result.valid,
            errors: result.errors.into_iter().map(|e| ParseError {
                line: u32::try_from(e.line).unwrap_or(u32::MAX),
                column: u32::try_from(e.column).unwrap_or(u32::MAX),
                message: e.message,
            }).collect(),
            ast: result.ast.map(|ast| serde_json::to_string(&ast).unwrap_or_default()),
        }))
    }

    async fn parse_dsl(&self, request: Request<ParseDslRequest>) -> Result<Response<ParseDslResponse>, Status> {
        let req = request.into_inner();
        let parse_req = crate::types::ParseDslRequest {
            source: req.source,
        };
        let result = self.db.dsl_parse(&parse_req).await?;
        Ok(Response::new(ParseDslResponse {
            ast: serde_json::to_string(&result.ast).unwrap_or_default(),
        }))
    }
}

// Config Service - mirrors routes/config.rs
pub struct ConfigServiceImpl {
    db: DbClient,
    ws: Arc<WsState>,
}

impl ConfigServiceImpl {
    pub fn new(db: DbClient, ws: Arc<WsState>) -> Self {
        Self { db, ws }
    }
}

#[tonic::async_trait]
impl config_service_server::ConfigService for ConfigServiceImpl {
    async fn get_config(&self, _request: Request<GetConfigRequest>) -> Result<Response<ConfigResponse>, Status> {
        let config = self.db.config_get().await?;
        Ok(Response::new(ConfigResponse {
            config: serde_json::to_string(&config.config).unwrap_or_default(),
            valid: config.valid,
            errors: config.errors,
        }))
    }

    async fn update_config(&self, request: Request<UpdateConfigRequest>) -> Result<Response<ConfigResponse>, Status> {
        let req = request.into_inner();
        let update_req = crate::types::UpdateConfigRequest {
            config: serde_json::from_str(&req.config).map_err(|_| Status::invalid_argument("Invalid config JSON"))?,
        };
        let config = self.db.config_update(&update_req).await?;
        self.ws.broadcast(WsEvent::ConfigUpdated {
            config: config.clone(),
        });
        Ok(Response::new(ConfigResponse {
            config: serde_json::to_string(&config.config).unwrap_or_default(),
            valid: config.valid,
            errors: config.errors,
        }))
    }

    async fn validate_config(&self, request: Request<ValidateConfigRequest>) -> Result<Response<ValidateConfigResponse>, Status> {
        let req = request.into_inner();
        let validate_req = crate::types::ValidateConfigRequest {
            config: serde_json::from_str(&req.config).map_err(|_| Status::invalid_argument("Invalid config JSON"))?,
        };
        let result = self.db.config_validate(&validate_req).await?;
        Ok(Response::new(ValidateConfigResponse {
            valid: result.valid,
            errors: result.errors,
        }))
    }
}

// Tenant Service - mirrors routes/tenant.rs
pub struct TenantServiceImpl {
    db: DbClient,
}

impl TenantServiceImpl {
    pub fn new(db: DbClient) -> Self {
        Self { db }
    }
}

#[tonic::async_trait]
impl tenant_service_server::TenantService for TenantServiceImpl {
    async fn list_tenants(&self, _request: Request<ListTenantsRequest>) -> Result<Response<ListTenantsResponse>, Status> {
        let tenants = self.db.tenant_list().await?;
        Ok(Response::new(ListTenantsResponse {
            tenants: tenants.into_iter().map(|t| TenantInfo {
                tenant_id: t.tenant_id.to_string(),
                name: t.name,
                status: t.status.to_string(),
                created_at: timestamp_to_millis(&t.created_at),
            }).collect(),
        }))
    }

    async fn get_tenant(&self, request: Request<GetTenantRequest>) -> Result<Response<TenantInfo>, Status> {
        let id = request.into_inner().tenant_id.parse().map_err(|_| Status::invalid_argument("Invalid tenant_id"))?;
        let tenant = self.db.tenant_get(id).await?.ok_or_else(|| Status::not_found("Tenant not found"))?;
        Ok(Response::new(TenantInfo {
            tenant_id: tenant.tenant_id.to_string(),
            name: tenant.name,
            status: tenant.status.to_string(),
            created_at: timestamp_to_millis(&tenant.created_at),
        }))
    }
}

// Search Service - mirrors search functionality
pub struct SearchServiceImpl {
    db: DbClient,
}

impl SearchServiceImpl {
    pub fn new(db: DbClient) -> Self {
        Self { db }
    }
}

#[tonic::async_trait]
impl search_service_server::SearchService for SearchServiceImpl {
    async fn search(&self, request: Request<SearchRequest>) -> Result<Response<SearchResponse>, Status> {
        let tenant_id = extract_tenant_id(&request)?;
        let req = request.into_inner();
        let filters = req.filters
            .into_iter()
            .map(|f| {
                let operator = parse_filter_operator(&f.operator)?;
                Ok(crate::types::FilterExpr {
                    field: f.field,
                    operator,
                    value: serde_json::from_str(&f.value)
                        .unwrap_or(serde_json::Value::Null),
                })
            })
            .collect::<Result<Vec<_>, Status>>()?;

        let search_req = crate::types::SearchRequest {
            query: req.query,
            entity_types: req.entity_types.into_iter().filter_map(|s| s.parse().ok()).collect(),
            filters,
            limit: req.limit,
        };
        let result = self.db.search(&search_req, tenant_id).await?;
        Ok(Response::new(SearchResponse {
            results: result.results.into_iter().map(|r| SearchResult {
                entity_type: r.entity_type.to_string(),
                id: r.id.to_string(),
                name: r.name,
                snippet: r.snippet,
                score: r.score,
            }).collect(),
            total: result.total,
        }))
    }
}

// ============================================================================
// PUBLIC API - Service Constructors
// ============================================================================

/// Tuple of all gRPC service servers for registration.
pub type GrpcServices = (
    trajectory_service_server::TrajectoryServiceServer<TrajectoryServiceImpl>,
    scope_service_server::ScopeServiceServer<ScopeServiceImpl>,
    artifact_service_server::ArtifactServiceServer<ArtifactServiceImpl>,
    note_service_server::NoteServiceServer<NoteServiceImpl>,
    turn_service_server::TurnServiceServer<TurnServiceImpl>,
    agent_service_server::AgentServiceServer<AgentServiceImpl>,
    lock_service_server::LockServiceServer<LockServiceImpl>,
    message_service_server::MessageServiceServer<MessageServiceImpl>,
    delegation_service_server::DelegationServiceServer<DelegationServiceImpl>,
    handoff_service_server::HandoffServiceServer<HandoffServiceImpl>,
    dsl_service_server::DslServiceServer<DslServiceImpl>,
    config_service_server::ConfigServiceServer<ConfigServiceImpl>,
    tenant_service_server::TenantServiceServer<TenantServiceImpl>,
    search_service_server::SearchServiceServer<SearchServiceImpl>,
    event_service_server::EventServiceServer<EventServiceImpl>,
);

/// Create all gRPC service implementations with shared state
pub fn create_services(
    db: DbClient,
    ws: Arc<WsState>,
) -> GrpcServices {
    (
        trajectory_service_server::TrajectoryServiceServer::new(TrajectoryServiceImpl::new(db.clone(), ws.clone())),
        scope_service_server::ScopeServiceServer::new(ScopeServiceImpl::new(db.clone(), ws.clone())),
        artifact_service_server::ArtifactServiceServer::new(ArtifactServiceImpl::new(db.clone(), ws.clone())),
        note_service_server::NoteServiceServer::new(NoteServiceImpl::new(db.clone(), ws.clone())),
        turn_service_server::TurnServiceServer::new(TurnServiceImpl::new(db.clone(), ws.clone())),
        agent_service_server::AgentServiceServer::new(AgentServiceImpl::new(db.clone(), ws.clone())),
        lock_service_server::LockServiceServer::new(LockServiceImpl::new(db.clone(), ws.clone())),
        message_service_server::MessageServiceServer::new(MessageServiceImpl::new(db.clone(), ws.clone())),
        delegation_service_server::DelegationServiceServer::new(DelegationServiceImpl::new(db.clone(), ws.clone())),
        handoff_service_server::HandoffServiceServer::new(HandoffServiceImpl::new(db.clone(), ws.clone())),
        dsl_service_server::DslServiceServer::new(DslServiceImpl::new(db.clone())),
        config_service_server::ConfigServiceServer::new(ConfigServiceImpl::new(db.clone(), ws.clone())),
        tenant_service_server::TenantServiceServer::new(TenantServiceImpl::new(db.clone())),
        search_service_server::SearchServiceServer::new(SearchServiceImpl::new(db.clone())),
        event_service_server::EventServiceServer::new(EventServiceImpl::new(ws)),
    )
}
