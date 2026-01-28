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

mod agent;
mod api_key;
mod artifact;
mod delegation;
mod edge;
mod handoff;
mod lock;
mod message;
mod note;
mod scope;
mod summarization_policy;
mod tenant;
mod trajectory;
mod turn;
mod webhook;

// Re-export all component types and filters
pub use agent::AgentListFilter;
pub use api_key::ApiKeyListFilter;
pub use artifact::ArtifactListFilter;
pub use delegation::DelegationListFilter;
pub use edge::EdgeListFilter;
pub use handoff::HandoffListFilter;
pub use lock::LockListFilter;
pub use message::MessageListFilter;
pub use note::NoteListFilter;
pub use scope::ScopeListFilter;
pub use summarization_policy::SummarizationPolicyListFilter;
pub use tenant::TenantListFilter;
pub use trajectory::TrajectoryListFilter;
pub use turn::TurnListFilter;
pub use webhook::WebhookListFilter;

// Note: The Component trait implementations are applied to the types
// in the types module, so they are automatically available when you
// import the types.
