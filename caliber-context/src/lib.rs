//! CALIBER Context - Context Assembly
//!
//! Provides intelligent context assembly with token budget management.
//! Combines all inputs into a single coherent prompt following the Context Conveyor pattern.

use caliber_core::{
    Artifact, CaliberConfig, CaliberResult, EntityId, EntityType, Note, Timestamp,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// CONTEXT PACKAGE (Task 7.1)
// ============================================================================

/// Context package - all inputs for assembly.
/// Similar to ContextPackage in the TypeScript CRM pattern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextPackage {
    /// Trajectory this context belongs to
    pub trajectory_id: EntityId,
    /// Scope this context belongs to
    pub scope_id: EntityId,
    /// Current user query/input
    pub user_input: Option<String>,
    /// Relevant notes (semantic memory)
    pub relevant_notes: Vec<Note>,
    /// Recent artifacts from current trajectory
    pub recent_artifacts: Vec<Artifact>,
    /// Scope summaries (compressed history)
    pub scope_summaries: Vec<ScopeSummary>,
    /// Session markers (active context)
    pub session_markers: SessionMarkers,
    /// Kernel/persona configuration
    pub kernel_config: Option<KernelConfig>,
}

/// Summary of a scope for context assembly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScopeSummary {
    /// ID of the scope being summarized
    pub scope_id: EntityId,
    /// Summary text
    pub summary: String,
    /// Token count of the summary
    pub token_count: i32,
}

/// Session markers for tracking active context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SessionMarkers {
    /// Currently active trajectory
    pub active_trajectory_id: Option<EntityId>,
    /// Currently active scope
    pub active_scope_id: Option<EntityId>,
    /// Recently accessed artifact IDs
    pub recent_artifact_ids: Vec<EntityId>,
    /// Current agent ID (if multi-agent)
    pub agent_id: Option<EntityId>,
}

/// Kernel configuration for persona and behavior.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct KernelConfig {
    /// Persona description
    pub persona: Option<String>,
    /// Tone of responses
    pub tone: Option<String>,
    /// Reasoning style preference
    pub reasoning_style: Option<String>,
    /// Domain focus area
    pub domain_focus: Option<String>,
}

impl ContextPackage {
    /// Create a new context package with required fields.
    pub fn new(trajectory_id: EntityId, scope_id: EntityId) -> Self {
        Self {
            trajectory_id,
            scope_id,
            user_input: None,
            relevant_notes: Vec::new(),
            recent_artifacts: Vec::new(),
            scope_summaries: Vec::new(),
            session_markers: SessionMarkers::default(),
            kernel_config: None,
        }
    }

    /// Set the user input.
    pub fn with_user_input(mut self, input: String) -> Self {
        self.user_input = Some(input);
        self
    }

    /// Add relevant notes.
    pub fn with_notes(mut self, notes: Vec<Note>) -> Self {
        self.relevant_notes = notes;
        self
    }

    /// Add recent artifacts.
    pub fn with_artifacts(mut self, artifacts: Vec<Artifact>) -> Self {
        self.recent_artifacts = artifacts;
        self
    }

    /// Set session markers.
    pub fn with_session_markers(mut self, markers: SessionMarkers) -> Self {
        self.session_markers = markers;
        self
    }

    /// Set kernel config.
    pub fn with_kernel_config(mut self, config: KernelConfig) -> Self {
        self.kernel_config = Some(config);
        self
    }
}


// ============================================================================
// CONTEXT WINDOW AND SECTION (Task 7.2)
// ============================================================================

/// Type of context section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SectionType {
    /// System instructions
    System,
    /// Persona/kernel configuration
    Persona,
    /// Relevant notes from semantic memory
    Notes,
    /// Conversation history
    History,
    /// Artifacts from current trajectory
    Artifacts,
    /// User input/query
    User,
}

/// Reference to a source entity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceRef {
    /// Type of the source entity
    pub source_type: EntityType,
    /// ID of the source entity (if applicable)
    pub id: Option<EntityId>,
    /// Relevance score (if computed)
    pub relevance_score: Option<f32>,
}

