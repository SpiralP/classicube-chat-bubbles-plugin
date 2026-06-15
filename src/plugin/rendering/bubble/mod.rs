#[cfg(test)]
mod tests;

mod easing;
mod helpers;
mod inner;

// CP437 glyphs for the menu-state icon bubbles. ClassiCube's font is code page
// 437; OwnedString::new maps these Unicode codepoints back to their CP437 byte
// (Convert_CodepointToCP437) before drawing. Written as \u{} escapes to keep
// the source ASCII -- the glyphs are bullet / box-corner / triple-bar.
const DOT: char = '\u{2219}'; // CP437 0xF9
const CORNER: char = '\u{250C}'; // CP437 0xDA
const BARS: char = '\u{2261}'; // CP437 0xF0

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
    helpers::BubbleStyle,
    inner::InnerBubble,
};
use super::{context::vertex_buffer::Texture_Render, render_hook::renderable::Renderable};
use crate::plugin::events::{
    chat_message::{get_chat_prefix, get_nick_name},
    local_presence::wordwrap::{wrap_for_display, wrap_typing_for_display},
    player_chat_event::{PlayerChatEvent, Presence, listener::PlayerChatEventListener},
};

const MESSAGE_LIFETIME: Duration = Duration::from_secs(5);
const SPAWN_DURATION: Duration = Duration::from_millis(200);
const FLY_AWAY_DURATION: Duration = Duration::from_millis(400);
const SPAWN_RISE: f32 = 0.15;
const FLY_AWAY_RISE: f32 = 0.30;
/// How much an older bubble's bottom (tail) overlaps the newer bubble's top
/// edge, in world units. Constant across single- and multi-line bubbles so
/// the visual gap stays consistent. Tuned to the original single-line look
/// (`BUBBLE_HEIGHT 0.5 - STACK_OVERLAP 0.20 = 0.30` advance).
const STACK_OVERLAP: f32 = 0.20;
const STACK_TWEEN_TAU: f32 = 0.08;

struct Message {
    spawn_instant: Instant,
    die_instant: Instant,
    inner: InnerBubble,
    /// Eye world position snapshotted at message-creation time. Sent bubbles
    /// stay anchored where the player was when they spoke (unlike the status
    /// bubble, which follows the player live).
    position: Vec3,
    rotation: Vec3,
    /// Distance from eye to nameplate at send time. Added to `y_offset` so the
    /// resting bubble sits on top of the head (rotated with the head pitch).
    head_top_offset: f32,
    /// Eased toward the cumulative stack target each frame.
    stack_y: f32,
}

pub struct Bubble {
    entity: Weak<Entity>,
    status: Option<InnerBubble>,
    messages: VecDeque<Message>,
    last_render: Option<Instant>,
}

