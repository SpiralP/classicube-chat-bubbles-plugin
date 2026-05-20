pub mod chat_screen;
pub mod options;
pub mod wordwrap;

use std::{
    cell::{Cell, RefCell},
    ptr::NonNull,
    rc::Rc,
};

use classicube_helpers::{entities::ENTITY_SELF_ID, events::input};
use classicube_sys::{
    Drawer2D, Gui_GetInputGrab, Gui_GetScreen, GuiPriority_GUI_PRIORITY_CHAT, InputBind__BIND_CHAT,
    InputBind__BIND_SEND_CHAT, InputButtons, InputButtons_CCKEY_ESCAPE,
    InputButtons_CCKEY_KP_ENTER, InputButtons_CCKEY_SLASH, PackedCol_A, Screen,
};
use tracing::{debug, warn};

use self::{chat_screen::ChatScreen, options::get_input_button};
use crate::plugin::events::{chat_message::is_in_whisper_mode, player_chat_event::PlayerChatEvent};

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
        let raw = chat_screen.input.base.text.to_string();
        let convert_percents = chat_screen.input.base.convertPercents != 0;
        let text = format_input_line(&raw, convert_percents, is_valid_color_code);
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

            if !is_sensitive_text(&text) && !is_in_whisper_mode() {
                PlayerChatEvent::InputTextChanged(text).emit(ENTITY_SELF_ID);
            }
        }
    }
}

/// Mirrors `Drawer2D_ValidColorCodeAt`: `Drawer2D.Colors` is sized
/// `DRAWER2D_MAX_COLORS` (256), indexed by raw byte; valid iff alpha != 0.
pub(super) fn is_valid_color_code(c: u8) -> bool {
    let color = unsafe { Drawer2D.Colors[c as usize] };
    PackedCol_A(color) != 0
}

/// Mirrors `InputWidget_FormatLine`: when `convert_percents` is set,
/// substitute `%X` → `&X` for any X that's a valid color code. ClassiCube
/// stores user-typed `%X` verbatim in `InputWidget::text` and only rewrites
/// them at render time, so the bubble snapshot must do the same conversion.
fn format_input_line(
    text: &str,
    convert_percents: bool,
    is_valid_code: impl Fn(u8) -> bool,
) -> String {
    if !convert_percents {
        return text.to_string();
    }
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'%' && i + 1 < bytes.len() && is_valid_code(bytes[i + 1]) {
            out.push(b'&');
        } else {
            out.push(c);
        }
        i += 1;
    }
    // Only ASCII `%` (0x25) was swapped for ASCII `&` (0x26); neither byte
    // can appear mid-UTF-8 codepoint, so the result is still valid UTF-8.
    String::from_utf8(out).expect("ascii-only byte swap preserves utf-8")
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

#[cfg(test)]
mod tests {
    use super::{format_input_line, is_sensitive_text};

    /// Default ClassiCube palette covers '0'..='9', 'a'..='f', 'A'..='F'.
    fn default_palette(c: u8) -> bool {
        c.is_ascii_hexdigit()
    }

    #[test]
    fn percent_to_amp_for_valid_code() {
        assert_eq!(
            format_input_line("%chello world", true, default_palette),
            "&chello world"
        );
    }

    #[test]
    fn percent_left_alone_for_invalid_code() {
        // 'z' is not in the default palette.
        assert_eq!(format_input_line("%zfoo", true, default_palette), "%zfoo");
    }

    #[test]
    fn trailing_percent_at_end_of_string() {
        // No byte follows '%', so it must be preserved.
        assert_eq!(format_input_line("done%", true, default_palette), "done%");
    }

    #[test]
    fn multiple_codes_in_one_string() {
        assert_eq!(
            format_input_line("%ared %bgreen %cblue", true, default_palette),
            "&ared &bgreen &cblue"
        );
    }

    #[test]
    fn convert_percents_off_is_identity() {
        // Classic mode: widget sets convertPercents = false, raw stays raw.
        assert_eq!(
            format_input_line("%chello", false, default_palette),
            "%chello"
        );
    }

    #[test]
    fn empty_string_is_empty() {
        assert_eq!(format_input_line("", true, default_palette), "");
        assert_eq!(format_input_line("", false, default_palette), "");
    }

    #[test]
    fn ampersand_passthrough() {
        // Already-formatted text from the post-send path must not be rewritten.
        assert_eq!(
            format_input_line("&chello", true, default_palette),
            "&chello"
        );
    }

    #[test]
    fn non_ascii_passes_through() {
        // UTF-8 multibyte chars should round-trip; '%' cannot appear inside
        // a codepoint (it's ASCII 0x25, never a continuation byte).
        assert_eq!(
            format_input_line("héllo %cwörld", true, default_palette),
            "héllo &cwörld"
        );
    }

    #[test]
    fn adjacent_percent_signs() {
        // First '%' is followed by '%' (not a valid code), so it stays;
        // second '%' is followed by 'c' (valid), so it converts.
        assert_eq!(
            format_input_line("%%chello", true, default_palette),
            "%&chello"
        );
    }

    #[test]
    fn empty_palette_never_converts() {
        // Mirrors ClassiCube startup before palette init (or all colors zero).
        assert_eq!(format_input_line("%chello", true, |_| false), "%chello");
    }

    #[test]
    fn is_sensitive_text_filters_whispers_and_commands() {
        assert!(is_sensitive_text("@SpiralP hi"));
        assert!(is_sensitive_text("/help"));
        assert!(is_sensitive_text("##secret"));
        assert!(is_sensitive_text("++admin"));
        assert!(!is_sensitive_text("hello"));
        assert!(!is_sensitive_text(""));
        assert!(!is_sensitive_text("#single"));
        assert!(!is_sensitive_text("+single"));
    }
}
