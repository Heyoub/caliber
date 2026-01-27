//! Markdown lint + extraction for pack prompts

use super::ir::{MarkdownError, PackError};
use super::schema::{PackManifest, ToolsSection};

#[derive(Debug, Clone)]
pub struct MarkdownDoc {
    pub file: String,
    pub system: String,
    pub pcp: String,
    pub users: Vec<UserSection>,
}

#[derive(Debug, Clone)]
pub struct UserSection {
    pub content: String,
    pub blocks: Vec<FencedBlock>,
}

#[derive(Debug, Clone)]
pub struct FencedBlock {
    pub tag: String,
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
                let finished = in_block.take().unwrap();
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
            let tag = line.trim().trim_start_matches("```").trim().to_string();
            if tag.is_empty() {
                return Err(PackError::Markdown(MarkdownError {
                    file: file.to_string(),
                    line: line_no,
                    column: 1,
                    message: "fenced block must have a tag".into(),
                }));
            }
            in_block = Some(FencedBlock {
                tag,
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

    // Validate fenced blocks per user section
    for user in &users {
        validate_blocks(file, user, &tool_ids, strict_refs)?;
    }

    Ok(MarkdownDoc {
        file: file.to_string(),
        system: system.trim().to_string(),
        pcp: pcp.trim().to_string(),
        users,
    })
}

fn validate_blocks(
    file: &str,
    user: &UserSection,
    tool_ids: &std::collections::HashSet<String>,
    strict_refs: bool,
) -> Result<(), PackError> {
    let mut i = 0;
    while i < user.blocks.len() {
        let block = &user.blocks[i];
        match block.tag.as_str() {
            "tool" => {
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
                    if next.tag == "json" || next.tag == "xml" {
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
            "json" | "xml" => {
                return Err(PackError::Markdown(MarkdownError {
                    file: file.to_string(),
                    line: block.line,
                    column: 1,
                    message: "payload block must follow a tool block".into(),
                }));
            }
            _ => {
                return Err(PackError::Markdown(MarkdownError {
                    file: file.to_string(),
                    line: block.line,
                    column: 1,
                    message: format!("invalid fenced block tag '{}'", block.tag),
                }));
            }
        }
    }
    Ok(())
}

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