/// A section of the assembled context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextSection {
    /// Unique identifier for this section
    pub section_id: EntityId,
    /// Type of this section
    pub section_type: SectionType,
    /// Content of this section
    pub content: String,
    /// Token count for this section
    pub token_count: i32,
    /// Priority (higher = more important)
    pub priority: i32,
    /// Whether this section can be compressed
    pub compressible: bool,
    /// Sources that contributed to this section
    pub sources: Vec<SourceRef>,
}

/// Action taken during context assembly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssemblyAction {
    /// Section was included in full
    Include,
    /// Section was excluded due to budget
    Exclude,
    /// Section was compressed
    Compress,
    /// Section was truncated to fit budget
    Truncate,
}

/// Decision made during context assembly for audit trail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssemblyDecision {
    /// When this decision was made
    pub timestamp: Timestamp,
    /// Action taken
    pub action: AssemblyAction,
    /// Type of target (e.g., "note", "artifact")
    pub target_type: String,
    /// ID of target entity (if applicable)
    pub target_id: Option<EntityId>,
    /// Reason for this decision
    pub reason: String,
    /// Tokens affected by this decision
    pub tokens_affected: i32,
}

/// Assembled context window with token budget management.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextWindow {
    /// Unique identifier for this window
    pub window_id: EntityId,
    /// When this window was assembled
    pub assembled_at: Timestamp,
    /// Maximum token budget
    pub max_tokens: i32,
    /// Tokens currently used
    pub used_tokens: i32,
    /// Sections in priority order
    pub sections: Vec<ContextSection>,
    /// Whether any section was truncated
    pub truncated: bool,
    /// Names of included sections
    pub included_sections: Vec<String>,
    /// Full audit trail of assembly decisions
    pub assembly_trace: Vec<AssemblyDecision>,
}

impl ContextSection {
    /// Create a new context section.
    pub fn new(section_type: SectionType, content: String, priority: i32) -> Self {
        let token_count = estimate_tokens(&content);
        Self {
            section_id: Uuid::now_v7(),
            section_type,
            content,
            token_count,
            priority,
            compressible: true,
            sources: Vec::new(),
        }
    }

    /// Set whether this section is compressible.
    pub fn with_compressible(mut self, compressible: bool) -> Self {
        self.compressible = compressible;
        self
    }

    /// Add source references.
    pub fn with_sources(mut self, sources: Vec<SourceRef>) -> Self {
        self.sources = sources;
        self
    }
}

impl ContextWindow {
    /// Create a new empty context window with the given token budget.
    pub fn new(max_tokens: i32) -> Self {
        Self {
            window_id: Uuid::now_v7(),
            assembled_at: Utc::now(),
            max_tokens,
            used_tokens: 0,
            sections: Vec::new(),
            truncated: false,
            included_sections: Vec::new(),
            assembly_trace: Vec::new(),
        }
    }

    /// Get remaining token budget.
    pub fn remaining_tokens(&self) -> i32 {
        self.max_tokens - self.used_tokens
    }

    /// Check if the window has room for more content.
    pub fn has_room(&self) -> bool {
        self.used_tokens < self.max_tokens
    }

    /// Add a section to the window.
    /// Returns true if the section was added, false if it didn't fit.
    pub fn add_section(&mut self, section: ContextSection) -> bool {
        if section.token_count <= self.remaining_tokens() {
            self.used_tokens += section.token_count;
            self.included_sections.push(format!("{:?}", section.section_type));
            self.assembly_trace.push(AssemblyDecision {
                timestamp: Utc::now(),
                action: AssemblyAction::Include,
                target_type: format!("{:?}", section.section_type),
                target_id: Some(section.section_id),
                reason: "Fits within budget".to_string(),
                tokens_affected: section.token_count,
            });
            self.sections.push(section);
            true
        } else {
            self.assembly_trace.push(AssemblyDecision {
                timestamp: Utc::now(),
                action: AssemblyAction::Exclude,
                target_type: format!("{:?}", section.section_type),
                target_id: Some(section.section_id),
                reason: format!(
                    "Exceeds budget: needs {} tokens, only {} available",
                    section.token_count,
                    self.remaining_tokens()
                ),
                tokens_affected: 0,
            });
            false
        }
    }

