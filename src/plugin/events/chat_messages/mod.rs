use classicube_helpers::{
    entities::ENTITY_SELF_ID,
    events::chat::{ChatReceivedEvent, ChatReceivedEventHandler},
    tab_list::TabList,
    WithBorrow,
};
use classicube_sys::{MsgType_MSG_TYPE_NORMAL, Server};
use std::cell::RefCell;

thread_local!(
    static CHAT_RECEIVED_HANDLER: RefCell<Option<ChatReceivedEventHandler>> = Default::default();
);

thread_local!(
    static TAB_LIST: RefCell<Option<TabList>> = Default::default();
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

                if let Some((id, message)) = find_player_from_message(message) {
                    // handle_local_event(InputEvent::InputTextChanged(text));
                    todo!();
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
}

fn find_player_from_message(full_msg: &str) -> Option<(u8, &str)> {
    if unsafe { Server.IsSinglePlayer } != 0 {
        // in singleplayer there is no tab list, even self id infos are null

        return Some((ENTITY_SELF_ID, full_msg));
    }

    // find colon from the left
    let opt = full_msg
        .find(": ")
        .and_then(|pos| if pos > 4 { Some(pos) } else { None });

    if let Some(pos) = opt {
        // > &fasdfasdf

        // &]SpiralP
        let full_nick = &full_msg.get(..pos)?; // left without colon

        // &faaa
        let said_text = &full_msg.get((pos + 2)..)?; // right without colon

        // TODO title is [ ] before nick, team is < > before nick, also there are rank
        // symbols? &f┬ &f♂&6 Goodly: &fhi

        TAB_LIST.with(move |cell| {
            let tab_list = &*cell.borrow();
            tab_list
                .as_ref()
                .unwrap()
                .find_entry_by_nick_name(full_nick)
                .and_then(|entry| entry.upgrade())
                .map(|entry| (entry.get_id(), *said_text))
        })
    } else {
        None
    }
}
