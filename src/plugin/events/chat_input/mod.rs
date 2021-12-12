pub mod chat_screen;
pub mod options;

use self::{chat_screen::ChatScreen, options::get_input_button};
use crate::plugin::events::player_chat_event::PlayerChatEvent;
use classicube_helpers::{entities::ENTITY_SELF_ID, events::input, WithBorrow};
use classicube_sys::{
    Gui_GetInputGrab, InputButtons, InputButtons_KEY_ESCAPE, InputButtons_KEY_KP_ENTER,
    InputButtons_KEY_SLASH, KeyBind__KEYBIND_CHAT, KeyBind__KEYBIND_SEND_CHAT,
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
    static INPUT_PRESS_HANDLER: RefCell<Option<input::PressEventHandler>> = Default::default();
);
thread_local!(
    static INPUT_UP_HANDLER: RefCell<Option<input::UpEventHandler>> = Default::default();
);

pub fn initialize() {
    let keybind_open_chat = get_input_button(KeyBind__KEYBIND_CHAT);
    let is_keybind_open_chat =
        move |key: InputButtons| keybind_open_chat.map(|k| key == k).unwrap_or(false);
    let keybind_send_chat = get_input_button(KeyBind__KEYBIND_SEND_CHAT);
    let is_keybind_send_chat =
        move |key: InputButtons| keybind_send_chat.map(|k| key == k).unwrap_or(false);

    let open = Rc::new(Cell::new(false));
    let chat_screen = Rc::new(Cell::new(None));

    {
        let open = open.clone();
        let chat_screen = chat_screen.clone();
        INPUT_DOWN_HANDLER.with_borrow_mut(move |option| {
            let mut input_down_handler = input::DownEventHandler::new();
            input_down_handler.on(move |&input::DownEvent { key, repeating }| {
                if open.get() {
                    // chat closed
                    if !repeating
                        && (is_keybind_send_chat(key)
                            || key == InputButtons_KEY_KP_ENTER
                            || key == InputButtons_KEY_ESCAPE)
                    {
                        open.set(false);
                        // let close = key == InputButtons_KEY_ESCAPE;
                        debug!("chat close");
                        PlayerChatEvent::ChatClosed.emit(ENTITY_SELF_ID);
                    } else {
                        check_input_changed(open.get(), chat_screen.get());
                    }
                } else {
                    // chat opened
                    if !repeating && (is_keybind_open_chat(key) || key == InputButtons_KEY_SLASH) {
                        open.set(true);
                        debug!("chat open");

                        unsafe {
                            if let Some(screen) = NonNull::new(Gui_GetInputGrab()) {
                                debug!(?screen);
                                chat_screen.set(Some(screen.cast::<ChatScreen>().as_ref()));
                            }
                        }
                    }
                }
            });
            *option = Some(input_down_handler);
        });
    }

    {
        let open = open.clone();
        let chat_screen = chat_screen.clone();
        INPUT_PRESS_HANDLER.with_borrow_mut(move |option| {
            let mut input_press_handler = input::PressEventHandler::new();
            input_press_handler.on(move |&input::PressEvent { .. }| {
                check_input_changed(open.get(), chat_screen.get());
            });
            *option = Some(input_press_handler);
        });
    }

    INPUT_UP_HANDLER.with_borrow_mut(move |option| {
        let mut input_up_handler = input::UpEventHandler::new();
        input_up_handler.on(move |&input::UpEvent { .. }| {
            check_input_changed(open.get(), chat_screen.get());
        });
        *option = Some(input_up_handler);
    });
}

thread_local!(
    static LAST_INPUT: RefCell<Option<String>> = Default::default();
);

fn check_input_changed(open: bool, chat_screen: Option<&'static ChatScreen>) {
    if !open {
        return;
    }
    if let Some(chat_screen) = chat_screen {
        let text = chat_screen.input.base.text.to_string();
        let changed = LAST_INPUT.with_borrow_mut(|option| {
            if option.as_ref().map(|last| last != &text).unwrap_or(true) {
                // changed
                *option = Some(text.clone());
                true
            } else {
                false
            }
        });

        if changed {
            debug!(?text, "changed");

            if !is_sensitive_text(&text) {
                PlayerChatEvent::InputTextChanged(text).emit(ENTITY_SELF_ID);
            }
        }
    }
}

fn is_sensitive_text(text: &str) -> bool {
    let c = text.get(0..1).unwrap_or("");
    // don't show whispers or commands
    if c == "@" || c == "/" {
        return true;
    }

    let c = text.get(0..2).unwrap_or("");
    // don't show "To Ops" or "To Admins"
    c == "##" || c == "++"
}

pub fn free() {
    INPUT_UP_HANDLER.with_borrow_mut(move |option| {
        drop(option.take());
    });
    INPUT_PRESS_HANDLER.with_borrow_mut(move |option| {
        drop(option.take());
    });
    INPUT_DOWN_HANDLER.with_borrow_mut(move |option| {
        drop(option.take());
    });
}