    /// Add a truncated section to the window.
    pub fn add_truncated_section(&mut self, mut section: ContextSection) {
        let available = self.remaining_tokens();
        if available <= 0 {
            self.assembly_trace.push(AssemblyDecision {
                timestamp: Utc::now(),
                action: AssemblyAction::Exclude,
                target_type: format!("{:?}", section.section_type),
                target_id: Some(section.section_id),
                reason: "No budget remaining".to_string(),
                tokens_affected: 0,
            });
            return;
        }

        let original_tokens = section.token_count;
        section.content = truncate_to_token_budget(&section.content, available);
        section.token_count = estimate_tokens(&section.content);

        self.used_tokens += section.token_count;
        self.truncated = true;
        self.included_sections.push(format!("{:?}", section.section_type));
        self.assembly_trace.push(AssemblyDecision {
            timestamp: Utc::now(),
            action: AssemblyAction::Truncate,
            target_type: format!("{:?}", section.section_type),
            target_id: Some(section.section_id),
            reason: format!(
                "Truncated from {} to {} tokens",
                original_tokens, section.token_count
            ),
            tokens_affected: section.token_count,
        });
        self.sections.push(section);
    }

    /// Get the assembled content as a single string.
    pub fn as_text(&self) -> String {
        self.sections
            .iter()
            .map(|s| s.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

impl std::fmt::Display for ContextWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_text())
    }
}


// ============================================================================
// TOKEN UTILITIES (Task 7.3)
// ============================================================================

/// Estimate token count for text.
/// Rough estimate: ~0.75 tokens per character (English).
/// More accurate than 1:4 ratio, less accurate than tiktoken.
///
/// # Arguments
/// * `text` - The text to estimate tokens for
///
/// # Returns
/// Estimated token count (always >= 0)
pub fn estimate_tokens(text: &str) -> i32 {
    if text.is_empty() {
        return 0;
    }
    (text.len() as f32 * 0.75).ceil() as i32
}


// ============================================================================
// SMART TRUNCATION (Task 7.4)
// ============================================================================

/// Truncate text to fit within token budget.
/// Prefers sentence boundaries, falls back to word boundaries.
///
/// # Arguments
/// * `text` - The text to truncate
/// * `budget` - Maximum token budget
///
/// # Returns
/// Truncated text that fits within the budget
pub fn truncate_to_token_budget(text: &str, budget: i32) -> String {
    if budget <= 0 {
        return String::new();
    }

    // Convert token budget to approximate character limit
    // Since we estimate 0.75 tokens per char, we can have ~1.33 chars per token
    let max_chars = (budget as f32 / 0.75).floor() as usize;

    if text.len() <= max_chars {
        return text.to_string();
    }

    // Get the truncated portion (respecting UTF-8 boundaries)
    let truncated = safe_truncate(text, max_chars);

    // Try to find a sentence boundary (., ?, !)
    let last_period = truncated.rfind('.');
    let last_question = truncated.rfind('?');
    let last_exclaim = truncated.rfind('!');

    // Find the latest sentence boundary
    let last_sentence = [last_period, last_question, last_exclaim]
        .into_iter()
        .flatten()
        .max();

    // If we found a sentence boundary in the latter half, use it
    if let Some(pos) = last_sentence {
        if pos > max_chars / 2 {
            return truncated[..=pos].to_string();
        }
    }

    // Fall back to word boundary
    if let Some(pos) = truncated.rfind(' ') {
        // Only use word boundary if it's in the latter 80% of the text
        if pos > max_chars * 4 / 5 {
            return truncated[..pos].to_string();
        }
    }

    // Last resort: just use the truncated text
    truncated.to_string()
}

/// Safely truncate a string at a UTF-8 boundary.
fn safe_truncate(s: &str, max_chars: usize) -> &str {
    if s.len() <= max_chars {
        return s;
    }

    // Find the last valid UTF-8 boundary at or before max_chars
    let mut end = max_chars;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }

    &s[..end]
}


