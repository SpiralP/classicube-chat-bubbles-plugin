use super::PlayerChatEvent;
use crate::plugin::networking::message::RelayMessage;
use classicube_helpers::{async_manager, CellGetSet, WithBorrow};
use classicube_relay::packet::MapScope;
use futures::future::AbortHandle;
use std::{
    cell::{Cell, RefCell},
    time::{Duration, Instant},
};
use tracing::{debug, error};

thread_local!(
    static DEBOUNCE_FUTURE: RefCell<Option<AbortHandle>> = Default::default();
);

thread_local!(
    static LAST_SEND: Cell<Option<Instant>> = Default::default();
);

const INTERVAL: Duration = Duration::from_millis(500);

pub fn handle_local_emit(event: PlayerChatEvent) {
    DEBOUNCE_FUTURE.with_borrow_mut(move |debounce_future| match &event {
        PlayerChatEvent::ChatClosed => {
            LAST_SEND.set(None);
            if let Some(handle) = debounce_future.take() {
                handle.abort()
            }

            send(event);
        }

        PlayerChatEvent::InputTextChanged(_) => {
            if let Some(handle) = debounce_future.take() {
                handle.abort()
            }

            let instant = if let Some(last_send) = LAST_SEND.get() {
                Instant::now().duration_since(last_send) > INTERVAL
            } else {
                true
            };

            if instant {
                LAST_SEND.set(Some(Instant::now()));
                send(event);
            } else {
                let (f, handle) = futures::future::abortable(async move {
                    async_manager::sleep(INTERVAL).await;

                    LAST_SEND.set(Some(Instant::now()));
                    send(event);
                });
                *debounce_future = Some(handle);
                async_manager::spawn_local_on_main_thread(async move {
                    let _ = f.await;
                });
            }
        }

        PlayerChatEvent::Message(_) => {
            // we only care about chat received events
        }
    });
}

#[tracing::instrument]
fn send(event: PlayerChatEvent) {
    debug!("");
    if let Err(e) = RelayMessage::PlayerChatEvent(event).send(MapScope { have_plugin: true }) {
        error!("{:?}", e);
    }
}

pub fn free() {
    DEBOUNCE_FUTURE.with_borrow_mut(move |debounce_future| {
        if let Some(handle) = debounce_future.take() {
            handle.abort()
        }
    });
}
