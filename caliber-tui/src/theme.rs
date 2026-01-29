//! SynthBrute theme and color utilities.

use crate::config::ThemeColorOverrides;
use caliber_core::{TrajectoryStatus, TurnRole};
use ratatui::style::Color;

#[derive(Debug, Clone)]
pub struct SynthBruteTheme {
    pub bg: Color,
    pub bg_secondary: Color,
    pub bg_highlight: Color,
    pub primary: Color,
    pub primary_dim: Color,
    pub secondary: Color,
    pub secondary_dim: Color,
    pub tertiary: Color,
    pub tertiary_dim: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub text: Color,
    pub text_dim: Color,
    pub text_muted: Color,
    pub border: Color,
    pub border_focus: Color,
}

impl SynthBruteTheme {
    /// Create a SynthBrute theme with optional color overrides.
    pub fn synthbrute(overrides: Option<&ThemeColorOverrides>) -> Self {
        let default_primary = (0, 255, 255);
        let default_secondary = (255, 0, 255);
        let default_tertiary = (255, 255, 0);
        let default_success = (0, 255, 0);
        let default_warning = (255, 255, 0);
        let default_error = (255, 0, 0);
        let default_info = (0, 255, 255);
        let default_bg = (10, 10, 10);
        let default_text = (255, 255, 255);
        let default_border_focus = (0, 255, 255);

        let primary = overrides
            .and_then(|o| o.primary)
            .unwrap_or(default_primary);
        let secondary = overrides
            .and_then(|o| o.secondary)
            .unwrap_or(default_secondary);
        let tertiary = overrides
            .and_then(|o| o.tertiary)
            .unwrap_or(default_tertiary);
        let success = overrides
            .and_then(|o| o.success)
            .unwrap_or(default_success);
        let warning = overrides
            .and_then(|o| o.warning)
            .unwrap_or(default_warning);
        let error = overrides.and_then(|o| o.error).unwrap_or(default_error);
        let info = overrides.and_then(|o| o.info).unwrap_or(default_info);
        let bg = overrides.and_then(|o| o.bg).unwrap_or(default_bg);
        let text = overrides.and_then(|o| o.text).unwrap_or(default_text);
        let border_focus = overrides
            .and_then(|o| o.border_focus)
            .unwrap_or(default_border_focus);

        Self {
            bg: Color::Rgb(bg.0, bg.1, bg.2),
            bg_secondary: Color::Rgb(26, 26, 26),
            bg_highlight: Color::Rgb(42, 42, 42),
            primary: Color::Rgb(primary.0, primary.1, primary.2),
            primary_dim: Color::Rgb(
                (primary.0 as f32 * 0.53) as u8,
                (primary.1 as f32 * 0.53) as u8,
                (primary.2 as f32 * 0.53) as u8,
            ),
            secondary: Color::Rgb(secondary.0, secondary.1, secondary.2),
            secondary_dim: Color::Rgb(
                (secondary.0 as f32 * 0.53) as u8,
                (secondary.1 as f32 * 0.53) as u8,
                (secondary.2 as f32 * 0.53) as u8,
            ),
            tertiary: Color::Rgb(tertiary.0, tertiary.1, tertiary.2),
            tertiary_dim: Color::Rgb(
                (tertiary.0 as f32 * 0.53) as u8,
                (tertiary.1 as f32 * 0.53) as u8,
                (tertiary.2 as f32 * 0.53) as u8,
            ),
            success: Color::Rgb(success.0, success.1, success.2),
            warning: Color::Rgb(warning.0, warning.1, warning.2),
            error: Color::Rgb(error.0, error.1, error.2),
            info: Color::Rgb(info.0, info.1, info.2),
            text: Color::Rgb(text.0, text.1, text.2),
            text_dim: Color::Rgb(136, 136, 136),
            text_muted: Color::Rgb(68, 68, 68),
            border: Color::Rgb(68, 68, 68),
            border_focus: Color::Rgb(border_focus.0, border_focus.1, border_focus.2),
        }
    }
}

pub fn trajectory_status_color(status: TrajectoryStatus, theme: &SynthBruteTheme) -> Color {
    match status {
        TrajectoryStatus::Active => theme.primary,
        TrajectoryStatus::Completed => theme.success,
        TrajectoryStatus::Failed => theme.error,
        TrajectoryStatus::Suspended => theme.warning,
    }
}

pub fn agent_status_color(status: &str, theme: &SynthBruteTheme) -> Color {
    match status.trim().to_ascii_lowercase().as_str() {
        "idle" => theme.primary_dim,
        "active" => theme.primary,
        "blocked" => theme.warning,
        "failed" => theme.error,
        _ => theme.text_dim,
    }
}

pub fn message_priority_color(priority: &str, theme: &SynthBruteTheme) -> Color {
    match priority.trim().to_ascii_lowercase().as_str() {
        "low" => theme.text_dim,
        "normal" => theme.text,
        "high" => theme.warning,
        "critical" => theme.error,
        _ => theme.text_dim,
    }
}

pub fn turn_role_color(role: TurnRole, theme: &SynthBruteTheme) -> Color {
    match role {
        TurnRole::User => theme.primary,
        TurnRole::Assistant => theme.secondary,
        TurnRole::System => theme.tertiary,
        TurnRole::Tool => theme.success,
    }
}

pub fn utilization_color(percent: f32, theme: &SynthBruteTheme) -> Color {
    if percent < 70.0 {
        theme.success
    } else if percent < 90.0 {
        theme.warning
    } else {
        theme.error
    }
}
