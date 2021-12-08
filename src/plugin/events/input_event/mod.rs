pub mod handle_local;
pub mod input_event_listener;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputEvent {
    ChatOpened,
    ChatClosed,
    InputTextChanged(String),
}

pub fn free() {
    handle_local::free();
}
