pub mod emit_local;
pub mod listener;

use self::listener::with_all_listeners;
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerChatEvent {
    ChatOpened,
    ChatClosed,
    InputTextChanged(String),
    Message(String),
}

impl PlayerChatEvent {
    pub fn emit(self, entity_id: u8) {
        debug!(?entity_id, ?self, "emit");

        with_all_listeners(|map| {
            if let Some(listeners) = map.get_mut(&entity_id) {
                listeners.retain(|listener| {
                    if let Some(listener) = listener.upgrade() {
                        listener.borrow_mut().handle_event(&self);
                        true
                    } else {
                        false
                    }
                })
            }
        })
    }
}

pub fn free() {
    emit_local::free();
}
