use anyhow::{Result, ensure};
use classicube_helpers::entities::ENTITY_SELF_ID;
use classicube_relay::{
    Stream,
    packet::{MapScope, Scope},
};
use classicube_sys::{INPUTWIDGET_LEN, INPUTWIDGET_MAX_LINES};
use serde::{Deserialize, Serialize};
use tracing::{error, trace, warn};

use crate::plugin::events::player_chat_event::{PlayerChatEvent, Presence, local_handler};

pub const RELAY_CHANNEL: u8 = 202;

/// Cap on the UTF-8 byte length of a `Presence::Typing` payload from the
/// relay. Local senders pull from a `ChatInputWidget` whose backing buffer
/// is hard-capped at `INPUTWIDGET_MAX_LINES * INPUTWIDGET_LEN = 3 * 64 = 192`
/// cp437 bytes (`Widgets.h:237`, `Widgets.c:1259-1260`). Every cp437 byte
/// maps to a BMP codepoint (`Convert_CP437ToUnicode`), expanding to at most
/// 3 UTF-8 bytes — so a well-behaved sender never exceeds 576 wire bytes.
/// Anything past this cap is malformed or hostile; drop it before it reaches
/// the renderer.
const MAX_INPUT_TEXT_BYTES: usize =
    (INPUTWIDGET_MAX_LINES as usize) * (INPUTWIDGET_LEN as usize) * 3;

#[derive(Debug, Serialize, Deserialize)]
pub enum RelayMessage {
    WhosThere,
    PlayerChatEvent(PlayerChatEvent),
}

impl RelayMessage {
    pub fn send<S: Into<Scope>>(&self, scope: S) -> Result<()> {
        trace!("send {:#?}", self);
        let data = bincode::serde::encode_to_vec(self, bincode::config::legacy())?;
        let compressed_data = zstd::encode_all(&*data, 0)?;
        let stream = Stream::new(compressed_data, scope)?;
        for packet in stream.packets()? {
            let mut data = packet.encode()?;

            unsafe {
                classicube_sys::CPE_SendPluginMessage(RELAY_CHANNEL, data.as_mut_ptr());
            }
        }

        Ok(())
    }

    #[tracing::instrument]
    pub fn handle_receive(player_id: u8, compressed_data: &[u8]) -> Result<()> {
        ensure!(player_id != ENTITY_SELF_ID, "got ENTITY_SELF_ID");

        let data = zstd::decode_all(compressed_data)?;
        let (relay_message, _): (RelayMessage, _) =
            bincode::serde::decode_from_slice(&data, bincode::config::legacy())?;
        trace!(?player_id, ?relay_message, "");
        match relay_message {
            RelayMessage::WhosThere => {
                if let Some(presence) = local_handler::current_broadcast_snapshot() {
                    let reply = RelayMessage::PlayerChatEvent(PlayerChatEvent::PresenceChanged(
                        Some(presence),
                    ));
                    if let Err(e) = reply.send(MapScope { have_plugin: true }) {
                        error!("WhosThere reply: {:?}", e);
                    }
                }
            }

            RelayMessage::PlayerChatEvent(event) => {
                match &event {
                    PlayerChatEvent::PresenceChanged(Some(Presence::Typing(text)))
                        if text.len() > MAX_INPUT_TEXT_BYTES =>
                    {
                        warn!(
                            ?player_id,
                            len = text.len(),
                            "PresenceChanged(Typing) exceeds cap, dropping"
                        );
                        return Ok(());
                    }
                    PlayerChatEvent::Message(_) | PlayerChatEvent::MessageContinuation(_) => {
                        // local_handler never relays these — receivers regenerate
                        // them from their own ChatReceivedEvent stream. Anything
                        // arriving here is malformed or hostile; drop it before
                        // it reaches the bubble renderer.
                        warn!(
                            ?player_id,
                            ?event,
                            "unexpected chat-derived event on relay, dropping"
                        );
                        return Ok(());
                    }
                    _ => {}
                }
                event.emit(player_id);
            }
        }

        Ok(())
    }
}
