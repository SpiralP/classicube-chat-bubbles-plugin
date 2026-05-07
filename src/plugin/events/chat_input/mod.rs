pub mod chat_screen;
pub mod options;

use std::{
    cell::{Cell, RefCell},
    ptr::NonNull,
    rc::Rc,
};

use classicube_helpers::{entities::ENTITY_SELF_ID, events::input};
use classicube_sys::{
    Gui_GetInputGrab, Gui_GetScreen, GuiPriority_GUI_PRIORITY_CHAT, InputBind__BIND_CHAT,
    InputBind__BIND_SEND_CHAT, InputButtons, InputButtons_CCKEY_ESCAPE,
    InputButtons_CCKEY_KP_ENTER, InputButtons_CCKEY_SLASH, Screen,
};
use tracing::{debug, warn};

use self::{chat_screen::ChatScreen, options::get_input_button};
use crate::plugin::events::player_chat_event::PlayerChatEvent;

thread_local!(
    static INPUT_DOWN_HANDLER: RefCell<Option<input::Down2EventHandler>> = Default::default();
);
thread_local!(
    static INPUT_PRESS_HANDLER: RefCell<Option<input::PressEventHandler>> = Default::default();
);
thread_local!(
    static INPUT_UP_HANDLER: RefCell<Option<input::Up2EventHandler>> = Default::default();
);

pub fn initialize() {
    let keybind_open_chat = get_input_button(InputBind__BIND_CHAT as _);
    let is_keybind_open_chat =
        move |key: InputButtons| keybind_open_chat.map(|k| key == k).unwrap_or(false);
    let keybind_send_chat = get_input_button(InputBind__BIND_SEND_CHAT as _);
    let is_keybind_send_chat =
        move |key: InputButtons| keybind_send_chat.map(|k| key == k).unwrap_or(false);

    let open = Rc::new(Cell::new(false));
    let chat_screen = Rc::new(Cell::new(None));

    {
        let open = open.clone();
        let chat_screen = chat_screen.clone();
        INPUT_DOWN_HANDLER.with_borrow_mut(move |option| {
            let mut input_down_handler = input::Down2EventHandler::new();
            input_down_handler.on(move |&input::Down2Event { key, repeating, .. }| {
                if open.get() {
                    // chat closed
                    if !repeating
                        && (is_keybind_send_chat(key)
                            || key == InputButtons_CCKEY_KP_ENTER
                            || key == InputButtons_CCKEY_ESCAPE)
                    {
                        open.set(false);
                        // let close = key == InputButtons_CCKEY_ESCAPE;
                        debug!("chat close");
                        PlayerChatEvent::ChatClosed.emit(ENTITY_SELF_ID);
                    } else {
                        check_input_changed(open.get(), chat_screen.get());
                    }
                } else {
                    // chat opened
                    if !repeating && (is_keybind_open_chat(key) || key == InputButtons_CCKEY_SLASH)
                    {
                        open.set(true);
                        debug!("chat open");

                        unsafe fn sanity_check(screen: NonNull<Screen>) -> bool {
                            unsafe {
                                // The screen at GUI_PRIORITY_CHAT is the
                                // ChatScreen singleton (registered by
                                // ChatScreen_Show at startup). Pointer-equal
                                // to the input-grab screen means it's chat
                                // and not e.g. PauseScreen / InventoryScreen
                                // / DisconnectScreen.
                                if Gui_GetScreen(GuiPriority_GUI_PRIORITY_CHAT as _)
                                    != screen.as_ptr()
                                {
                                    return false;
                                }
                                // Belt-and-braces field checks, since other
                                // screens could in theory register at
                                // PRIORITY_CHAT. ChatScreen never sets these.
                                let s = screen.as_ref();
                                s.grabsInput == 1
                                    && s.blocksWorld == 0
                                    && s.closable == 0
                                    && s.widgets.is_null()
                                    && s.numWidgets == 0
                            }
                        }
                        unsafe {
                            if let Some(screen) = NonNull::new(Gui_GetInputGrab()) {
                                debug!(?screen);

                                if sanity_check(screen) {
                                    chat_screen.set(Some(screen.cast::<ChatScreen>().as_ref()));
                                } else {
                                    warn!("Screen sanity_check failed");
                                }
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
        let mut input_up_handler = input::Up2EventHandler::new();
        input_up_handler.on(move |&input::Up2Event { .. }| {
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
    LAST_INPUT.with_borrow_mut(|option| {
        option.take();
    });
}
