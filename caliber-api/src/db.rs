//! Database Connection Pool Module
//!
//! This module provides PostgreSQL connection pooling using deadpool-postgres
//! and wrapper functions that call caliber_* pg_extern functions from the
//! caliber-pg extension.
//!
//! CRITICAL: This module does NOT write raw SQL queries. All operations go
//! through the caliber_* functions which internally use direct heap operations
//! for maximum performance.

use crate::error::{ApiError, ApiResult};
use crate::types::*;
use caliber_core::{EntityId, Timestamp};
use deadpool_postgres::{Config, ManagerConfig, Pool, RecyclingMethod, Runtime};
use serde_json::Value as JsonValue;
use std::time::Duration;
use tokio_postgres::NoTls;
use uuid::Uuid;

// ============================================================================
// CONNECTION POOL CONFIGURATION
// ============================================================================

/// Database connection pool configuration.
#[derive(Debug, Clone)]
pub struct DbConfig {
    /// PostgreSQL host
    pub host: String,
    /// PostgreSQL port
    pub port: u16,
    /// Database name
    pub dbname: String,
    /// Database user
    pub user: String,
    /// Database password
    pub password: String,
    /// Maximum pool size
    pub max_size: usize,
    /// Connection timeout
    pub timeout: Duration,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5432,
            dbname: "caliber".to_string(),
            user: "postgres".to_string(),
            password: "".to_string(),
            max_size: 16,
            timeout: Duration::from_secs(30),
        }
    }
}

impl DbConfig {
    /// Create a new database configuration from environment variables.
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("CALIBER_DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
            port: std::env::var("CALIBER_DB_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5432),
            dbname: std::env::var("CALIBER_DB_NAME").unwrap_or_else(|_| "caliber".to_string()),
            user: std::env::var("CALIBER_DB_USER").unwrap_or_else(|_| "postgres".to_string()),
            password: std::env::var("CALIBER_DB_PASSWORD").unwrap_or_default(),
            max_size: std::env::var("CALIBER_DB_POOL_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(16),
            timeout: Duration::from_secs(
                std::env::var("CALIBER_DB_TIMEOUT")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(30),
            ),
        }
    }

    /// Create a connection pool from this configuration.
    pub fn create_pool(&self) -> ApiResult<Pool> {
        let mut cfg = Config::new();
        cfg.host = Some(self.host.clone());
        cfg.port = Some(self.port);
        cfg.dbname = Some(self.dbname.clone());
        cfg.user = Some(self.user.clone());
        cfg.password = Some(self.password.clone());

        cfg.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });

        let pool = cfg
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .map_err(|e| ApiError::database_error(format!("Failed to create pool: {}", e)))?;

        Ok(pool)
    }
}

// ============================================================================
// DATABASE CLIENT WRAPPER
// ============================================================================

/// Database client that wraps a connection pool and provides
/// high-level operations that call caliber_* pg_extern functions.
#[derive(Clone)]
pub struct DbClient {
    pool: Pool,
}

/// Parameters for message_list filters.
pub struct MessageListParams<'a> {
    pub from_agent_id: Option<EntityId>,
    pub to_agent_id: Option<EntityId>,
    pub to_agent_type: Option<&'a str>,
    pub trajectory_id: Option<EntityId>,
    pub message_type: Option<&'a str>,
    pub priority: Option<&'a str>,
    pub undelivered_only: bool,
    pub unacknowledged_only: bool,
    pub limit: i32,
    pub offset: i32,
}

impl DbClient {
    /// Create a new database client with the given pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Create a new database client from configuration.
    pub fn from_config(config: &DbConfig) -> ApiResult<Self> {
        let pool = config.create_pool()?;
        Ok(Self::new(pool))
    }

    /// Get the current pool size for observability.
    pub fn pool_size(&self) -> usize {
        let status = self.pool.status();
        status.size
    }

    /// Get a connection from the pool.
    async fn get_conn(&self) -> ApiResult<deadpool_postgres::Object> {
        self.pool.get().await.map_err(ApiError::from)
    }

    // ========================================================================
    // TRAJECTORY OPERATIONS
    // ========================================================================

    /// Create a new trajectory by calling caliber_trajectory_create.
    ///
    /// The `tenant_id` parameter ensures tenant isolation by associating
    /// the trajectory with the authenticated user's tenant.
    pub async fn trajectory_create(
        &self,
        req: &CreateTrajectoryRequest,
        tenant_id: EntityId,
    ) -> ApiResult<TrajectoryResponse> {
        let conn = self.get_conn().await?;

        let agent_uuid = req.agent_id;

        let row = conn
            .query_one(
                "SELECT caliber_trajectory_create($1, $2, $3, $4)",
                &[&req.name, &req.description, &agent_uuid, &tenant_id],
            )
            .await?;

        let trajectory_id: Uuid = row.get(0);

        // Get the full trajectory details
        self.trajectory_get(trajectory_id, tenant_id).await?
            .ok_or_else(|| ApiError::internal_error("Failed to retrieve created trajectory"))
    }

    /// Get a trajectory by ID by calling caliber_trajectory_get.
    pub async fn trajectory_get(
        &self,
        id: EntityId,
        tenant_id: EntityId,
    ) -> ApiResult<Option<TrajectoryResponse>> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one(
                "SELECT caliber_trajectory_get($1, $2)",
                &[&id, &tenant_id],
            )
            .await?;

        let json_opt: Option<JsonValue> = row.get(0);

