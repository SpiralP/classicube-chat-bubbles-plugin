pub mod chat_input;
pub mod chat_message;
pub mod player_chat_event;

pub fn initialize() {
    chat_input::initialize();
    chat_message::initialize();
}

pub fn free() {
    player_chat_event::free();
    chat_message::free();
    chat_input::free();
}