impl Bubble {
    pub fn new(entity: Weak<Entity>) -> Self {
        Self {
            entity,
            status: Default::default(),
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

        let stack_factor = decay_factor(dt, STACK_TWEEN_TAU);

        // Ease each bubble's stack_y toward its cumulative target, newest
        // first. Doing this in a separate pass avoids allocating a per-frame
        // targets vec while keeping the render pass below in oldest→newest
        // order (so newer bubbles composite on top; depth-write is off).
        let status_advance = self
            .status
            .as_ref()
            .map(|t| t.height_world() - STACK_OVERLAP)
            .unwrap_or(0.0);
        let mut y_acc = status_advance;
        for message in self.messages.iter_mut().rev() {
            message.stack_y += (y_acc - message.stack_y) * stack_factor;
            y_acc += message.inner.height_world() - STACK_OVERLAP;
        }

        for message in self.messages.iter_mut() {
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

            let y_offset = spawn_y + fly_y + message.stack_y + message.head_top_offset;
            message
                .inner
                .update_transform(message.position, message.rotation, y_offset);
            Self::render_inner(&mut message.inner, alpha);
        }

        // Status bubble renders LAST so it draws on top of the message stack
        // (depth-write is off, so render order decides overlap). It also
        // follows the player live, unlike sent messages.
        if let Some(status) = self.status.as_mut() {
            let entity = match self.entity.upgrade() {
                Some(e) => e,
                None => {
                    warn!("entity Rc Weak dropped?");
                    return;
                }
            };
            status.update_transform_entity(&entity, 0.0);
            Self::render_inner(status, 1.0);
        }
    }
}

impl PlayerChatEventListener for Bubble {
    fn handle_event(&mut self, event: &PlayerChatEvent) {
        match event {
            PlayerChatEvent::PresenceChanged(opt) => {
                self.status = match opt {
                    None => None,

                    Some(Presence::Typing(text)) => {
                        // Pre-wrap so the typing preview matches what the server
                        // will send when the player hits enter. Strip the `> `
                        // each continuation line gets — server-received
                        // continuations are already `> `-stripped before reaching
                        // the renderer, so this keeps both display paths consistent.
                        //
                        // Bubbles are per-entity and PresenceChanged is only
                        // emitted on ENTITY_SELF_ID, so `self.entity` is the local
                        // player. We feed a chat-line prefix into the wrap so the
                        // first line's 64-byte budget accounts for the `{nick}: `
                        // the server prepends. Prefer the most recently observed
                        // chat prefix (captures server-only titles/flair) and fall
                        // back to the tab-list nick; singleplayer / pre-tab-list /
                        // never-spoken cases fall back to bare-text wrap.
                        let lines = self
                            .entity
                            .upgrade()
                            .and_then(|e| {
                                let id = e.get_id();
                                get_chat_prefix(id).or_else(|| get_nick_name(id))
                            })
                            .map(|nick| wrap_typing_for_display(text, &nick))
                            .unwrap_or_else(|| wrap_for_display(text));
                        if lines.is_empty() {
                            None
                        } else {
                            InnerBubble::new(&lines, BubbleStyle::Bordered)
                        }
                    }

                    Some(Presence::EscapeMenu) => {
                        InnerBubble::new(&[format!("&f[&6{CORNER}&f]")], BubbleStyle::Borderless)
                    }

                    Some(Presence::BlockMenu) => InnerBubble::new(
                        &[format!("&f[&a{DOT} &s{DOT} &7{DOT}&f]")],
                        BubbleStyle::Borderless,
                    ),

                    Some(Presence::TabList) => {
                        InnerBubble::new(&[format!("&f[&7{BARS}&f]")], BubbleStyle::Borderless)
                    }
                };
            }

            PlayerChatEvent::Message(text) => {
                let entity = match self.entity.upgrade() {
                    Some(e) => e,
                    None => {
                        warn!("entity Rc Weak dropped?");
                        return;
                    }
                };
                let (position, rotation, head_top_offset) = match helpers::get_transform(&entity) {
                    Ok(t) => t,
                    Err(e) => {
                        warn!("get_transform: {:?}", e);
                        return;
                    }
                };
                let Some(inner) =
                    InnerBubble::new(std::slice::from_ref(text), BubbleStyle::Bordered)
                else {
                    warn!("InnerBubble::new returned None (context lost?), skipping message");
                    return;
                };
                let now = Instant::now();
                self.messages.push_back(Message {
                    spawn_instant: now,
                    die_instant: now + MESSAGE_LIFETIME,
                    inner,
                    position,
                    rotation,
                    head_top_offset,
                    stack_y: 0.0,
                });
            }

            PlayerChatEvent::MessageContinuation(lines) => {
                // Re-bake the most recent message with the accumulated
                // server-split lines, keeping spawn/die timing + anchor so the
                // bubble's lifetime doesn't reset. We use the server's break
                // points verbatim (rather than re-wrapping the join) so the
                // bubble shows what every other client sees. Best-effort: in
                // the unlikely case another message arrived for this speaker
                // between the original line and its `> ...` continuation, we
                // still edit `.back()` — the race is rare and harmless
                // visually.
                let Some(last) = self.messages.back_mut() else {
                    warn!("MessageContinuation with no prior message");
                    return;
                };
                if let Some(inner) = InnerBubble::new(lines, BubbleStyle::Bordered) {
                    last.inner = inner;
                } else {
                    warn!("InnerBubble::new returned None (context lost?), keeping prior bubble");
                }
            }
        }
    }
}