        match json_opt {
            Some(json) => {
                let response = self.parse_trajectory_json(&json)?;
                Ok(Some(response))
            }
            None => Ok(None),
        }
    }

    /// Update a trajectory by calling caliber_trajectory_update.
    pub async fn trajectory_update(
        &self,
        id: EntityId,
        req: &UpdateTrajectoryRequest,
        tenant_id: EntityId,
    ) -> ApiResult<TrajectoryResponse> {
        let conn = self.get_conn().await?;

        // Build update JSON
        let mut updates = serde_json::Map::new();
        if let Some(name) = &req.name {
            updates.insert("name".to_string(), JsonValue::String(name.clone()));
        }
        if let Some(description) = &req.description {
            updates.insert("description".to_string(), JsonValue::String(description.clone()));
        }
        if let Some(status) = &req.status {
            let status_str = match status {
                caliber_core::TrajectoryStatus::Active => "active",
                caliber_core::TrajectoryStatus::Completed => "completed",
                caliber_core::TrajectoryStatus::Failed => "failed",
                caliber_core::TrajectoryStatus::Suspended => "suspended",
            };
            updates.insert("status".to_string(), JsonValue::String(status_str.to_string()));
        }
        if let Some(metadata) = &req.metadata {
            updates.insert("metadata".to_string(), metadata.clone());
        }

        let updates_json = JsonValue::Object(updates);

        let updated: bool = conn
            .query_one(
                "SELECT caliber_trajectory_update($1, $2, $3)",
                &[&id, &updates_json, &tenant_id],
            )
            .await?
            .get(0);

        if !updated {
            return Err(ApiError::trajectory_not_found(id));
        }

        self.trajectory_get(id, tenant_id).await?
            .ok_or_else(|| ApiError::trajectory_not_found(id))
    }

    /// List trajectories by status by calling caliber_trajectory_list_by_status.
    pub async fn trajectory_list_by_status(
        &self,
        status: caliber_core::TrajectoryStatus,
        tenant_id: EntityId,
    ) -> ApiResult<Vec<TrajectoryResponse>> {
        let conn = self.get_conn().await?;

        let status_str = match status {
            caliber_core::TrajectoryStatus::Active => "active",
            caliber_core::TrajectoryStatus::Completed => "completed",
            caliber_core::TrajectoryStatus::Failed => "failed",
            caliber_core::TrajectoryStatus::Suspended => "suspended",
        };

        let row = conn
            .query_one(
                "SELECT caliber_trajectory_list_by_status($1, $2)",
                &[&status_str, &tenant_id],
            )
            .await?;

        let json: JsonValue = row.get(0);
        let trajectories_json = json.as_array()
            .ok_or_else(|| ApiError::internal_error("Expected array from trajectory list"))?;

        let mut trajectories = Vec::new();
        for traj_json in trajectories_json {
            trajectories.push(self.parse_trajectory_json(traj_json)?);
        }

        Ok(trajectories)
    }

    /// Parse trajectory JSON into TrajectoryResponse.
    fn parse_trajectory_json(&self, json: &JsonValue) -> ApiResult<TrajectoryResponse> {
        Ok(TrajectoryResponse {
            trajectory_id: self.parse_uuid(json, "trajectory_id")?,
            tenant_id: self.parse_optional_uuid(json, "tenant_id"),
            name: self.parse_string(json, "name")?,
            description: self.parse_optional_string(json, "description"),
            status: self.parse_trajectory_status(json, "status")?,
            parent_trajectory_id: self.parse_optional_uuid(json, "parent_trajectory_id"),
            root_trajectory_id: self.parse_optional_uuid(json, "root_trajectory_id"),
            agent_id: self.parse_optional_uuid(json, "agent_id"),
            created_at: self.parse_timestamp(json, "created_at")?,
            updated_at: self.parse_timestamp(json, "updated_at")?,
            completed_at: self.parse_optional_timestamp(json, "completed_at"),
            outcome: self.parse_optional_outcome(json, "outcome"),
            metadata: json.get("metadata").and_then(|v| if v.is_null() { None } else { Some(v.clone()) }),
        })
    }

    // ========================================================================
    // SCOPE OPERATIONS
    // ========================================================================

    /// Get a scope by ID by calling caliber_scope_get.
    pub async fn scope_get(
        &self,
        id: EntityId,
        tenant_id: EntityId,
    ) -> ApiResult<Option<ScopeResponse>> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one("SELECT caliber_scope_get($1, $2)", &[&id, &tenant_id])
            .await?;

        let json_opt: Option<JsonValue> = row.get(0);

        match json_opt {
            Some(json) => {
                let response = self.parse_scope_json(&json)?;
                Ok(Some(response))
            }
            None => Ok(None),
        }
    }

    /// Close a scope by calling caliber_scope_close.
    pub async fn scope_close(&self, id: EntityId, tenant_id: EntityId) -> ApiResult<ScopeResponse> {
        let conn = self.get_conn().await?;

        let closed: bool = conn
            .query_one("SELECT caliber_scope_close($1, $2)", &[&id, &tenant_id])
            .await?
            .get(0);

        if !closed {
            return Err(ApiError::scope_not_found(id));
        }

        self.scope_get(id, tenant_id).await?
            .ok_or_else(|| ApiError::scope_not_found(id))
    }

    /// Create a checkpoint for a scope.
    pub async fn scope_create_checkpoint(
        &self,
        id: EntityId,
        req: &CreateCheckpointRequest,
        tenant_id: EntityId,
    ) -> ApiResult<CheckpointResponse> {
        let conn = self.get_conn().await?;

        // Build checkpoint JSON
        let checkpoint_json = serde_json::json!({
            "context_state": req.context_state,
            "recoverable": req.recoverable,
        });

        let updated: bool = conn
            .query_one(
                "SELECT caliber_scope_update($1, $2, $3)",
                &[&id, &serde_json::json!({ "checkpoint": checkpoint_json }), &tenant_id],
            )
            .await?
            .get(0);

        if !updated {
            return Err(ApiError::scope_not_found(id));
        }

        Ok(CheckpointResponse {
            context_state: req.context_state.clone(),
            recoverable: req.recoverable,
        })
    }

    /// Parse scope JSON into ScopeResponse.
    fn parse_scope_json(&self, json: &JsonValue) -> ApiResult<ScopeResponse> {
        Ok(ScopeResponse {
            scope_id: self.parse_uuid(json, "scope_id")?,
            tenant_id: self.parse_optional_uuid(json, "tenant_id"),
            trajectory_id: self.parse_uuid(json, "trajectory_id")?,
            parent_scope_id: self.parse_optional_uuid(json, "parent_scope_id"),
            name: self.parse_string(json, "name")?,
            purpose: self.parse_optional_string(json, "purpose"),
            is_active: self.parse_bool(json, "is_active")?,
            created_at: self.parse_timestamp(json, "created_at")?,
            closed_at: self.parse_optional_timestamp(json, "closed_at"),
            checkpoint: self.parse_optional_checkpoint(json, "checkpoint"),
            token_budget: self.parse_i32(json, "token_budget")?,
            tokens_used: self.parse_i32(json, "tokens_used")?,
            metadata: json.get("metadata").and_then(|v| if v.is_null() { None } else { Some(v.clone()) }),
        })
    }

    // ========================================================================
    // ARTIFACT OPERATIONS
    // ========================================================================

    /// Get an artifact by ID by calling caliber_artifact_get.
    pub async fn artifact_get(
        &self,
        id: EntityId,
        tenant_id: EntityId,
    ) -> ApiResult<Option<ArtifactResponse>> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one("SELECT caliber_artifact_get($1, $2)", &[&id, &tenant_id])
            .await?;

        let json_opt: Option<JsonValue> = row.get(0);

        match json_opt {
            Some(json) => {
                let response = self.parse_artifact_json(&json)?;
                Ok(Some(response))
            }
            None => Ok(None),
        }
    }

    /// List recent artifacts across all trajectories for a tenant.
    pub async fn artifact_list_recent(
        &self,
        tenant_id: EntityId,
        limit: usize,
    ) -> ApiResult<Vec<ArtifactResponse>> {
        let statuses = [
            caliber_core::TrajectoryStatus::Active,
            caliber_core::TrajectoryStatus::Completed,
            caliber_core::TrajectoryStatus::Failed,
            caliber_core::TrajectoryStatus::Suspended,
        ];

        let mut artifacts = Vec::new();
        for status in statuses {
            let trajectories = self.trajectory_list_by_status(status, tenant_id).await?;
            for trajectory in trajectories {
                let mut traj_artifacts = self
                    .artifact_list_by_trajectory_and_tenant(trajectory.trajectory_id, tenant_id)
                    .await?;
                artifacts.append(&mut traj_artifacts);
            }
        }

        artifacts.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        artifacts.truncate(limit);
        Ok(artifacts)
    }

    /// Query artifacts by trajectory for a specific tenant.
    pub async fn artifact_list_by_trajectory_and_tenant(
        &self,
        trajectory_id: EntityId,
        tenant_id: EntityId,
    ) -> ApiResult<Vec<ArtifactResponse>> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one(
                "SELECT caliber_artifact_query_by_trajectory_and_tenant($1, $2)",
                &[&trajectory_id, &tenant_id],
            )
            .await?;

        let json: JsonValue = row.get(0);
        let artifacts_json = json.as_array()
            .ok_or_else(|| ApiError::internal_error("Expected array from artifact list"))?;

        let mut artifacts = Vec::new();
        for artifact_json in artifacts_json {
            artifacts.push(self.parse_artifact_json(artifact_json)?);
        }

        Ok(artifacts)
    }

    /// Parse artifact JSON into ArtifactResponse.
    fn parse_artifact_json(&self, json: &JsonValue) -> ApiResult<ArtifactResponse> {
        Ok(ArtifactResponse {
            artifact_id: self.parse_uuid(json, "artifact_id")?,
            tenant_id: self.parse_optional_uuid(json, "tenant_id"),
            trajectory_id: self.parse_uuid(json, "trajectory_id")?,
            scope_id: self.parse_uuid(json, "scope_id")?,
            artifact_type: self.parse_artifact_type(json, "artifact_type")?,
            name: self.parse_string(json, "name")?,
            content: self.parse_string(json, "content")?,
            content_hash: self.parse_content_hash(json, "content_hash")?,
            embedding: self.parse_optional_embedding(json, "embedding"),
            provenance: self.parse_provenance(json, "provenance")?,
            ttl: self.parse_ttl(json, "ttl")?,
            created_at: self.parse_timestamp(json, "created_at")?,
            updated_at: self.parse_timestamp(json, "updated_at")?,
            superseded_by: self.parse_optional_uuid(json, "superseded_by"),
            metadata: json.get("metadata").and_then(|v| if v.is_null() { None } else { Some(v.clone()) }),
        })
    }

    // ========================================================================
    // NOTE OPERATIONS
    // ========================================================================

    /// Get a note by ID by calling caliber_note_get.
    pub async fn note_get(
        &self,
        id: EntityId,
        tenant_id: EntityId,
    ) -> ApiResult<Option<NoteResponse>> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one("SELECT caliber_note_get($1, $2)", &[&id, &tenant_id])
            .await?;

        let json_opt: Option<JsonValue> = row.get(0);

        match json_opt {
            Some(json) => {
                let response = self.parse_note_json(&json)?;
                Ok(Some(response))
            }
            None => Ok(None),
        }
    }

    /// Search notes by content similarity.
    pub async fn note_search(&self, query: &str, limit: i32) -> ApiResult<Vec<NoteResponse>> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one(
                "SELECT caliber_note_search($1, $2)",
                &[&query, &limit],
            )
            .await?;

        let json: JsonValue = row.get(0);
        let notes_json = json.as_array()
            .ok_or_else(|| ApiError::internal_error("Expected array from note search"))?;

        let mut notes = Vec::new();
        for note_json in notes_json {
            notes.push(self.parse_note_json(note_json)?);
        }

        Ok(notes)
    }

    /// Parse note JSON into NoteResponse.
    fn parse_note_json(&self, json: &JsonValue) -> ApiResult<NoteResponse> {
        Ok(NoteResponse {
            note_id: self.parse_uuid(json, "note_id")?,
            tenant_id: self.parse_optional_uuid(json, "tenant_id"),
            note_type: self.parse_note_type(json, "note_type")?,
            title: self.parse_string(json, "title")?,
            content: self.parse_string(json, "content")?,
            content_hash: self.parse_content_hash(json, "content_hash")?,
            embedding: self.parse_optional_embedding(json, "embedding"),
            source_trajectory_ids: self.parse_uuid_array(json, "source_trajectory_ids")?,
            source_artifact_ids: self.parse_uuid_array(json, "source_artifact_ids")?,
            ttl: self.parse_ttl(json, "ttl")?,
            created_at: self.parse_timestamp(json, "created_at")?,
            updated_at: self.parse_timestamp(json, "updated_at")?,
            accessed_at: self.parse_timestamp(json, "accessed_at")?,
            access_count: self.parse_i32(json, "access_count")?,
            superseded_by: self.parse_optional_uuid(json, "superseded_by"),
            metadata: json.get("metadata").and_then(|v| if v.is_null() { None } else { Some(v.clone()) }),
        })
    }

    // ========================================================================
    // TURN OPERATIONS
    // ========================================================================

    // ========================================================================
    // AGENT OPERATIONS
    // ========================================================================

    /// Register a new agent by calling caliber_agent_register.
    ///
    /// The `tenant_id` parameter ensures tenant isolation by associating
    /// the agent with the authenticated user's tenant.
    pub async fn agent_register(&self, req: &RegisterAgentRequest, tenant_id: EntityId) -> ApiResult<AgentResponse> {
        let conn = self.get_conn().await?;

        let capabilities_json = serde_json::to_value(&req.capabilities)?;
        let memory_access_json = serde_json::to_value(&req.memory_access)?;
        let can_delegate_to_json = serde_json::to_value(&req.can_delegate_to)?;

        let row = conn
            .query_one(
                "SELECT caliber_agent_register($1, $2, $3, $4, $5, $6)",
                &[
                    &req.agent_type,
                    &capabilities_json,
                    &memory_access_json,
                    &can_delegate_to_json,
                    &req.reports_to,
                    &tenant_id,
                ],
            )
            .await?;

        let agent_id: Uuid = row.get(0);

        self.agent_get(agent_id).await?
            .ok_or_else(|| ApiError::internal_error("Failed to retrieve registered agent"))
    }

    /// Get an agent by ID by calling caliber_agent_get.
    pub async fn agent_get(&self, id: EntityId) -> ApiResult<Option<AgentResponse>> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one("SELECT caliber_agent_get($1)", &[&id])
            .await?;

        let json_opt: Option<JsonValue> = row.get(0);

        match json_opt {
            Some(json) => {
                let response = self.parse_agent_json(&json)?;
                Ok(Some(response))
            }
            None => Ok(None),
        }
    }

    /// Update agent heartbeat by calling caliber_agent_heartbeat.
    pub async fn agent_heartbeat(&self, id: EntityId) -> ApiResult<()> {
        let conn = self.get_conn().await?;

        conn.execute("SELECT caliber_agent_heartbeat($1)", &[&id])
            .await?;

        Ok(())
    }

    /// Update an agent by calling caliber_agent_update.
    pub async fn agent_update(
        &self,
        id: EntityId,
        req: &UpdateAgentRequest,
    ) -> ApiResult<AgentResponse> {
        let conn = self.get_conn().await?;

        // Build update JSON
        let mut updates = serde_json::Map::new();
        if let Some(status) = &req.status {
            updates.insert("status".to_string(), JsonValue::String(status.clone()));
        }
        if let Some(trajectory_id) = req.current_trajectory_id {
            updates.insert("current_trajectory_id".to_string(), JsonValue::String(trajectory_id.to_string()));
        }
        if let Some(scope_id) = req.current_scope_id {
            updates.insert("current_scope_id".to_string(), JsonValue::String(scope_id.to_string()));
        }
        if let Some(capabilities) = &req.capabilities {
            updates.insert("capabilities".to_string(), serde_json::to_value(capabilities)?);
        }
        if let Some(memory_access) = &req.memory_access {
            updates.insert("memory_access".to_string(), serde_json::to_value(memory_access)?);
        }

        let updates_json = JsonValue::Object(updates);

        let updated: bool = conn
            .query_one(
                "SELECT caliber_agent_update($1, $2)",
                &[&id, &updates_json],
            )
            .await?
            .get(0);

        if !updated {
            return Err(ApiError::agent_not_found(id));
        }

        self.agent_get(id).await?
            .ok_or_else(|| ApiError::agent_not_found(id))
    }

    /// List agents by type by calling caliber_agent_list_by_type.
    pub async fn agent_list_by_type(&self, agent_type: &str) -> ApiResult<Vec<AgentResponse>> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one(
                "SELECT caliber_agent_list_by_type($1)",
                &[&agent_type],
            )
            .await?;

        let json: JsonValue = row.get(0);
        let agents_json = json.as_array()
            .ok_or_else(|| ApiError::internal_error("Expected array from agent list"))?;

        let mut agents = Vec::new();
        for agent_json in agents_json {
            agents.push(self.parse_agent_json(agent_json)?);
        }

        Ok(agents)
    }

    /// List all active agents by calling caliber_agent_list_active.
    pub async fn agent_list_active(&self) -> ApiResult<Vec<AgentResponse>> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one("SELECT caliber_agent_list_active()", &[])
            .await?;

        let json: JsonValue = row.get(0);
        let agents_json = json.as_array()
            .ok_or_else(|| ApiError::internal_error("Expected array from agent list"))?;

        let mut agents = Vec::new();
        for agent_json in agents_json {
            agents.push(self.parse_agent_json(agent_json)?);
        }

        Ok(agents)
    }

    /// List all agents by calling caliber_agent_list_all.
    pub async fn agent_list_all(&self) -> ApiResult<Vec<AgentResponse>> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one("SELECT caliber_agent_list_all()", &[])
            .await?;

        let json: JsonValue = row.get(0);
        let agents_json = json.as_array()
            .ok_or_else(|| ApiError::internal_error("Expected array from agent list"))?;

        let mut agents = Vec::new();
        for agent_json in agents_json {
            agents.push(self.parse_agent_json(agent_json)?);
        }

        Ok(agents)
    }

    /// Unregister an agent by calling caliber_agent_unregister.
    pub async fn agent_unregister(&self, id: EntityId) -> ApiResult<()> {
        let conn = self.get_conn().await?;

        let unregistered: bool = conn
            .query_one("SELECT caliber_agent_unregister($1)", &[&id])
            .await?
            .get(0);

        if !unregistered {
            return Err(ApiError::agent_not_found(id));
        }

        Ok(())
    }

    /// Parse agent JSON into AgentResponse.
    fn parse_agent_json(&self, json: &JsonValue) -> ApiResult<AgentResponse> {
        Ok(AgentResponse {
            tenant_id: self.parse_uuid(json, "tenant_id")?,
            agent_id: self.parse_uuid(json, "agent_id")?,
            agent_type: self.parse_string(json, "agent_type")?,
            capabilities: self.parse_string_array(json, "capabilities")?,
            memory_access: serde_json::from_value(
                json.get("memory_access")
                    .ok_or_else(|| ApiError::internal_error("Missing memory_access"))?
                    .clone()
            )?,
            status: self.parse_string(json, "status")?,
            current_trajectory_id: self.parse_optional_uuid(json, "current_trajectory_id"),
            current_scope_id: self.parse_optional_uuid(json, "current_scope_id"),
            can_delegate_to: self.parse_string_array(json, "can_delegate_to")?,
            reports_to: self.parse_optional_uuid(json, "reports_to"),
            created_at: self.parse_timestamp(json, "created_at")?,
            last_heartbeat: self.parse_timestamp(json, "last_heartbeat")?,
        })
    }

    // ========================================================================
    // LOCK OPERATIONS
    // ========================================================================

    /// Acquire a lock by calling caliber_lock_acquire.
    pub async fn lock_acquire(&self, req: &AcquireLockRequest, tenant_id: EntityId) -> ApiResult<LockResponse> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one(
                "SELECT caliber_lock_acquire($1, $2, $3, $4, $5, $6)",
                &[
                    &req.holder_agent_id,
                    &req.resource_type,
                    &req.resource_id,
                    &req.timeout_ms,
                    &req.mode,
                    &tenant_id,
                ],
            )
            .await?;

        let lock_id: Uuid = row.get(0);

        self.lock_get(lock_id).await?
            .ok_or_else(|| ApiError::internal_error("Failed to retrieve acquired lock"))
    }

    /// Release a lock by calling caliber_lock_release.
    pub async fn lock_release(&self, id: EntityId) -> ApiResult<()> {
        let conn = self.get_conn().await?;

        let released: bool = conn
            .query_one("SELECT caliber_lock_release($1)", &[&id])
            .await?
            .get(0);

        if !released {
            return Err(ApiError::lock_not_found(id));
        }

        Ok(())
    }

    /// Get a lock by ID by calling caliber_lock_get.
    pub async fn lock_get(&self, id: EntityId) -> ApiResult<Option<LockResponse>> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one("SELECT caliber_lock_get($1)", &[&id])
            .await?;

        let json_opt: Option<JsonValue> = row.get(0);

        match json_opt {
            Some(json) => {
                let response = self.parse_lock_json(&json)?;
                Ok(Some(response))
            }
            None => Ok(None),
        }
    }

    /// Extend a lock's expiration by additional duration.
    pub async fn lock_extend(&self, id: EntityId, additional: Duration) -> ApiResult<LockResponse> {
        let conn = self.get_conn().await?;
        let additional_ms = i64::try_from(additional.as_millis())
            .map_err(|_| ApiError::invalid_range("additional_ms", 1, i64::MAX))?;

        let updated: bool = conn
            .query_one("SELECT caliber_lock_extend($1, $2)", &[&id, &additional_ms])
            .await?
            .get(0);

        if !updated {
            return Err(ApiError::lock_not_found(id));
        }

        self.lock_get(id).await?
            .ok_or_else(|| ApiError::lock_not_found(id))
    }

    /// List all active locks.
    pub async fn lock_list_active(&self) -> ApiResult<Vec<LockResponse>> {
        let conn = self.get_conn().await?;
        let row = conn.query_one("SELECT caliber_lock_list_active()", &[]).await?;
        let json: JsonValue = row.get(0);
        let locks_json = json.as_array().ok_or_else(|| {
            ApiError::internal_error("caliber_lock_list_active returned non-array")
        })?;

        let mut locks = Vec::with_capacity(locks_json.len());
        for lock_json in locks_json {
            let response = self.parse_lock_json(lock_json)?;
            locks.push(response);
        }

        Ok(locks)
    }

    /// List all active locks for a specific tenant.
    pub async fn lock_list_active_by_tenant(&self, tenant_id: EntityId) -> ApiResult<Vec<LockResponse>> {
        let conn = self.get_conn().await?;
        let row = conn.query_one("SELECT caliber_lock_list_active_by_tenant($1)", &[&tenant_id]).await?;
        let json: JsonValue = row.get(0);
        let locks_json = json.as_array().ok_or_else(|| {
            ApiError::internal_error("caliber_lock_list_active_by_tenant returned non-array")
        })?;

        let mut locks = Vec::with_capacity(locks_json.len());
        for lock_json in locks_json {
            let response = self.parse_lock_json(lock_json)?;
            locks.push(response);
        }

        Ok(locks)
    }

    /// Parse lock JSON into LockResponse.
    fn parse_lock_json(&self, json: &JsonValue) -> ApiResult<LockResponse> {
        Ok(LockResponse {
            tenant_id: self.parse_uuid(json, "tenant_id")?,
            lock_id: self.parse_uuid(json, "lock_id")?,
            resource_type: self.parse_string(json, "resource_type")?,
            resource_id: self.parse_uuid(json, "resource_id")?,
            holder_agent_id: self.parse_uuid(json, "holder_agent_id")?,
            acquired_at: self.parse_timestamp(json, "acquired_at")?,
            expires_at: self.parse_timestamp(json, "expires_at")?,
            mode: self.parse_string(json, "mode")?,
        })
    }

    // ========================================================================
    // MESSAGE OPERATIONS
    // ========================================================================

    /// Send a message by calling caliber_message_send.
    pub async fn message_send(&self, req: &SendMessageRequest, tenant_id: EntityId) -> ApiResult<MessageResponse> {
        let conn = self.get_conn().await?;
        let artifact_ids = req.artifact_ids.to_vec();

        let row = conn
            .query_one(
                "SELECT caliber_message_send($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
                &[
                    &req.from_agent_id,
                    &req.to_agent_id,
                    &req.to_agent_type,
                    &req.message_type,
                    &req.payload,
                    &req.trajectory_id,
                    &req.scope_id,
                    &artifact_ids,
                    &req.priority,
                    &req.expires_at.map(|ts| ts.timestamp()),
                    &tenant_id,
                ],
            )
            .await?;

        let message_id: Uuid = row.get(0);

        self.message_get(message_id).await?
            .ok_or_else(|| ApiError::internal_error("Failed to retrieve sent message"))
    }

    /// Get a message by ID by calling caliber_message_get.
    pub async fn message_get(&self, id: EntityId) -> ApiResult<Option<MessageResponse>> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one("SELECT caliber_message_get($1)", &[&id])
            .await?;

        let json_opt: Option<JsonValue> = row.get(0);

        match json_opt {
            Some(json) => {
                let response = self.parse_message_json(&json)?;
                Ok(Some(response))
            }
            None => Ok(None),
        }
    }

    /// Mark a message as acknowledged by calling caliber_message_mark_acknowledged.
    pub async fn message_acknowledge(&self, id: EntityId) -> ApiResult<()> {
        let conn = self.get_conn().await?;

        let updated: bool = conn
            .query_one("SELECT caliber_message_mark_acknowledged($1)", &[&id])
            .await?
            .get(0);

        if !updated {
            return Err(ApiError::message_not_found(id));
        }

        Ok(())
    }

    /// Mark a message as delivered by calling caliber_message_mark_delivered.
    pub async fn message_deliver(&self, id: EntityId) -> ApiResult<()> {
        let conn = self.get_conn().await?;

        let updated: bool = conn
            .query_one("SELECT caliber_message_mark_delivered($1)", &[&id])
            .await?
            .get(0);

        if !updated {
            return Err(ApiError::message_not_found(id));
        }

        Ok(())
    }

    /// List messages with filters by calling caliber_message_list.
    pub async fn message_list(
        &self,
        params: MessageListParams<'_>,
    ) -> ApiResult<Vec<MessageResponse>> {
        let conn = self.get_conn().await?;

        let filters = serde_json::json!({
            "from_agent_id": params.from_agent_id,
            "to_agent_id": params.to_agent_id,
            "to_agent_type": params.to_agent_type,
            "trajectory_id": params.trajectory_id,
            "message_type": params.message_type,
            "priority": params.priority,
            "undelivered_only": params.undelivered_only,
            "unacknowledged_only": params.unacknowledged_only,
            "limit": params.limit,
            "offset": params.offset,
        });

        let row = conn
            .query_one("SELECT caliber_message_list($1)", &[&filters])
            .await?;

        let json: JsonValue = row.get(0);
        let messages_json = json.as_array()
            .ok_or_else(|| ApiError::internal_error("Expected array from message list"))?;

        let mut messages = Vec::new();
        for msg_json in messages_json {
            messages.push(self.parse_message_json(msg_json)?);
        }

        Ok(messages)
    }

    /// List messages with filters by tenant.
    pub async fn message_list_by_tenant(
        &self,
        params: MessageListParams<'_>,
        tenant_id: EntityId,
    ) -> ApiResult<Vec<MessageResponse>> {
        let conn = self.get_conn().await?;

        let filters = serde_json::json!({
            "from_agent_id": params.from_agent_id,
            "to_agent_id": params.to_agent_id,
            "to_agent_type": params.to_agent_type,
            "trajectory_id": params.trajectory_id,
            "message_type": params.message_type,
            "priority": params.priority,
            "undelivered_only": params.undelivered_only,
            "unacknowledged_only": params.unacknowledged_only,
            "limit": params.limit,
            "offset": params.offset,
            "tenant_id": tenant_id,
        });

        let row = conn
            .query_one("SELECT caliber_message_list_by_tenant($1)", &[&filters])
            .await?;

        let json: JsonValue = row.get(0);
        let messages_json = json.as_array()
            .ok_or_else(|| ApiError::internal_error("Expected array from message list"))?;

        let mut messages = Vec::new();
        for msg_json in messages_json {
            messages.push(self.parse_message_json(msg_json)?);
        }

        Ok(messages)
    }

    /// Parse message JSON into MessageResponse.
    fn parse_message_json(&self, json: &JsonValue) -> ApiResult<MessageResponse> {
        Ok(MessageResponse {
            tenant_id: self.parse_uuid(json, "tenant_id")?,
            message_id: self.parse_uuid(json, "message_id")?,
            from_agent_id: self.parse_uuid(json, "from_agent_id")?,
            to_agent_id: self.parse_optional_uuid(json, "to_agent_id"),
            to_agent_type: self.parse_optional_string(json, "to_agent_type"),
            message_type: self.parse_string(json, "message_type")?,
            payload: self.parse_string(json, "payload")?,
            trajectory_id: self.parse_optional_uuid(json, "trajectory_id"),
            scope_id: self.parse_optional_uuid(json, "scope_id"),
            artifact_ids: self.parse_uuid_array(json, "artifact_ids")?,
            created_at: self.parse_timestamp(json, "created_at")?,
            delivered_at: self.parse_optional_timestamp(json, "delivered_at"),
            acknowledged_at: self.parse_optional_timestamp(json, "acknowledged_at"),
            priority: self.parse_string(json, "priority")?,
            expires_at: self.parse_optional_timestamp(json, "expires_at"),
        })
    }

    // ========================================================================
    // DELEGATION OPERATIONS
    // ========================================================================

    /// Create a delegation by calling caliber_delegation_create.
    pub async fn delegation_create(&self, req: &CreateDelegationRequest, tenant_id: EntityId) -> ApiResult<DelegationResponse> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one(
                "SELECT caliber_delegation_create($1, $2, $3, $4, $5, $6, $7, $8)",
                &[
                    &req.from_agent_id,
                    &req.to_agent_id,
                    &req.trajectory_id,
                    &req.scope_id,
                    &req.task_description,
                    &req.expected_completion.map(|ts| ts.timestamp()),
                    &req.context,
                    &tenant_id,
                ],
            )
            .await?;

        let delegation_id: Uuid = row.get(0);

        self.delegation_get(delegation_id).await?
            .ok_or_else(|| ApiError::internal_error("Failed to retrieve created delegation"))
    }

    /// Get a delegation by ID by calling caliber_delegation_get.
    pub async fn delegation_get(&self, id: EntityId) -> ApiResult<Option<DelegationResponse>> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one("SELECT caliber_delegation_get($1)", &[&id])
            .await?;

        let json_opt: Option<JsonValue> = row.get(0);

        match json_opt {
            Some(json) => {
                let response = self.parse_delegation_json(&json)?;
                Ok(Some(response))
            }
            None => Ok(None),
        }
    }

    /// Accept a delegation by calling caliber_delegation_accept.
    pub async fn delegation_accept(
        &self,
        id: EntityId,
        accepting_agent_id: EntityId,
    ) -> ApiResult<DelegationResponse> {
        let conn = self.get_conn().await?;

        let updated: bool = conn
            .query_one(
                "SELECT caliber_delegation_accept($1, $2)",
                &[&id, &accepting_agent_id],
            )
            .await?
            .get(0);

        if !updated {
            return Err(ApiError::entity_not_found("Delegation", id));
        }

        self.delegation_get(id).await?
            .ok_or_else(|| ApiError::entity_not_found("Delegation", id))
    }

    /// Reject a delegation by calling caliber_delegation_reject.
    pub async fn delegation_reject(
        &self,
        id: EntityId,
        reason: String,
    ) -> ApiResult<DelegationResponse> {
        let conn = self.get_conn().await?;

        let updated: bool = conn
            .query_one(
                "SELECT caliber_delegation_reject($1, $2)",
                &[&id, &reason],
            )
            .await?
            .get(0);

        if !updated {
            return Err(ApiError::entity_not_found("Delegation", id));
        }

        self.delegation_get(id).await?
            .ok_or_else(|| ApiError::entity_not_found("Delegation", id))
    }

    /// Complete a delegation by calling caliber_delegation_complete.
    pub async fn delegation_complete(
        &self,
        id: EntityId,
        result_json: JsonValue,
    ) -> ApiResult<DelegationResponse> {
        let conn = self.get_conn().await?;

        let updated: bool = conn
            .query_one(
                "SELECT caliber_delegation_complete($1, $2)",
                &[&id, &result_json],
            )
            .await?
            .get(0);

        if !updated {
            return Err(ApiError::entity_not_found("Delegation", id));
        }

        self.delegation_get(id).await?
            .ok_or_else(|| ApiError::entity_not_found("Delegation", id))
    }

    /// Parse delegation JSON into DelegationResponse.
    fn parse_delegation_json(&self, json: &JsonValue) -> ApiResult<DelegationResponse> {
        Ok(DelegationResponse {
            delegation_id: self.parse_uuid(json, "delegation_id")?,
            tenant_id: self.parse_optional_uuid(json, "tenant_id"),
            from_agent_id: self.parse_uuid(json, "from_agent_id")?,
            to_agent_id: self.parse_uuid(json, "to_agent_id")?,
            trajectory_id: self.parse_uuid(json, "trajectory_id")?,
            scope_id: self.parse_uuid(json, "scope_id")?,
            task_description: self.parse_string(json, "task_description")?,
            status: self.parse_string(json, "status")?,
            created_at: self.parse_timestamp(json, "created_at")?,
            accepted_at: self.parse_optional_timestamp(json, "accepted_at"),
            completed_at: self.parse_optional_timestamp(json, "completed_at"),
            expected_completion: self.parse_optional_timestamp(json, "expected_completion"),
            result: json.get("result").and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    serde_json::from_value(v.clone()).ok()
                }
            }),
            context: json.get("context").and_then(|v| if v.is_null() { None } else { Some(v.clone()) }),
        })
    }

    // ========================================================================
    // HANDOFF OPERATIONS
    // ========================================================================

    /// Create a handoff by calling caliber_handoff_create.
    pub async fn handoff_create(&self, req: &CreateHandoffRequest, tenant_id: EntityId) -> ApiResult<HandoffResponse> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one(
                "SELECT caliber_handoff_create($1, $2, $3, $4, $5, $6, $7)",
                &[
                    &req.from_agent_id,
                    &req.to_agent_id,
                    &req.trajectory_id,
                    &req.scope_id,
                    &req.reason,
                    &req.context_snapshot,
                    &tenant_id,
                ],
            )
            .await?;

        let handoff_id: Uuid = row.get(0);

        self.handoff_get(handoff_id).await?
            .ok_or_else(|| ApiError::internal_error("Failed to retrieve created handoff"))
    }

    /// Get a handoff by ID by calling caliber_handoff_get.
    pub async fn handoff_get(&self, id: EntityId) -> ApiResult<Option<HandoffResponse>> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one("SELECT caliber_handoff_get($1)", &[&id])
            .await?;

        let json_opt: Option<JsonValue> = row.get(0);

        match json_opt {
            Some(json) => {
                let response = self.parse_handoff_json(&json)?;
                Ok(Some(response))
            }
            None => Ok(None),
        }
    }

    /// Accept a handoff by calling caliber_handoff_accept.
    pub async fn handoff_accept(
        &self,
        id: EntityId,
        accepting_agent_id: EntityId,
    ) -> ApiResult<HandoffResponse> {
        let conn = self.get_conn().await?;

        let updated: bool = conn
            .query_one(
                "SELECT caliber_handoff_accept($1, $2)",
                &[&id, &accepting_agent_id],
            )
            .await?
            .get(0);

        if !updated {
            return Err(ApiError::entity_not_found("Handoff", id));
        }

        self.handoff_get(id).await?
            .ok_or_else(|| ApiError::entity_not_found("Handoff", id))
    }

    /// Complete a handoff by calling caliber_handoff_complete.
    pub async fn handoff_complete(&self, id: EntityId) -> ApiResult<HandoffResponse> {
        let conn = self.get_conn().await?;

        let updated: bool = conn
            .query_one("SELECT caliber_handoff_complete($1)", &[&id])
            .await?
            .get(0);

        if !updated {
            return Err(ApiError::entity_not_found("Handoff", id));
        }

        self.handoff_get(id).await?
            .ok_or_else(|| ApiError::entity_not_found("Handoff", id))
    }

    /// Parse handoff JSON into HandoffResponse.
    fn parse_handoff_json(&self, json: &JsonValue) -> ApiResult<HandoffResponse> {
        Ok(HandoffResponse {
            tenant_id: self.parse_uuid(json, "tenant_id")?,
            handoff_id: self.parse_uuid(json, "handoff_id")?,
            from_agent_id: self.parse_uuid(json, "from_agent_id")?,
            to_agent_id: self.parse_uuid(json, "to_agent_id")?,
            trajectory_id: self.parse_uuid(json, "trajectory_id")?,
            scope_id: self.parse_uuid(json, "scope_id")?,
            reason: self.parse_string(json, "reason")?,
            status: self.parse_string(json, "status")?,
            created_at: self.parse_timestamp(json, "created_at")?,
            accepted_at: self.parse_optional_timestamp(json, "accepted_at"),
            completed_at: self.parse_optional_timestamp(json, "completed_at"),
            context_snapshot: self.parse_bytes(json, "context_snapshot")?,
        })
    }

    // ========================================================================
    // JSON PARSING HELPERS
    // ========================================================================

    /// Parse a UUID from JSON field.
    fn parse_uuid(&self, json: &JsonValue, field: &str) -> ApiResult<EntityId> {
        let uuid_str = json
            .get(field)
            .and_then(|v| v.as_str())
            .ok_or_else(|| ApiError::internal_error(format!("Missing or invalid field: {}", field)))?;
        
        let uuid = Uuid::parse_str(uuid_str)
            .map_err(|_| ApiError::internal_error(format!("Invalid UUID in field: {}", field)))?;
        
        Ok(uuid)
    }

    /// Parse an optional UUID from JSON field.
    fn parse_optional_uuid(&self, json: &JsonValue, field: &str) -> Option<EntityId> {
        json.get(field)
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
    }

    /// Parse a string from JSON field.
    fn parse_string(&self, json: &JsonValue, field: &str) -> ApiResult<String> {
        json.get(field)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| ApiError::internal_error(format!("Missing or invalid field: {}", field)))
    }

    /// Parse an optional string from JSON field.
    fn parse_optional_string(&self, json: &JsonValue, field: &str) -> Option<String> {
        json.get(field)
            .and_then(|v| if v.is_null() { None } else { v.as_str() })
            .map(|s| s.to_string())
    }

    /// Parse a boolean from JSON field.
    fn parse_bool(&self, json: &JsonValue, field: &str) -> ApiResult<bool> {
        json.get(field)
            .and_then(|v| v.as_bool())
            .ok_or_else(|| ApiError::internal_error(format!("Missing or invalid field: {}", field)))
    }

    /// Parse an i32 from JSON field.
    fn parse_i32(&self, json: &JsonValue, field: &str) -> ApiResult<i32> {
        json.get(field)
            .and_then(|v| v.as_i64())
            .map(|n| n as i32)
            .ok_or_else(|| ApiError::internal_error(format!("Missing or invalid field: {}", field)))
    }

    /// Parse a timestamp from JSON field.
    fn parse_timestamp(&self, json: &JsonValue, field: &str) -> ApiResult<Timestamp> {
        let ts_str = json
            .get(field)
            .and_then(|v| v.as_str())
            .ok_or_else(|| ApiError::internal_error(format!("Missing or invalid field: {}", field)))?;
        
        chrono::DateTime::parse_from_rfc3339(ts_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .map_err(|_| ApiError::internal_error(format!("Invalid timestamp in field: {}", field)))
    }

    /// Parse an optional timestamp from JSON field.
    fn parse_optional_timestamp(&self, json: &JsonValue, field: &str) -> Option<Timestamp> {
        json.get(field)
            .and_then(|v| if v.is_null() { None } else { v.as_str() })
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
    }

    /// Parse a trajectory status from JSON field.
    fn parse_trajectory_status(&self, json: &JsonValue, field: &str) -> ApiResult<caliber_core::TrajectoryStatus> {
        let status_str = self.parse_string(json, field)?;
        match status_str.as_str() {
            "active" => Ok(caliber_core::TrajectoryStatus::Active),
            "completed" => Ok(caliber_core::TrajectoryStatus::Completed),
            "failed" => Ok(caliber_core::TrajectoryStatus::Failed),
            "suspended" => Ok(caliber_core::TrajectoryStatus::Suspended),
            _ => Err(ApiError::internal_error(format!("Invalid trajectory status: {}", status_str))),
        }
    }

    /// Parse an artifact type from JSON field.
    fn parse_artifact_type(&self, json: &JsonValue, field: &str) -> ApiResult<caliber_core::ArtifactType> {
        let type_str = self.parse_string(json, field)?;
        // Parse the Debug format back to enum
        match type_str.as_str() {
            "ErrorLog" => Ok(caliber_core::ArtifactType::ErrorLog),
            "CodePatch" => Ok(caliber_core::ArtifactType::CodePatch),
            "DesignDecision" => Ok(caliber_core::ArtifactType::DesignDecision),
            "UserPreference" => Ok(caliber_core::ArtifactType::UserPreference),
            "Fact" => Ok(caliber_core::ArtifactType::Fact),
            "Constraint" => Ok(caliber_core::ArtifactType::Constraint),
            "ToolResult" => Ok(caliber_core::ArtifactType::ToolResult),
            "IntermediateOutput" => Ok(caliber_core::ArtifactType::IntermediateOutput),
            "Custom" => Ok(caliber_core::ArtifactType::Custom),
            "Code" => Ok(caliber_core::ArtifactType::Code),
            "Document" => Ok(caliber_core::ArtifactType::Document),
            "Data" => Ok(caliber_core::ArtifactType::Data),
            "Config" => Ok(caliber_core::ArtifactType::Config),
            "Log" => Ok(caliber_core::ArtifactType::Log),
            "Summary" => Ok(caliber_core::ArtifactType::Summary),
            "Decision" => Ok(caliber_core::ArtifactType::Decision),
            "Plan" => Ok(caliber_core::ArtifactType::Plan),
            _ => Err(ApiError::internal_error(format!("Invalid artifact type: {}", type_str))),
        }
    }

    /// Parse a note type from JSON field.
    fn parse_note_type(&self, json: &JsonValue, field: &str) -> ApiResult<caliber_core::NoteType> {
        let type_str = self.parse_string(json, field)?;
        match type_str.as_str() {
            "Convention" => Ok(caliber_core::NoteType::Convention),
            "Strategy" => Ok(caliber_core::NoteType::Strategy),
            "Gotcha" => Ok(caliber_core::NoteType::Gotcha),
            "Fact" => Ok(caliber_core::NoteType::Fact),
            "Preference" => Ok(caliber_core::NoteType::Preference),
            "Relationship" => Ok(caliber_core::NoteType::Relationship),
            "Procedure" => Ok(caliber_core::NoteType::Procedure),
            "Meta" => Ok(caliber_core::NoteType::Meta),
            _ => Err(ApiError::internal_error(format!("Invalid note type: {}", type_str))),
        }
    }

    /// Parse a turn role from JSON field.
    fn parse_turn_role(&self, json: &JsonValue, field: &str) -> ApiResult<caliber_core::TurnRole> {
        let role_str = self.parse_string(json, field)?;
        match role_str.as_str() {
            "user" => Ok(caliber_core::TurnRole::User),
            "assistant" => Ok(caliber_core::TurnRole::Assistant),
            "system" => Ok(caliber_core::TurnRole::System),
            "tool" => Ok(caliber_core::TurnRole::Tool),
            _ => Err(ApiError::internal_error(format!("Invalid turn role: {}", role_str))),
        }
    }

    /// Parse a content hash from JSON field.
    fn parse_content_hash(&self, json: &JsonValue, field: &str) -> ApiResult<[u8; 32]> {
        let hash_array = json
            .get(field)
            .and_then(|v| v.as_array())
            .ok_or_else(|| ApiError::internal_error(format!("Missing or invalid field: {}", field)))?;
        
        if hash_array.len() != 32 {
            return Err(ApiError::internal_error(format!("Invalid hash length in field: {}", field)));
        }

        let mut hash = [0u8; 32];
        for (i, byte_val) in hash_array.iter().enumerate() {
            hash[i] = byte_val.as_u64().unwrap_or(0) as u8;
        }
        
        Ok(hash)
    }

    /// Parse bytes from JSON field.
    fn parse_bytes(&self, json: &JsonValue, field: &str) -> ApiResult<Vec<u8>> {
        let byte_array = json
            .get(field)
            .and_then(|v| v.as_array())
            .ok_or_else(|| ApiError::internal_error(format!("Missing or invalid field: {}", field)))?;
        
        Ok(byte_array.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect())
    }

    /// Parse a UUID array from JSON field.
    fn parse_uuid_array(&self, json: &JsonValue, field: &str) -> ApiResult<Vec<EntityId>> {
        let array = json
            .get(field)
            .and_then(|v| v.as_array())
            .ok_or_else(|| ApiError::internal_error(format!("Missing or invalid field: {}", field)))?;
        
        let mut uuids = Vec::new();
        for item in array {
            if let Some(uuid_str) = item.as_str() {
                if let Ok(uuid) = Uuid::parse_str(uuid_str) {
                    uuids.push(uuid);
                }
            }
        }
        
        Ok(uuids)
    }

    /// Parse a string array from JSON field.
    fn parse_string_array(&self, json: &JsonValue, field: &str) -> ApiResult<Vec<String>> {
        let array = json
            .get(field)
            .and_then(|v| v.as_array())
            .ok_or_else(|| ApiError::internal_error(format!("Missing or invalid field: {}", field)))?;
        
        Ok(array.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
    }

    /// Parse an optional trajectory outcome from JSON field.
    fn parse_optional_outcome(&self, json: &JsonValue, field: &str) -> Option<TrajectoryOutcomeResponse> {
        json.get(field).and_then(|v| {
            if v.is_null() {
                None
            } else {
                Some(TrajectoryOutcomeResponse {
                    status: v.get("status")
                        .and_then(|s| s.as_str())
                        .and_then(|s| match s {
                            "success" => Some(caliber_core::OutcomeStatus::Success),
                            "failure" => Some(caliber_core::OutcomeStatus::Failure),
                            "partial" => Some(caliber_core::OutcomeStatus::Partial),
                            _ => None,
                        })
                        .unwrap_or(caliber_core::OutcomeStatus::Success),
                    summary: v.get("summary").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                    produced_artifacts: v.get("produced_artifacts")
                        .and_then(|a| a.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|id| id.as_str().and_then(|s| Uuid::parse_str(s).ok()))
                                .collect()
                        })
                        .unwrap_or_default(),
                    produced_notes: v.get("produced_notes")
                        .and_then(|a| a.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|id| id.as_str().and_then(|s| Uuid::parse_str(s).ok()))
                                .collect()
                        })
                        .unwrap_or_default(),
                    error: v.get("error").and_then(|e| e.as_str()).map(|s| s.to_string()),
                })
            }
        })
    }

    /// Parse an optional checkpoint from JSON field.
    fn parse_optional_checkpoint(&self, json: &JsonValue, field: &str) -> Option<CheckpointResponse> {
        json.get(field).and_then(|v| {
            if v.is_null() {
                None
            } else {
                Some(CheckpointResponse {
                    context_state: v.get("context_state")
                        .and_then(|cs| cs.as_array())
                        .map(|arr| arr.iter().filter_map(|b| b.as_u64().map(|n| n as u8)).collect())
                        .unwrap_or_default(),
                    recoverable: v.get("recoverable").and_then(|r| r.as_bool()).unwrap_or(false),
                })
            }
        })
    }

    /// Parse an optional embedding from JSON field.
    fn parse_optional_embedding(&self, json: &JsonValue, field: &str) -> Option<EmbeddingResponse> {
        json.get(field).and_then(|v| {
            if v.is_null() {
                None
            } else {
                Some(EmbeddingResponse {
                    data: v.get("data")
                        .and_then(|d| d.as_array())
                        .map(|arr| arr.iter().filter_map(|f| f.as_f64().map(|n| n as f32)).collect())
                        .unwrap_or_default(),
                    model_id: v.get("model_id").and_then(|m| m.as_str()).unwrap_or("").to_string(),
                    dimensions: v.get("dimensions").and_then(|d| d.as_i64()).unwrap_or(0) as i32,
                })
            }
        })
    }

    /// Parse provenance from JSON field.
    fn parse_provenance(&self, json: &JsonValue, field: &str) -> ApiResult<ProvenanceResponse> {
        let prov = json
            .get(field)
            .ok_or_else(|| ApiError::internal_error(format!("Missing field: {}", field)))?;
        
        Ok(ProvenanceResponse {
            source_turn: prov.get("source_turn").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            extraction_method: prov.get("extraction_method")
                .and_then(|v| v.as_str())
                .and_then(|s| match s {
                    "explicit" => Some(caliber_core::ExtractionMethod::Explicit),
                    "inferred" => Some(caliber_core::ExtractionMethod::Inferred),
                    "user_provided" => Some(caliber_core::ExtractionMethod::UserProvided),
                    _ => None,
                })
                .unwrap_or(caliber_core::ExtractionMethod::Explicit),
            confidence: prov.get("confidence").and_then(|v| v.as_f64()).map(|f| f as f32),
        })
    }

    /// Parse TTL from JSON field.
    fn parse_ttl(&self, json: &JsonValue, field: &str) -> ApiResult<caliber_core::TTL> {
        let ttl_str = self.parse_string(json, field)?;
        match ttl_str.as_str() {
            "persistent" => Ok(caliber_core::TTL::Persistent),
            "session" => Ok(caliber_core::TTL::Session),
            "scope" => Ok(caliber_core::TTL::Scope),
            "ephemeral" => Ok(caliber_core::TTL::Ephemeral),
            "short_term" => Ok(caliber_core::TTL::ShortTerm),
            "medium_term" => Ok(caliber_core::TTL::MediumTerm),
            "long_term" => Ok(caliber_core::TTL::LongTerm),
            "permanent" => Ok(caliber_core::TTL::Permanent),
            s if s.starts_with("duration_") => {
                let ms_str = s.trim_start_matches("duration_");
                let ms = ms_str.parse::<i64>()
                    .map_err(|_| ApiError::internal_error(format!("Invalid TTL duration: {}", s)))?;
                Ok(caliber_core::TTL::Duration(ms))
            }
            _ => Err(ApiError::internal_error(format!("Invalid TTL: {}", ttl_str))),
        }
    }

    /// Convert TTL to string for database calls.
    fn ttl_to_string(&self, ttl: &caliber_core::TTL) -> String {
        match ttl {
            caliber_core::TTL::Persistent => "persistent".to_string(),
            caliber_core::TTL::Session => "session".to_string(),
            caliber_core::TTL::Scope => "scope".to_string(),
            caliber_core::TTL::Ephemeral => "ephemeral".to_string(),
            caliber_core::TTL::ShortTerm => "short_term".to_string(),
            caliber_core::TTL::MediumTerm => "medium_term".to_string(),
            caliber_core::TTL::LongTerm => "long_term".to_string(),
            caliber_core::TTL::Permanent => "permanent".to_string(),
            caliber_core::TTL::Duration(ms) => format!("duration_{}", ms),
        }
    }

    // ========================================================================
    // ADDITIONAL CRUD OPERATIONS (for batch endpoints)
    // ========================================================================

    /// Delete a trajectory by calling caliber_trajectory_delete.
    pub async fn trajectory_delete(&self, id: EntityId) -> ApiResult<()> {
        let conn = self.get_conn().await?;

        let deleted: bool = conn
            .query_one("SELECT caliber_trajectory_delete($1)", &[&id])
            .await?
            .get(0);

        if !deleted {
            return Err(ApiError::trajectory_not_found(id));
        }

        Ok(())
    }

    /// List child trajectories by calling caliber_trajectory_list_children.
    pub async fn trajectory_list_children(&self, id: EntityId) -> ApiResult<Vec<TrajectoryResponse>> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one("SELECT caliber_trajectory_list_children($1)", &[&id])
            .await?;

        let json: JsonValue = row.get(0);
        let trajectories_json = json.as_array()
            .ok_or_else(|| ApiError::internal_error("Expected array from trajectory children list"))?;

        let mut trajectories = Vec::new();
        for traj_json in trajectories_json {
            trajectories.push(self.parse_trajectory_json(traj_json)?);
        }

        Ok(trajectories)
    }

    /// Health check - verifies database connectivity.
    pub async fn health_check(&self) -> ApiResult<()> {
        let conn = self.get_conn().await?;

        // Simple query to verify connectivity
        conn.query_one("SELECT 1", &[]).await?;

        Ok(())
    }

    // ========================================================================
    // BATTLE INTEL: EDGE OPERATIONS
    // ========================================================================

    /// Get an edge by ID by calling caliber_edge_get.
    pub async fn edge_get(&self, id: EntityId) -> ApiResult<Option<EdgeResponse>> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one("SELECT caliber_edge_get($1)", &[&id])
            .await?;

        let json_value: Option<JsonValue> = row.get(0);

        match json_value {
            Some(json) => {
                let edge: EdgeResponse = serde_json::from_value(json)?;
                Ok(Some(edge))
            }
            None => Ok(None),
        }
    }

    // ========================================================================
    // BATTLE INTEL: SUMMARIZATION POLICY OPERATIONS
    // ========================================================================

    /// Create a summarization policy by calling caliber_summarization_policy_create.
    pub async fn summarization_policy_create(
        &self,
        req: &CreateSummarizationPolicyRequest,
    ) -> ApiResult<SummarizationPolicyResponse> {
        let conn = self.get_conn().await?;

        let triggers_json = serde_json::to_value(&req.triggers)?;
        let source_level_str = format!("{:?}", req.source_level);
        let target_level_str = format!("{:?}", req.target_level);

        let row = conn
            .query_one(
                "SELECT caliber_summarization_policy_create($1, $2, $3, $4, $5, $6, $7, $8)",
                &[
                    &req.name,
                    &triggers_json,
                    &source_level_str,
                    &target_level_str,
                    &req.max_sources,
                    &req.create_edges,
                    &req.trajectory_id,
                    &req.metadata,
                ],
            )
            .await?;

        let policy_id: Option<Uuid> = row.get(0);
        let policy_id = policy_id.ok_or_else(|| ApiError::internal_error("Failed to create policy"))?;

        self.summarization_policy_get(policy_id)
            .await?
            .ok_or_else(|| ApiError::internal_error("Policy created but not found"))
    }

    /// Get a summarization policy by ID.
    pub async fn summarization_policy_get(
        &self,
        id: EntityId,
    ) -> ApiResult<Option<SummarizationPolicyResponse>> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one("SELECT caliber_summarization_policy_get($1)", &[&id])
            .await?;

        let json_value: Option<JsonValue> = row.get(0);

        match json_value {
            Some(json) => {
                let policy: SummarizationPolicyResponse = serde_json::from_value(json)?;
                Ok(Some(policy))
            }
            None => Ok(None),
        }
    }

    /// List summarization policies for a trajectory.
    pub async fn summarization_policies_for_trajectory(
        &self,
        trajectory_id: EntityId,
    ) -> ApiResult<Vec<SummarizationPolicyResponse>> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one(
                "SELECT caliber_summarization_policies_by_trajectory($1)",
                &[&trajectory_id],
            )
            .await?;

        let json_value: JsonValue = row.get(0);
        let policies: Vec<SummarizationPolicyResponse> = serde_json::from_value(json_value)?;

        Ok(policies)
    }

    /// Delete a summarization policy by ID.
    pub async fn summarization_policy_delete(&self, id: EntityId) -> ApiResult<()> {
        let conn = self.get_conn().await?;

        let deleted: bool = conn
            .query_one("SELECT caliber_summarization_policy_delete($1)", &[&id])
            .await?
            .get(0);

        if !deleted {
            return Err(ApiError::entity_not_found("SummarizationPolicy", id));
        }

        Ok(())
    }

    // ========================================================================
    // DSL OPERATIONS
    // ========================================================================

    /// Validate DSL source.
    pub async fn dsl_validate(&self, req: &ValidateDslRequest) -> ApiResult<ValidateDslResponse> {
        // Use caliber_dsl to validate the source
        let parse_result = caliber_dsl::parse(&req.source);

        match parse_result {
            Ok(ast) => {
                let ast_json = serde_json::to_value(&ast)?;
                Ok(ValidateDslResponse {
                    valid: true,
                    errors: vec![],
                    ast: Some(ast_json),
                })
            }
            Err(e) => {
                Ok(ValidateDslResponse {
                    valid: false,
                    errors: vec![ParseErrorResponse {
                        line: 0,
                        column: 0,
                        message: e.to_string(),
                    }],
                    ast: None,
                })
            }
        }
    }

    /// Parse DSL source and return AST.
    pub async fn dsl_parse(&self, req: &ParseDslRequest) -> ApiResult<ValidateDslResponse> {
        // Same as validate but focused on parsing
        let validate_req = ValidateDslRequest {
            source: req.source.clone(),
        };
        self.dsl_validate(&validate_req).await
    }

    // ========================================================================
    // CONFIG OPERATIONS
    // ========================================================================

    /// Get current configuration.
    pub async fn config_get(&self) -> ApiResult<ConfigResponse> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one("SELECT caliber_config_get()", &[])
            .await?;

        let json: JsonValue = row.get(0);

        Ok(ConfigResponse {
            config: json,
            valid: true,
            errors: vec![],
        })
    }

    /// Update configuration.
    pub async fn config_update(&self, req: &UpdateConfigRequest) -> ApiResult<ConfigResponse> {
        let conn = self.get_conn().await?;

        let config_str = serde_json::to_string(&req.config)?;

        let updated: bool = conn
            .query_one("SELECT caliber_config_update($1::jsonb)", &[&config_str])
            .await?
            .get(0);

        if !updated {
            return Err(ApiError::internal_error("Failed to update config"));
        }

        self.config_get().await
    }

    /// Validate configuration without applying.
    pub async fn config_validate(&self, req: &ValidateConfigRequest) -> ApiResult<ConfigResponse> {
        // Validate the config structure
        // For now, just check if it's valid JSON (which it is if we got here)
        Ok(ConfigResponse {
            config: req.config.clone(),
            valid: true,
            errors: vec![],
        })
    }

    // ========================================================================
    // TENANT OPERATIONS
    // ========================================================================

    /// List all tenants.
    pub async fn tenant_list(&self) -> ApiResult<Vec<TenantInfo>> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one("SELECT caliber_tenant_list()", &[])
            .await?;

        let json: JsonValue = row.get(0);
        let tenants: Vec<TenantInfo> = serde_json::from_value(json)?;

        Ok(tenants)
    }

    /// Get a tenant by ID.
    pub async fn tenant_get(&self, id: EntityId) -> ApiResult<Option<TenantInfo>> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one("SELECT caliber_tenant_get($1)", &[&id])
            .await?;

        let json_opt: Option<JsonValue> = row.get(0);

        match json_opt {
            Some(json) => {
                let tenant: TenantInfo = serde_json::from_value(json)?;
                Ok(Some(tenant))
            }
            None => Ok(None),
        }
    }

    // ========================================================================
    // SEARCH OPERATIONS
    // ========================================================================

    /// Search across entities.
    pub async fn search(&self, req: &SearchRequest, tenant_id: EntityId) -> ApiResult<SearchResponse> {
        let conn = self.get_conn().await?;

        let query_json = serde_json::to_string(&req)?;

        let row = conn
            .query_one("SELECT caliber_search($1::jsonb, $2)", &[&query_json, &tenant_id])
            .await?;

        let json: JsonValue = row.get(0);
        let response: SearchResponse = serde_json::from_value(json)?;

        Ok(response)
    }

    // ========================================================================
    // USER OPERATIONS
    // ========================================================================

    /// Get API key for a user.
    pub async fn user_get_api_key(&self, user_id: &str) -> ApiResult<Option<String>> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_opt(
                "SELECT api_key FROM caliber_users WHERE user_id = $1",
                &[&user_id],
            )
            .await?;

        Ok(row.and_then(|r| r.get::<_, Option<String>>(0)))
    }

    /// Set API key for a user (creates user if doesn't exist).
    pub async fn user_set_api_key(&self, user_id: &str, api_key: &str) -> ApiResult<()> {
        let conn = self.get_conn().await?;

        conn.execute(
            "INSERT INTO caliber_users (user_id, api_key, created_at, updated_at)
             VALUES ($1, $2, NOW(), NOW())
             ON CONFLICT (user_id) DO UPDATE SET api_key = $2, updated_at = NOW()",
            &[&user_id, &api_key],
        )
        .await?;

        Ok(())
    }

    // ========================================================================
    // BILLING OPERATIONS
    // ========================================================================

    /// Get billing status for a tenant.
    pub async fn billing_get_status(&self, tenant_id: EntityId) -> ApiResult<crate::routes::billing::BillingStatus> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_opt(
                "SELECT plan, trial_ends_at, storage_used_bytes, storage_limit_bytes,
                        hot_cache_used_bytes, hot_cache_limit_bytes
                 FROM caliber_billing WHERE tenant_id = $1",
                &[&tenant_id],
            )
            .await?;

        match row {
            Some(r) => {
                use crate::routes::billing::{BillingPlan, BillingStatus};

                let plan_str: Option<String> = r.get(0);
                let plan = match plan_str.as_deref() {
                    Some("pro") => BillingPlan::Pro,
                    Some("enterprise") => BillingPlan::Enterprise,
                    _ => BillingPlan::Trial,
                };

                Ok(BillingStatus {
                    tenant_id,
                    plan,
                    trial_ends_at: r.get(1),
                    storage_used_bytes: r.get::<_, Option<i64>>(2).unwrap_or(0),
                    storage_limit_bytes: r.get::<_, Option<i64>>(3).unwrap_or(1024 * 1024 * 1024),
                    hot_cache_used_bytes: r.get::<_, Option<i64>>(4).unwrap_or(0),
                    hot_cache_limit_bytes: r.get::<_, Option<i64>>(5).unwrap_or(100 * 1024 * 1024),
                })
            }
            None => {
                // Return default billing status for new tenants
                use crate::routes::billing::{BillingPlan, BillingStatus};
                Ok(BillingStatus {
                    tenant_id,
                    plan: BillingPlan::Trial,
                    trial_ends_at: Some(chrono::Utc::now() + chrono::Duration::days(14)),
                    storage_used_bytes: 0,
                    storage_limit_bytes: 1024 * 1024 * 1024, // 1 GB
                    hot_cache_used_bytes: 0,
                    hot_cache_limit_bytes: 100 * 1024 * 1024, // 100 MB
                })
            }
        }
    }

    /// Update billing plan for a tenant.
    pub async fn billing_update_plan(
        &self,
        tenant_id: EntityId,
        plan: crate::routes::billing::BillingPlan,
    ) -> ApiResult<()> {
        let conn = self.get_conn().await?;

        let plan_str = match plan {
            crate::routes::billing::BillingPlan::Trial => "trial",
            crate::routes::billing::BillingPlan::Pro => "pro",
            crate::routes::billing::BillingPlan::Enterprise => "enterprise",
        };

        conn.execute(
            "INSERT INTO caliber_billing (tenant_id, plan, updated_at)
             VALUES ($1, $2, NOW())
             ON CONFLICT (tenant_id) DO UPDATE SET plan = $2, updated_at = NOW()",
            &[&tenant_id, &plan_str],
        )
        .await?;

        Ok(())
    }

    /// Get LemonSqueezy customer ID for a tenant.
    pub async fn billing_get_customer_id(&self, tenant_id: EntityId) -> ApiResult<String> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_opt(
                "SELECT customer_id FROM caliber_billing WHERE tenant_id = $1 AND customer_id IS NOT NULL",
                &[&tenant_id],
            )
            .await?;

        row.and_then(|r| r.get::<_, Option<String>>(0))
            .ok_or_else(|| ApiError::not_found("No customer ID found for tenant"))
    }

    /// Set LemonSqueezy customer ID for a tenant.
    pub async fn billing_set_customer_id(&self, tenant_id: EntityId, customer_id: &str) -> ApiResult<()> {
        let conn = self.get_conn().await?;

        conn.execute(
            "INSERT INTO caliber_billing (tenant_id, customer_id, updated_at)
             VALUES ($1, $2, NOW())
             ON CONFLICT (tenant_id) DO UPDATE SET customer_id = $2, updated_at = NOW()",
            &[&tenant_id, &customer_id],
        )
        .await?;

        Ok(())
    }

    // ========================================================================
    // TENANT OPERATIONS (for SSO auto-provisioning)
    // ========================================================================

    /// Check if an email domain is a public domain (gmail, outlook, etc.).
    pub async fn is_public_email_domain(&self, domain: &str) -> ApiResult<bool> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one(
                "SELECT caliber_is_public_email_domain($1)",
                &[&domain],
            )
            .await?;

        Ok(row.get(0))
    }

    /// Create a new tenant.
    pub async fn tenant_create(
        &self,
        name: &str,
        domain: Option<&str>,
        workos_organization_id: Option<&str>,
    ) -> ApiResult<EntityId> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one(
                "SELECT caliber_tenant_create($1, $2, $3)",
                &[&name, &domain, &workos_organization_id],
            )
            .await?;

        let tenant_id: Uuid = row.get(0);
        Ok(tenant_id)
    }

    /// Get a tenant by domain.
    pub async fn tenant_get_by_domain(&self, domain: &str) -> ApiResult<Option<TenantInfo>> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one(
                "SELECT caliber_tenant_get_by_domain($1)",
                &[&domain],
            )
            .await?;

        let json_opt: Option<JsonValue> = row.get(0);

        match json_opt {
            Some(json) => Ok(Some(self.parse_tenant_json(&json)?)),
            None => Ok(None),
        }
    }

    /// Get a tenant by WorkOS organization ID.
    pub async fn tenant_get_by_workos_org(&self, workos_org_id: &str) -> ApiResult<Option<TenantInfo>> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one(
                "SELECT caliber_tenant_get_by_workos_org($1)",
                &[&workos_org_id],
            )
            .await?;

        let json_opt: Option<JsonValue> = row.get(0);

        match json_opt {
            Some(json) => Ok(Some(self.parse_tenant_json(&json)?)),
            None => Ok(None),
        }
    }

    /// Upsert a tenant member (insert or update).
    pub async fn tenant_member_upsert(
        &self,
        tenant_id: EntityId,
        user_id: &str,
        email: &str,
        role: &str,
        first_name: Option<&str>,
        last_name: Option<&str>,
    ) -> ApiResult<EntityId> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one(
                "SELECT caliber_tenant_member_upsert($1, $2, $3, $4, $5, $6)",
                &[&tenant_id, &user_id, &email, &role, &first_name, &last_name],
            )
            .await?;

        let member_id: Uuid = row.get(0);
        Ok(member_id)
    }

    /// Count the number of members in a tenant.
    pub async fn tenant_member_count(&self, tenant_id: EntityId) -> ApiResult<i32> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one(
                "SELECT caliber_tenant_member_count($1)",
                &[&tenant_id],
            )
            .await?;

        Ok(row.get(0))
    }

    /// Parse tenant JSON into TenantInfo struct.
    fn parse_tenant_json(&self, json: &JsonValue) -> ApiResult<crate::types::TenantInfo> {
        let status_str = self.parse_string(json, "status")?;
        let status = status_str.parse::<crate::types::TenantStatus>()
            .map_err(|_| ApiError::internal_error(format!("Invalid tenant status: {}", status_str)))?;

        // Parse created_at, default to now if not present
        let created_at = json.get("created_at")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);

        Ok(crate::types::TenantInfo {
            tenant_id: self.parse_uuid(json, "tenant_id")?,
            name: self.parse_string(json, "name")?,
            domain: self.parse_optional_string(json, "domain"),
            workos_organization_id: self.parse_optional_string(json, "workos_organization_id"),
            status,
            created_at,
        })
    }

    // ========================================================================
    // CHANGE JOURNAL OPERATIONS (Cache Invalidation)
    // ========================================================================

    /// Get the current watermark (highest change_id) for a tenant.
    ///
    /// This is used to establish a baseline for cache invalidation polling.
    /// Returns 0 if no changes have been recorded for the tenant.
    pub async fn current_watermark(&self, tenant_id: EntityId) -> ApiResult<i64> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one(
                "SELECT caliber_current_watermark($1)",
                &[&tenant_id],
            )
            .await?;

        let watermark: i64 = row.get(0);
        Ok(watermark)
    }

    /// Check if there are any changes since the given watermark.
    ///
    /// This is a lightweight check that can be used for efficient polling.
    /// Optionally filter by entity types for selective cache invalidation.
    pub async fn has_changes_since(
        &self,
        tenant_id: EntityId,
        watermark: i64,
        entity_types: Option<&[&str]>,
    ) -> ApiResult<bool> {
        let conn = self.get_conn().await?;

        let entity_types_arr: Option<Vec<String>> = entity_types.map(|types| {
            types.iter().map(|s| s.to_string()).collect()
        });

        let row = conn
            .query_one(
                "SELECT caliber_has_changes_since($1, $2, $3)",
                &[&tenant_id, &watermark, &entity_types_arr],
            )
            .await?;

        let has_changes: bool = row.get(0);
        Ok(has_changes)
    }

    /// Get all changes since the given watermark.
    ///
    /// Returns a list of change records in ascending order by change_id.
    /// Use the limit parameter to paginate through large result sets.
    pub async fn changes_since(
        &self,
        tenant_id: EntityId,
        watermark: i64,
        limit: i32,
    ) -> ApiResult<Vec<crate::types::ChangeRecord>> {
        let conn = self.get_conn().await?;

        let rows = conn
            .query(
                "SELECT change_id, entity_type, entity_id, operation, changed_at
                 FROM caliber_changes_since($1, $2, $3)",
                &[&tenant_id, &watermark, &limit],
            )
            .await?;

        let mut changes = Vec::with_capacity(rows.len());
        for row in rows {
            let change_id: i64 = row.get(0);
            let entity_type: String = row.get(1);
            let entity_id: Uuid = row.get(2);
            let operation_str: String = row.get(3);
            let changed_at: chrono::DateTime<chrono::Utc> = row.get(4);

            let operation = operation_str.parse::<crate::types::ChangeOperation>()
                .map_err(|e| ApiError::internal_error(format!("Invalid operation: {}", e)))?;

            changes.push(crate::types::ChangeRecord {
                change_id,
                tenant_id,
                entity_type,
                entity_id,
                operation,
                changed_at,
            });
        }

        Ok(changes)
    }

    /// Get changes since watermark with response metadata.
    ///
    /// This is a convenience method that also returns the new watermark
    /// and whether there are more changes beyond the limit.
    pub async fn changes_since_with_metadata(
        &self,
        tenant_id: EntityId,
        watermark: i64,
        limit: i32,
    ) -> ApiResult<crate::types::ChangesResponse> {
        let changes = self.changes_since(tenant_id, watermark, limit).await?;

        // Calculate new watermark (highest change_id in results, or input watermark)
        let new_watermark = changes.iter()
            .map(|c| c.change_id)
            .max()
            .unwrap_or(watermark);

        // Check if there are more changes
        let has_more = if changes.len() as i32 >= limit {
            self.has_changes_since(tenant_id, new_watermark, None).await?
        } else {
            false
        };

        Ok(crate::types::ChangesResponse {
            changes,
            watermark: new_watermark,
            has_more,
        })
    }

    /// Cleanup old change journal entries.
    ///
    /// Deletes entries older than the specified retention period.
    /// Returns the number of entries deleted.
    pub async fn cleanup_change_journal(&self, retention_days: i32) -> ApiResult<i64> {
        let conn = self.get_conn().await?;

        let row = conn
            .query_one(
                "SELECT caliber_cleanup_change_journal($1)",
                &[&retention_days],
            )
            .await?;

        let deleted: i64 = row.get(0);
        Ok(deleted)
    }
}

