pub mod chat_screen;
pub mod options;

use self::{chat_screen::ChatScreen, options::get_input_button};
use crate::plugin::events::input_event::{handle_local::handle_local_event, InputEvent};
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
    static INPUT_PRESS_HANDLER: RefCell<Option<input::PressEventHandler>> = Default::default();
);
thread_local!(
    static INPUT_UP_HANDLER: RefCell<Option<input::UpEventHandler>> = Default::default();
);

pub fn initialize() {
    let keybind_open_chat = get_input_button(KeyBind__KEYBIND_CHAT).unwrap();
    let keybind_send_chat = get_input_button(KeyBind__KEYBIND_SEND_CHAT).unwrap();

    let open = Rc::new(Cell::new(false));
    let chat_screen = Rc::new(Cell::new(None));

    {
        let open = open.clone();
        let chat_screen = chat_screen.clone();
        INPUT_DOWN_HANDLER.with_borrow_mut(move |option| {
            let mut input_down_handler = input::DownEventHandler::new();
            input_down_handler.on(move |&input::DownEvent { key, repeating }| {
                if open.get() {
                    if !repeating
                        && (key == keybind_send_chat
                            || key == InputButtons_KEY_KP_ENTER
                            || key == InputButtons_KEY_ESCAPE)
                    {
                        open.set(false);
                        // let close = key == InputButtons_KEY_ESCAPE;
                        debug!("chat close");
                        handle_local_event(InputEvent::ChatClosed);
                    }
                } else {
                    check_input_changed(open.get(), chat_screen.get());

                    if !repeating && (key == keybind_open_chat || key == InputButtons_KEY_SLASH) {
                        open.set(true);
                        debug!("chat open");
                        handle_local_event(InputEvent::ChatOpened);

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
            handle_local_event(InputEvent::InputTextChanged(text));
        }
    }
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
