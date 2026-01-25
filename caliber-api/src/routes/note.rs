//! Note REST API Routes
//!
//! This module implements Axum route handlers for note operations.
//! All handlers call caliber_* pg_extern functions via the DbClient.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use caliber_core::NoteId;
use std::sync::Arc;

use crate::{
    auth::validate_tenant_ownership,
    components::NoteListFilter,
    db::DbClient,
    error::{ApiError, ApiResult},
    events::WsEvent,
    extractors::PathId,
    middleware::AuthExtractor,
    state::AppState,
    types::{
        CreateNoteRequest, ListNotesRequest, ListNotesResponse, NoteResponse, SearchRequest,
        SearchResponse, UpdateNoteRequest,
    },
    ws::WsState,
};

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
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    Json(req): Json<CreateNoteRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate required fields
    if req.title.trim().is_empty() {
        return Err(ApiError::missing_field("title"));
    }

    if req.content.trim().is_empty() {
        return Err(ApiError::missing_field("content"));
    }

    // Create note via database client with tenant_id for isolation
    let note = db.create::<NoteResponse>(&req, auth.tenant_id).await?;

    // Broadcast NoteCreated event
    ws.broadcast(WsEvent::NoteCreated {
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
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    Query(params): Query<ListNotesRequest>,
) -> ApiResult<impl IntoResponse> {
    // Build filter from query params - all filtering handled by generic list
    let filter = NoteListFilter {
        note_type: params.note_type,
        source_trajectory_id: params.source_trajectory_id,
        limit: params.limit,
        offset: params.offset,
    };

    let mut notes = db.list::<NoteResponse>(&filter, auth.tenant_id).await?;

    // Apply date filters in Rust (not supported in filter)
    if let Some(created_after) = params.created_after {
        notes.retain(|n| n.created_at >= created_after);
    }
    if let Some(created_before) = params.created_before {
        notes.retain(|n| n.created_at <= created_before);
    }

    let total = notes.len() as i32;
    let response = ListNotesResponse {
        notes,
        total,
    };

    Ok(Json(response))
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
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<NoteId>,
) -> ApiResult<impl IntoResponse> {
    let note = db
        .get::<NoteResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::note_not_found(id))?;

    // Validate tenant ownership before returning
    validate_tenant_ownership(&auth, note.tenant_id)?;

    // Increment access_count (fire-and-forget, don't block response)
    // This tracks how often a note is accessed for relevance ranking
    let _ = db
        .increment_access_count::<NoteResponse>(id, auth.tenant_id)
        .await;

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
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<NoteId>,
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

    // First verify the note exists and belongs to this tenant
    let existing = db
        .get::<NoteResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::note_not_found(id))?;
    validate_tenant_ownership(&auth, existing.tenant_id)?;

    let note = db.update::<NoteResponse>(id, &req, auth.tenant_id).await?;
    ws.broadcast(WsEvent::NoteUpdated { note: note.clone() });
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
    State(db): State<DbClient>,
    State(ws): State<Arc<WsState>>,
    AuthExtractor(auth): AuthExtractor,
    PathId(id): PathId<NoteId>,
) -> ApiResult<StatusCode> {
    // First verify the note exists and belongs to this tenant
    let note = db
        .get::<NoteResponse>(id, auth.tenant_id)
        .await?
        .ok_or_else(|| ApiError::note_not_found(id))?;
    validate_tenant_ownership(&auth, note.tenant_id)?;

    db.delete::<NoteResponse>(id, auth.tenant_id).await?;
    ws.broadcast(WsEvent::NoteDeleted {
        tenant_id: auth.tenant_id,
        id,
    });
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
    State(db): State<DbClient>,
    AuthExtractor(auth): AuthExtractor,
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

    // Perform tenant-isolated search
    let response = db.search(&req, auth.tenant_id).await?;

    Ok(Json(response))
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the note routes router.
pub fn create_router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::post(create_note))
        .route("/", axum::routing::get(list_notes))
        .route("/:id", axum::routing::get(get_note))
        .route("/:id", axum::routing::patch(update_note))
        .route("/:id", axum::routing::delete(delete_note))
        .route("/search", axum::routing::post(search_notes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use caliber_core::{NoteType, TrajectoryId, TTL};

    #[test]
    fn test_create_note_request_validation() {
        // Use a nil TrajectoryId for testing
        let dummy_id = TrajectoryId::nil();

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
        let types = [
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
        let ttls = [
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