// ============================================================================
// GENERIC CRUD OPERATIONS (Component Trait Based)
// ============================================================================

use crate::component::{Component, Listable, ListFilter, SqlParam, TenantScoped};

impl DbClient {
    /// Generic create operation for any Component type.
    ///
    /// Creates a new entity by calling the appropriate stored procedure
    /// (e.g., `caliber_trajectory_create`) with parameters built from the
    /// Component trait implementation.
    ///
    /// # Type Parameters
    ///
    /// - `C`: The component type implementing `Component + TenantScoped`
    ///
    /// # Arguments
    ///
    /// - `req`: The create request containing the entity data
    /// - `tenant_id`: The tenant ID for isolation
    ///
    /// # Returns
    ///
    /// The created entity on success, or an error on failure.
    pub async fn create<C>(&self, req: &C::Create, tenant_id: EntityId) -> ApiResult<C>
    where
        C: Component + TenantScoped,
    {
        let conn = self.get_conn().await?;
        let params = C::create_params(req, tenant_id);
        let sql_params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
            params.iter().map(|p| p.as_to_sql()).collect();

        let sql = format!(
            "SELECT row_to_json(t.*) FROM {}($1{}) AS t",
            C::sql_function("create"),
            (2..=params.len())
                .map(|i| format!(", ${}", i))
                .collect::<String>()
        );

        let row = conn
            .query_one(&sql, &sql_params[..])
            .await
            .map_err(|e| ApiError::database_error(e.to_string()))?;

        let json: serde_json::Value = row.get(0);
        C::from_json(&json).map_err(|e| ApiError::internal_error(e.to_string()))
    }