// ============================================================================
// CONTEXT ASSEMBLER (Task 7.5)
// ============================================================================

/// Context assembler that builds context windows from packages.
/// Adds sections by priority until budget is exhausted.
#[derive(Debug, Clone)]
pub struct ContextAssembler {
    /// Configuration for assembly
    config: CaliberConfig,
}

impl ContextAssembler {
    /// Create a new context assembler with the given configuration.
    pub fn new(config: CaliberConfig) -> CaliberResult<Self> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Assemble context from a package with token budget management.
    /// Sections are added in priority order until budget is exhausted.
    ///
    /// # Arguments
    /// * `pkg` - The context package to assemble
    ///
    /// # Returns
    /// An assembled ContextWindow with sections ordered by priority
    pub fn assemble(&self, pkg: ContextPackage) -> CaliberResult<ContextWindow> {
        let mut window = ContextWindow::new(self.config.token_budget);

        // Build sections from the package
        let mut sections = self.build_sections(&pkg);

        // Sort sections by priority (descending - higher priority first)
        sections.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Add sections in priority order until budget is exhausted
        for section in sections {
            if window.remaining_tokens() <= 0 {
                // No more budget, record exclusion
                window.assembly_trace.push(AssemblyDecision {
                    timestamp: Utc::now(),
                    action: AssemblyAction::Exclude,
                    target_type: format!("{:?}", section.section_type),
                    target_id: Some(section.section_id),
                    reason: "Budget exhausted".to_string(),
                    tokens_affected: 0,
                });
                continue;
            }

            if section.token_count <= window.remaining_tokens() {
                // Section fits completely
                window.add_section(section);
            } else if section.compressible {
                // Section doesn't fit but can be truncated
                window.add_truncated_section(section);
            } else {
                // Section doesn't fit and can't be truncated
                window.assembly_trace.push(AssemblyDecision {
                    timestamp: Utc::now(),
                    action: AssemblyAction::Exclude,
                    target_type: format!("{:?}", section.section_type),
                    target_id: Some(section.section_id),
                    reason: format!(
                        "Exceeds budget ({} tokens) and not compressible",
                        section.token_count
                    ),
                    tokens_affected: 0,
                });
            }
        }

        Ok(window)
    }

    /// Build sections from a context package.
    fn build_sections(&self, pkg: &ContextPackage) -> Vec<ContextSection> {
        let mut sections = Vec::new();

        // Add persona/kernel config section (highest priority typically)
        if let Some(ref kernel) = pkg.kernel_config {
            let content = self.format_kernel_config(kernel);
            if !content.is_empty() {
                let mut section =
                    ContextSection::new(SectionType::Persona, content, self.config.section_priorities.persona);
                section.compressible = false; // Persona shouldn't be truncated
                sections.push(section);
            }
        }

        // Add user input section
        if let Some(ref input) = pkg.user_input {
            let mut section =
                ContextSection::new(SectionType::User, input.clone(), self.config.section_priorities.user);
            section.compressible = false; // User input shouldn't be truncated
            sections.push(section);
        }

        // Add notes section
        if !pkg.relevant_notes.is_empty() {
            let content = self.format_notes(&pkg.relevant_notes);
            let sources: Vec<SourceRef> = pkg
                .relevant_notes
                .iter()
                .map(|n| SourceRef {
                    source_type: EntityType::Note,
                    id: Some(n.note_id),
                    relevance_score: None,
                })
                .collect();
            let section = ContextSection::new(
                SectionType::Notes,
                content,
                self.config.section_priorities.notes,
            )
            .with_sources(sources);
            sections.push(section);
        }

        // Add artifacts section
        if !pkg.recent_artifacts.is_empty() {
            let content = self.format_artifacts(&pkg.recent_artifacts);
            let sources: Vec<SourceRef> = pkg
                .recent_artifacts
                .iter()
                .map(|a| SourceRef {
                    source_type: EntityType::Artifact,
                    id: Some(a.artifact_id),
                    relevance_score: None,
                })
                .collect();
            let section = ContextSection::new(
                SectionType::Artifacts,
                content,
                self.config.section_priorities.artifacts,
            )
            .with_sources(sources);
            sections.push(section);
        }

        // Add history section (scope summaries)
        if !pkg.scope_summaries.is_empty() {
            let content = self.format_scope_summaries(&pkg.scope_summaries);
            let sources: Vec<SourceRef> = pkg
                .scope_summaries
                .iter()
                .map(|s| SourceRef {
                    source_type: EntityType::Scope,
                    id: Some(s.scope_id),
                    relevance_score: None,
                })
                .collect();
            let section = ContextSection::new(
                SectionType::History,
                content,
                self.config.section_priorities.history,
            )
            .with_sources(sources);
            sections.push(section);
        }

        sections
    }

