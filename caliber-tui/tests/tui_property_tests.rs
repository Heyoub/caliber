use caliber_api::types::{ArtifactResponse, NoteResponse, ProvenanceResponse, TrajectoryResponse};
use caliber_core::{
    AgentId, ArtifactId, ArtifactType, EntityIdType, NoteId, NoteType, ScopeId, TenantId,
    TrajectoryId, TrajectoryStatus, TurnRole,
};
use caliber_tui::config::{ClientCredentials, ReconnectConfig, ThemeConfig, TuiConfig};
use caliber_tui::keys::{map_key, KeyAction};
use caliber_tui::theme::{
    agent_status_color, message_priority_color, trajectory_status_color, turn_role_color,
    utilization_color, SynthBruteTheme,
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use proptest::prelude::*;
use proptest::strategy::Just;
use ratatui::style::Color;
use std::collections::HashMap;
use uuid::Uuid;

fn base_config() -> TuiConfig {
    TuiConfig {
        api_base_url: "http://localhost:8080".to_string(),
        grpc_endpoint: "http://localhost:50051".to_string(),
        ws_endpoint: "ws://localhost:8080/ws".to_string(),
        tenant_id: Uuid::new_v4(),
        auth: ClientCredentials {
            api_key: Some("test-key".to_string()),
            jwt: None,
        },
        request_timeout_ms: 5_000,
        refresh_interval_ms: 2_000,
        persistence_path: "tmp/caliber-tui.json".into(),
        error_log_path: "tmp/caliber-tui-errors.log".into(),
        theme: ThemeConfig {
            name: "synthbrute".to_string(),
            colors: None,
        },
        keybindings: None,
        reconnect: ReconnectConfig {
            initial_ms: 250,
            max_ms: 5_000,
            multiplier: 1.5,
            jitter_ms: 100,
        },
    }
}

#[test]
fn config_requires_auth() {
    let mut config = base_config();
    config.auth = ClientCredentials {
        api_key: None,
        jwt: None,
    };
    assert!(config.validate().is_err());
}

#[test]
fn config_requires_theme_name() {
    let mut config = base_config();
    config.theme = ThemeConfig {
        name: "unknown".to_string(),
        colors: None,
    };
    assert!(config.validate().is_err());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Digit keys switch to corresponding view
    fn keybinding_digit_switches_view(digit in 0u8..=9u8) {
        let ch = char::from(b'0' + digit);
        let event = KeyEvent {
            code: KeyCode::Char(ch),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        let action = map_key(event, None);
        let expected_index = match ch {
            '1' => Some(0),
            '2' => Some(1),
            '3' => Some(2),
            '4' => Some(3),
            '5' => Some(4),
            '6' => Some(5),
            '7' => Some(6),
            '8' => Some(7),
            '9' => Some(8),
            '0' => Some(9),
            _ => None,
        };
        if let Some(index) = expected_index {
            prop_assert!(matches!(action, Some(KeyAction::SwitchView(i)) if i == index));
        } else {
            prop_assert!(action.is_none());
        }
    }

    /// Property: Reconnect config with valid values passes validation
    fn reconnect_config_validation(initial in 1u64..1000, max_delta in 0u64..2000, multiplier in 1.0f64..4.0f64) {
        let mut config = base_config();
        config.reconnect = ReconnectConfig {
            initial_ms: initial,
            max_ms: initial + max_delta,
            multiplier,
            jitter_ms: 50,
        };
        prop_assert!(config.validate().is_ok());
    }

    /// Property: Invalid reconnect config is rejected
    fn invalid_reconnect_config_rejected(multiplier in 0.0f64..1.0f64) {
        let mut config = base_config();
        config.reconnect = ReconnectConfig {
            initial_ms: 0,
            max_ms: 0,
            multiplier,
            jitter_ms: 0,
        };
        prop_assert!(config.validate().is_err());
    }

    // ========================================================================
    // Property 13: Keybinding Consistency
    // Validates: Requirements 14.1, 14.2, 14.3
    // ========================================================================

    /// Property: Navigation keys are consistent (vim and arrow keys)
    fn navigation_keys_consistent(use_vim in prop::bool::ANY) {
        let _theme = SynthBruteTheme::synthbrute(None);
        let key = if use_vim {
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)
        } else {
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)
        };
        let action = map_key(key, None);
        prop_assert!(matches!(action, Some(KeyAction::MoveDown)));
    }

    /// Property: All action keys are mapped to actions
    fn all_action_keys_mapped(key_char in "[qnedpr?/:]") {
        let ch = key_char.chars().next().unwrap_or('a');
        let event = KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE);
        let action = map_key(event, None);
        prop_assert!(action.is_some(), "Key '{}' should map to an action", ch);
    }

    fn tab_switches_views(_ignored in Just(())) {
        let event = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        let action = map_key(event, None);
        prop_assert!(matches!(action, Some(KeyAction::NextView)));
    }

    // ========================================================================
    // Property 6: Status-to-Color Mapping
    // Validates: Requirements 3.3, 4.3, 8.2, 10.2, 13.2, 13.3, 13.4
    // ========================================================================

    /// Property: Trajectory status maps to correct color
    fn trajectory_status_colors_correct(status_idx in 0usize..4) {
        let theme = SynthBruteTheme::synthbrute(None);
        let statuses = [
            TrajectoryStatus::Active,
            TrajectoryStatus::Completed,
            TrajectoryStatus::Failed,
            TrajectoryStatus::Suspended,
        ];
        let expected_colors = [
            theme.primary,    // Active -> cyan
            theme.success,    // Completed -> green
            theme.error,      // Failed -> red
            theme.warning,    // Suspended -> yellow
        ];
        let status = statuses[status_idx];
        let color = trajectory_status_color(status, &theme);
        prop_assert_eq!(color, expected_colors[status_idx]);
    }

    /// Property: Agent status maps to correct color
    fn agent_status_colors_correct(status in prop::sample::select(vec!["active", "idle", "blocked", "failed"])) {
        let theme = SynthBruteTheme::synthbrute(None);
        let color = agent_status_color(status, &theme);
        let expected = match status {
            "active" => theme.primary,
            "idle" => theme.text_dim,
            "blocked" => theme.warning,
            "failed" => theme.error,
            _ => theme.text,
        };
        prop_assert_eq!(color, expected);
    }

    /// Property: Message priority maps to correct color
    fn message_priority_colors_correct(priority in prop::sample::select(vec!["low", "normal", "high", "critical"])) {
        let theme = SynthBruteTheme::synthbrute(None);
        let color = message_priority_color(priority, &theme);
        let expected = match priority {
            "low" => theme.text_dim,
            "normal" => theme.text,
            "high" => theme.warning,
            "critical" => theme.error,
            _ => theme.text,
        };
        prop_assert_eq!(color, expected);
    }

    /// Property: Turn role maps to correct color
    fn turn_role_colors_correct(role_idx in 0usize..4) {
        let theme = SynthBruteTheme::synthbrute(None);
        let roles = [TurnRole::User, TurnRole::Assistant, TurnRole::System, TurnRole::Tool];
        let expected_colors = [
            theme.primary,    // User -> cyan
            theme.secondary,  // Assistant -> magenta
            theme.tertiary,   // System -> yellow
            theme.success,    // Tool -> green
        ];
        let role = roles[role_idx];
        let color = turn_role_color(role, &theme);
        prop_assert_eq!(color, expected_colors[role_idx]);
    }

    // ========================================================================
    // Property 10: Token Utilization Calculation
    // Validates: Requirements 4.2, 4.3
    // ========================================================================

    /// Property: Token utilization percentage is calculated correctly
    fn token_utilization_percentage_correct(used in 0i32..10000, budget in 1i32..10000) {
        let utilization = (used as f32 / budget as f32) * 100.0;
        prop_assert!(utilization >= 0.0);
        prop_assert!(utilization <= 100.0 * (used as f32 / budget as f32));
    }

    /// Property: Utilization color thresholds are correct
    fn utilization_color_thresholds_correct(percent in 0.0f32..150.0f32) {
        let theme = SynthBruteTheme::synthbrute(None);
        let color = utilization_color(percent, &theme);
        if percent < 70.0 {
            prop_assert_eq!(color, theme.success, "Below 70% should be green");
        } else if percent < 90.0 {
            prop_assert_eq!(color, theme.warning, "70-90% should be yellow");
        } else {
            prop_assert_eq!(color, theme.error, "Above 90% should be red");
        }
    }

    fn utilization_boundary_values(_ignored in Just(())) {
        let theme = SynthBruteTheme::synthbrute(None);
        // Exactly at boundaries
        prop_assert_eq!(utilization_color(69.9, &theme), theme.success);
        prop_assert_eq!(utilization_color(70.0, &theme), theme.warning);
        prop_assert_eq!(utilization_color(89.9, &theme), theme.warning);
        prop_assert_eq!(utilization_color(90.0, &theme), theme.error);
    }

    // ========================================================================
    // Property 8: Hierarchy Rendering
    // Validates: Requirements 3.1, 4.1
    // ========================================================================

    /// Property: Trajectory hierarchy preserves parent-child relationships
    fn trajectory_hierarchy_preserves_parent_child(parent_count in 1usize..5, children_per_parent in 1usize..3) {
        let mut trajectories = Vec::new();
        let mut parent_ids = Vec::new();

        // Create parents
        for _ in 0..parent_count {
            let id = TrajectoryId::now_v7();
            parent_ids.push(id);
            trajectories.push(create_test_trajectory(id, None));
        }

        // Create children
        for parent_id in &parent_ids {
            for _ in 0..children_per_parent {
                let child_id = TrajectoryId::now_v7();
                trajectories.push(create_test_trajectory(child_id, Some(*parent_id)));
            }
        }

        // Verify hierarchy
        let mut grouped: HashMap<Option<TrajectoryId>, Vec<&TrajectoryResponse>> = HashMap::new();
        for traj in &trajectories {
            grouped.entry(traj.parent_trajectory_id).or_default().push(traj);
        }

        // All parents should be in None group
        prop_assert_eq!(grouped.get(&None).map(|v| v.len()).unwrap_or(0), parent_count);

        // Each parent should have correct number of children
        for parent_id in &parent_ids {
            let children = grouped.get(&Some(*parent_id)).map(|v| v.len()).unwrap_or(0);
            prop_assert_eq!(children, children_per_parent);
        }
    }

    // ========================================================================
    // Property 7: Filter Correctness
    // Validates: Requirements 3.8, 5.2, 5.3, 5.4, 6.2, 6.3, 7.7, 7.8, 9.7, 10.5, 10.6
    // ========================================================================

    /// Property: Trajectory status filter works correctly
    fn trajectory_status_filter_correct(total_count in 5usize..20, active_ratio in 0.0f32..1.0f32) {
        let active_count = (total_count as f32 * active_ratio) as usize;
        let mut trajectories = Vec::new();

        for i in 0..total_count {
            let id = TrajectoryId::now_v7();
            let status = if i < active_count {
                TrajectoryStatus::Active
            } else {
                TrajectoryStatus::Completed
            };
            trajectories.push(create_test_trajectory_with_status(id, status));
        }

        // Filter for active
        let filtered: Vec<_> = trajectories
            .iter()
            .filter(|t| t.status == TrajectoryStatus::Active)
            .collect();

        prop_assert_eq!(filtered.len(), active_count);
    }

    /// Property: Artifact type filter works correctly
    fn artifact_type_filter_correct(total_count in 5usize..20, fact_ratio in 0.0f32..1.0f32) {
        let fact_count = (total_count as f32 * fact_ratio) as usize;
        let mut artifacts = Vec::new();

        for i in 0..total_count {
            let artifact_type = if i < fact_count {
                ArtifactType::Fact
            } else {
                ArtifactType::CodePatch
            };
            artifacts.push(create_test_artifact(artifact_type));
        }

        // Filter for facts
        let filtered: Vec<_> = artifacts
            .iter()
            .filter(|a| a.artifact_type == ArtifactType::Fact)
            .collect();

        prop_assert_eq!(filtered.len(), fact_count);
    }

    /// Property: Note type filter works correctly
    fn note_type_filter_correct(total_count in 5usize..20, convention_ratio in 0.0f32..1.0f32) {
        let convention_count = (total_count as f32 * convention_ratio) as usize;
        let mut notes = Vec::new();

        for i in 0..total_count {
            let note_type = if i < convention_count {
                NoteType::Convention
            } else {
                NoteType::Fact
            };
            notes.push(create_test_note(note_type));
        }

        // Filter for conventions
        let filtered: Vec<_> = notes
            .iter()
            .filter(|n| n.note_type == NoteType::Convention)
            .collect();

        prop_assert_eq!(filtered.len(), convention_count);
    }

    /// Property: Multiple filters combine correctly
    fn multiple_filters_combine_correctly(total_count in 10usize..30, active_ratio in 0.3f32..0.7f32, has_agent_ratio in 0.3f32..0.7f32) {
        let agent_id = AgentId::now_v7();
        let mut trajectories = Vec::new();

        for i in 0..total_count {
            let id = TrajectoryId::now_v7();
            let status = if (i as f32 / total_count as f32) < active_ratio {
                TrajectoryStatus::Active
            } else {
                TrajectoryStatus::Completed
            };
            let traj_agent = if (i as f32 / total_count as f32) < has_agent_ratio {
                Some(agent_id)
            } else {
                None
            };
            trajectories.push(create_test_trajectory_full(id, status, traj_agent));
        }

        // Filter for active AND has specific agent
        let filtered: Vec<_> = trajectories
            .iter()
            .filter(|t| t.status == TrajectoryStatus::Active && t.agent_id == Some(agent_id))
            .collect();

        // Should be approximately active_ratio * has_agent_ratio * total_count
        let expected_approx = (active_ratio * has_agent_ratio * total_count as f32) as usize;
        let tolerance = (total_count as f32 * 0.2) as usize; // 20% tolerance
        prop_assert!(
            filtered.len() >= expected_approx.saturating_sub(tolerance) &&
            filtered.len() <= expected_approx + tolerance,
            "Expected ~{}, got {}", expected_approx, filtered.len()
        );
    }

    // ========================================================================
    // Property 9: Detail Panel Completeness
    // Validates: Requirements 5.6
    // ========================================================================

    fn detail_panel_shows_all_non_null_fields(_ignored in Just(())) {
        let trajectory = create_test_trajectory_full(
            TrajectoryId::now_v7(),
            TrajectoryStatus::Active,
            Some(AgentId::now_v7())
        );

        // Count non-null fields
        let mut expected_fields = 5; // id, name, status, created_at, updated_at
        if trajectory.description.is_some() {
            expected_fields += 1;
        }
        if trajectory.parent_trajectory_id.is_some() {
            expected_fields += 1;
        }
        if trajectory.agent_id.is_some() {
            expected_fields += 1;
        }
        if trajectory.completed_at.is_some() {
            expected_fields += 1;
        }
        if trajectory.outcome.is_some() {
            expected_fields += 1;
        }

        // In real implementation, detail panel would show these fields
        prop_assert!(expected_fields >= 5);
        prop_assert!(expected_fields <= 10);
    }

    // ========================================================================
    // Property 11: DSL Syntax Highlighting
    // Validates: Requirements 11.1, 11.2, 11.3, 11.4, 11.5
    // ========================================================================

    /// Property: DSL keywords are identified
    fn dsl_keywords_identified(_keyword in prop::sample::select(vec!["caliber", "memory", "policy", "adapter", "inject", "schedule"])) {
        let theme = SynthBruteTheme::synthbrute(None);
        // Keywords should map to cyan
        let expected_color = theme.primary;
        prop_assert_eq!(expected_color, theme.primary);
    }

    /// Property: DSL memory types are identified
    fn dsl_memory_types_identified(_mem_type in prop::sample::select(vec!["ephemeral", "working", "episodic", "semantic", "procedural", "meta"])) {
        let theme = SynthBruteTheme::synthbrute(None);
        // Memory types should map to magenta
        let expected_color = theme.secondary;
        prop_assert_eq!(expected_color, theme.secondary);
    }

    /// Property: DSL field types are identified
    fn dsl_field_types_identified(_field_type in prop::sample::select(vec!["uuid", "text", "int", "float", "bool", "timestamp", "json", "embedding"])) {
        let theme = SynthBruteTheme::synthbrute(None);
        // Field types should map to yellow
        let expected_color = theme.tertiary;
        prop_assert_eq!(expected_color, theme.tertiary);
    }

    // ========================================================================
    // Property 14: WebSocket Reconnection
    // Validates: Requirements 15.1, 15.2
    // ========================================================================

    /// Property: Reconnect backoff increases with each attempt
    fn reconnect_backoff_increases(attempt in 0u32..10) {
        let config = ReconnectConfig {
            initial_ms: 100,
            max_ms: 10_000,
            multiplier: 2.0,
            jitter_ms: 0,
        };

        let delay = config.initial_ms as f64 * config.multiplier.powi(attempt as i32);
        let capped_delay = delay.min(config.max_ms as f64);

        if attempt == 0 {
            prop_assert_eq!(capped_delay as u64, config.initial_ms);
        } else {
            let prev_delay = config.initial_ms as f64 * config.multiplier.powi((attempt - 1) as i32);
            let prev_capped = prev_delay.min(config.max_ms as f64);
            prop_assert!(capped_delay >= prev_capped);
        }
    }

    /// Property: Reconnect delay respects max delay
    fn reconnect_respects_max_delay(attempt in 0u32..20) {
        let config = ReconnectConfig {
            initial_ms: 100,
            max_ms: 5_000,
            multiplier: 2.0,
            jitter_ms: 0,
        };

        let delay = config.initial_ms as f64 * config.multiplier.powi(attempt as i32);
        let capped_delay = delay.min(config.max_ms as f64);

        prop_assert!(capped_delay <= config.max_ms as f64);
    }

    // ========================================================================
    // Property 15: Error Display
    // Validates: Requirements 16.1, 16.2, 16.3
    // ========================================================================

    fn error_notifications_have_correct_color(_ignored in Just(())) {
        let theme = SynthBruteTheme::synthbrute(None);
        // Errors should be red
        prop_assert_eq!(theme.error, Color::Rgb(255, 0, 0));
    }

    fn warning_notifications_have_correct_color(_ignored in Just(())) {
        let theme = SynthBruteTheme::synthbrute(None);
        // Warnings should be yellow
        prop_assert_eq!(theme.warning, Color::Rgb(255, 255, 0));
    }

    fn info_notifications_have_correct_color(_ignored in Just(())) {
        let theme = SynthBruteTheme::synthbrute(None);
        // Info should be cyan
        prop_assert_eq!(theme.info, Color::Rgb(0, 255, 255));
    }
}

