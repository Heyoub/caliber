//! GraphQL API Routes
//!
//! This module implements a GraphQL endpoint using async-graphql.
//! It provides Query, Mutation, and Subscription resolvers for
//! all CALIBER entities.
//!
//! Endpoints:
//! - POST /api/v1/graphql - Execute GraphQL queries/mutations
//! - GET /api/v1/graphql/playground - GraphiQL playground

use async_graphql::{
    Context, EmptySubscription, Enum, InputObject, Object, Result as GqlResult, Schema,
    SimpleObject, ID,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use caliber_core::{AgentId, ArtifactId, EntityIdType, NoteId, ScopeId, TrajectoryId};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    auth::AuthContext,
    components::{ArtifactListFilter, NoteListFilter, ScopeListFilter, TrajectoryListFilter},
    db::DbClient,
    events::WsEvent,
    middleware::AuthExtractor,
    state::AppState,
    types::*,
    ws::WsState,
};

// ============================================================================
// GRAPHQL TYPES
// ============================================================================

/// GraphQL representation of TrajectoryStatus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum GqlTrajectoryStatus {
    Active,
    Completed,
    Failed,
    Suspended,
}

impl From<caliber_core::TrajectoryStatus> for GqlTrajectoryStatus {
    fn from(status: caliber_core::TrajectoryStatus) -> Self {
        match status {
            caliber_core::TrajectoryStatus::Active => GqlTrajectoryStatus::Active,
            caliber_core::TrajectoryStatus::Completed => GqlTrajectoryStatus::Completed,
            caliber_core::TrajectoryStatus::Failed => GqlTrajectoryStatus::Failed,
            caliber_core::TrajectoryStatus::Suspended => GqlTrajectoryStatus::Suspended,
        }
    }
}

impl From<GqlTrajectoryStatus> for caliber_core::TrajectoryStatus {
    fn from(status: GqlTrajectoryStatus) -> Self {
        match status {
            GqlTrajectoryStatus::Active => caliber_core::TrajectoryStatus::Active,
            GqlTrajectoryStatus::Completed => caliber_core::TrajectoryStatus::Completed,
            GqlTrajectoryStatus::Failed => caliber_core::TrajectoryStatus::Failed,
            GqlTrajectoryStatus::Suspended => caliber_core::TrajectoryStatus::Suspended,
        }
    }
}

/// GraphQL representation of NoteType.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum GqlNoteType {
    Convention,
    Strategy,
    Gotcha,
    Fact,
    Preference,
    Relationship,
    Procedure,
    Meta,
    Insight,
    Correction,
    Summary,
}

impl From<caliber_core::NoteType> for GqlNoteType {
    fn from(nt: caliber_core::NoteType) -> Self {
        match nt {
            caliber_core::NoteType::Convention => GqlNoteType::Convention,
            caliber_core::NoteType::Strategy => GqlNoteType::Strategy,
            caliber_core::NoteType::Gotcha => GqlNoteType::Gotcha,
            caliber_core::NoteType::Fact => GqlNoteType::Fact,
            caliber_core::NoteType::Preference => GqlNoteType::Preference,
            caliber_core::NoteType::Relationship => GqlNoteType::Relationship,
            caliber_core::NoteType::Procedure => GqlNoteType::Procedure,
            caliber_core::NoteType::Meta => GqlNoteType::Meta,
            caliber_core::NoteType::Insight => GqlNoteType::Insight,
            caliber_core::NoteType::Correction => GqlNoteType::Correction,
            caliber_core::NoteType::Summary => GqlNoteType::Summary,
        }
    }
}

