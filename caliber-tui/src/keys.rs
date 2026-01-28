//! Keybinding definitions for the TUI.

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

pub fn map_key(event: KeyEvent) -> Option<KeyAction> {
    let KeyEvent {
        code, modifiers, ..
    } = event;

    if modifiers.contains(KeyModifiers::CONTROL) {
        return match code {
            KeyCode::Char('c') => Some(KeyAction::Cancel),
            KeyCode::Char('r') => Some(KeyAction::Refresh),
            _ => None,
        };
    }

    match code {
        KeyCode::Char('q') => Some(KeyAction::Quit),
        KeyCode::Char('?') => Some(KeyAction::OpenHelp),
        KeyCode::Char('/') => Some(KeyAction::OpenSearch),
        KeyCode::Char(':') => Some(KeyAction::OpenCommand),
        KeyCode::Char('p') => Some(KeyAction::PauseUpdates),
        KeyCode::Char('n') => Some(KeyAction::NewItem),
        KeyCode::Char('e') => Some(KeyAction::EditItem),
        KeyCode::Char('d') => Some(KeyAction::DeleteItem),
        KeyCode::Char('a') => Some(KeyAction::ToggleLinks),
        KeyCode::Char('[') => Some(KeyAction::PrevLink),
        KeyCode::Char(']') => Some(KeyAction::NextLink),
        KeyCode::Char('g') => Some(KeyAction::ExecuteLink),
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
