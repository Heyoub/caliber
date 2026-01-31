//! Markdown lint + extraction for pack prompts

use super::ir::{MarkdownError, PackError};
use super::schema::{PackManifest, ToolsSection};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct MarkdownDoc {
    pub file: String,
    pub system: String,
    pub pcp: String,
    pub users: Vec<UserSection>,
    /// Constraints extracted from ```constraints blocks.
    pub extracted_constraints: Vec<String>,
    /// Tool references extracted from ```tools blocks (validated against TOML).
    pub extracted_tool_refs: Vec<String>,
    /// RAG configuration extracted from ```rag block.
    pub extracted_rag_config: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UserSection {
    pub content: String,
    pub blocks: Vec<FencedBlock>,
}

/// Supported fence block types (single source of truth)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FenceKind {
    // Config types (NEW)
    Adapter,
    Memory,
    Policy,
    Injection,
    Provider,
    Cache,
    Trajectory,
    Agent,

    // Existing types
    Tool,
    Rag,
    Json,
    Xml,
    Constraints,
    Tools,

    // Metadata
    Manifest,
}

impl FromStr for FenceKind {
    type Err = PackError;

    /// Parses a fence kind identifier into a `FenceKind`.
    ///
    /// Recognized case-sensitive identifiers: `"adapter"`, `"memory"`, `"policy"`, `"injection"`,
    /// `"provider"`, `"cache"`, `"trajectory"`, `"agent"`, `"tool"`, `"rag"`, `"json"`, `"xml"`,
    /// `"constraints"`, `"tools"`, and `"manifest"`.
    ///
    /// # Returns
    ///
    /// `Ok(FenceKind)` for a recognized identifier, `Err(PackError::Validation)` for unsupported input.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::str::FromStr;
    /// let k = FenceKind::from_str("tool").unwrap();
    /// assert_eq!(k, FenceKind::Tool);
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Case-insensitive matching for fence types
        match s.to_lowercase().as_str() {
            "adapter" => Ok(FenceKind::Adapter),
            "memory" => Ok(FenceKind::Memory),
            "policy" => Ok(FenceKind::Policy),
            "injection" => Ok(FenceKind::Injection),
            "provider" => Ok(FenceKind::Provider),
            "cache" => Ok(FenceKind::Cache),
            "trajectory" => Ok(FenceKind::Trajectory),
            "agent" => Ok(FenceKind::Agent),
            "tool" => Ok(FenceKind::Tool),
            "rag" => Ok(FenceKind::Rag),
            "json" => Ok(FenceKind::Json),
            "xml" => Ok(FenceKind::Xml),
            "constraints" => Ok(FenceKind::Constraints),
            "tools" => Ok(FenceKind::Tools),
            "manifest" => Ok(FenceKind::Manifest),
            other => Err(PackError::Validation(format!(
                "unsupported fence type '{}'",
                other
            ))),
        }
    }
}

impl std::fmt::Display for FenceKind {
    /// Formats the FenceKind as its lowercase string representation.
    ///
    /// # Examples
    ///
    /// ```
    /// use caliber_dsl::pack::markdown::FenceKind;
    /// assert_eq!(format!("{}", FenceKind::Adapter), "adapter");
    /// assert_eq!(format!("{}", FenceKind::Json), "json");
    /// assert_eq!(format!("{}", FenceKind::Constraints), "constraints");
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            FenceKind::Adapter => "adapter",
            FenceKind::Memory => "memory",
            FenceKind::Policy => "policy",
            FenceKind::Injection => "injection",
            FenceKind::Provider => "provider",
            FenceKind::Cache => "cache",
            FenceKind::Trajectory => "trajectory",
            FenceKind::Agent => "agent",
            FenceKind::Tool => "tool",
            FenceKind::Rag => "rag",
            FenceKind::Json => "json",
            FenceKind::Xml => "xml",
            FenceKind::Constraints => "constraints",
            FenceKind::Tools => "tools",
            FenceKind::Manifest => "manifest",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone)]