impl From<GqlNoteType> for caliber_core::NoteType {
    fn from(nt: GqlNoteType) -> Self {
        match nt {
            GqlNoteType::Convention => caliber_core::NoteType::Convention,
            GqlNoteType::Strategy => caliber_core::NoteType::Strategy,
            GqlNoteType::Gotcha => caliber_core::NoteType::Gotcha,
            GqlNoteType::Fact => caliber_core::NoteType::Fact,
            GqlNoteType::Preference => caliber_core::NoteType::Preference,
            GqlNoteType::Relationship => caliber_core::NoteType::Relationship,
            GqlNoteType::Procedure => caliber_core::NoteType::Procedure,
            GqlNoteType::Meta => caliber_core::NoteType::Meta,
            GqlNoteType::Insight => caliber_core::NoteType::Insight,
            GqlNoteType::Correction => caliber_core::NoteType::Correction,
            GqlNoteType::Summary => caliber_core::NoteType::Summary,
        }
    }
}

/// GraphQL trajectory object.
#[derive(Debug, Clone, SimpleObject)]
pub struct GqlTrajectory {
    pub id: ID,
    pub name: String,
    pub description: Option<String>,
    pub status: GqlTrajectoryStatus,
    pub parent_trajectory_id: Option<ID>,
    pub root_trajectory_id: Option<ID>,
    pub agent_id: Option<ID>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub metadata: Option<String>,
}

impl From<TrajectoryResponse> for GqlTrajectory {
    fn from(t: TrajectoryResponse) -> Self {
        Self {
            id: ID(t.trajectory_id.to_string()),
            name: t.name,
            description: t.description,
            status: t.status.into(),
            parent_trajectory_id: t.parent_trajectory_id.map(|id| ID(id.to_string())),
            root_trajectory_id: t.root_trajectory_id.map(|id| ID(id.to_string())),
            agent_id: t.agent_id.map(|id| ID(id.to_string())),
            created_at: t.created_at.to_rfc3339(),
            updated_at: t.updated_at.to_rfc3339(),
            completed_at: t.completed_at.map(|ts| ts.to_rfc3339()),
            metadata: t.metadata.map(|m| m.to_string()),
        }
    }
}

/// GraphQL scope object.
#[derive(Debug, Clone, SimpleObject)]
pub struct GqlScope {
    pub id: ID,
    pub trajectory_id: ID,
    pub parent_scope_id: Option<ID>,
    pub name: String,
    pub purpose: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub closed_at: Option<String>,
    pub token_budget: i32,
    pub tokens_used: i32,
    pub metadata: Option<String>,
}

impl From<ScopeResponse> for GqlScope {
    fn from(s: ScopeResponse) -> Self {
        Self {
            id: ID(s.scope_id.to_string()),
            trajectory_id: ID(s.trajectory_id.to_string()),
            parent_scope_id: s.parent_scope_id.map(|id| ID(id.to_string())),
            name: s.name,
            purpose: s.purpose,
            is_active: s.is_active,
            created_at: s.created_at.to_rfc3339(),
            closed_at: s.closed_at.map(|ts| ts.to_rfc3339()),
            token_budget: s.token_budget,
            tokens_used: s.tokens_used,
            metadata: s.metadata.map(|m| m.to_string()),
        }
    }
}

/// GraphQL artifact object.
#[derive(Debug, Clone, SimpleObject)]
pub struct GqlArtifact {
    pub id: ID,
    pub trajectory_id: ID,
    pub scope_id: ID,
    pub artifact_type: String,
    pub name: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata: Option<String>,
}

impl From<ArtifactResponse> for GqlArtifact {
    fn from(a: ArtifactResponse) -> Self {
        Self {
            id: ID(a.artifact_id.to_string()),
            trajectory_id: ID(a.trajectory_id.to_string()),
            scope_id: ID(a.scope_id.to_string()),
            artifact_type: format!("{:?}", a.artifact_type),
            name: a.name,
            content: a.content,
            created_at: a.created_at.to_rfc3339(),
            updated_at: a.updated_at.to_rfc3339(),
            metadata: a.metadata.map(|m| m.to_string()),
        }
    }
}

/// GraphQL note object.
#[derive(Debug, Clone, SimpleObject)]
pub struct GqlNote {
    pub id: ID,
    pub note_type: GqlNoteType,
    pub title: String,
    pub content: String,
    pub source_trajectory_ids: Vec<ID>,
    pub source_artifact_ids: Vec<ID>,
    pub created_at: String,
    pub updated_at: String,
    pub accessed_at: String,
    pub access_count: i32,
    pub metadata: Option<String>,
}

