//! Keybinding definitions for the TUI.

use crate::config::KeybindingsConfig;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    Quit,
    NextView,
    PrevView,
    SwitchView(usize),
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    Select,
    ToggleExpand,
    NewItem,
    EditItem,
    DeleteItem,
    OpenHelp,
    OpenSearch,
    OpenCommand,
    Refresh,
    PauseUpdates,
    Confirm,
    Cancel,
    /// Toggle links panel visibility.
    ToggleLinks,
    /// Navigate to next link in the links panel.
    NextLink,
    /// Navigate to previous link in the links panel.
    PrevLink,
    /// Execute the selected link action.
    ExecuteLink,
}

/// Map a key event to an action, with optional keybinding overrides.
pub fn map_key(event: KeyEvent, overrides: Option<&KeybindingsConfig>) -> Option<KeyAction> {
    let KeyEvent {
        code, modifiers, ..
    } = event;

    // Extract configured keys or use defaults
    let key_quit = overrides.and_then(|o| o.quit).unwrap_or('q');
    let key_help = overrides.and_then(|o| o.help).unwrap_or('?');
    let key_search = overrides.and_then(|o| o.search).unwrap_or('/');
    let key_command = overrides.and_then(|o| o.command).unwrap_or(':');
    let key_refresh = overrides.and_then(|o| o.refresh).unwrap_or('r');
    let key_new_item = overrides.and_then(|o| o.new_item).unwrap_or('n');
    let key_edit_item = overrides.and_then(|o| o.edit_item).unwrap_or('e');
    let key_delete_item = overrides.and_then(|o| o.delete_item).unwrap_or('d');
    let key_toggle_links = overrides.and_then(|o| o.toggle_links).unwrap_or('a');
    let key_execute_link = overrides.and_then(|o| o.execute_link).unwrap_or('g');

    if modifiers.contains(KeyModifiers::CONTROL) {
        return match code {
            KeyCode::Char('c') => Some(KeyAction::Cancel),
            KeyCode::Char(c) if c == key_refresh => Some(KeyAction::Refresh),
            _ => None,
        };
    }

    match code {
        KeyCode::Char(c) if c == key_quit => Some(KeyAction::Quit),
        KeyCode::Char(c) if c == key_help => Some(KeyAction::OpenHelp),
        KeyCode::Char(c) if c == key_search => Some(KeyAction::OpenSearch),
        KeyCode::Char(c) if c == key_command => Some(KeyAction::OpenCommand),
        KeyCode::Char('p') => Some(KeyAction::PauseUpdates),
        KeyCode::Char(c) if c == key_new_item => Some(KeyAction::NewItem),
        KeyCode::Char(c) if c == key_edit_item => Some(KeyAction::EditItem),
        KeyCode::Char(c) if c == key_delete_item => Some(KeyAction::DeleteItem),
        KeyCode::Char(c) if c == key_toggle_links => Some(KeyAction::ToggleLinks),
        KeyCode::Char('[') => Some(KeyAction::PrevLink),
        KeyCode::Char(']') => Some(KeyAction::NextLink),
        KeyCode::Char(c) if c == key_execute_link => Some(KeyAction::ExecuteLink),
        KeyCode::Enter => Some(KeyAction::Confirm),
        KeyCode::Esc => Some(KeyAction::Cancel),
        KeyCode::Tab => Some(KeyAction::NextView),
        KeyCode::BackTab => Some(KeyAction::PrevView),
        KeyCode::Up | KeyCode::Char('k') => Some(KeyAction::MoveUp),
        KeyCode::Down | KeyCode::Char('j') => Some(KeyAction::MoveDown),
        KeyCode::Left | KeyCode::Char('h') => Some(KeyAction::MoveLeft),
        KeyCode::Right | KeyCode::Char('l') => Some(KeyAction::MoveRight),
        KeyCode::Char(' ') => Some(KeyAction::Select),
        KeyCode::Char('x') => Some(KeyAction::ToggleExpand),
        KeyCode::Char(c) if c.is_ascii_digit() => {
            let idx = match c {
                '1' => 0,
                '2' => 1,
                '3' => 2,
                '4' => 3,
                '5' => 4,
                '6' => 5,
                '7' => 6,
                '8' => 7,
                '9' => 8,
                '0' => 9,
                _ => return None,
            };
            Some(KeyAction::SwitchView(idx))
        }
        _ => None,
    }
}