    /// Generic get operation for any Component type.
    ///
    /// Retrieves an entity by ID by calling the appropriate stored procedure
    /// (e.g., `caliber_trajectory_get`).
    ///
    /// # Type Parameters
    ///
    /// - `C`: The component type implementing `Component + TenantScoped`
    ///
    /// # Arguments
    ///
    /// - `id`: The entity ID to retrieve
    /// - `tenant_id`: The tenant ID for isolation
    ///
    /// # Returns
    ///
    /// `Some(entity)` if found, `None` if not found, or an error on failure.
    pub async fn get<C>(&self, id: EntityId, tenant_id: EntityId) -> ApiResult<Option<C>>
    where
        C: Component + TenantScoped,
    {
        let conn = self.get_conn().await?;
        let sql = format!(
            "SELECT row_to_json(t.*) FROM {}($1, $2) AS t",
            C::sql_function("get")
        );

        let row = conn
            .query_opt(&sql, &[&id, &tenant_id])
            .await
            .map_err(|e| ApiError::database_error(e.to_string()))?;

        match row {
            Some(r) => {
                let json: serde_json::Value = r.get(0);
                if json.is_null() {
                    Ok(None)
                } else {
                    C::from_json(&json)
                        .map(Some)
                        .map_err(|e| ApiError::internal_error(e.to_string()))
                }
            }
            None => Ok(None),
        }
    }