pub struct FencedBlock {
    pub kind: FenceKind,
    pub header_name: Option<String>,
    pub content: String,
    pub line: usize,
}

pub fn parse_markdown_files(
    manifest: &PackManifest,
    files: &[super::PackMarkdownFile],
) -> Result<Vec<MarkdownDoc>, PackError> {
    let mut out = Vec::new();
    for file in files {
        out.push(parse_markdown(
            manifest,
            &file.path.display().to_string(),
            &file.content,
        )?);
    }
    Ok(out)
}

/// Parse a pack markdown document into a structured MarkdownDoc, validating headings and fenced blocks and extracting constraints, tool references, and RAG configuration.
///
/// On success returns a MarkdownDoc containing the trimmed System and PCP sections, a list of User sections with their fenced blocks, and aggregated extracted metadata (constraints, validated tool refs, optional RAG config). On failure returns a PackError describing the first structural or validation issue encountered (missing or misordered headings, unterminated or invalid fenced blocks, invalid tool references, etc.).
///
/// # Examples
///
/// ```no_run
/// let manifest = PackManifest::default();
/// let content = r#"
/// # System
///
/// system text
///
/// ## PCP
///
/// pcp text
///
/// ### User
///
/// user prompt
/// ```constraints
/// must not reveal secrets
/// ```
/// "#;
///
/// let doc = parse_markdown(&manifest, "prompt.md", content).unwrap();
/// assert_eq!(doc.file, "prompt.md");
/// ```
fn parse_markdown(
    manifest: &PackManifest,
    file: &str,
    content: &str,
) -> Result<MarkdownDoc, PackError> {
    let strict_refs = manifest
        .defaults
        .as_ref()
        .and_then(|d| d.strict_refs)
        .unwrap_or(false);

    let tool_ids = collect_tool_ids(&manifest.tools);
    let mut system = String::new();
    let mut pcp = String::new();
    let mut users: Vec<UserSection> = Vec::new();

    enum Section {
        None,
        System,
        Pcp,
        User,
    }
    let mut section = Section::None;
    let mut current_user: Option<UserSection> = None;
    let mut in_block: Option<FencedBlock> = None;
    let mut last_heading = 0;

    for (idx, line) in content.lines().enumerate() {
        let line_no = idx + 1;
        if let Some(block) = &mut in_block {
            if line.trim_start().starts_with("```") {
                // close
                let finished = in_block.take().expect("in_block verified as Some above");
                if let Section::User = section {
                    if let Some(u) = &mut current_user {
                        u.blocks.push(finished);
                    }
                } else {
                    return Err(PackError::Markdown(MarkdownError {
                        file: file.to_string(),
                        line: line_no,
                        column: 1,
                        message: "fenced blocks only allowed under ### User".into(),
                    }));
                }
                continue;
            }
            block.content.push_str(line);
            block.content.push('\n');
            continue;
        }

        if let Some(heading) = heading_level(line) {
            match heading {
                1 => {
                    if line.trim() != "# System" {
                        return Err(PackError::Markdown(MarkdownError {
                            file: file.to_string(),
                            line: line_no,
                            column: 1,
                            message: "first H1 must be '# System'".into(),
                        }));
                    }
                    if last_heading > 1 {
                        return Err(PackError::Markdown(MarkdownError {
                            file: file.to_string(),
                            line: line_no,
                            column: 1,
                            message: "H1 must come before H2/H3".into(),
                        }));
                    }
                    section = Section::System;
                    last_heading = 1;
                    continue;
                }
                2 => {
                    if line.trim() != "## PCP" {
                        return Err(PackError::Markdown(MarkdownError {
                            file: file.to_string(),
                            line: line_no,
                            column: 1,
                            message: "H2 must be '## PCP'".into(),
                        }));
                    }
                    if last_heading < 1 {
                        return Err(PackError::Markdown(MarkdownError {
                            file: file.to_string(),
                            line: line_no,
                            column: 1,
                            message: "H2 must follow '# System'".into(),
                        }));
                    }
                    if let Some(u) = current_user.take() {
                        users.push(u);
                    }
                    section = Section::Pcp;
                    last_heading = 2;
                    continue;
                }
                3 => {
                    if line.trim() != "### User" {
                        return Err(PackError::Markdown(MarkdownError {
                            file: file.to_string(),
                            line: line_no,
                            column: 1,
                            message: "H3 must be '### User'".into(),
                        }));
                    }
                    if last_heading < 2 {
                        return Err(PackError::Markdown(MarkdownError {
                            file: file.to_string(),
                            line: line_no,
                            column: 1,
                            message: "H3 must follow '## PCP'".into(),
                        }));
                    }
                    if let Some(u) = current_user.take() {
                        users.push(u);
                    }
                    section = Section::User;
                    last_heading = 3;
                    current_user = Some(UserSection {
                        content: String::new(),
                        blocks: Vec::new(),
                    });
                    continue;
                }
                _ => {}
            }
        }

        if line.trim_start().starts_with("```") {
            let info = line.trim().trim_start_matches("```").trim();
            if info.is_empty() {
                return Err(PackError::Markdown(MarkdownError {
                    file: file.to_string(),
                    line: line_no,
                    column: 1,
                    message: "fenced block must have a type".into(),
                }));
            }
            let (kind, header_name) = parse_fence_info(info).map_err(|e| MarkdownError {
                file: file.to_string(),
                line: line_no,
                column: 1,
                message: e.to_string(),
            })?;
            in_block = Some(FencedBlock {
                kind,
                header_name,
                content: String::new(),
                line: line_no,
            });
            continue;
        }

        match section {
            Section::System => {
                system.push_str(line);
                system.push('\n');
            }
            Section::Pcp => {
                pcp.push_str(line);
                pcp.push('\n');
            }
            Section::User => {
                if let Some(u) = &mut current_user {
                    u.content.push_str(line);
                    u.content.push('\n');
                }
            }
            Section::None => {}
        }
    }

    if in_block.is_some() {
        return Err(PackError::Markdown(MarkdownError {
            file: file.to_string(),
            line: content.lines().count(),
            column: 1,
            message: "unterminated fenced block".into(),
        }));
    }
    if let Some(u) = current_user.take() {
        users.push(u);
    }
    if system.trim().is_empty() || pcp.trim().is_empty() || users.is_empty() {
        return Err(PackError::Markdown(MarkdownError {
            file: file.to_string(),
            line: 1,
            column: 1,
            message: "missing required sections (# System, ## PCP, ### User)".into(),
        }));
    }

    // Validate fenced blocks per user section and collect extracted metadata
    let mut all_constraints = Vec::new();
    let mut all_tool_refs = Vec::new();
    let mut rag_config = None;
    for user in &users {
        let extracted = validate_blocks(file, user, &tool_ids, strict_refs)?;
        all_constraints.extend(extracted.constraints);
        all_tool_refs.extend(extracted.tool_refs);
        if extracted.rag_config.is_some() {
            rag_config = extracted.rag_config;
        }
    }

    Ok(MarkdownDoc {
        file: file.to_string(),
        system: system.trim().to_string(),
        pcp: pcp.trim().to_string(),
        users,
        extracted_constraints: all_constraints,
        extracted_tool_refs: all_tool_refs,
        extracted_rag_config: rag_config,
    })
}

