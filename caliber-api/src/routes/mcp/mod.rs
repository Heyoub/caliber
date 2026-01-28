//! Model Context Protocol (MCP) routes
//!
//! Implements the MCP specification for tool-based agent interaction.

use crate::state::AppState;
use axum::{
    routing::{get, post},
    Router,
};

// Sub-modules
pub mod handlers;
pub mod prompts;
pub mod resources;
pub mod tools;
pub mod types;

// Re-export key types
pub use handlers::*;
pub use prompts::*;
pub use resources::*;
pub use tools::*;
pub use types::*;

/// MCP protocol version
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// Create the MCP router with all endpoints.
pub fn create_router() -> Router<AppState> {
    Router::new()
        // Server information
        .route("/mcp/info", get(|| async { "CALIBER MCP Server" }))
        // Core protocol endpoints
        .route("/mcp/initialize", post(initialize))
        .route("/mcp/tools/list", post(list_tools))
        .route("/mcp/tools/call", post(call_tool))
        .route("/mcp/resources/list", post(list_resources))
        .route("/mcp/resources/read", post(read_resource))
        // Prompts capability
        .route("/mcp/prompts/list", post(list_prompts))
        .route("/mcp/prompts/get", post(get_prompt))
}
