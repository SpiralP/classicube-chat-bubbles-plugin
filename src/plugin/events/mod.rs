mod chat_input;
mod handle_local;
mod input_event_listener;

pub use self::input_event_listener::{
    emit_input_event, InputEvent, InputEventListener, StartStopListening,
};

pub fn initialize() {
    chat_input::initialize();
}

pub fn free() {
    handle_local::free();
    chat_input::free();
}
