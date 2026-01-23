//! Component implementations for ECS pattern.
//!
//! Each entity type has a corresponding module with:
//! - Component trait implementation via `impl_component!` macro
//! - ListFilter implementation for query filtering
//! - TenantScoped and Listable marker trait implementations
//!
//! # Usage
//!
//! The component implementations are used by the generic CRUD functions
//! in the `DbClient` to provide type-safe database operations.
//!
//! ```ignore
//! // Instead of:
//! db.trajectory_create(req, tenant_id).await?;
//! db.scope_create(req, tenant_id).await?;
//! db.artifact_create(req, tenant_id).await?;
//!
//! // You can use:
//! db.create::<TrajectoryResponse>(req, tenant_id).await?;
//! db.create::<ScopeResponse>(req, tenant_id).await?;
//! db.create::<ArtifactResponse>(req, tenant_id).await?;
//! ```

mod trajectory;
mod scope;
mod artifact;
mod note;
mod turn;
mod edge;
mod delegation;
mod handoff;
mod tenant;
mod api_key;
mod webhook;
mod agent;
mod message;
mod lock;

// Re-export all component types and filters
pub use trajectory::TrajectoryListFilter;
pub use scope::ScopeListFilter;
pub use artifact::ArtifactListFilter;
pub use note::NoteListFilter;
pub use turn::TurnListFilter;
pub use edge::EdgeListFilter;
pub use delegation::DelegationListFilter;
pub use handoff::HandoffListFilter;
pub use tenant::TenantListFilter;
pub use api_key::ApiKeyListFilter;
pub use webhook::WebhookListFilter;
pub use agent::AgentListFilter;
pub use message::MessageListFilter;
pub use lock::LockListFilter;

// Note: The Component trait implementations are applied to the types
// in the types module, so they are automatically available when you
// import the types.
