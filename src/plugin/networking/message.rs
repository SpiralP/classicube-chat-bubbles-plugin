use crate::plugin::events::{emit_input_event, InputEvent};
use anyhow::Result;
use classicube_relay::{packet::Scope, Stream};
use serde::{Deserialize, Serialize};
use tracing::trace;

pub const RELAY_CHANNEL: u8 = 202;

#[derive(Debug, Serialize, Deserialize)]
pub enum RelayMessage {
    WhosThere,
    ChatInputEvent(InputEvent),
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

    pub fn handle_receive(player_id: u8, compressed_data: &[u8]) -> Result<()> {
        let data = zstd::decode_all(compressed_data)?;
        let relay_message = bincode::deserialize::<RelayMessage>(&data)?;
        trace!(?player_id, ?relay_message, "handle_receive");
        match relay_message {
            RelayMessage::WhosThere => {
                // respond to request to connect, with offer
                // send_offer(player_id);
            }

            RelayMessage::ChatInputEvent(input_event) => {
                emit_input_event(player_id, input_event);
            }
        }

        Ok(())
    }
}
