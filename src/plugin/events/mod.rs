mod chat_input;
mod input_event_listener;
mod options;

pub use self::input_event_listener::{
    emit_input_event, InputEvent, InputEventListener, StartStopListening,
};
use self::{chat_input::ChatScreen, options::get_input_button};
use classicube_helpers::{async_manager, entities::ENTITY_SELF_ID, events::input, WithBorrow};
use classicube_sys::{
    Gui_GetInputGrab, InputButtons_KEY_ESCAPE, InputButtons_KEY_KP_ENTER, InputButtons_KEY_SLASH,
    KeyBind__KEYBIND_CHAT, KeyBind__KEYBIND_SEND_CHAT,
};
use futures::future::AbortHandle;
use std::{
    cell::{Cell, RefCell},
    ptr::NonNull,
    rc::Rc,
    time::Duration,
};
use tracing::debug;

thread_local!(
    static INPUT_DOWN_HANDLER: RefCell<Option<input::DownEventHandler>> = Default::default();
);

thread_local!(
    static CHAT_SCREEN: RefCell<Option<&'static ChatScreen>> = Default::default();
);

thread_local!(
    static UPDATE_FUTURE_HANDLE: RefCell<Option<AbortHandle>> = Default::default();
);

pub fn initialize() {
    let keybind_open_chat = get_input_button(KeyBind__KEYBIND_CHAT).unwrap();
    let keybind_send_chat = get_input_button(KeyBind__KEYBIND_SEND_CHAT).unwrap();

    let open = Rc::new(Cell::new(false));
    let mut input_down_handler = input::DownEventHandler::new();

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
                emit_input_event(ENTITY_SELF_ID, InputEvent::ChatClosed);
            }
            return;
        }
        if key == keybind_open_chat || key == InputButtons_KEY_SLASH {
            open.set(true);
            debug!("chat open");
            emit_input_event(ENTITY_SELF_ID, InputEvent::ChatOpened);

            let chat_screen = if let Some(screen) = NonNull::new(unsafe { Gui_GetInputGrab() }) {
                debug!(?screen);
                unsafe {
                    let chat_screen = screen.cast::<ChatScreen>();
                    chat_screen.as_ref()
                }
            } else {
                return;
            };

            let open = open.clone();

            let (f, handle) = futures::future::abortable(async move {
                let mut old = String::new();
                loop {
                    if !open.get() {
                        break;
                    }
                    let text = chat_screen.input.base.text.to_string();

                    if text != old {
                        old = text.clone();

                        debug!("{:?}", text);
                        emit_input_event(ENTITY_SELF_ID, InputEvent::InputTextChanged(text));
                    }

                    async_manager::sleep(Duration::from_millis(500)).await;
                }
            });

            UPDATE_FUTURE_HANDLE.with_borrow_mut(move |option| {
                if let Some(handle) = option.take() {
                    handle.abort();
                }
                *option = Some(handle);

                async_manager::spawn_local_on_main_thread(async move {
                    let _ = f.await;
                });
            });
        }
    });

    INPUT_DOWN_HANDLER.with_borrow_mut(move |option| {
        *option = Some(input_down_handler);
    });
}

pub fn free() {
    INPUT_DOWN_HANDLER.with_borrow_mut(move |option| {
        drop(option.take());
    });
}