/// Extracted metadata from markdown fenced blocks.
#[derive(Debug, Clone, Default)]
pub struct ExtractedBlocks {
    pub constraints: Vec<String>,
    pub tool_refs: Vec<String>,
    pub rag_config: Option<String>,
}

/// Validates and extracts metadata from the fenced blocks in a user section.
///
/// This function walks the user's fenced blocks, validates block sequences and references,
/// and collects extracted metadata into an `ExtractedBlocks` struct:
/// - Tool blocks must contain a single `${...}` reference that matches a known tool ID; if followed by a JSON or XML block it is treated as a payload pair (payload must be a `${...}` ref when `strict_refs` is true).
/// - Standalone JSON or XML blocks are invalid and produce an error.
/// - Constraints blocks produce one extracted constraint per non-empty, non-comment line.
/// - Tools blocks list tool IDs (one per line, optionally prefixed with `-`) which are validated against `tool_ids` and collected into `tool_refs`.
/// - Rag blocks store their trimmed content as the optional `rag_config`.
/// - New config kinds (Adapter, Memory, Policy, Injection, Provider, Cache, Trajectory, Agent, Manifest) are accepted and skipped for later processing.
///
/// Errors are returned as `PackError::Markdown` with file/line context for structural or validation failures.
///
/// # Returns
///
/// The collected `ExtractedBlocks` containing `constraints`, `tool_refs`, and optional `rag_config`.
///
/// # Examples
///
/// ```
/// use std::collections::HashSet;
///
/// // Construct a simple user section with a constraints block
/// let user = UserSection {
///     content: String::new(),
///     blocks: vec![FencedBlock {
///         kind: FenceKind::Constraints,
///         header_name: None,
///         content: "must do X\n# comment\nmust not do Y\n".into(),
///         line: 1,
///     }],
/// };
///
/// let tool_ids: HashSet<String> = HashSet::new();
/// let extracted = validate_blocks("file.md", &user, &tool_ids, false).unwrap();
/// assert_eq!(extracted.constraints, vec!["must do X".to_string(), "must not do Y".to_string()]);
/// ```
fn validate_blocks(
    file: &str,
    user: &UserSection,
    tool_ids: &std::collections::HashSet<String>,
    strict_refs: bool,
) -> Result<ExtractedBlocks, PackError> {
    let mut extracted = ExtractedBlocks::default();
    let mut i = 0;
    while i < user.blocks.len() {
        let block = &user.blocks[i];
        match block.kind {
            FenceKind::Tool => {
                let tool_ref = block.content.trim();
                if !is_ref(tool_ref) {
                    return Err(PackError::Markdown(MarkdownError {
                        file: file.to_string(),
                        line: block.line,
                        column: 1,
                        message: "tool block must contain a single ${...} ref".into(),
                    }));
                }
                let tool_id = strip_ref(tool_ref);
                if !tool_ids.contains(tool_id) {
                    return Err(PackError::Markdown(MarkdownError {
                        file: file.to_string(),
                        line: block.line,
                        column: 1,
                        message: format!("unknown tool id '{}'", tool_id),
                    }));
                }
                if strict_refs {
                    // ok
                }
                // payload pairing
                if i + 1 < user.blocks.len() {
                    let next = &user.blocks[i + 1];
                    if next.kind == FenceKind::Json || next.kind == FenceKind::Xml {
                        if strict_refs && !is_ref(next.content.trim()) {
                            return Err(PackError::Markdown(MarkdownError {
                                file: file.to_string(),
                                line: next.line,
                                column: 1,
                                message: "payload block must be a ${...} ref in strict_refs".into(),
                            }));
                        }
                        i += 2;
                        continue;
                    }
                }
                i += 1;
            }
            FenceKind::Json | FenceKind::Xml => {
                return Err(PackError::Markdown(MarkdownError {
                    file: file.to_string(),
                    line: block.line,
                    column: 1,
                    message: "payload block must follow a tool block".into(),
                }));
            }
            // Extended block types for agent metadata extraction
            FenceKind::Constraints => {
                // Extract constraints as individual lines
                for line in block.content.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() && !trimmed.starts_with('#') {
                        extracted.constraints.push(trimmed.to_string());
                    }
                }
                i += 1;
            }
            FenceKind::Tools => {
                // Extract tool references, validate against known tool IDs
                for line in block.content.lines() {
                    let trimmed = line.trim().trim_start_matches('-').trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }
                    if !tool_ids.contains(trimmed) {
                        return Err(PackError::Markdown(MarkdownError {
                            file: file.to_string(),
                            line: block.line,
                            column: 1,
                            message: format!(
                                "tools block references unknown tool '{}'. Must match TOML-declared tool IDs.",
                                trimmed
                            ),
                        }));
                    }
                    extracted.tool_refs.push(trimmed.to_string());
                }
                i += 1;
            }
            FenceKind::Rag => {
                // Extract RAG configuration as-is
                let content = block.content.trim();
                if !content.is_empty() {
                    extracted.rag_config = Some(content.to_string());
                }
                i += 1;
            }
            // New config types - accept for now (will be processed later)
            FenceKind::Adapter
            | FenceKind::Memory
            | FenceKind::Policy
            | FenceKind::Injection
            | FenceKind::Provider
            | FenceKind::Cache
            | FenceKind::Trajectory
            | FenceKind::Agent
            | FenceKind::Manifest => {
                // Allow new config types to pass through validation
                // They will be processed in the config parser layer
                i += 1;
            }
        }
    }
    Ok(extracted)
}

