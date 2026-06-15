pub mod chat_screen;
pub mod wordwrap;

#[cfg(test)]
mod tests;

use std::{cell::RefCell, ptr::NonNull};

use classicube_helpers::entities::ENTITY_SELF_ID;
use classicube_sys::{
    Drawer2D, Gui_GetInputGrab, Gui_GetScreen, GuiPriority_GUI_PRIORITY_CHAT,
    GuiPriority_GUI_PRIORITY_INVENTORY, GuiPriority_GUI_PRIORITY_MENU,
    GuiPriority_GUI_PRIORITY_TABLIST, PackedCol_A, Screen,
};

use self::chat_screen::ChatScreen;
use crate::plugin::events::{
    chat_message::is_in_whisper_mode,
    player_chat_event::{PlayerChatEvent, Presence},
};

thread_local!(
    static LAST_PRESENCE: RefCell<Option<Presence>> = Default::default();
);

pub fn poll() {
    let presence = compute_presence();
    let changed = LAST_PRESENCE.with_borrow_mut(|last| {
        if last.as_ref() != presence.as_ref() {
            *last = presence.clone();
            true
        } else {
            false
        }
    });
    if changed {
        PlayerChatEvent::PresenceChanged(presence).emit(ENTITY_SELF_ID);
    }
}

fn compute_presence() -> Option<Presence> {
    unsafe {
        if !Gui_GetScreen(GuiPriority_GUI_PRIORITY_MENU as _).is_null() {
            return Some(Presence::EscapeMenu);
        }
        if let Some(text) = read_chat_input() {
            return Some(Presence::Typing(text));
        }
        if !Gui_GetScreen(GuiPriority_GUI_PRIORITY_INVENTORY as _).is_null() {
            return Some(Presence::BlockMenu);
        }
        if !Gui_GetScreen(GuiPriority_GUI_PRIORITY_TABLIST as _).is_null() {
            return Some(Presence::TabList);
        }
        None
    }
}

fn read_chat_input() -> Option<String> {
    unsafe {
        let input_grab = Gui_GetInputGrab();
        let input_grab_nn = NonNull::new(input_grab)?;
        if !sanity_check(input_grab_nn) {
            return None;
        }
        let chat_screen = input_grab_nn.cast::<ChatScreen>().as_ref();
        let raw = chat_screen.input.base.text.to_string();
        let convert_percents = chat_screen.input.base.convertPercents != 0;
        let text = format_input_line(&raw, convert_percents, is_valid_color_code);
        let display_text = display_for_input(&text, is_in_whisper_mode());
        if display_text.is_empty() {
            None
        } else {
            Some(display_text)
        }
    }
}

// The screen at GUI_PRIORITY_CHAT is the ChatScreen singleton. This check
// guards against misidentifying other screens (PauseScreen, InventoryScreen,
// DisconnectScreen) as ChatScreen before casting the pointer.
unsafe fn sanity_check(screen: NonNull<Screen>) -> bool {
    unsafe {
        if Gui_GetScreen(GuiPriority_GUI_PRIORITY_CHAT as _) != screen.as_ptr() {
            return false;
        }
        let s = screen.as_ref();
        s.grabsInput == 1
            && s.blocksWorld == 0
            && s.closable == 0
            && s.widgets.is_null()
            && s.numWidgets == 0
    }
}

pub(super) fn is_valid_color_code(c: u8) -> bool {
    let color = unsafe { Drawer2D.Colors[c as usize] };
    PackedCol_A(color) != 0
}

/// Mirrors `InputWidget_FormatLine`: when `convert_percents` is set,
/// substitute `%X` -> `&X` for any X that's a valid color code. ClassiCube
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

/// Maps the formatted chat-input line to what the typing bubble should show.
/// Empty input hides the bubble; private commands / whispers / ops-messages
/// and whisper-mode collapse to a `...` placeholder so the contents never leak
/// (locally or over the relay -- only the literal `...` is ever emitted, never
/// the private text); everything else shows verbatim.
///
/// The empty check comes first so erasing the input to empty hides the bubble
/// even in whisper-mode or right after typing a command.
fn display_for_input(text: &str, whisper_mode: bool) -> String {
    if text.is_empty() {
        String::new()
    } else if is_sensitive_text(text) || whisper_mode {
        "...".to_string()
    } else {
        text.to_string()
    }
}

fn is_sensitive_text(text: &str) -> bool {
    let c = text.get(0..1).unwrap_or("");
    if matches!(c, "@" | "/") {
        return true;
    }
    // A lone '#' or '+' could be the start of '##' (Ops) or '++' (Admins) chat
    // -- mask from the first character to avoid leaking which channel. But
    // '#word' / '+word' is public (e.g. "#1", "+rep").
    if c == "#" {
        return text.len() == 1 || text.starts_with("##");
    }
    if c == "+" {
        return text.len() == 1 || text.starts_with("++");
    }
    false
}

pub fn free() {
    LAST_PRESENCE.with_borrow_mut(|option| {
        option.take();
    });
}
