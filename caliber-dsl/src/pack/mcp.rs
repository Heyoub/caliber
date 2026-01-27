//! MCP (Model Context Protocol) schema generation.
//!
//! Generates deterministic `mcp.json` from compiled pack configuration.
//! The output is canonical JSON with sorted keys for reproducibility.

use crate::compiler::{CompiledConfig, CompiledToolConfig, CompiledToolKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// MCP schema representation for tool serving.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSchema {
    /// Schema version
    pub version: String,
    /// Tools available via MCP
    pub tools: Vec<McpTool>,
    /// File hashes for drift detection
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_hashes: BTreeMap<String, String>,
}

/// Individual tool in MCP schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    /// Tool identifier
    pub name: String,
    /// Tool description (from prompt or command)
    pub description: String,
    /// Input schema (if contract defined)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
}

/// Generate deterministic MCP schema from compiled configuration.
///
/// The output is canonical:
/// - Tools are sorted by ID
/// - File hashes are in a BTreeMap (sorted keys)
/// - JSON output will be consistent across runs
pub fn generate_mcp_schema(config: &CompiledConfig) -> McpSchema {
    let mut tools: Vec<McpTool> = config
        .tools
        .iter()
        .map(|t| tool_to_mcp(t))
        .collect();

    // Sort by name for deterministic output
    tools.sort_by(|a, b| a.name.cmp(&b.name));

    // Convert file_hashes to BTreeMap for sorted output
    let file_hashes: BTreeMap<String, String> = config
        .file_hashes
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    McpSchema {
        version: "1.0".to_string(),
        tools,
        file_hashes,
    }
}

fn tool_to_mcp(tool: &CompiledToolConfig) -> McpTool {
    let description = match tool.kind {
        CompiledToolKind::Exec => {
            tool.cmd.as_deref().unwrap_or("Executable tool").to_string()
        }
        CompiledToolKind::Prompt => {
            format!("Prompt tool: {}", tool.prompt_md.as_deref().unwrap_or("unknown"))
        }
    };

    McpTool {
        name: tool.id.clone(),
        description,
        input_schema: tool.compiled_schema.clone(),
    }
}

/// Serialize MCP schema to canonical JSON.
///
/// Uses sorted keys and consistent formatting for byte-identical output.
pub fn to_canonical_json(schema: &McpSchema) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(schema)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_output() {
        let config1 = CompiledConfig::default();
        let config2 = CompiledConfig::default();

        let json1 = to_canonical_json(&generate_mcp_schema(&config1)).unwrap();
        let json2 = to_canonical_json(&generate_mcp_schema(&config2)).unwrap();

        assert_eq!(json1, json2, "Same config should produce identical JSON");
    }
}