/// Parse a fence header string into a `FenceKind` and an optional header name.
///
/// Accepts two forms: `"kind"` (e.g., `"adapter"`) or `"kind name"` (e.g., `"adapter postgres_main"`).
///
/// # Errors
///
/// Returns a `PackError::Validation` if the input is empty, contains more than two whitespace-separated tokens,
/// or if the kind token is not a recognized `FenceKind`.
///
/// # Examples
///
/// ```
/// let (kind, name) = parse_fence_info("adapter postgres_main").unwrap();
/// assert_eq!(kind, FenceKind::Adapter);
/// assert_eq!(name.as_deref(), Some("postgres_main"));
///
/// let (kind_only, none_name) = parse_fence_info("adapter").unwrap();
/// assert_eq!(kind_only, FenceKind::Adapter);
/// assert!(none_name.is_none());
/// ```
fn parse_fence_info(info: &str) -> Result<(FenceKind, Option<String>), PackError> {
    let parts: Vec<&str> = info.split_whitespace().collect();

    match parts.as_slice() {
        [] => Err(PackError::Validation(
            "fence block must have a type".to_string(),
        )),
        [kind_str] => {
            // Form B: "kind" only
            let kind = FenceKind::from_str(kind_str)?;
            Ok((kind, None))
        }
        [kind_str, name] => {
            // Form A: "kind name"
            let kind = FenceKind::from_str(kind_str)?;
            Ok((kind, Some(name.to_string())))
        }
        _ => Err(PackError::Validation(format!(
            "invalid fence header '{}' (expected 'kind' or 'kind name')",
            info
        ))),
    }
}