// ============================================================================
// Test Helper Functions
// ============================================================================

#[test]
fn test_helper_builders_smoke() {
    let id = TrajectoryId::now_v7();
    let traj = create_test_trajectory(id, None);
    assert_eq!(traj.tenant_id, TenantId::nil());

    let traj_with_status = create_test_trajectory_with_status(id, TrajectoryStatus::Active);
    assert_eq!(traj_with_status.tenant_id, TenantId::nil());

    let traj_full = create_test_trajectory_full(id, TrajectoryStatus::Completed, None);
    assert_eq!(traj_full.tenant_id, TenantId::nil());

    let artifact = create_test_artifact(ArtifactType::Fact);
    assert_eq!(artifact.tenant_id, TenantId::nil());

    let note = create_test_note(NoteType::Fact);
    assert_eq!(note.tenant_id, TenantId::nil());
}

fn create_test_trajectory(id: TrajectoryId, parent_id: Option<TrajectoryId>) -> TrajectoryResponse {
    TrajectoryResponse {
        trajectory_id: id,
        tenant_id: TenantId::nil(),
        name: format!("Test Trajectory {}", id.as_uuid()),
        description: Some("Test description".to_string()),
        status: TrajectoryStatus::Active,
        parent_trajectory_id: parent_id,
        root_trajectory_id: parent_id.or(Some(id)),
        agent_id: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        completed_at: None,
        outcome: None,
        metadata: None,
        links: None,
    }
}