    /// Generic update operation for any Component type.
    ///
    /// Updates an existing entity by calling the appropriate stored procedure
    /// (e.g., `caliber_trajectory_update`) with a JSON object containing the
    /// fields to update.
    ///
    /// # Type Parameters
    ///
    /// - `C`: The component type implementing `Component + TenantScoped`
    ///
    /// # Arguments
    ///
    /// - `id`: The entity ID to update
    /// - `req`: The update request containing fields to modify
    /// - `tenant_id`: The tenant ID for isolation
    ///
    /// # Returns
    ///
    /// The updated entity on success, or an error if not found or on failure.
    pub async fn update<C>(&self, id: EntityId, req: &C::Update, tenant_id: EntityId) -> ApiResult<C>
    where
        C: Component + TenantScoped,
    {
        let conn = self.get_conn().await?;
        let updates = C::build_updates(req);

        let sql = format!(
            "SELECT row_to_json(t.*) FROM {}($1, $2, $3) AS t",
            C::sql_function("update")
        );

        let row = conn
            .query_opt(&sql, &[&id, &updates, &tenant_id])
            .await
            .map_err(|e| ApiError::database_error(e.to_string()))?;

        match row {
            Some(r) => {
                let json: serde_json::Value = r.get(0);
                if json.is_null() {
                    Err(C::not_found_error(id))
                } else {
                    C::from_json(&json).map_err(|e| ApiError::internal_error(e.to_string()))
                }
            }
            None => Err(C::not_found_error(id)),
        }
    }

