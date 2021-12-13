mod helpers;
mod inner;

use self::inner::InnerBubble;
use super::{context::vertex_buffer::Texture_Render, render_hook::renderable::Renderable};
use crate::plugin::events::player_chat_event::{
    listener::PlayerChatEventListener, PlayerChatEvent,
};
use classicube_helpers::entities::Entity;
use classicube_sys::{
    Gfx, Gfx_LoadMatrix, Gfx_SetAlphaTest, Gfx_SetFaceCulling, Gfx_SetTexturing,
    MatrixType__MATRIX_VIEW,
};
use std::{
    collections::VecDeque,
    rc::Weak,
    time::{Duration, Instant},
};
use tracing::warn;

pub struct Bubble {
    entity: Weak<Entity>,
    typing: Option<InnerBubble>,
    messages: VecDeque<(Instant, InnerBubble)>,
}

impl Bubble {
    pub fn new(entity: Weak<Entity>) -> Self {
        Self {
            entity,
            typing: Default::default(),
            messages: Default::default(),
        }
    }

    fn update_typing_transforms(&mut self) {
        let entity = if let Some(entity) = self.entity.upgrade() {
            entity
        } else {
            warn!("entity Rc Weak dropped?");
            return;
        };

        if let Some(typing) = self.typing.as_mut() {
            typing.update_transform_entity(&entity);
        }
    }

    fn render_inner(inner: &mut InnerBubble) {
        let front_texture = inner.textures.0.as_texture_mut();
        let back_texture = inner.textures.1.as_texture_mut();

        for (front, texture) in [(true, front_texture), (false, back_texture)] {
            unsafe {
                let m = inner.transform * Gfx.View;
                Gfx_LoadMatrix(MatrixType__MATRIX_VIEW, &m);

                Gfx_SetAlphaTest(1);
                Gfx_SetTexturing(1);
                Gfx_SetFaceCulling(1);

                Texture_Render(texture, front);

                Gfx_SetFaceCulling(0);

                Gfx_LoadMatrix(MatrixType__MATRIX_VIEW, &Gfx.View);
            }
        }
    }
}

impl Renderable for Bubble {
    fn render(&mut self) {
        self.update_typing_transforms();

        if let Some(typing) = self.typing.as_mut() {
            Self::render_inner(typing);
        }

        let now = Instant::now();
        self.messages.retain(|(die_instant, _)| now < *die_instant);

        for (_, message) in &mut self.messages {
            Self::render_inner(message);
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
                if let Some(entity) = self.entity.upgrade() {
                    let mut inner = InnerBubble::new(text);
                    inner.update_transform_entity(&entity);

                    self.messages
                        .push_back((Instant::now() + Duration::from_secs(5), inner));
                } else {
                    warn!("entity Rc Weak dropped?");
                }
            }
        }
    }
}
