//! SynthBrute theme and color utilities.

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
    pub fn synthbrute() -> Self {
        Self {
            bg: Color::Rgb(10, 10, 10),
            bg_secondary: Color::Rgb(26, 26, 26),
            bg_highlight: Color::Rgb(42, 42, 42),
            primary: Color::Rgb(0, 255, 255),
            primary_dim: Color::Rgb(0, 136, 136),
            secondary: Color::Rgb(255, 0, 255),
            secondary_dim: Color::Rgb(136, 0, 136),
            tertiary: Color::Rgb(255, 255, 0),
            tertiary_dim: Color::Rgb(136, 136, 0),
            success: Color::Rgb(0, 255, 0),
            warning: Color::Rgb(255, 255, 0),
            error: Color::Rgb(255, 0, 0),
            info: Color::Rgb(0, 255, 255),
            text: Color::Rgb(255, 255, 255),
            text_dim: Color::Rgb(136, 136, 136),
            text_muted: Color::Rgb(68, 68, 68),
            border: Color::Rgb(68, 68, 68),
            border_focus: Color::Rgb(0, 255, 255),
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
