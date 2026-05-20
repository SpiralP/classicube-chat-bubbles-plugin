use std::{cell::RefCell, collections::HashMap};

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

                if let Some(continuation) = is_continuation_message(message) {
                    let result = LAST_CHAT.with_borrow_mut(|cell| {
                        let (id, mut lines) = cell.take()?;
                        lines.push(continuation.to_string());
                        *cell = Some((id, lines.clone()));
                        Some((id, lines))
                    });
                    let Some((player_id, lines)) = result else {
                        warn!(?continuation, "continuation with no prior message");
                        return;
                    };
                    PlayerChatEvent::MessageContinuation(lines).emit(player_id);
                } else if let Some((player_id, full_nick, said_text)) =
                    find_player_from_message(message)
                {
                    let said_text = said_text.to_string();
                    if let Some(nick) = full_nick {
                        OBSERVED_CHAT_PREFIX.with_borrow_mut(|map| {
                            map.insert(player_id, nick.to_string());
                        });
                    }
                    LAST_CHAT.with_borrow_mut(|cell| {
                        *cell = Some((player_id, vec![said_text.clone()]));
                    });
                    PlayerChatEvent::Message(said_text).emit(player_id);
                } else {
                    warn!(?message, "find_player_from_message failed");
                    LAST_CHAT.with_borrow_mut(|cell| {
                        *cell = None;
                    });
                }
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
    use super::is_continuation_message;

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
}