    /// Format kernel config into a string.
    fn format_kernel_config(&self, kernel: &KernelConfig) -> String {
        let mut parts = Vec::new();

        if let Some(ref persona) = kernel.persona {
            parts.push(format!("Persona: {}", persona));
        }
        if let Some(ref tone) = kernel.tone {
            parts.push(format!("Tone: {}", tone));
        }
        if let Some(ref style) = kernel.reasoning_style {
            parts.push(format!("Reasoning Style: {}", style));
        }
        if let Some(ref focus) = kernel.domain_focus {
            parts.push(format!("Domain Focus: {}", focus));
        }

        parts.join("\n")
    }

    /// Format notes into a string.
    fn format_notes(&self, notes: &[Note]) -> String {
        notes
            .iter()
            .map(|n| format!("[{}] {}: {}", n.note_type as u8, n.title, n.content))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Format artifacts into a string.
    fn format_artifacts(&self, artifacts: &[Artifact]) -> String {
        artifacts
            .iter()
            .map(|a| format!("[{:?}] {}: {}", a.artifact_type, a.name, a.content))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Format scope summaries into a string.
    fn format_scope_summaries(&self, summaries: &[ScopeSummary]) -> String {
        summaries
            .iter()
            .map(|s| s.summary.clone())
            .collect::<Vec<_>>()
            .join("\n\n---\n\n")
    }

    /// Get the token budget from config.
    pub fn token_budget(&self) -> i32 {
        self.config.token_budget
    }
}


// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use caliber_core::{
        ContextPersistence, NoteType, RetryConfig,
        SectionPriorities, TTL, ValidationMode,
    };
    use std::time::Duration;

    fn make_test_config(token_budget: i32) -> CaliberConfig {
        CaliberConfig {
            token_budget,
            section_priorities: SectionPriorities {
                user: 100,
                system: 90,
                persona: 85,
                artifacts: 80,
                notes: 70,
                history: 60,
                custom: vec![],
            },
            checkpoint_retention: 10,
            stale_threshold: Duration::from_secs(3600),
            contradiction_threshold: 0.8,
            context_window_persistence: ContextPersistence::Ephemeral,
            validation_mode: ValidationMode::OnMutation,
            embedding_provider: None,
            summarization_provider: None,
            llm_retry_config: RetryConfig {
                max_retries: 3,
                initial_backoff: Duration::from_millis(100),
                max_backoff: Duration::from_secs(10),
                backoff_multiplier: 2.0,
            },
            lock_timeout: Duration::from_secs(30),
            message_retention: Duration::from_secs(86400),
            delegation_timeout: Duration::from_secs(300),
        }
    }

    fn make_test_note(title: &str, content: &str) -> Note {
        Note {
            note_id: Uuid::now_v7(),
            note_type: NoteType::Fact,
            title: title.to_string(),
            content: content.to_string(),
            content_hash: [0u8; 32],
            embedding: None,
            source_trajectory_ids: vec![],
            source_artifact_ids: vec![],
            ttl: TTL::Persistent,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            accessed_at: Utc::now(),
            access_count: 0,
            superseded_by: None,
            metadata: None,
            abstraction_level: caliber_core::AbstractionLevel::Raw,
            source_note_ids: vec![],
        }
    }

