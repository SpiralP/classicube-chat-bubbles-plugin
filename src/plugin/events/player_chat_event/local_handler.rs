use std::{
    cell::{Cell, RefCell},
    time::{Duration, Instant},
};

use classicube_helpers::async_manager;
use classicube_relay::packet::MapScope;
use futures::future::AbortHandle;
use tracing::{debug, error};

use super::PlayerChatEvent;
use crate::plugin::networking::message::RelayMessage;

thread_local!(
    static DEBOUNCE_FUTURE: RefCell<Option<AbortHandle>> = Default::default();
);

thread_local!(
    static LAST_SEND: Cell<Option<Instant>> = Default::default();
);

thread_local!(
    static BROADCAST_SNAPSHOT: RefCell<Option<String>> = Default::default();
);

pub fn current_broadcast_snapshot() -> Option<String> {
    BROADCAST_SNAPSHOT.with_borrow(|s| s.clone())
}

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

        PlayerChatEvent::Message(_) | PlayerChatEvent::MessageContinuation(_) => {
            // chat-received-derived events are never relayed; the receiving
            // side regenerates them from its own ChatReceivedEvent stream.
        }
    });
}

#[tracing::instrument]
fn send(event: PlayerChatEvent) {
    debug!("");
    match &event {
        PlayerChatEvent::InputTextChanged(text) => {
            BROADCAST_SNAPSHOT.with_borrow_mut(|s| *s = Some(text.clone()));
        }
        PlayerChatEvent::ChatClosed => {
            BROADCAST_SNAPSHOT.with_borrow_mut(|s| *s = None);
        }
        PlayerChatEvent::Message(_) | PlayerChatEvent::MessageContinuation(_) => {}
    }
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
    LAST_SEND.set(None);
    BROADCAST_SNAPSHOT.with_borrow_mut(|s| {
        s.take();
    });
}