    /// Generic update operation with raw JSON updates.
    ///
    /// This is used by state transition methods on Response types to apply
    /// arbitrary updates without needing a typed Update request.
    ///
    /// # Type Parameters
    ///
    /// - `C`: The component type implementing `Component + TenantScoped`
    ///
    /// # Arguments
    ///
    /// - `id`: The entity ID to update
    /// - `updates`: JSON object with fields to update
    /// - `tenant_id`: The tenant ID for isolation
    ///
    /// # Returns
    ///
    /// The updated entity on success, or an error if not found or on failure.
    pub async fn update_raw<C>(&self, id: EntityId, updates: JsonValue, tenant_id: EntityId) -> ApiResult<C>
    where
        C: Component + TenantScoped,
    {
        let conn = self.get_conn().await?;

        let sql = format!(
            "SELECT row_to_json(t.*) FROM {}($1, $2, $3) AS t",
            C::sql_function("update")
        );

        let row = conn
            .query_opt(&sql, &[&id, &updates, &tenant_id])
            .await
            .map_err(|e| ApiError::database_error(e.to_string()))?;

        match row {
            Some(r) => {
                let json: serde_json::Value = r.get(0);
                if json.is_null() {
                    Err(C::not_found_error(id))
                } else {
                    C::from_json(&json).map_err(|e| ApiError::internal_error(e.to_string()))
                }
            }
            None => Err(C::not_found_error(id)),
        }
    }

