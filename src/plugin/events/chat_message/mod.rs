use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
};

use classicube_helpers::{
    entities::ENTITY_SELF_ID,
    events::chat::{ChatReceivedEvent, ChatReceivedEventHandler},
    tab_list::TabList,
};
use classicube_sys::{MsgType_MSG_TYPE_NORMAL, Server};
use tracing::{debug, warn};

use super::player_chat_event::PlayerChatEvent;

thread_local!(
    static CHAT_RECEIVED_HANDLER: RefCell<Option<ChatReceivedEventHandler>> = Default::default();
);

thread_local!(
    static TAB_LIST: RefCell<Option<TabList>> = Default::default();
);

// Tracks the most recent non-continuation chat line so `> ...` continuation
// lines can be merged onto the right speaker's bubble. Cleared on `free`.
//
// Stores each server-split line in order — first the original `Message` text
// (what `PlayerChatEvent::Message` emitted), then each `> ...` continuation
// with its prefix stripped. Keeping the splits lets the bubble render the
// same break points the server used instead of re-wrapping the join.
// Mirrors `classicube-cef-plugin/src/chat/mod.rs:25-44` without the
// `FUTURE_HANDLE` cancel (no async task to abort here).
thread_local!(
    static LAST_CHAT: RefCell<Option<(u8, Vec<String>)>> = const { RefCell::new(None) };
);

// Most recent chat-line prefix observed per player id — the `full_nick` slice
// `find_player_from_message` peels off the left of `": "`. Servers sometimes
// add titles or flair only in chat (not on nameplates or the tab list), so
// caching what the server actually prepends lets the typing-preview wrap
// against the same byte budget the server will. First message a player sends
// still falls back to the tab-list nick — we haven't observed them yet.
thread_local!(
    static OBSERVED_CHAT_PREFIX: RefCell<HashMap<u8, String>> = RefCell::new(HashMap::new());
);

// Local mirror of the server's `p.whisper` flag, kept in sync by watching the
// system-feedback lines MCGalaxy emits when `/whisper` toggles auto-whisper
// mode. While set, the typing-preview broadcast in `chat_input` is muted so
// keystrokes destined for a private whisper don't leak to everyone on the map
// as a bubble above the speaker.
thread_local!(
    static WHISPER_MODE: Cell<bool> = const { Cell::new(false) };
);

pub fn is_in_whisper_mode() -> bool {
    WHISPER_MODE.with(Cell::get)
}

pub fn initialize() {
    TAB_LIST.with_borrow_mut(|option| {
        *option = Some(TabList::new());
    });

    CHAT_RECEIVED_HANDLER.with_borrow_mut(move |option| {
        let mut chat_received_handler = ChatReceivedEventHandler::new();
        chat_received_handler.on(
            move |ChatReceivedEvent {
                      message,
                      message_type,
                  }| {
                if message_type != &MsgType_MSG_TYPE_NORMAL {
                    return;
                }

                if let Some(new_state) = detect_whisper_mode_transition(message) {
                    WHISPER_MODE.set(new_state);
                }

                if let Some(continuation) = is_continuation_message(message) {
                    let result = LAST_CHAT.with_borrow_mut(|cell| {
                        let (id, lines) = cell.as_mut()?;
                        lines.push(continuation.to_string());
                        Some((*id, lines.clone()))
                    });
                    let Some((player_id, lines)) = result else {
                        warn!(?continuation, "continuation with no prior message");
                        return;
                    };
                    PlayerChatEvent::MessageContinuation(lines).emit(player_id);
                    return;
                }

                let Some((player_id, said_text, observed_prefix)) = resolve_message(message) else {
                    warn!(?message, "could not resolve player from message");
                    LAST_CHAT.with_borrow_mut(|cell| *cell = None);
                    return;
                };

                if let Some(prefix) = observed_prefix {
                    OBSERVED_CHAT_PREFIX.with_borrow_mut(|map| {
                        map.insert(player_id, prefix);
                    });
                }
                LAST_CHAT.with_borrow_mut(|cell| {
                    *cell = Some((player_id, vec![said_text.clone()]));
                });
                PlayerChatEvent::Message(said_text).emit(player_id);
            },
        );
        *option = Some(chat_received_handler);
    });
}

pub fn free() {
    CHAT_RECEIVED_HANDLER.with_borrow_mut(move |option| {
        drop(option.take());
    });
    TAB_LIST.with_borrow_mut(move |option| {
        drop(option.take());
    });
    LAST_CHAT.with_borrow_mut(|cell| {
        *cell = None;
    });
    OBSERVED_CHAT_PREFIX.with_borrow_mut(|map| map.clear());
    WHISPER_MODE.set(false);
}