    #[test]
    fn test_estimate_tokens_empty() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_estimate_tokens_short() {
        // "hello" = 5 chars * 0.75 = 3.75, ceil = 4
        assert_eq!(estimate_tokens("hello"), 4);
    }

    #[test]
    fn test_estimate_tokens_longer() {
        // 100 chars * 0.75 = 75 tokens
        let text = "a".repeat(100);
        assert_eq!(estimate_tokens(&text), 75);
    }

    #[test]
    fn test_truncate_empty_budget() {
        let result = truncate_to_token_budget("hello world", 0);
        assert_eq!(result, "");
    }

    #[test]
    fn test_truncate_fits() {
        let text = "hello";
        let result = truncate_to_token_budget(text, 100);
        assert_eq!(result, text);
    }

    #[test]
    fn test_truncate_sentence_boundary() {
        let text = "First sentence. Second sentence. Third sentence.";
        // Budget for ~20 chars (15 tokens)
        let result = truncate_to_token_budget(text, 15);
        // Should truncate at a sentence boundary
        assert!(result.ends_with('.'));
    }

    #[test]
    fn test_context_window_new() {
        let window = ContextWindow::new(1000);
        assert_eq!(window.max_tokens, 1000);
        assert_eq!(window.used_tokens, 0);
        assert!(window.sections.is_empty());
    }

    #[test]
    fn test_context_window_add_section() {
        let mut window = ContextWindow::new(1000);
        let section = ContextSection::new(SectionType::User, "Hello".to_string(), 100);
        assert!(window.add_section(section));
        assert_eq!(window.sections.len(), 1);
        assert!(window.used_tokens > 0);
    }

    #[test]
    fn test_context_assembler_basic() -> CaliberResult<()> {
        let config = make_test_config(10000);
        let assembler = ContextAssembler::new(config)?;

        let pkg = ContextPackage::new(Uuid::now_v7(), Uuid::now_v7())
            .with_user_input("What is the weather?".to_string());

        let window = assembler.assemble(pkg)?;
        assert!(window.used_tokens > 0);
        assert!(window.used_tokens <= window.max_tokens);
        Ok(())
    }

    #[test]
    fn test_context_assembler_with_notes() -> CaliberResult<()> {
        let config = make_test_config(10000);
        let assembler = ContextAssembler::new(config)?;

        let notes = vec![
            make_test_note("Note 1", "Content of note 1"),
            make_test_note("Note 2", "Content of note 2"),
        ];

        let pkg = ContextPackage::new(Uuid::now_v7(), Uuid::now_v7())
            .with_user_input("Query".to_string())
            .with_notes(notes);

        let window = assembler.assemble(pkg)?;
        assert!(window.sections.len() >= 2); // User + Notes
        Ok(())
    }

    #[test]
    fn test_context_assembler_respects_budget() -> CaliberResult<()> {
        // Very small budget
        let config = make_test_config(10);
        let assembler = ContextAssembler::new(config)?;

        let pkg = ContextPackage::new(Uuid::now_v7(), Uuid::now_v7())
            .with_user_input("This is a very long user input that should exceed the token budget".to_string());

        let window = assembler.assemble(pkg)?;
        // Should respect budget
        assert!(window.used_tokens <= window.max_tokens);
        Ok(())
    }
}

// ============================================================================
// PROPERTY-BASED TESTS (Task 7.6)
// ============================================================================

#[cfg(test)]
mod prop_tests {
    use super::*;
    use caliber_core::{
        ArtifactType, ContextPersistence, ExtractionMethod, NoteType, Provenance, RetryConfig,
        SectionPriorities, TTL, ValidationMode,
    };
    use proptest::prelude::*;
    use std::time::Duration;