impl From<NoteResponse> for GqlNote {
    fn from(n: NoteResponse) -> Self {
        Self {
            id: ID(n.note_id.to_string()),
            note_type: n.note_type.into(),
            title: n.title,
            content: n.content,
            source_trajectory_ids: n
                .source_trajectory_ids
                .into_iter()
                .map(|id| ID(id.to_string()))
                .collect(),
            source_artifact_ids: n
                .source_artifact_ids
                .into_iter()
                .map(|id| ID(id.to_string()))
                .collect(),
            created_at: n.created_at.to_rfc3339(),
            updated_at: n.updated_at.to_rfc3339(),
            accessed_at: n.accessed_at.to_rfc3339(),
            access_count: n.access_count,
            metadata: n.metadata.map(|m| m.to_string()),
        }
    }
}

/// GraphQL agent object.
#[derive(Debug, Clone, SimpleObject)]
pub struct GqlAgent {
    pub id: ID,
    pub agent_type: String,
    pub capabilities: Vec<String>,
    pub status: String,
    pub current_trajectory_id: Option<ID>,
    pub current_scope_id: Option<ID>,
    pub can_delegate_to: Vec<String>,
    pub reports_to: Option<ID>,
    pub created_at: String,
    pub last_heartbeat: String,
}

impl From<AgentResponse> for GqlAgent {
    fn from(a: AgentResponse) -> Self {
        Self {
            id: ID(a.agent_id.to_string()),
            agent_type: a.agent_type,
            capabilities: a.capabilities,
            status: a.status.to_string(),
            current_trajectory_id: a.current_trajectory_id.map(|id| ID(id.to_string())),
            current_scope_id: a.current_scope_id.map(|id| ID(id.to_string())),
            can_delegate_to: a.can_delegate_to,
            reports_to: a.reports_to.map(|id| ID(id.to_string())),
            created_at: a.created_at.to_rfc3339(),
            last_heartbeat: a.last_heartbeat.to_rfc3339(),
        }
    }
}

// ============================================================================
// INPUT TYPES
// ============================================================================

/// Input for creating a trajectory.
#[derive(Debug, Clone, InputObject)]
pub struct CreateTrajectoryInput {
    pub name: String,
    pub description: Option<String>,
    pub parent_trajectory_id: Option<ID>,
    pub agent_id: Option<ID>,
}

/// Input for updating a trajectory.
#[derive(Debug, Clone, InputObject)]
pub struct UpdateTrajectoryInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<GqlTrajectoryStatus>,
}

/// Input for creating a note.
#[derive(Debug, Clone, InputObject)]
pub struct CreateNoteInput {
    pub note_type: GqlNoteType,
    pub title: String,
    pub content: String,
    pub source_trajectory_ids: Vec<ID>,
    pub source_artifact_ids: Option<Vec<ID>>,
}

/// Input for creating a scope.
#[derive(Debug, Clone, InputObject)]
pub struct CreateScopeInput {
    pub trajectory_id: ID,
    pub name: String,
    pub purpose: Option<String>,
    pub token_budget: Option<i32>,
}