/// `> rest of message` → `Some("rest of message")`. Anything else → `None`.
///
/// Server-side wrap re-emits `&<lastColor>` at the start of a continuation
/// only when the next user text doesn't already begin with a color code (see
/// MCGalaxy `LineWrapper.Wordwrap`). Leaving any leading `&X` intact joins
/// cleanly: a re-emitted code is redundant with the prior line's active
/// color (idempotent on render), while a user-typed color is preserved.
fn is_continuation_message(message: &str) -> Option<&str> {
    message.strip_prefix("> ")
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum WhisperKind {
    Incoming,
    Outgoing,
}

/// Peels optional leading `&X` color codes followed by the literal `[>] ` /
/// `[<] ` whisper marker. Returns the slice that follows the marker so the
/// caller can hand it back to the regular parser. Servers wrap the brackets
/// in arbitrary colors (`&9[>] `, `&7[<] `, …), so the color codes are
/// skipped rather than matched on a specific palette.
fn detect_whisper_prefix(message: &str) -> Option<(WhisperKind, &str)> {
    let bytes = message.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() && bytes[i] == b'&' {
        i += 2;
    }
    let kind = match bytes.get(i..i + 4)? {
        b"[>] " => WhisperKind::Incoming,
        b"[<] " => WhisperKind::Outgoing,
        _ => return None,
    };
    Some((kind, &message[i + 4..]))
}

/// Server-emitted toggle messages for auto-whisper mode (MCGalaxy
/// `CmdWhisper.cs`). `Some(true)` enters whisper mode, `Some(false)` exits.
/// Leading `&X` color codes are skipped so the suppression isn't keyed to a
/// specific palette. The "No online players match" lookup-failure line is
/// deliberately not matched — when the whisper target logs off mid-mode, the
/// server keeps `p.whisper = true`, so the local mirror must stay set too.
fn detect_whisper_mode_transition(message: &str) -> Option<bool> {
    let bytes = message.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() && bytes[i] == b'&' {
        i += 2;
    }
    let rest = message.get(i..)?;
    if rest.starts_with("Auto-whisper enabled. All messages will now be sent to ")
        || rest == "All messages sent will now auto-whisper"
    {
        Some(true)
    } else if rest == "Whisper chat turned off" {
        Some(false)
    } else {
        None
    }
}

/// Returns `(player_id, said_text, observed_prefix)` for a non-continuation
/// chat line. `observed_prefix` is the full nick slice (color + title + name)
/// to cache for the typing-preview wrap budget, set only on regular chat —
/// whispers leave it `None` because the `[>] Sender` / `[<] Recipient` prefix
/// doesn't match what the server prepends to that player's regular chat.
fn resolve_message(message: &str) -> Option<(u8, String, Option<String>)> {
    if let Some((kind, remainder)) = detect_whisper_prefix(message) {
        match kind {
            // Drop the recipient nick; we are the speaker.
            WhisperKind::Outgoing => {
                let pos = remainder.find(": ")?;
                Some((ENTITY_SELF_ID, remainder[pos + 2..].to_string(), None))
            }
            // Reuse the regular parser on the post-marker slice for the
            // colon split + tab-list lookup (which color-strips internally).
            WhisperKind::Incoming => {
                let (player_id, _, said_text) = find_player_from_message(remainder)?;
                Some((player_id, said_text.to_string(), None))
            }
        }
    } else {
        let (player_id, full_nick, said_text) = find_player_from_message(message)?;
        Some((
            player_id,
            said_text.to_string(),
            full_nick.map(str::to_string),
        ))
    }
}

/// Tab-list nick (color + title + name) the server prepends to chat lines for
/// `id`. Returns `None` in singleplayer or when the tab list hasn't seen `id`
/// yet. The bubble's typing preview uses this so it can budget the same 64
/// CP437 bytes the server's `LineWrapper.Wordwrap` will operate on.
pub fn get_nick_name(id: u8) -> Option<String> {
    if unsafe { Server.IsSinglePlayer } != 0 {
        return None;
    }
    TAB_LIST.with_borrow(|cell| {
        cell.as_ref()?
            .get(id)
            .and_then(|w| w.upgrade())
            .map(|entry| entry.get_nick_name())
    })
}

/// Cached chat-line prefix the server actually prepended for `id`, captured
/// from the most recent chat message that player sent. Preferred over the
/// tab-list nick for sizing the typing-preview wrap since some servers add
/// chat-only titles/flair the tab list doesn't carry. Returns `None` until the
/// player has chatted at least once this session.
pub fn get_chat_prefix(id: u8) -> Option<String> {
    OBSERVED_CHAT_PREFIX.with_borrow(|map| map.get(&id).cloned())
}

fn find_player_from_message(full_msg: &str) -> Option<(u8, Option<&str>, &str)> {
    if unsafe { Server.IsSinglePlayer } != 0 {
        // in singleplayer there is no tab list, even self id infos are null

        return Some((ENTITY_SELF_ID, None, full_msg));
    }

    // find colon from the left
    let pos = full_msg
        .find(": ")
        .and_then(|pos| if pos >= 3 { Some(pos) } else { None })?;

    // > &fasdfasdf

    // &]SpiralP
    let full_nick = &full_msg.get(..pos)?; // left without colon

    // &faaa
    let said_text = &full_msg.get((pos + 2)..)?; // right without colon

    debug!(?full_nick, ?said_text);
    // TODO title is [ ] before nick, team is < > before nick, also there are rank
    // symbols? &f┬ &f♂&6 Goodly: &fhi

    TAB_LIST.with(move |cell| {
        let tab_list = &*cell.borrow();
        tab_list
            .as_ref()
            .unwrap()
            .find_entry_by_nick_name(full_nick)
            .and_then(|entry| entry.upgrade())
            .map(|entry| (entry.get_id(), Some(*full_nick), *said_text))
    })
}

#[cfg(test)]
mod tests {
    use super::{
        WhisperKind, detect_whisper_mode_transition, detect_whisper_prefix, is_continuation_message,
    };

    #[test]
    fn keeps_leading_color_intact() {
        // Re-emitted `&<lastColor>` stays in the returned slice; rejoining
        // with the prior line keeps the user's intent regardless of whether
        // it's a server re-emit or a user-typed color change.
        assert_eq!(is_continuation_message("> &feiusmod"), Some("&feiusmod"));
    }

    #[test]
    fn detects_continuation_without_color() {
        assert_eq!(is_continuation_message("> plain"), Some("plain"));
    }

    #[test]
    fn rejects_non_continuation() {
        assert_eq!(is_continuation_message("&7Player: &fhi"), None);
        assert_eq!(is_continuation_message(">no space"), None);
    }

    #[test]
    fn detects_incoming_whisper() {
        assert_eq!(
            detect_whisper_prefix("&9[>] &rFloaty: &fhi"),
            Some((WhisperKind::Incoming, "&rFloaty: &fhi"))
        );
    }

    #[test]
    fn detects_outgoing_whisper() {
        assert_eq!(
            detect_whisper_prefix("&7[<] &rFloaty: &fhi"),
            Some((WhisperKind::Outgoing, "&rFloaty: &fhi"))
        );
    }

    #[test]
    fn detects_whisper_without_leading_color() {
        assert_eq!(
            detect_whisper_prefix("[>] Floaty: hi"),
            Some((WhisperKind::Incoming, "Floaty: hi"))
        );
    }

    #[test]
    fn detects_whisper_with_multiple_leading_colors() {
        // Belt-and-suspenders: if a server stacks two color codes before the
        // bracket, walk past both.
        assert_eq!(
            detect_whisper_prefix("&9&l[>] &rFloaty: &fhi"),
            Some((WhisperKind::Incoming, "&rFloaty: &fhi"))
        );
    }

    #[test]
    fn rejects_non_whisper() {
        assert_eq!(detect_whisper_prefix("&7Player: &fhi"), None);
        // `[<3]` emote — only the literal 4-byte `[>] ` / `[<] ` markers match.
        assert_eq!(detect_whisper_prefix("&9[<3] heart"), None);
        assert_eq!(detect_whisper_prefix("> &fcontinuation"), None);
        assert_eq!(detect_whisper_prefix(""), None);
    }

    #[test]
    fn detects_auto_whisper_enabled_with_target() {
        assert_eq!(
            detect_whisper_mode_transition(
                "&7Auto-whisper enabled. All messages will now be sent to &cFloaty."
            ),
            Some(true)
        );
        // Same sentence with no leading color code.
        assert_eq!(
            detect_whisper_mode_transition(
                "Auto-whisper enabled. All messages will now be sent to Floaty."
            ),
            Some(true)
        );
    }

    #[test]
    fn detects_auto_whisper_toggle_on_without_target() {
        assert_eq!(
            detect_whisper_mode_transition("&7All messages sent will now auto-whisper"),
            Some(true)
        );
        assert_eq!(
            detect_whisper_mode_transition("All messages sent will now auto-whisper"),
            Some(true)
        );
    }

    #[test]
    fn detects_whisper_chat_turned_off() {
        assert_eq!(
            detect_whisper_mode_transition("&7Whisper chat turned off"),
            Some(false)
        );
        assert_eq!(
            detect_whisper_mode_transition("Whisper chat turned off"),
            Some(false)
        );
    }

    #[test]
    fn ignores_no_online_players_match() {
        // Target left mid-mode: server keeps p.whisper = true, so we must too.
        assert_eq!(
            detect_whisper_mode_transition("&7No online players match \"Floaty\"."),
            None
        );
    }

    #[test]
    fn ignores_unrelated_lines() {
        assert_eq!(detect_whisper_mode_transition("&7Player: &fhi"), None);
        assert_eq!(detect_whisper_mode_transition("> &fcontinuation"), None);
        assert_eq!(detect_whisper_mode_transition(""), None);
    }
}