    fn make_test_config(token_budget: i32) -> CaliberConfig {
        CaliberConfig {
            token_budget,
            section_priorities: SectionPriorities {
                user: 100,
                system: 90,
                persona: 85,
                artifacts: 80,
                notes: 70,
                history: 60,
                custom: vec![],
            },
            checkpoint_retention: 10,
            stale_threshold: Duration::from_secs(3600),
            contradiction_threshold: 0.8,
            context_window_persistence: ContextPersistence::Ephemeral,
            validation_mode: ValidationMode::OnMutation,
            embedding_provider: None,
            summarization_provider: None,
            llm_retry_config: RetryConfig {
                max_retries: 3,
                initial_backoff: Duration::from_millis(100),
                max_backoff: Duration::from_secs(10),
                backoff_multiplier: 2.0,
            },
            lock_timeout: Duration::from_secs(30),
            message_retention: Duration::from_secs(86400),
            delegation_timeout: Duration::from_secs(300),
        }
    }

    fn arb_note() -> impl Strategy<Value = Note> {
        (any::<[u8; 16]>(), ".*", ".*").prop_map(|(id_bytes, title, content)| Note {
            note_id: Uuid::from_bytes(id_bytes),
            note_type: NoteType::Fact,
            title,
            content,
            content_hash: [0u8; 32],
            embedding: None,
            source_trajectory_ids: vec![],
            source_artifact_ids: vec![],
            ttl: TTL::Persistent,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            accessed_at: Utc::now(),
            access_count: 0,
            superseded_by: None,
            metadata: None,
            abstraction_level: caliber_core::AbstractionLevel::Raw,
            source_note_ids: vec![],
        })
    }

    fn arb_artifact() -> impl Strategy<Value = Artifact> {
        (any::<[u8; 16]>(), any::<[u8; 16]>(), any::<[u8; 16]>(), ".*", ".*").prop_map(
            |(id_bytes, traj_bytes, scope_bytes, name, content)| Artifact {
                artifact_id: Uuid::from_bytes(id_bytes),
                trajectory_id: Uuid::from_bytes(traj_bytes),
                scope_id: Uuid::from_bytes(scope_bytes),
                artifact_type: ArtifactType::Fact,
                name,
                content,
                content_hash: [0u8; 32],
                embedding: None,
                provenance: Provenance {
                    source_turn: 1,
                    extraction_method: ExtractionMethod::Explicit,
                    confidence: Some(1.0),
                },
                ttl: TTL::Persistent,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                superseded_by: None,
                metadata: None,
            },
        )
    }

