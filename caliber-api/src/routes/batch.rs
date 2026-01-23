//! Batch REST API Routes
//!
//! This module implements Axum route handlers for batch operations.
//! Batch endpoints allow multiple CRUD operations in a single request.
//!
//! All operations are processed sequentially and individual results
//! are returned for each operation, allowing partial success.

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use std::sync::Arc;

use crate::{
    auth::{validate_tenant_ownership, AuthContext},
    db::DbClient,
    error::ApiError,
    events::WsEvent,
    middleware::AuthExtractor,
    state::AppState,
    types::{
        ArtifactBatchItem, ArtifactResponse, BatchArtifactRequest, BatchArtifactResponse,
        BatchItemResult, BatchNoteRequest, BatchNoteResponse, BatchOperation,
        BatchTrajectoryRequest, BatchTrajectoryResponse, NoteBatchItem, NoteResponse,
        TrajectoryBatchItem, TrajectoryResponse,
    },
    ws::WsState,
};

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// POST /api/v1/batch/trajectories - Batch trajectory operations
#[utoipa::path(
    post,
    path = "/api/v1/batch/trajectories",
    tag = "Batch",
    request_body = BatchTrajectoryRequest,
    responses(
        (status = 200, description = "Batch operations completed", body = BatchTrajectoryResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn batch_trajectories(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    Json(req): Json<BatchTrajectoryRequest>,
) -> impl IntoResponse {
    let mut results: Vec<BatchItemResult<TrajectoryResponse>> = Vec::with_capacity(req.items.len());
    let mut succeeded = 0i32;
    let mut failed = 0i32;

    for item in req.items {
        let result = process_trajectory_item(&db, &ws, item, &auth).await;

        match &result {
            BatchItemResult::Success { .. } => succeeded += 1,
            BatchItemResult::Error { .. } => {
                failed += 1;
                if req.stop_on_error {
                    results.push(result);
                    break;
                }
            }
        }
        results.push(result);
    }

    let response = BatchTrajectoryResponse {
        results,
        succeeded,
        failed,
    };

    (StatusCode::OK, Json(response))
}

async fn process_trajectory_item(
    db: &DbClient,
    ws: &Arc<WsState>,
    item: TrajectoryBatchItem,
    auth: &AuthContext,
) -> BatchItemResult<TrajectoryResponse> {
    match item.operation {
        BatchOperation::Create => {
            let Some(create_req) = item.create else {
                return BatchItemResult::Error {
                    message: "create data required for create operation".to_string(),
                    code: "MISSING_DATA".to_string(),
                };
            };

            if create_req.name.trim().is_empty() {
                return BatchItemResult::Error {
                    message: "name is required".to_string(),
                    code: "VALIDATION_ERROR".to_string(),
                };
            }

            match db.create::<TrajectoryResponse>(&create_req, auth.tenant_id).await {
                Ok(trajectory) => {
                    ws.broadcast(WsEvent::TrajectoryCreated {
                        trajectory: trajectory.clone(),
                    });
                    BatchItemResult::Success { data: trajectory }
                }
                Err(e) => BatchItemResult::Error {
                    message: e.message.clone(),
                    code: e.code.to_string(),
                },
            }
        }

        BatchOperation::Update => {
            let Some(id) = item.trajectory_id else {
                return BatchItemResult::Error {
                    message: "trajectory_id required for update operation".to_string(),
                    code: "MISSING_ID".to_string(),
                };
            };

            // Validate tenant ownership before update
            match db.get::<TrajectoryResponse>(id, auth.tenant_id).await {
                Ok(Some(existing)) => {
                    if validate_tenant_ownership(auth, existing.tenant_id).is_err() {
                        return BatchItemResult::Error {
                            message: "Access denied: resource belongs to different tenant".to_string(),
                            code: "FORBIDDEN".to_string(),
                        };
                    }
                }
                Ok(None) => {
                    return BatchItemResult::Error {
                        message: format!("trajectory {} not found", id),
                        code: "NOT_FOUND".to_string(),
                    };
                }
                Err(e) => {
                    return BatchItemResult::Error {
                        message: e.message.clone(),
                        code: e.code.to_string(),
                    };
                }
            }

            let Some(update_req) = item.update else {
                return BatchItemResult::Error {
                    message: "update data required for update operation".to_string(),
                    code: "MISSING_DATA".to_string(),
                };
            };

            if update_req.name.is_none()
                && update_req.description.is_none()
                && update_req.status.is_none()
                && update_req.metadata.is_none()
            {
                return BatchItemResult::Error {
                    message: "at least one field must be provided for update".to_string(),
                    code: "VALIDATION_ERROR".to_string(),
                };
            }

            match db.update::<TrajectoryResponse>(id, &update_req, auth.tenant_id).await {
                Ok(trajectory) => {
                    ws.broadcast(WsEvent::TrajectoryUpdated {
                        trajectory: trajectory.clone(),
                    });
                    BatchItemResult::Success { data: trajectory }
                }
                Err(e) => BatchItemResult::Error {
                    message: e.message.clone(),
                    code: e.code.to_string(),
                },
            }
        }

        BatchOperation::Delete => {
            let Some(id) = item.trajectory_id else {
                return BatchItemResult::Error {
                    message: "trajectory_id required for delete operation".to_string(),
                    code: "MISSING_ID".to_string(),
                };
            };

            // First get the trajectory to validate ownership and return in response
            match db.get::<TrajectoryResponse>(id, auth.tenant_id).await {
                Ok(Some(trajectory)) => {
                    // Validate tenant ownership before delete
                    if validate_tenant_ownership(auth, trajectory.tenant_id).is_err() {
                        return BatchItemResult::Error {
                            message: "Access denied: resource belongs to different tenant".to_string(),
                            code: "FORBIDDEN".to_string(),
                        };
                    }

                    match db.delete::<TrajectoryResponse>(id, auth.tenant_id).await {
                        Ok(_) => {
                            ws.broadcast(WsEvent::TrajectoryDeleted { tenant_id: auth.tenant_id, id });
                            BatchItemResult::Success { data: trajectory }
                        }
                        Err(e) => BatchItemResult::Error {
                            message: e.message.clone(),
                            code: e.code.to_string(),
                        },
                    }
                }
                Ok(None) => BatchItemResult::Error {
                    message: format!("trajectory {} not found", id),
                    code: "NOT_FOUND".to_string(),
                },
                Err(e) => BatchItemResult::Error {
                    message: e.message.clone(),
                    code: e.code.to_string(),
                },
            }
        }
    }
}


/// POST /api/v1/batch/artifacts - Batch artifact operations
#[utoipa::path(
    post,
    path = "/api/v1/batch/artifacts",
    tag = "Batch",
    request_body = BatchArtifactRequest,
    responses(
        (status = 200, description = "Batch operations completed", body = BatchArtifactResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn batch_artifacts(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    Json(req): Json<BatchArtifactRequest>,
) -> impl IntoResponse {
    let mut results: Vec<BatchItemResult<ArtifactResponse>> = Vec::with_capacity(req.items.len());
    let mut succeeded = 0i32;
    let mut failed = 0i32;

    for item in req.items {
        let result = process_artifact_item(&db, &ws, item, &auth).await;

        match &result {
            BatchItemResult::Success { .. } => succeeded += 1,
            BatchItemResult::Error { .. } => {
                failed += 1;
                if req.stop_on_error {
                    results.push(result);
                    break;
                }
            }
        }
        results.push(result);
    }

    let response = BatchArtifactResponse {
        results,
        succeeded,
        failed,
    };

    (StatusCode::OK, Json(response))
}

async fn process_artifact_item(
    db: &DbClient,
    ws: &Arc<WsState>,
    item: ArtifactBatchItem,
    auth: &AuthContext,
) -> BatchItemResult<ArtifactResponse> {
    match item.operation {
        BatchOperation::Create => {
            let Some(create_req) = item.create else {
                return BatchItemResult::Error {
                    message: "create data required for create operation".to_string(),
                    code: "MISSING_DATA".to_string(),
                };
            };

            if create_req.name.trim().is_empty() {
                return BatchItemResult::Error {
                    message: "name is required".to_string(),
                    code: "VALIDATION_ERROR".to_string(),
                };
            }

            if create_req.content.trim().is_empty() {
                return BatchItemResult::Error {
                    message: "content is required".to_string(),
                    code: "VALIDATION_ERROR".to_string(),
                };
            }

            if create_req.source_turn < 0 {
                return BatchItemResult::Error {
                    message: "source_turn must be non-negative".to_string(),
                    code: "VALIDATION_ERROR".to_string(),
                };
            }

            if let Some(confidence) = create_req.confidence {
                if !(0.0..=1.0).contains(&confidence) {
                    return BatchItemResult::Error {
                        message: "confidence must be between 0.0 and 1.0".to_string(),
                        code: "VALIDATION_ERROR".to_string(),
                    };
                }
            }

            match db.create::<ArtifactResponse>(&create_req, auth.tenant_id).await {
                Ok(artifact) => {
                    ws.broadcast(WsEvent::ArtifactCreated {
                        artifact: artifact.clone(),
                    });
                    BatchItemResult::Success { data: artifact }
                }
                Err(e) => BatchItemResult::Error {
                    message: e.message.clone(),
                    code: e.code.to_string(),
                },
            }
        }

        BatchOperation::Update => {
            let Some(id) = item.artifact_id else {
                return BatchItemResult::Error {
                    message: "artifact_id required for update operation".to_string(),
                    code: "MISSING_ID".to_string(),
                };
            };

            // Validate tenant ownership before update
            match db.get::<ArtifactResponse>(id, auth.tenant_id).await {
                Ok(Some(existing)) => {
                    if validate_tenant_ownership(auth, existing.tenant_id).is_err() {
                        return BatchItemResult::Error {
                            message: "Access denied: resource belongs to different tenant".to_string(),
                            code: "FORBIDDEN".to_string(),
                        };
                    }
                }
                Ok(None) => {
                    return BatchItemResult::Error {
                        message: format!("artifact {} not found", id),
                        code: "NOT_FOUND".to_string(),
                    };
                }
                Err(e) => {
                    return BatchItemResult::Error {
                        message: e.message.clone(),
                        code: e.code.to_string(),
                    };
                }
            }

            let Some(update_req) = item.update else {
                return BatchItemResult::Error {
                    message: "update data required for update operation".to_string(),
                    code: "MISSING_DATA".to_string(),
                };
            };

            if update_req.name.is_none()
                && update_req.content.is_none()
                && update_req.artifact_type.is_none()
                && update_req.ttl.is_none()
                && update_req.metadata.is_none()
            {
                return BatchItemResult::Error {
                    message: "at least one field must be provided for update".to_string(),
                    code: "VALIDATION_ERROR".to_string(),
                };
            }

            match db.update::<ArtifactResponse>(id, &update_req, auth.tenant_id).await {
                Ok(artifact) => {
                    ws.broadcast(WsEvent::ArtifactUpdated {
                        artifact: artifact.clone(),
                    });
                    BatchItemResult::Success { data: artifact }
                }
                Err(e) => BatchItemResult::Error {
                    message: e.message.clone(),
                    code: e.code.to_string(),
                },
            }
        }

        BatchOperation::Delete => {
            let Some(id) = item.artifact_id else {
                return BatchItemResult::Error {
                    message: "artifact_id required for delete operation".to_string(),
                    code: "MISSING_ID".to_string(),
                };
            };

            match db.get::<ArtifactResponse>(id, auth.tenant_id).await {
                Ok(Some(artifact)) => {
                    // Validate tenant ownership before delete
                    if validate_tenant_ownership(auth, artifact.tenant_id).is_err() {
                        return BatchItemResult::Error {
                            message: "Access denied: resource belongs to different tenant".to_string(),
                            code: "FORBIDDEN".to_string(),
                        };
                    }

                    match db.delete::<ArtifactResponse>(id, auth.tenant_id).await {
                        Ok(_) => {
                            ws.broadcast(WsEvent::ArtifactDeleted { tenant_id: auth.tenant_id, id });
                            BatchItemResult::Success { data: artifact }
                        }
                        Err(e) => BatchItemResult::Error {
                            message: e.message.clone(),
                            code: e.code.to_string(),
                        },
                    }
                }
                Ok(None) => BatchItemResult::Error {
                    message: format!("artifact {} not found", id),
                    code: "NOT_FOUND".to_string(),
                },
                Err(e) => BatchItemResult::Error {
                    message: e.message.clone(),
                    code: e.code.to_string(),
                },
            }
        }
    }
}

/// POST /api/v1/batch/notes - Batch note operations
#[utoipa::path(
    post,
    path = "/api/v1/batch/notes",
    tag = "Batch",
    request_body = BatchNoteRequest,
    responses(
        (status = 200, description = "Batch operations completed", body = BatchNoteResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn batch_notes(
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    Json(req): Json<BatchNoteRequest>,
) -> impl IntoResponse {
    let mut results: Vec<BatchItemResult<NoteResponse>> = Vec::with_capacity(req.items.len());
    let mut succeeded = 0i32;
    let mut failed = 0i32;

    for item in req.items {
        let result = process_note_item(&db, &ws, item, &auth).await;

        match &result {
            BatchItemResult::Success { .. } => succeeded += 1,
            BatchItemResult::Error { .. } => {
                failed += 1;
                if req.stop_on_error {
                    results.push(result);
                    break;
                }
            }
        }
        results.push(result);
    }

    let response = BatchNoteResponse {
        results,
        succeeded,
        failed,
    };

    (StatusCode::OK, Json(response))
}

async fn process_note_item(
    db: &DbClient,
    ws: &Arc<WsState>,
    item: NoteBatchItem,
    auth: &AuthContext,
) -> BatchItemResult<NoteResponse> {
    match item.operation {
        BatchOperation::Create => {
            let Some(create_req) = item.create else {
                return BatchItemResult::Error {
                    message: "create data required for create operation".to_string(),
                    code: "MISSING_DATA".to_string(),
                };
            };

            if create_req.title.trim().is_empty() {
                return BatchItemResult::Error {
                    message: "title is required".to_string(),
                    code: "VALIDATION_ERROR".to_string(),
                };
            }

            if create_req.content.trim().is_empty() {
                return BatchItemResult::Error {
                    message: "content is required".to_string(),
                    code: "VALIDATION_ERROR".to_string(),
                };
            }

            match db.create::<NoteResponse>(&create_req, auth.tenant_id).await {
                Ok(note) => {
                    ws.broadcast(WsEvent::NoteCreated {
                        note: note.clone(),
                    });
                    BatchItemResult::Success { data: note }
                }
                Err(e) => BatchItemResult::Error {
                    message: e.message.clone(),
                    code: e.code.to_string(),
                },
            }
        }

        BatchOperation::Update => {
            let Some(id) = item.note_id else {
                return BatchItemResult::Error {
                    message: "note_id required for update operation".to_string(),
                    code: "MISSING_ID".to_string(),
                };
            };

            // Validate tenant ownership before update
            match db.get::<NoteResponse>(id, auth.tenant_id).await {
                Ok(Some(existing)) => {
                    if validate_tenant_ownership(auth, existing.tenant_id).is_err() {
                        return BatchItemResult::Error {
                            message: "Access denied: resource belongs to different tenant".to_string(),
                            code: "FORBIDDEN".to_string(),
                        };
                    }
                }
                Ok(None) => {
                    return BatchItemResult::Error {
                        message: format!("note {} not found", id),
                        code: "NOT_FOUND".to_string(),
                    };
                }
                Err(e) => {
                    return BatchItemResult::Error {
                        message: e.message.clone(),
                        code: e.code.to_string(),
                    };
                }
            }

            let Some(update_req) = item.update else {
                return BatchItemResult::Error {
                    message: "update data required for update operation".to_string(),
                    code: "MISSING_DATA".to_string(),
                };
            };

            if update_req.title.is_none()
                && update_req.content.is_none()
                && update_req.note_type.is_none()
                && update_req.ttl.is_none()
                && update_req.metadata.is_none()
            {
                return BatchItemResult::Error {
                    message: "at least one field must be provided for update".to_string(),
                    code: "VALIDATION_ERROR".to_string(),
                };
            }

            match db.update::<NoteResponse>(id, &update_req, auth.tenant_id).await {
                Ok(note) => {
                    ws.broadcast(WsEvent::NoteUpdated {
                        note: note.clone(),
                    });
                    BatchItemResult::Success { data: note }
                }
                Err(e) => BatchItemResult::Error {
                    message: e.message.clone(),
                    code: e.code.to_string(),
                },
            }
        }

        BatchOperation::Delete => {
            let Some(id) = item.note_id else {
                return BatchItemResult::Error {
                    message: "note_id required for delete operation".to_string(),
                    code: "MISSING_ID".to_string(),
                };
            };

            match db.get::<NoteResponse>(id, auth.tenant_id).await {
                Ok(Some(note)) => {
                    // Validate tenant ownership before delete
                    if validate_tenant_ownership(auth, note.tenant_id).is_err() {
                        return BatchItemResult::Error {
                            message: "Access denied: resource belongs to different tenant".to_string(),
                            code: "FORBIDDEN".to_string(),
                        };
                    }

                    match db.delete::<NoteResponse>(id, auth.tenant_id).await {
                        Ok(_) => {
                            ws.broadcast(WsEvent::NoteDeleted { tenant_id: auth.tenant_id, id });
                            BatchItemResult::Success { data: note }
                        }
                        Err(e) => BatchItemResult::Error {
                            message: e.message.clone(),
                            code: e.code.to_string(),
                        },
                    }
                }
                Ok(None) => BatchItemResult::Error {
                    message: format!("note {} not found", id),
                    code: "NOT_FOUND".to_string(),
                },
                Err(e) => BatchItemResult::Error {
                    message: e.message.clone(),
                    code: e.code.to_string(),
                },
            }
        }
    }
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the batch routes router.
pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/trajectories", post(batch_trajectories))
        .route("/artifacts", post(batch_artifacts))
        .route("/notes", post(batch_notes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CreateTrajectoryRequest;

    #[test]
    fn test_batch_operation_variants() {
        let ops = [
            BatchOperation::Create,
            BatchOperation::Update,
            BatchOperation::Delete,
        ];
        assert_eq!(ops.len(), 3);
    }

    #[test]
    fn test_trajectory_batch_item_create() {
        let item = TrajectoryBatchItem {
            operation: BatchOperation::Create,
            trajectory_id: None,
            create: Some(CreateTrajectoryRequest {
                name: "Test Trajectory".to_string(),
                description: Some("A test".to_string()),
                parent_trajectory_id: None,
                agent_id: None,
                metadata: None,
            }),
            update: None,
        };

        assert!(item.create.is_some());
        assert!(item.trajectory_id.is_none());
    }

    #[test]
    fn test_batch_request_serialization() -> Result<(), serde_json::Error> {
        let request = BatchTrajectoryRequest {
            items: vec![],
            stop_on_error: true,
        };

        let json = serde_json::to_string(&request)?;
        assert!(json.contains("\"stop_on_error\":true"));
        Ok(())
    }

    #[test]
    fn test_batch_item_result_success() -> Result<(), serde_json::Error> {
        let result: BatchItemResult<String> = BatchItemResult::Success {
            data: "test".to_string(),
        };

        let json = serde_json::to_string(&result)?;
        assert!(json.contains("\"status\":\"success\""));
        assert!(json.contains("\"data\":\"test\""));
        Ok(())
    }

    #[test]
    fn test_batch_item_result_error() -> Result<(), serde_json::Error> {
        let result: BatchItemResult<String> = BatchItemResult::Error {
            message: "Something went wrong".to_string(),
            code: "ERR_001".to_string(),
        };

        let json = serde_json::to_string(&result)?;
        assert!(json.contains("\"status\":\"error\""));
        assert!(json.contains("\"message\":\"Something went wrong\""));
        assert!(json.contains("\"code\":\"ERR_001\""));
        Ok(())
    }

    #[test]
    fn test_batch_response_counters() {
        let response = BatchTrajectoryResponse {
            results: vec![],
            succeeded: 5,
            failed: 2,
        };

        assert_eq!(response.succeeded, 5);
        assert_eq!(response.failed, 2);
    }
}
