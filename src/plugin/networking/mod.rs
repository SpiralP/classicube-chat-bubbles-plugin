mod message;

use self::message::RELAY_CHANNEL;
use crate::plugin::networking::message::RelayMessage;
use anyhow::Error;
use classicube_helpers::{async_manager, WithBorrow};
use classicube_relay::{packet::MapScope, RelayListener};
use std::cell::RefCell;
use tracing::{error, trace};

thread_local!(
    static RELAY_LISTENER: RefCell<Option<RelayListener>> = Default::default();
);

pub fn initialize() {
    let mut relay_listener = RelayListener::new(RELAY_CHANNEL).unwrap();
    relay_listener.on(|player_id, compressed_data| {
        trace!(player_id, ?compressed_data, "relay data");

        if let Err(e) = RelayMessage::handle_receive(player_id, compressed_data) {
            error!("handle_receive: {:#?}", e);
        }
    });

    RELAY_LISTENER.with_borrow_mut(move |option| {
        *option = Some(relay_listener);
    });
}

pub fn on_new_map_loaded() {
    async_manager::spawn_local_on_main_thread(async move {
        if let Err(e) = async move {
            // send request to everyone in map
            RelayMessage::WhosThere.send(MapScope { have_plugin: true })?;
            Ok::<_, Error>(())
        }
        .await
        {
            error!("{:?}", e);
        }
    });
}

pub fn free() {
    RELAY_LISTENER.with_borrow_mut(|option| {
        drop(option.take());
    });
}
