use super::PlayerChatEvent;
use crate::plugin::networking::message::RelayMessage;
use classicube_helpers::{async_manager, entities::ENTITY_SELF_ID, CellGetSet, WithBorrow};
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

// TODO check (and don't send) if empty string right after ChatOpened

const INTERVAL: Duration = Duration::from_millis(500);

impl PlayerChatEvent {
    pub fn emit_local(self) {
        self.clone().emit(ENTITY_SELF_ID);

        DEBOUNCE_FUTURE.with_borrow_mut(move |debounce_future| match &self {
            PlayerChatEvent::ChatOpened | PlayerChatEvent::ChatClosed => {
                LAST_SEND.set(None);
                if let Some(handle) = debounce_future.take() {
                    handle.abort()
                }

                send(self);
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

                debug!(?instant);
                if instant {
                    LAST_SEND.set(Some(Instant::now()));
                    send(self);
                } else {
                    let (f, handle) = futures::future::abortable(async move {
                        async_manager::sleep(INTERVAL).await;

                        LAST_SEND.set(Some(Instant::now()));
                        send(self);
                    });
                    *debounce_future = Some(handle);
                    async_manager::spawn_local_on_main_thread(async move {
                        let _ = f.await;
                    });
                }
            }

            PlayerChatEvent::Message(m) => {
                todo!();
            }
        });
    }
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
