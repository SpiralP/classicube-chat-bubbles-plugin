pub mod chat_input;
pub mod chat_messages;
pub mod input_event;

pub fn initialize() {
    chat_input::initialize();
    chat_messages::initialize();
}

pub fn free() {
    input_event::free();
    chat_messages::free();
    chat_input::free();
}
