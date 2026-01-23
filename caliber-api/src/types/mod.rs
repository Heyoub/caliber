//! API Request and Response Types
//!
//! This module defines all request and response types for the CALIBER API.
//! These types are used by both REST and gRPC endpoints.

// Trajectory types
mod trajectory;
pub use trajectory::*;

// Scope types
mod scope;
pub use scope::*;

// Artifact types
mod artifact;
pub use artifact::*;

// Note types
mod note;
pub use note::*;

// Turn types
mod turn;
pub use turn::*;

// Agent types
mod agent;
pub use agent::*;

// Lock types
mod lock;
pub use lock::*;

// Message types
mod message;
pub use message::*;

// Delegation types
mod delegation;
pub use delegation::*;

// Handoff types
mod handoff;
pub use handoff::*;

// Search types
mod search;
pub use search::*;

// DSL types
mod dsl;
pub use dsl::*;

// Batch operation types
mod batch;
pub use batch::*;

// Configuration types
mod config;
pub use config::*;

// Tenant types
mod tenant;
pub use tenant::*;

// Battle Intel: Edge types
mod edge;
pub use edge::*;

// Battle Intel: Summarization types
mod summarization;
pub use summarization::*;

// API Key types
mod api_key;
pub use api_key::*;

// Webhook types
mod webhook;
pub use webhook::*;

// ============================================================================
// Entity trait implementations using macros
// ============================================================================

// Core response types
crate::impl_entity!(TrajectoryResponse, trajectory_id, tenant_id);
crate::impl_entity!(ScopeResponse, scope_id, tenant_id);
crate::impl_entity!(ArtifactResponse, artifact_id, tenant_id);
crate::impl_entity!(NoteResponse, note_id, tenant_id);
crate::impl_entity!(TurnResponse, turn_id, tenant_id);

// Agent and communication types
crate::impl_entity!(AgentResponse, agent_id, tenant_id);
crate::impl_entity!(MessageResponse, message_id, tenant_id);
crate::impl_entity!(LockResponse, lock_id, tenant_id);
crate::impl_entity!(DelegationResponse, delegation_id, tenant_id);
crate::impl_entity!(HandoffResponse, handoff_id, tenant_id);

// Battle Intel types
crate::impl_entity!(EdgeResponse, edge_id);

// Admin/Infrastructure types
crate::impl_entity!(ApiKeyResponse, api_key_id, tenant_id);
crate::impl_entity!(WebhookResponse, webhook_id, tenant_id);