    /// Generic delete operation for any Component type.
    ///
    /// Deletes an entity by calling the appropriate stored procedure
    /// (e.g., `caliber_trajectory_delete`).
    ///
    /// # Type Parameters
    ///
    /// - `C`: The component type implementing `Component + TenantScoped`
    ///
    /// # Arguments
    ///
    /// - `id`: The entity ID to delete
    /// - `tenant_id`: The tenant ID for isolation
    ///
    /// # Returns
    ///
    /// `true` if the entity was deleted, `false` if not found.
    pub async fn delete<C>(&self, id: EntityId, tenant_id: EntityId) -> ApiResult<bool>
    where
        C: Component + TenantScoped,
    {
        let conn = self.get_conn().await?;
        let sql = format!("SELECT {}($1, $2)", C::sql_function("delete"));

        let row = conn
            .query_one(&sql, &[&id, &tenant_id])
            .await
            .map_err(|e| ApiError::database_error(e.to_string()))?;

        let deleted: bool = row.get(0);
        Ok(deleted)
    }

    /// Generic list operation for any Component type.
    ///
    /// Lists entities with optional filtering by calling a direct SQL query
    /// on the entity's table with the WHERE clause built from the filter.
    ///
    /// # Type Parameters
    ///
    /// - `C`: The component type implementing `Component + Listable + TenantScoped`
    ///
    /// # Arguments
    ///
    /// - `filter`: The filter to apply to the query
    /// - `tenant_id`: The tenant ID for isolation
    ///
    /// # Returns
    ///
    /// A vector of matching entities, ordered by `created_at DESC`.
    pub async fn list<C>(&self, filter: &C::ListFilter, tenant_id: EntityId) -> ApiResult<Vec<C>>
    where
        C: Component + Listable + TenantScoped,
    {
        let conn = self.get_conn().await?;
        let (where_clause, params) = filter.build_where(tenant_id);

        let sql_params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
            params.iter().map(|p| p.as_to_sql()).collect();

        let where_sql = where_clause.unwrap_or_else(|| "TRUE".to_string());
        let sql = format!(
            "SELECT row_to_json(t.*) FROM caliber_{} t WHERE {} ORDER BY created_at DESC LIMIT {} OFFSET {}",
            C::ENTITY_NAME,
            where_sql,
            filter.limit(),
            filter.offset()
        );

        let rows = conn
            .query(&sql, &sql_params[..])
            .await
            .map_err(|e| ApiError::database_error(e.to_string()))?;

        rows.into_iter()
            .map(|row| {
                let json: serde_json::Value = row.get(0);
                C::from_json(&json).map_err(|e| ApiError::internal_error(e.to_string()))
            })
            .collect()
    }
}

