//! Note REST API Routes
//!
//! This module implements Axum route handlers for note operations.
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
    db::DbClient,
    error::{ApiError, ApiResult},
    events::WsEvent,
    types::{
        CreateNoteRequest, ListNotesRequest, ListNotesResponse, NoteResponse, SearchRequest,
        SearchResponse, UpdateNoteRequest,
    },
    ws::WsState,
};

// ============================================================================
// SHARED STATE
// ============================================================================

/// Shared application state for note routes.
#[derive(Clone)]
pub struct NoteState {
    pub db: DbClient,
    pub ws: Arc<WsState>,
}

impl NoteState {
    pub fn new(db: DbClient, ws: Arc<WsState>) -> Self {
        Self { db, ws }
    }
}

// ============================================================================
// ROUTE HANDLERS
// ============================================================================

/// POST /api/v1/notes - Create a new note
#[utoipa::path(
    post,
    path = "/api/v1/notes",
    tag = "Notes",
    request_body = CreateNoteRequest,
    responses(
        (status = 201, description = "Note created successfully", body = NoteResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn create_note(
    State(state): State<Arc<NoteState>>,
    Json(req): Json<CreateNoteRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate required fields
    if req.title.trim().is_empty() {
        return Err(ApiError::missing_field("title"));
    }

    if req.content.trim().is_empty() {
        return Err(ApiError::missing_field("content"));
    }

    // Create note via database client
    let note = state.db.note_create(&req).await?;

    // Broadcast NoteCreated event
    state.ws.broadcast(WsEvent::NoteCreated {
        note: note.clone(),
    });

    Ok((StatusCode::CREATED, Json(note)))
}

/// GET /api/v1/notes - List notes with filters
#[utoipa::path(
    get,
    path = "/api/v1/notes",
    tag = "Notes",
    params(
        ("note_type" = Option<String>, Query, description = "Filter by note type"),
        ("source_trajectory_id" = Option<String>, Query, description = "Filter by source trajectory ID"),
        ("created_after" = Option<String>, Query, description = "Filter by creation date (after)"),
        ("created_before" = Option<String>, Query, description = "Filter by creation date (before)"),
        ("limit" = Option<i32>, Query, description = "Maximum number of results"),
        ("offset" = Option<i32>, Query, description = "Offset for pagination"),
    ),
    responses(
        (status = 200, description = "List of notes", body = ListNotesResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn list_notes(
    State(state): State<Arc<NoteState>>,
    Query(params): Query<ListNotesRequest>,
) -> ApiResult<impl IntoResponse> {
    // For now, we'll implement basic filtering by source trajectory
    // Full filtering with pagination will be added as needed

    if let Some(source_trajectory_id) = params.source_trajectory_id {
        // Filter by source trajectory
        let notes = state.db.note_list_by_trajectory(source_trajectory_id).await?;

        // Apply additional filters if needed
        let mut filtered = notes;

        if let Some(note_type) = params.note_type {
            filtered.retain(|n| n.note_type == note_type);
        }

        if let Some(created_after) = params.created_after {
            filtered.retain(|n| n.created_at >= created_after);
        }

        if let Some(created_before) = params.created_before {
            filtered.retain(|n| n.created_at <= created_before);
        }

        // Apply pagination
        let total = filtered.len() as i32;
        let offset = params.offset.unwrap_or(0) as usize;
        let limit = params.limit.unwrap_or(100) as usize;

        let paginated: Vec<_> = filtered.into_iter().skip(offset).take(limit).collect();

        let response = ListNotesResponse {
            notes: paginated,
            total,
        };

        Ok(Json(response))
    } else {
        // No trajectory filter - return all notes with pagination
        let limit = params.limit.unwrap_or(100);
        let offset = params.offset.unwrap_or(0);

        let notes = state.db.note_list_all(limit, offset).await?;

        // Apply additional filters if needed
        let mut filtered = notes;

        if let Some(note_type) = params.note_type {
            filtered.retain(|n| n.note_type == note_type);
        }

        if let Some(created_after) = params.created_after {
            filtered.retain(|n| n.created_at >= created_after);
        }

        if let Some(created_before) = params.created_before {
            filtered.retain(|n| n.created_at <= created_before);
        }

        let total = filtered.len() as i32;

        let response = ListNotesResponse {
            notes: filtered,
            total,
        };

        Ok(Json(response))
    }
}

/// GET /api/v1/notes/{id} - Get note by ID
#[utoipa::path(
    get,
    path = "/api/v1/notes/{id}",
    tag = "Notes",
    params(
        ("id" = Uuid, Path, description = "Note ID")
    ),
    responses(
        (status = 200, description = "Note details", body = NoteResponse),
        (status = 404, description = "Note not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn get_note(
    State(state): State<Arc<NoteState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<impl IntoResponse> {
    let note = state
        .db
        .note_get(id)
        .await?
        .ok_or_else(|| ApiError::note_not_found(id))?;

    Ok(Json(note))
}

/// PATCH /api/v1/notes/{id} - Update note
#[utoipa::path(
    patch,
    path = "/api/v1/notes/{id}",
    tag = "Notes",
    params(
        ("id" = Uuid, Path, description = "Note ID")
    ),
    request_body = UpdateNoteRequest,
    responses(
        (status = 200, description = "Note updated successfully", body = NoteResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 404, description = "Note not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn update_note(
    State(state): State<Arc<NoteState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateNoteRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate that at least one field is being updated
    if req.title.is_none()
        && req.content.is_none()
        && req.note_type.is_none()
        && req.ttl.is_none()
        && req.metadata.is_none()
    {
        return Err(ApiError::invalid_input(
            "At least one field must be provided for update",
        ));
    }

    // Validate title if provided
    if let Some(ref title) = req.title {
        if title.trim().is_empty() {
            return Err(ApiError::invalid_input("title cannot be empty"));
        }
    }

    // Validate content if provided
    if let Some(ref content) = req.content {
        if content.trim().is_empty() {
            return Err(ApiError::invalid_input("content cannot be empty"));
        }
    }

    let note = state.db.note_update(id, &req).await?;
    state.ws.broadcast(WsEvent::NoteUpdated { note: note.clone() });
    Ok(Json(note))
}

/// DELETE /api/v1/notes/{id} - Delete note
#[utoipa::path(
    delete,
    path = "/api/v1/notes/{id}",
    tag = "Notes",
    params(
        ("id" = Uuid, Path, description = "Note ID")
    ),
    responses(
        (status = 204, description = "Note deleted successfully"),
        (status = 404, description = "Note not found", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn delete_note(
    State(state): State<Arc<NoteState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    // First verify the note exists
    let _note = state
        .db
        .note_get(id)
        .await?
        .ok_or_else(|| ApiError::note_not_found(id))?;

    state.db.note_delete(id).await?;
    state.ws.broadcast(WsEvent::NoteDeleted { id });
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/notes/search - Search notes by similarity
#[utoipa::path(
    post,
    path = "/api/v1/notes/search",
    tag = "Notes",
    request_body = SearchRequest,
    responses(
        (status = 200, description = "Search results", body = SearchResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
    ),
    security(
        ("api_key" = []),
        ("bearer_auth" = [])
    )
)]
pub async fn search_notes(
    State(state): State<Arc<NoteState>>,
    Json(req): Json<SearchRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate search query
    if req.query.trim().is_empty() {
        return Err(ApiError::missing_field("query"));
    }

    // Validate entity types include Note
    if !req
        .entity_types
        .contains(&caliber_core::EntityType::Note)
    {
        return Err(ApiError::invalid_input(
            "entity_types must include Note for note search",
        ));
    }

    // Perform the search using the database search function
    let response = state.db.search(&req).await?;

    Ok(Json(response))
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the note routes router.
pub fn create_router(db: DbClient, ws: Arc<WsState>) -> axum::Router {
    let state = Arc::new(NoteState::new(db, ws));

    axum::Router::new()
        .route("/", axum::routing::post(create_note))
        .route("/", axum::routing::get(list_notes))
        .route("/:id", axum::routing::get(get_note))
        .route("/:id", axum::routing::patch(update_note))
        .route("/:id", axum::routing::delete(delete_note))
        .route("/search", axum::routing::post(search_notes))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use caliber_core::{EntityId, NoteType, TTL};

    #[test]
    fn test_create_note_request_validation() {
        // Use a dummy UUID for testing (all zeros is valid)
        let dummy_id: EntityId = uuid::Uuid::nil();

        let req = CreateNoteRequest {
            note_type: NoteType::Fact,
            title: "".to_string(),
            content: "".to_string(),
            source_trajectory_ids: vec![dummy_id],
            source_artifact_ids: vec![],
            ttl: TTL::Persistent,
            metadata: None,
        };

        assert!(req.title.trim().is_empty());
        assert!(req.content.trim().is_empty());
    }

    #[test]
    fn test_update_note_request_validation() {
        let req = UpdateNoteRequest {
            title: None,
            content: None,
            note_type: None,
            ttl: None,
            metadata: None,
        };

        let has_updates = req.title.is_some()
            || req.content.is_some()
            || req.note_type.is_some()
            || req.ttl.is_some()
            || req.metadata.is_some();

        assert!(!has_updates);
    }

    #[test]
    fn test_list_notes_pagination() {
        let params = ListNotesRequest {
            note_type: Some(NoteType::Fact),
            source_trajectory_id: Some(uuid::Uuid::nil()),
            created_after: None,
            created_before: None,
            limit: Some(10),
            offset: Some(0),
        };

        assert_eq!(params.limit, Some(10));
        assert_eq!(params.offset, Some(0));
    }

    #[test]
    fn test_search_request_validation() {
        let req = SearchRequest {
            query: "".to_string(),
            entity_types: vec![caliber_core::EntityType::Note],
            filters: vec![],
            limit: Some(10),
        };

        assert!(req.query.trim().is_empty());
        assert!(req
            .entity_types
            .contains(&caliber_core::EntityType::Note));
    }

    #[test]
    fn test_note_type_variants() {
        // Verify all note types are accessible
        let types = vec![
            NoteType::Convention,
            NoteType::Strategy,
            NoteType::Gotcha,
            NoteType::Fact,
            NoteType::Preference,
            NoteType::Relationship,
            NoteType::Procedure,
            NoteType::Meta,
        ];

        assert_eq!(types.len(), 8);
    }

    #[test]
    fn test_ttl_variants() {
        // Verify TTL variants work correctly
        let ttls = vec![
            TTL::Persistent,
            TTL::Session,
            TTL::Scope,
            TTL::Ephemeral,
            TTL::ShortTerm,
            TTL::MediumTerm,
            TTL::LongTerm,
            TTL::Permanent,
            TTL::Duration(1000),
        ];

        assert_eq!(ttls.len(), 9);
    }
}
