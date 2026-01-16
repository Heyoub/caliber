//! Event types for the TUI event loop.

use caliber_api::events::WsEvent;
use crossterm::event::KeyEvent;

#[derive(Debug, Clone)]
pub enum TuiEvent {
    Input(KeyEvent),
    Tick,
    Resize { width: u16, height: u16 },
    Ws(Box<WsEvent>),
    ApiError(String),
}