/// Determine the Markdown heading level for a single line.
///
/// Returns `Some(1)`, `Some(2)`, or `Some(3)` when the line starts with `"# "`, `"## "`, or `"### "`
/// respectively, and `None` for any other input.
///
/// # Examples
///
/// ```
/// assert_eq!(heading_level("# Title"), Some(1));
/// assert_eq!(heading_level("## Subtitle"), Some(2));
/// assert_eq!(heading_level("### Section"), Some(3));
/// assert_eq!(heading_level("#### Too deep"), None);
/// assert_eq!(heading_level("Not a heading"), None);
/// ```
fn heading_level(line: &str) -> Option<usize> {
    if line.starts_with("# ") {
        Some(1)
    } else if line.starts_with("## ") {
        Some(2)
    } else if line.starts_with("### ") {
        Some(3)
    } else {
        None
    }
}

fn is_ref(s: &str) -> bool {
    s.starts_with("${") && s.ends_with('}')
}

fn strip_ref(s: &str) -> &str {
    s.trim().trim_start_matches("${").trim_end_matches('}')
}

fn collect_tool_ids(tools: &ToolsSection) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();
    for name in tools.bin.keys() {
        ids.insert(format!("tools.bin.{}", name));
    }
    for name in tools.prompts.keys() {
        ids.insert(format!("tools.prompts.{}", name));
    }
    ids
}
