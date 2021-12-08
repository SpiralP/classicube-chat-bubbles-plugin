use crate::plugin::events::player_chat_event::PlayerChatEvent;
use anyhow::{ensure, Result};
use classicube_helpers::entities::ENTITY_SELF_ID;
use classicube_relay::{packet::Scope, Stream};
use serde::{Deserialize, Serialize};
use tracing::trace;

pub const RELAY_CHANNEL: u8 = 202;

#[derive(Debug, Serialize, Deserialize)]
pub enum RelayMessage {
    WhosThere,
    PlayerChatEvent(PlayerChatEvent),
}

impl RelayMessage {
    pub fn send<S: Into<Scope>>(&self, scope: S) -> Result<()> {
        trace!("send {:#?}", self);
        let data = bincode::serialize(self)?;
        let compressed_data = zstd::encode_all(&*data, 0)?;
        let stream = Stream::new(compressed_data, scope).unwrap();
        for packet in stream.packets().unwrap() {
            let mut data = packet.encode().unwrap();

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
        let relay_message = bincode::deserialize::<RelayMessage>(&data)?;
        trace!(?player_id, ?relay_message, "");
        match relay_message {
            RelayMessage::WhosThere => {
                //
            }

            RelayMessage::PlayerChatEvent(event) => {
                event.emit(player_id);
            }
        }

        Ok(())
    }
}