// ============================================================================
// QUERY ROOT
// ============================================================================

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get a trajectory by ID.
    async fn trajectory(&self, ctx: &Context<'_>, id: ID) -> GqlResult<Option<GqlTrajectory>> {
        let db = ctx.data::<DbClient>()?;
        let auth = ctx.data::<AuthContext>()?;
        let uuid = Uuid::parse_str(&id.0)
            .map_err(|_| async_graphql::Error::new("Invalid UUID"))?;
        let trajectory_id = TrajectoryId::new(uuid);

        match db.get::<crate::types::TrajectoryResponse>(TrajectoryId::new(uuid), auth.tenant_id).await {
            Ok(Some(t)) => Ok(Some(t.into())),
            Ok(None) => Ok(None),
            Err(e) => Err(async_graphql::Error::new(e.message)),
        }
    }

    /// List trajectories by status.
    async fn trajectories(
        &self,
        ctx: &Context<'_>,
        status: GqlTrajectoryStatus,
    ) -> GqlResult<Vec<GqlTrajectory>> {
        let db = ctx.data::<DbClient>()?;
        let auth = ctx.data::<AuthContext>()?;

        let filter = TrajectoryListFilter {
            status: Some(status.into()),
            ..Default::default()
        };
        match db.list::<crate::types::TrajectoryResponse>(&filter, auth.tenant_id).await {
            Ok(trajectories) => Ok(trajectories.into_iter().map(|t| t.into()).collect()),
            Err(e) => Err(async_graphql::Error::new(e.message)),
        }
    }

    /// Get a scope by ID.
    async fn scope(&self, ctx: &Context<'_>, id: ID) -> GqlResult<Option<GqlScope>> {
        let db = ctx.data::<DbClient>()?;
        let auth = ctx.data::<AuthContext>()?;
        let uuid = Uuid::parse_str(&id.0)
            .map_err(|_| async_graphql::Error::new("Invalid UUID"))?;

        match db.get::<crate::types::ScopeResponse>(ScopeId::new(uuid), auth.tenant_id).await {
            Ok(Some(s)) => Ok(Some(s.into())),
            Ok(None) => Ok(None),
            Err(e) => Err(async_graphql::Error::new(e.message)),
        }
    }

    /// List scopes for a trajectory.
    async fn scopes(&self, ctx: &Context<'_>, trajectory_id: ID) -> GqlResult<Vec<GqlScope>> {
        let db = ctx.data::<DbClient>()?;
        let auth = ctx.data::<AuthContext>()?;
        let uuid = Uuid::parse_str(&trajectory_id.0)
            .map_err(|_| async_graphql::Error::new("Invalid UUID"))?;

        let filter = ScopeListFilter {
            trajectory_id: Some(TrajectoryId::new(uuid)),
            ..Default::default()
        };
        match db.list::<crate::types::ScopeResponse>(&filter, auth.tenant_id).await {
            Ok(scopes) => Ok(scopes.into_iter().map(|s| s.into()).collect()),
            Err(e) => Err(async_graphql::Error::new(e.message)),
        }
    }

    /// Get an artifact by ID.
    async fn artifact(&self, ctx: &Context<'_>, id: ID) -> GqlResult<Option<GqlArtifact>> {
        let db = ctx.data::<DbClient>()?;
        let auth = ctx.data::<AuthContext>()?;
        let uuid = Uuid::parse_str(&id.0)
            .map_err(|_| async_graphql::Error::new("Invalid UUID"))?;

        match db.get::<crate::types::ArtifactResponse>(ArtifactId::new(uuid), auth.tenant_id).await {
            Ok(Some(a)) => Ok(Some(a.into())),
            Ok(None) => Ok(None),
            Err(e) => Err(async_graphql::Error::new(e.message)),
        }
    }

    /// List artifacts for a scope.
    async fn artifacts(&self, ctx: &Context<'_>, scope_id: ID) -> GqlResult<Vec<GqlArtifact>> {
        let db = ctx.data::<DbClient>()?;
        let auth = ctx.data::<AuthContext>()?;
        let uuid = Uuid::parse_str(&scope_id.0)
            .map_err(|_| async_graphql::Error::new("Invalid UUID"))?;

        let filter = ArtifactListFilter {
            scope_id: Some(ScopeId::new(uuid)),
            ..Default::default()
        };
        match db.list::<crate::types::ArtifactResponse>(&filter, auth.tenant_id).await {
            Ok(artifacts) => Ok(artifacts.into_iter().map(|a| a.into()).collect()),
            Err(e) => Err(async_graphql::Error::new(e.message)),
        }
    }

    /// Get a note by ID.
    async fn note(&self, ctx: &Context<'_>, id: ID) -> GqlResult<Option<GqlNote>> {
        let db = ctx.data::<DbClient>()?;
        let auth = ctx.data::<AuthContext>()?;
        let uuid = Uuid::parse_str(&id.0)
            .map_err(|_| async_graphql::Error::new("Invalid UUID"))?;

        match db.get::<crate::types::NoteResponse>(NoteId::new(uuid), auth.tenant_id).await {
            Ok(Some(n)) => Ok(Some(n.into())),
            Ok(None) => Ok(None),
            Err(e) => Err(async_graphql::Error::new(e.message)),
        }
    }

    /// List notes for a trajectory.
    async fn notes(&self, ctx: &Context<'_>, trajectory_id: ID) -> GqlResult<Vec<GqlNote>> {
        let db = ctx.data::<DbClient>()?;
        let auth = ctx.data::<AuthContext>()?;
        let uuid = Uuid::parse_str(&trajectory_id.0)
            .map_err(|_| async_graphql::Error::new("Invalid UUID"))?;

        let filter = NoteListFilter {
            source_trajectory_id: Some(TrajectoryId::new(uuid)),
            ..Default::default()
        };
        match db.list::<crate::types::NoteResponse>(&filter, auth.tenant_id).await {
            Ok(notes) => Ok(notes.into_iter().map(|n| n.into()).collect()),
            Err(e) => Err(async_graphql::Error::new(e.message)),
        }
    }

    /// Get an agent by ID.
    async fn agent(&self, ctx: &Context<'_>, id: ID) -> GqlResult<Option<GqlAgent>> {
        let db = ctx.data::<DbClient>()?;
        let auth = ctx.data::<AuthContext>()?;
        let uuid = Uuid::parse_str(&id.0)
            .map_err(|_| async_graphql::Error::new("Invalid UUID"))?;

        match db.get::<crate::types::AgentResponse>(AgentId::new(uuid), auth.tenant_id).await {
            Ok(Some(a)) => Ok(Some(a.into())),
            Ok(None) => Ok(None),
            Err(e) => Err(async_graphql::Error::new(e.message)),
        }
    }

    /// List active agents.
    async fn agents(&self, ctx: &Context<'_>) -> GqlResult<Vec<GqlAgent>> {
        let db = ctx.data::<DbClient>()?;

        match db.agent_list_active().await {
            Ok(agents) => Ok(agents.into_iter().map(|a| a.into()).collect()),
            Err(e) => Err(async_graphql::Error::new(e.message)),
        }
    }
}