    // ========================================================================
    // Property 8: Context assembly respects token budget
    // Feature: caliber-core-implementation, Property 8: Context assembly respects token budget
    // Validates: Requirements 9.3
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 8: For any ContextWindow assembled with max_tokens = N,
        /// used_tokens SHALL be <= N
        #[test]
        fn prop_context_assembly_respects_token_budget(
            token_budget in 1i32..10000,
            user_input in ".*",
            notes in prop::collection::vec(arb_note(), 0..5),
            artifacts in prop::collection::vec(arb_artifact(), 0..5),
        ) {
            let config = make_test_config(token_budget);
            let assembler = match ContextAssembler::new(config) {
                Ok(assembler) => assembler,
                Err(err) => {
                    prop_assert!(false, "Failed to build ContextAssembler: {:?}", err);
                    return;
                }
            };

            let pkg = ContextPackage::new(Uuid::now_v7(), Uuid::now_v7())
                .with_user_input(user_input)
                .with_notes(notes)
                .with_artifacts(artifacts);

            let window = match assembler.assemble(pkg) {
                Ok(window) => window,
                Err(err) => {
                    prop_assert!(false, "Failed to assemble context: {:?}", err);
                    return;
                }
            };

            prop_assert!(
                window.used_tokens <= window.max_tokens,
                "used_tokens ({}) should be <= max_tokens ({})",
                window.used_tokens,
                window.max_tokens
            );
        }
    }

    // ========================================================================
    // Property 11: Context sections ordered by priority
    // Feature: caliber-core-implementation, Property 11: Context sections ordered by priority
    // Validates: Requirements 9.2
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 11: For any assembled ContextWindow, sections SHALL be
        /// ordered by descending priority
        #[test]
        fn prop_context_sections_ordered_by_priority(
            token_budget in 1000i32..50000,
            user_input in ".{1,100}",
            notes in prop::collection::vec(arb_note(), 1..3),
            artifacts in prop::collection::vec(arb_artifact(), 1..3),
        ) {
            let config = make_test_config(token_budget);
            let assembler = match ContextAssembler::new(config) {
                Ok(assembler) => assembler,
                Err(err) => {
                    prop_assert!(false, "Failed to build ContextAssembler: {:?}", err);
                    return;
                }
            };

            let pkg = ContextPackage::new(Uuid::now_v7(), Uuid::now_v7())
                .with_user_input(user_input)
                .with_notes(notes)
                .with_artifacts(artifacts);

            let window = match assembler.assemble(pkg) {
                Ok(window) => window,
                Err(err) => {
                    prop_assert!(false, "Failed to assemble context: {:?}", err);
                    return;
                }
            };

            // Check that sections are in descending priority order
            for i in 1..window.sections.len() {
                prop_assert!(
                    window.sections[i - 1].priority >= window.sections[i].priority,
                    "Section {} (priority {}) should have >= priority than section {} (priority {})",
                    i - 1,
                    window.sections[i - 1].priority,
                    i,
                    window.sections[i].priority
                );
            }
        }
    }

    // ========================================================================
    // Property 12: Token estimation consistency
    // Feature: caliber-core-implementation, Property 12: Token estimation consistency
    // Validates: Context assembly token management
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 12: For any text T, estimate_tokens(T) SHALL be >= 0
        /// AND approximately proportional to T.len()
        #[test]
        fn prop_token_estimation_consistency(text in ".*") {
            let tokens = estimate_tokens(&text);

            // Tokens should always be non-negative
            prop_assert!(tokens >= 0, "Token count should be >= 0, got {}", tokens);

            // Tokens should be approximately proportional to length
            // With 0.75 tokens per char, tokens should be roughly 0.75 * len
            if !text.is_empty() {
                let expected_approx = (text.len() as f32 * 0.75).ceil() as i32;
                prop_assert_eq!(
                    tokens,
                    expected_approx,
                    "Token count {} should equal expected {} for text of length {}",
                    tokens,
                    expected_approx,
                    text.len()
                );
            }
        }

        /// Property 12: Empty text should have 0 tokens
        #[test]
        fn prop_empty_text_zero_tokens(_iteration in 0..100u32) {
            prop_assert_eq!(estimate_tokens(""), 0);
        }
    }

    // ========================================================================
    // Property 13: Truncation respects budget
    // Feature: caliber-core-implementation, Property 13: Truncation respects budget
    // Validates: Context assembly truncation
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 13: For any text T and budget B,
        /// estimate_tokens(truncate_to_token_budget(T, B)) SHALL be <= B
        #[test]
        fn prop_truncation_respects_budget(
            text in ".{0,1000}",
            budget in 1i32..500,
        ) {
            let truncated = truncate_to_token_budget(&text, budget);
            let truncated_tokens = estimate_tokens(&truncated);

            prop_assert!(
                truncated_tokens <= budget,
                "Truncated text has {} tokens, should be <= budget {}",
                truncated_tokens,
                budget
            );
        }

        /// Property 13: Zero budget should produce empty string
        #[test]
        fn prop_zero_budget_empty_result(text in ".*") {
            let truncated = truncate_to_token_budget(&text, 0);
            prop_assert_eq!(truncated, "", "Zero budget should produce empty string");
        }

        /// Property 13: Negative budget should produce empty string
        #[test]
        fn prop_negative_budget_empty_result(
            text in ".*",
            budget in i32::MIN..-1,
        ) {
            let truncated = truncate_to_token_budget(&text, budget);
            prop_assert_eq!(truncated, "", "Negative budget should produce empty string");
        }

        /// Property 13: If text fits in budget, it should be unchanged
        #[test]
        fn prop_text_fits_unchanged(text in ".{0,100}") {
            // Large budget that should fit any text up to 100 chars
            let budget = 1000;
            let truncated = truncate_to_token_budget(&text, budget);
            prop_assert_eq!(
                truncated,
                text,
                "Text that fits should be unchanged"
            );
        }
    }
}
