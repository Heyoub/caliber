//! Model Context Protocol (MCP) routes
//!
//! Implements the MCP specification for tool-based agent interaction.

use axum::{routing::{get, post}, Router};
use crate::state::AppState;

// Sub-modules
pub mod types;
pub mod tools;
pub mod handlers;
pub mod resources;
pub mod prompts;

// Re-export key types
pub use types::*;
pub use tools::*;
pub use handlers::*;
pub use resources::*;
pub use prompts::*;

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