// ============================================================================
// MUTATION ROOT
// ============================================================================

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Create a new trajectory.
    async fn create_trajectory(
        &self,
        ctx: &Context<'_>,
        input: CreateTrajectoryInput,
    ) -> GqlResult<GqlTrajectory> {
        let db = ctx.data::<DbClient>()?;
        let ws = ctx.data::<Arc<WsState>>()?;
        let auth = ctx.data::<AuthContext>()?;

        let parent_id = if let Some(id) = input.parent_trajectory_id {
            Some(TrajectoryId::new(
                Uuid::parse_str(&id.0)
                    .map_err(|_| async_graphql::Error::new("Invalid parent_trajectory_id"))?,
            ))
        } else {
            None
        };

        let agent_id = if let Some(id) = input.agent_id {
            Some(AgentId::new(
                Uuid::parse_str(&id.0)
                    .map_err(|_| async_graphql::Error::new("Invalid agent_id"))?,
            ))
        } else {
            None
        };

        let req = CreateTrajectoryRequest {
            name: input.name,
            description: input.description,
            parent_trajectory_id: parent_id,
            agent_id,
            metadata: None,
        };

        match db.create::<crate::types::TrajectoryResponse>(&req, auth.tenant_id).await {
            Ok(trajectory) => {
                ws.broadcast(WsEvent::TrajectoryCreated {
                    trajectory: trajectory.clone(),
                });
                Ok(trajectory.into())
            }
            Err(e) => Err(async_graphql::Error::new(e.message)),
        }
    }

    /// Update an existing trajectory.
    async fn update_trajectory(
        &self,
        ctx: &Context<'_>,
        id: ID,
        input: UpdateTrajectoryInput,
    ) -> GqlResult<GqlTrajectory> {
        let db = ctx.data::<DbClient>()?;
        let ws = ctx.data::<Arc<WsState>>()?;
        let auth = ctx.data::<AuthContext>()?;

        let uuid = Uuid::parse_str(&id.0)
            .map_err(|_| async_graphql::Error::new("Invalid UUID"))?;

        let req = UpdateTrajectoryRequest {
            name: input.name,
            description: input.description,
            status: input.status.map(|s| s.into()),
            metadata: None,
        };

        match db.update::<crate::types::TrajectoryResponse>(TrajectoryId::new(uuid), &req, auth.tenant_id).await {
            Ok(trajectory) => {
                ws.broadcast(WsEvent::TrajectoryUpdated {
                    trajectory: trajectory.clone(),
                });
                Ok(trajectory.into())
            }
            Err(e) => Err(async_graphql::Error::new(e.message)),
        }
    }

    /// Delete a trajectory.
    async fn delete_trajectory(&self, ctx: &Context<'_>, id: ID) -> GqlResult<bool> {
        let db = ctx.data::<DbClient>()?;
        let ws = ctx.data::<Arc<WsState>>()?;
        let auth = ctx.data::<AuthContext>()?;

        let uuid = Uuid::parse_str(&id.0)
            .map_err(|_| async_graphql::Error::new("Invalid UUID"))?;
        let trajectory_id = TrajectoryId::new(uuid);

        match db.delete::<crate::types::TrajectoryResponse>(trajectory_id, auth.tenant_id).await {
            Ok(_) => {
                ws.broadcast(WsEvent::TrajectoryDeleted {
                    tenant_id: auth.tenant_id,
                    id: trajectory_id,
                });
                Ok(true)
            }
            Err(e) => Err(async_graphql::Error::new(e.message)),
        }
    }

    /// Create a new scope.
    async fn create_scope(&self, ctx: &Context<'_>, input: CreateScopeInput) -> GqlResult<GqlScope> {
        let db = ctx.data::<DbClient>()?;
        let ws = ctx.data::<Arc<WsState>>()?;
        let auth = ctx.data::<AuthContext>()?;

        let trajectory_id = Uuid::parse_str(&input.trajectory_id.0)
            .map_err(|_| async_graphql::Error::new("Invalid trajectory_id"))?;

        let req = CreateScopeRequest {
            trajectory_id: TrajectoryId::new(trajectory_id),
            parent_scope_id: None,
            name: input.name,
            purpose: input.purpose,
            token_budget: input.token_budget.unwrap_or(4096),
            metadata: None,
        };

        match db.create::<crate::types::ScopeResponse>(&req, auth.tenant_id).await {
            Ok(scope) => {
                ws.broadcast(WsEvent::ScopeCreated { scope: scope.clone() });
                Ok(scope.into())
            }
            Err(e) => Err(async_graphql::Error::new(e.message)),
        }
    }

    /// Close a scope.
    async fn close_scope(&self, ctx: &Context<'_>, id: ID) -> GqlResult<GqlScope> {
        let db = ctx.data::<DbClient>()?;
        let ws = ctx.data::<Arc<WsState>>()?;
        let auth = ctx.data::<AuthContext>()?;

        let uuid = Uuid::parse_str(&id.0)
            .map_err(|_| async_graphql::Error::new("Invalid UUID"))?;

        // Get the scope first
        let existing = db
            .get::<crate::types::ScopeResponse>(ScopeId::new(uuid), auth.tenant_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.message))?
            .ok_or_else(|| async_graphql::Error::new("Scope not found"))?;

        // Close via Response method
        match existing.close(&db).await {
            Ok(scope) => {
                ws.broadcast(WsEvent::ScopeClosed { scope: scope.clone() });
                Ok(scope.into())
            }
            Err(e) => Err(async_graphql::Error::new(e.message)),
        }
    }

    /// Create a new note.
    async fn create_note(&self, ctx: &Context<'_>, input: CreateNoteInput) -> GqlResult<GqlNote> {
        let db = ctx.data::<DbClient>()?;
        let ws = ctx.data::<Arc<WsState>>()?;
        let auth = ctx.data::<AuthContext>()?;

        let source_trajectory_ids: Result<Vec<_>, _> = input
            .source_trajectory_ids
            .into_iter()
            .map(|id| {
                Uuid::parse_str(&id.0)
                    .map(TrajectoryId::new)
                    .map_err(|_| async_graphql::Error::new("Invalid source_trajectory_id"))
            })
            .collect();

        let source_artifact_ids: Result<Vec<_>, _> = input
            .source_artifact_ids
            .unwrap_or_default()
            .into_iter()
            .map(|id| {
                Uuid::parse_str(&id.0)
                    .map(ArtifactId::new)
                    .map_err(|_| async_graphql::Error::new("Invalid source_artifact_id"))
            })
            .collect();

        let req = CreateNoteRequest {
            note_type: input.note_type.into(),
            title: input.title,
            content: input.content,
            source_trajectory_ids: source_trajectory_ids?,
            source_artifact_ids: source_artifact_ids?,
            ttl: caliber_core::TTL::Persistent,
            metadata: None,
        };

        match db.create::<crate::types::NoteResponse>(&req, auth.tenant_id).await {
            Ok(note) => {
                ws.broadcast(WsEvent::NoteCreated { note: note.clone() });
                Ok(note.into())
            }
            Err(e) => Err(async_graphql::Error::new(e.message)),
        }
    }
}