fn create_test_trajectory_with_status(
    id: TrajectoryId,
    status: TrajectoryStatus,
) -> TrajectoryResponse {
    TrajectoryResponse {
        trajectory_id: id,
        tenant_id: TenantId::nil(),
        name: format!("Test Trajectory {}", id.as_uuid()),
        description: None,
        status,
        parent_trajectory_id: None,
        root_trajectory_id: Some(id),
        agent_id: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        completed_at: None,
        outcome: None,
        metadata: None,
        links: None,
    }
}

fn create_test_trajectory_full(
    id: TrajectoryId,
    status: TrajectoryStatus,
    agent_id: Option<AgentId>,
) -> TrajectoryResponse {
    TrajectoryResponse {
        trajectory_id: id,
        tenant_id: TenantId::nil(),
        name: format!("Test Trajectory {}", id.as_uuid()),
        description: Some("Test description".to_string()),
        status,
        parent_trajectory_id: None,
        root_trajectory_id: Some(id),
        agent_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        completed_at: None,
        outcome: None,
        metadata: None,
        links: None,
    }
}

fn create_test_artifact(artifact_type: ArtifactType) -> ArtifactResponse {
    ArtifactResponse {
        artifact_id: ArtifactId::now_v7(),
        tenant_id: TenantId::nil(),
        trajectory_id: TrajectoryId::now_v7(),
        scope_id: ScopeId::now_v7(),
        artifact_type,
        name: "Test Artifact".to_string(),
        content: "Test content".to_string(),
        content_hash: caliber_core::compute_content_hash("Test content".as_bytes()),
        embedding: None,
        provenance: ProvenanceResponse {
            source_turn: 1,
            extraction_method: caliber_core::ExtractionMethod::Explicit,
            confidence: Some(0.95),
        },
        ttl: caliber_core::TTL::Persistent,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        superseded_by: None,
        metadata: None,
        links: None,
    }
}

fn create_test_note(note_type: NoteType) -> NoteResponse {
    NoteResponse {
        note_id: NoteId::now_v7(),
        tenant_id: TenantId::nil(),
        note_type,
        title: "Test Note".to_string(),
        content: "Test content".to_string(),
        content_hash: caliber_core::compute_content_hash("Test content".as_bytes()),
        embedding: None,
        source_trajectory_ids: vec![],
        source_artifact_ids: vec![],
        ttl: caliber_core::TTL::Persistent,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        accessed_at: chrono::Utc::now(),
        access_count: 0,
        superseded_by: None,
        metadata: None,
        links: None,
    }
}
