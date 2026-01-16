//! Keybinding definitions for the TUI.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
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
}

pub fn map_key(event: KeyEvent) -> Option<Action> {
    let KeyEvent { code, modifiers, .. } = event;

    if modifiers.contains(KeyModifiers::CONTROL) {
        return match code {
            KeyCode::Char('c') => Some(Action::Cancel),
            KeyCode::Char('r') => Some(Action::Refresh),
            _ => None,
        };
    }

    match code {
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Char('?') => Some(Action::OpenHelp),
        KeyCode::Char('/') => Some(Action::OpenSearch),
        KeyCode::Char(':') => Some(Action::OpenCommand),
        KeyCode::Char('p') => Some(Action::PauseUpdates),
        KeyCode::Char('n') => Some(Action::NewItem),
        KeyCode::Char('e') => Some(Action::EditItem),
        KeyCode::Char('d') => Some(Action::DeleteItem),
        KeyCode::Enter => Some(Action::Confirm),
        KeyCode::Esc => Some(Action::Cancel),
        KeyCode::Tab => Some(Action::NextView),
        KeyCode::BackTab => Some(Action::PrevView),
        KeyCode::Up | KeyCode::Char('k') => Some(Action::MoveUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::MoveDown),
        KeyCode::Left | KeyCode::Char('h') => Some(Action::MoveLeft),
        KeyCode::Right | KeyCode::Char('l') => Some(Action::MoveRight),
        KeyCode::Char(' ') => Some(Action::Select),
        KeyCode::Char('x') => Some(Action::ToggleExpand),
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
            Some(Action::SwitchView(idx))
        }
        _ => None,
    }
}
