use caliber_tui::config::{AuthConfig, ReconnectConfig, ThemeConfig, TuiConfig};
use caliber_tui::keys::{map_key, Action};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use proptest::prelude::*;
use uuid::Uuid;

fn base_config() -> TuiConfig {
    TuiConfig {
        api_base_url: "http://localhost:8080".to_string(),
        grpc_endpoint: "http://localhost:50051".to_string(),
        ws_endpoint: "ws://localhost:8080/ws".to_string(),
        tenant_id: Uuid::new_v4(),
        auth: AuthConfig {
            api_key: Some("test-key".to_string()),
            jwt: None,
        },
        request_timeout_ms: 5_000,
        refresh_interval_ms: 2_000,
        persistence_path: "tmp/caliber-tui.json".into(),
        error_log_path: "tmp/caliber-tui-errors.log".into(),
        theme: ThemeConfig {
            name: "synthbrute".to_string(),
        },
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
    config.auth = AuthConfig {
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
    };
    assert!(config.validate().is_err());
}

proptest! {
    #[test]
    fn keybinding_digit_switches_view(digit in 0u8..=9u8) {
        let ch = char::from(b'0' + digit);
        let event = KeyEvent {
            code: KeyCode::Char(ch),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        let action = map_key(event);
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
            prop_assert!(matches!(action, Some(Action::SwitchView(i)) if i == index));
        } else {
            prop_assert!(action.is_none());
        }
    }

    #[test]
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

    #[test]
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
}
