//! Custom path extractors for type-safe entity IDs.
//!
//! Provides `PathId<T>` extractor that works with EntityIdType newtypes
//! and provides rich error messages.

use axum::{
    async_trait,
    extract::{FromRequestParts, Path},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use caliber_core::EntityIdType;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use uuid::Uuid;

/// Extractor for type-safe entity IDs from path parameters.
///
/// Unlike the standard `Path<Uuid>` extractor, `PathId<T>` provides:
/// - Type-safe extraction into specific ID types (TenantId, TrajectoryId, etc.)
/// - Rich error messages with entity type context
/// - Integration with the EntityIdType trait
///
/// # Example
///
/// ```rust,ignore
/// use caliber_core::TrajectoryId;
///
/// async fn get_trajectory(
///     PathId(trajectory_id): PathId<TrajectoryId>,
/// ) -> ApiResult<impl IntoResponse> {
///     // trajectory_id is TrajectoryId, not Uuid
///     db.get::<TrajectoryResponse>(trajectory_id).await
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct PathId<T: EntityIdType>(pub T);

/// Error returned when PathId extraction fails.
#[derive(Debug)]
pub struct PathIdError {
    pub entity_name: &'static str,
    pub path_param: String,
    pub message: String,
}

impl std::fmt::Display for PathIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invalid {} ID '{}': {}",
            self.entity_name, self.path_param, self.message
        )
    }
}

impl std::error::Error for PathIdError {}

impl IntoResponse for PathIdError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "error": "invalid_path_parameter",
            "message": self.to_string(),
            "entity_type": self.entity_name,
            "path_param": self.path_param,
        });
        (StatusCode::BAD_REQUEST, Json(body)).into_response()
    }
}

#[async_trait]
impl<S, T> FromRequestParts<S> for PathId<T>
where
    S: Send + Sync,
    T: EntityIdType,
{
    type Rejection = PathIdError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extract the raw path parameter as Uuid
        let Path(uuid): Path<Uuid> = Path::from_request_parts(parts, state)
            .await
            .map_err(|e| PathIdError {
                entity_name: T::ENTITY_NAME,
                path_param: parts.uri.path().to_string(),
                message: format!("Failed to extract UUID from path: {}", e),
            })?;

        // Convert to the specific ID type
        Ok(PathId(T::new(uuid)))
    }
}

// Convenience wrapper for multiple path params
// PathIds<(TrajectoryId, ScopeId)> extracts two IDs from path

/// Extractor for multiple type-safe entity IDs from path parameters.
///
/// # Example
///
/// ```rust,ignore
/// // For route: /trajectories/:trajectory_id/scopes/:scope_id
/// async fn get_scope(
///     PathIds((trajectory_id, scope_id)): PathIds<(TrajectoryId, ScopeId)>,
/// ) -> ApiResult<impl IntoResponse> {
///     // ...
/// }
/// ```
#[derive(Debug, Clone)]
pub struct PathIds<T>(pub T);

#[async_trait]
impl<S, T1, T2> FromRequestParts<S> for PathIds<(T1, T2)>
where
    S: Send + Sync,
    T1: EntityIdType,
    T2: EntityIdType,
{
    type Rejection = PathIdError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path((uuid1, uuid2)): Path<(Uuid, Uuid)> = Path::from_request_parts(parts, state)
            .await
            .map_err(|e| PathIdError {
                entity_name: "path",
                path_param: parts.uri.path().to_string(),
                message: format!("Failed to extract UUIDs from path: {}", e),
            })?;

        Ok(PathIds((T1::new(uuid1), T2::new(uuid2))))
    }
}

#[async_trait]
impl<S, T1, T2, T3> FromRequestParts<S> for PathIds<(T1, T2, T3)>
where
    S: Send + Sync,
    T1: EntityIdType,
    T2: EntityIdType,
    T3: EntityIdType,
{
    type Rejection = PathIdError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path((uuid1, uuid2, uuid3)): Path<(Uuid, Uuid, Uuid)> =
            Path::from_request_parts(parts, state)
                .await
                .map_err(|e| PathIdError {
                    entity_name: "path",
                    path_param: parts.uri.path().to_string(),
                    message: format!("Failed to extract UUIDs from path: {}", e),
                })?;

        Ok(PathIds((T1::new(uuid1), T2::new(uuid2), T3::new(uuid3))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use caliber_core::{TrajectoryId, ScopeId};

    #[test]
    fn test_path_id_error_display() {
        let err = PathIdError {
            entity_name: "trajectory",
            path_param: "invalid-uuid".to_string(),
            message: "not a valid UUID".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("trajectory"));
        assert!(display.contains("invalid-uuid"));
    }
}