// ============================================================================
// SCHEMA & HANDLERS
// ============================================================================

/// The GraphQL schema type.
pub type CaliberSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

/// Create the GraphQL schema.
pub fn create_schema(db: DbClient, ws: Arc<WsState>) -> CaliberSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(db)
        .data(ws)
        .finish()
}

/// Handler for GraphQL requests.
pub async fn graphql_handler(
    State(schema): State<CaliberSchema>,
    AuthExtractor(auth): AuthExtractor,
    req: GraphQLRequest,
) -> GraphQLResponse {
    // Add auth context to the request so resolvers can access tenant_id
    let request = req.into_inner().data(auth);
    schema.execute(request).await.into()
}

/// Handler for GraphiQL playground.
pub async fn graphiql_handler() -> impl IntoResponse {
    Html(async_graphql::http::GraphiQLSource::build()
        .endpoint("/api/v1/graphql")
        .finish())
}

// ============================================================================
// ROUTER SETUP
// ============================================================================

/// Create the GraphQL routes router.
pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/", post(graphql_handler))
        .route("/playground", get(graphiql_handler))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gql_trajectory_status_conversion() {
        let active = caliber_core::TrajectoryStatus::Active;
        let gql_active: GqlTrajectoryStatus = active.into();
        assert_eq!(gql_active, GqlTrajectoryStatus::Active);

        let back: caliber_core::TrajectoryStatus = gql_active.into();
        assert_eq!(back, active);
    }

    #[test]
    fn test_gql_note_type_conversion() {
        let fact = caliber_core::NoteType::Fact;
        let gql_fact: GqlNoteType = fact.into();
        assert_eq!(gql_fact, GqlNoteType::Fact);

        let back: caliber_core::NoteType = gql_fact.into();
        assert_eq!(back, fact);
    }

    #[test]
    fn test_create_trajectory_input() {
        let input = CreateTrajectoryInput {
            name: "Test Trajectory".to_string(),
            description: Some("A test".to_string()),
            parent_trajectory_id: None,
            agent_id: None,
        };

        assert_eq!(input.name, "Test Trajectory");
        assert!(input.description.is_some());
    }

    #[test]
    fn test_gql_trajectory_from_response() {
        let response = TrajectoryResponse {
            trajectory_id: Uuid::new_v4(),
            tenant_id: Some(Uuid::new_v4()),
            name: "Test".to_string(),
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
        };

        let gql: GqlTrajectory = response.into();
        assert_eq!(gql.name, "Test");
        assert_eq!(gql.status, GqlTrajectoryStatus::Active);
    }
}
