//! Notification system for the TUI.

use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
    Success,
}

#[derive(Debug, Clone)]
pub enum NotificationAction {
    Retry,
    Reconnect,
    Dismiss,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub level: NotificationLevel,
    pub message: String,
    pub action: Option<NotificationAction>,
    pub created_at: DateTime<Utc>,
}

impl Notification {
    pub fn new(level: NotificationLevel, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
            action: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_action(mut self, action: NotificationAction) -> Self {
        self.action = Some(action);
        self
    }
}
