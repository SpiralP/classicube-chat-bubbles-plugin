mod easing;
mod helpers;
mod inner;

pub fn free() {
    helpers::free();
}

use std::{
    collections::VecDeque,
    rc::Weak,
    time::{Duration, Instant},
};

use classicube_helpers::entities::Entity;
use classicube_sys::{
    Gfx, Gfx_LoadMatrix, Gfx_SetAlphaBlending, Gfx_SetFaceCulling, Gfx_SetTexturing,
    MatrixType__MATRIX_VIEW, PackedCol_Make, Vec3,
};
use tracing::warn;

use self::{
    easing::{clamp01, decay_factor, ease_in_cubic, ease_out_cubic, smoothstep},
    inner::InnerBubble,
};
use super::{context::vertex_buffer::Texture_Render, render_hook::renderable::Renderable};
use crate::plugin::events::player_chat_event::{
    PlayerChatEvent, listener::PlayerChatEventListener,
};

const MESSAGE_LIFETIME: Duration = Duration::from_secs(5);
const SPAWN_DURATION: Duration = Duration::from_millis(200);
const FLY_AWAY_DURATION: Duration = Duration::from_millis(400);
const SPAWN_RISE: f32 = 0.15;
const FLY_AWAY_RISE: f32 = 0.30;
const STACK_GAP: f32 = 0.30;
const STACK_TWEEN_TAU: f32 = 0.08;

struct Message {
    spawn_instant: Instant,
    die_instant: Instant,
    inner: InnerBubble,
    /// World-space position snapshotted at message-creation time. Sent bubbles
    /// stay anchored where the player was when they spoke (unlike the typing
    /// bubble, which follows the player live).
    position: Vec3,
    rotation: Vec3,
    /// Eased toward `(messages.len() - 1 - i) * STACK_GAP` each frame.
    stack_y: f32,
}

pub struct Bubble {
    entity: Weak<Entity>,
    typing: Option<InnerBubble>,
    messages: VecDeque<Message>,
    last_render: Option<Instant>,
}

impl Bubble {
    pub fn new(entity: Weak<Entity>) -> Self {
        Self {
            entity,
            typing: Default::default(),
            messages: Default::default(),
            last_render: None,
        }
    }

    fn render_inner(inner: &mut InnerBubble, alpha: f32) {
        let alpha_byte = (clamp01(alpha) * 255.0) as u8;
        let col = PackedCol_Make(255, 255, 255, alpha_byte);

        let front_texture = inner.textures.0.as_texture_mut();
        let back_texture = inner.textures.1.as_texture_mut();

        for (front, texture) in [(true, front_texture), (false, back_texture)] {
            unsafe {
                let m = inner.transform * Gfx.View;
                Gfx_LoadMatrix(MatrixType__MATRIX_VIEW, &m);

                Gfx_SetAlphaBlending(1);
                Gfx_SetTexturing(1);
                Gfx_SetFaceCulling(1);

                Texture_Render(texture, col, front);

                Gfx_SetFaceCulling(0);
                Gfx_SetAlphaBlending(0);

                Gfx_LoadMatrix(MatrixType__MATRIX_VIEW, &raw const Gfx.View);
            }
        }
    }
}

impl Renderable for Bubble {
    fn render(&mut self) {
        let now = Instant::now();
        let dt = match self.last_render.replace(now) {
            Some(prev) => (now - prev).as_secs_f32(),
            None => 0.0,
        };

        // Keep bubbles alive through the fly-away phase so they can animate out.
        self.messages
            .retain(|m| now < m.die_instant + FLY_AWAY_DURATION);

        let len = self.messages.len();
        let stack_factor = decay_factor(dt, STACK_TWEEN_TAU);
        // Typing bubble (when present) occupies slot 0 closest to the head;
        // sent messages stack starting one slot above it.
        let typing_offset = if self.typing.is_some() { 1 } else { 0 };

        // Sent messages render first; they're anchored to their snapshotted
        // positions and don't follow the player.
        for (i, message) in self.messages.iter_mut().enumerate() {
            // Newest at the bottom of the message stack, older ones above.
            let target_stack_y = (len - 1 - i + typing_offset) as f32 * STACK_GAP;
            message.stack_y += (target_stack_y - message.stack_y) * stack_factor;

            let age = (now - message.spawn_instant).as_secs_f32();
            let spawn_t = clamp01(age / SPAWN_DURATION.as_secs_f32());
            let spawn_y = -SPAWN_RISE * (1.0 - ease_out_cubic(spawn_t));

            let (fly_y, alpha) = if now > message.die_instant {
                let past = (now - message.die_instant).as_secs_f32();
                let t = clamp01(past / FLY_AWAY_DURATION.as_secs_f32());
                (FLY_AWAY_RISE * ease_in_cubic(t), 1.0 - smoothstep(t))
            } else {
                (0.0, 1.0)
            };

            let y_offset = spawn_y + fly_y + message.stack_y;
            message
                .inner
                .update_transform(message.position, message.rotation, y_offset);
            Self::render_inner(&mut message.inner, alpha);
        }

        // Typing bubble renders LAST so it draws on top of the message stack
        // (depth-write is off, so render order decides overlap). It also
        // follows the player live, unlike sent messages.
        if let Some(typing) = self.typing.as_mut() {
            let entity = match self.entity.upgrade() {
                Some(e) => e,
                None => {
                    warn!("entity Rc Weak dropped?");
                    return;
                }
            };
            typing.update_transform_entity(&entity, 0.0);
            Self::render_inner(typing, 1.0);
        }
    }
}

impl PlayerChatEventListener for Bubble {
    fn handle_event(&mut self, event: &PlayerChatEvent) {
        match event {
            PlayerChatEvent::ChatClosed => {
                self.typing = None;
            }

            PlayerChatEvent::InputTextChanged(text) => {
                if text.is_empty() {
                    self.typing = None;
                } else {
                    self.typing = Some(InnerBubble::new(text));
                }
            }

            PlayerChatEvent::Message(text) => {
                let entity = match self.entity.upgrade() {
                    Some(e) => e,
                    None => {
                        warn!("entity Rc Weak dropped?");
                        return;
                    }
                };
                let (position, rotation) = match helpers::get_transform(&entity) {
                    Ok(t) => t,
                    Err(e) => {
                        warn!("get_transform: {:?}", e);
                        return;
                    }
                };
                let now = Instant::now();
                self.messages.push_back(Message {
                    spawn_instant: now,
                    die_instant: now + MESSAGE_LIFETIME,
                    inner: InnerBubble::new(text),
                    position,
                    rotation,
                    stack_y: 0.0,
                });
            }
        }
    }
}
