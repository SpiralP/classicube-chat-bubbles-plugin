mod chat_input;
mod input_event_listener;
mod options;

pub use self::input_event_listener::{InputEvent, InputEventListener, StartStopListening};
use self::{chat_input::ChatScreen, options::get_input_button};
use crate::plugin::events::input_event_listener::emit_input_event;
use classicube_helpers::{events::input, WithBorrow};
use classicube_sys::{
    Gui_GetInputGrab, InputButtons_KEY_ESCAPE, InputButtons_KEY_KP_ENTER, InputButtons_KEY_SLASH,
    KeyBind__KEYBIND_CHAT, KeyBind__KEYBIND_SEND_CHAT,
};
use std::{
    cell::{Cell, RefCell},
    ptr::NonNull,
    rc::Rc,
};
use tracing::debug;

thread_local!(
    static INPUT_DOWN_HANDLER: RefCell<Option<input::DownEventHandler>> = Default::default();
);

thread_local!(
    static INPUT_UP_HANDLER: RefCell<Option<input::UpEventHandler>> = Default::default();
);

pub fn initialize() {
    let keybind_open_chat = get_input_button(KeyBind__KEYBIND_CHAT).unwrap();
    let keybind_send_chat = get_input_button(KeyBind__KEYBIND_SEND_CHAT).unwrap();

    let open = Rc::new(Cell::new(false));
    let mut input_down_handler = input::DownEventHandler::new();

    {
        let open = open.clone();
        input_down_handler.on(move |&input::DownEvent { key, repeating }| {
            if repeating {
                return;
            }

            if open.get() {
                if key == keybind_send_chat
                    || key == InputButtons_KEY_KP_ENTER
                    || key == InputButtons_KEY_ESCAPE
                {
                    open.set(false);
                    // let close = key == InputButtons_KEY_ESCAPE;
                    debug!("chat close");
                    emit_input_event(InputEvent::ChatClosed);
                }
                return;
            }
            if key == keybind_open_chat || key == InputButtons_KEY_SLASH {
                open.set(true);
                debug!("chat open");
                emit_input_event(InputEvent::ChatOpened);
            }
        });
    }

    let mut input_up_handler = input::UpEventHandler::new();
    input_up_handler.on(move |&input::UpEvent { .. }| {
        if open.get() {
            if let Some(screen) = NonNull::new(unsafe { Gui_GetInputGrab() }) {
                debug!(?screen);
                unsafe {
                    let chat_screen = screen.cast::<ChatScreen>();
                    let chat_screen = chat_screen.as_ref();
                    debug!("{:?}", chat_screen.input.base.text.to_string());
                    emit_input_event(InputEvent::InputTextChanged(
                        chat_screen.input.base.text.to_string(),
                    ));
                }
            }
        }
    });

    INPUT_DOWN_HANDLER.with_borrow_mut(move |option| {
        *option = Some(input_down_handler);
    });

    INPUT_UP_HANDLER.with_borrow_mut(move |option| {
        *option = Some(input_up_handler);
    });
}

pub fn free() {
    INPUT_DOWN_HANDLER.with_borrow_mut(move |option| {
        drop(option.take());
    });
}
