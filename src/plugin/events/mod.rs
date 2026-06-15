pub mod chat_message;
pub mod local_presence;
pub mod player_chat_event;

pub fn initialize() {
    chat_message::initialize();
}

pub fn free() {
    player_chat_event::free();
    chat_message::free();
    local_presence::free();
}
