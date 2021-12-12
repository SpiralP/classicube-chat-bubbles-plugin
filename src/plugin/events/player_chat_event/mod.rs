pub mod listener;
pub mod local_handler;

use self::listener::with_all_listeners;
use classicube_helpers::entities::ENTITY_SELF_ID;
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerChatEvent {
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
        });

        if entity_id == ENTITY_SELF_ID {
            local_handler::handle_local_emit(self);
        }
    }
}

pub fn free() {
    local_handler::free();
}
