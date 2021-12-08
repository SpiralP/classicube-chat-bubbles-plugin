use super::{context::vertex_buffer::Texture_Render, render_hook::renderable::Renderable};
use crate::plugin::events::player_chat_event::{
    listener::PlayerChatEventListener, PlayerChatEvent,
};
use anyhow::{Error, Result};
use classicube_helpers::{entities::Entity, WithBorrow};
use classicube_sys::{
    cc_int16, DrawTextArgs, Drawer2D_DrawText, Drawer2D_MakeFont, Drawer2D_TextHeight,
    Drawer2D_TextWidth, FontDesc, Gfx, Gfx_LoadMatrix, Gfx_SetAlphaTest, Gfx_SetFaceCulling,
    Gfx_SetTexturing, Matrix, MatrixType__MATRIX_VIEW, OwnedBitmap, OwnedString, OwnedTexture,
    TextureRec, Vec3, FONT_FLAGS_FONT_FLAGS_NONE, MATH_DEG2RAD,
};
use std::{
    cell::RefCell,
    collections::VecDeque,
    mem,
    os::raw::c_float,
    rc::Weak,
    time::{Duration, Instant},
};
use tracing::{debug, warn};

pub const BUBBLE_WIDTH: u8 = 4;
pub const BUBBLE_HEIGHT: u8 = 1;

thread_local!(
    static FONT: RefCell<FontDesc> = RefCell::new(unsafe {
        let mut font = mem::zeroed();
        Drawer2D_MakeFont(&mut font, 16, FONT_FLAGS_FONT_FLAGS_NONE as _);
        font
    });
);

struct InnerBubble {
    /// (front, back)
    textures: (OwnedTexture, OwnedTexture),
    transforms: (Matrix, Matrix),
}
impl InnerBubble {
    pub fn new(text: &str) -> InnerBubble {
        InnerBubble {
            textures: create_textures(text),
            transforms: (Matrix::IDENTITY, Matrix::IDENTITY),
        }
    }

    pub fn update_transform(&mut self, position: Vec3, rotation: Vec3) {
        let width = self.textures.0.as_texture().Width;

        // let ratio = width as f32 / height as f32;
        let width = BUBBLE_WIDTH as f32 / width as f32;
        // let height = ratio * width;
        let scale = Vec3::create(width, width, 1.0);

        let translation = Matrix::translate(position.X, position.Y, position.Z);
        let scale = Matrix::scale(scale.X, scale.Y, scale.Z);

        let front = scale
            * Matrix::rotate_z(-rotation.Z * MATH_DEG2RAD as c_float)
            * Matrix::rotate_x(-rotation.X * MATH_DEG2RAD as c_float)
            * Matrix::rotate_y(-rotation.Y * MATH_DEG2RAD as c_float)
            * translation;

        let back = scale
            * Matrix::rotate_z((-rotation.Z + 0.0) * MATH_DEG2RAD as c_float)
            * Matrix::rotate_x((-rotation.X + 0.0) * MATH_DEG2RAD as c_float)
            * Matrix::rotate_y((-rotation.Y + 180.0) * MATH_DEG2RAD as c_float)
            * translation;

        self.transforms = (front, back);
    }

    pub fn update_transform_entity(&mut self, entity: &Entity) {
        let (position, rotation) = match get_transform(entity) {
            Ok(ok) => ok,
            Err(e) => {
                warn!("get_transform: {:?}", e);
                return;
            }
        };
        self.update_transform(position, rotation);
    }
}

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

        for (transform, texture) in [
            (inner.transforms.0, front_texture),
            (inner.transforms.1, back_texture),
        ] {
            unsafe {
                let m = transform * Gfx.View;
                Gfx_LoadMatrix(MatrixType__MATRIX_VIEW, &m);

                Gfx_SetAlphaTest(1);
                Gfx_SetTexturing(1);
                Gfx_SetFaceCulling(1);

                Texture_Render(texture);

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
            PlayerChatEvent::ChatOpened => {
                self.typing = Some(InnerBubble::new(""));
            }

            PlayerChatEvent::ChatClosed => {
                self.typing = None;
            }

            PlayerChatEvent::InputTextChanged(text) => {
                self.typing = Some(InnerBubble::new(text));
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

/// returns (front, back)
#[tracing::instrument]
fn create_textures(text: &str) -> (OwnedTexture, OwnedTexture) {
    debug!("");

    let (mut bitmap, width, height) = FONT.with_borrow_mut(|font| {
        let string = OwnedString::new(text);
        let (bitmap, width, height) = unsafe {
            let mut text_args = DrawTextArgs {
                text: string.get_cc_string(),
                font,
                useShadow: 1,
            };

            let text_width = Drawer2D_TextWidth(&mut text_args);
            let text_height = if text_width == 0 {
                0
            } else {
                Drawer2D_TextHeight(&mut text_args)
            };
            let width = text_width + 4;
            let height = text_height + 4;

            debug!(?text_width, ?text_height);

            let mut bitmap = OwnedBitmap::new_pow_of_2(width, height, 0xFFFF_00FF);
            Drawer2D_DrawText(bitmap.as_bitmap_mut(), &mut text_args, 1, 1);

            (bitmap, width, height)
        };

        drop(string);

        (bitmap, width, height)
    });

    let u2 = width as f32 / bitmap.as_bitmap().width as f32;
    let v2 = height as f32 / bitmap.as_bitmap().height as f32;

    let front_texture = OwnedTexture::new(
        bitmap.as_bitmap_mut(),
        (-(width as cc_int16 / 2), -(height as cc_int16)),
        (width as _, height as _),
        TextureRec {
            U1: u2,
            V1: v2,
            U2: 0.0,
            V2: 0.0,
        },
    );

    let mut bitmap = OwnedBitmap::new(1, 1, 0xFFFF_00FF);
    let back_texture = OwnedTexture::new(
        bitmap.as_bitmap_mut(),
        (-(width as cc_int16 / 2), -(height as cc_int16)),
        (width as _, height as _),
        TextureRec {
            U1: u2,
            V1: v2,
            U2: 0.0,
            V2: 0.0,
        },
    );

    // let tex = texture.as_texture_mut();
    // tex.Width = width as _;
    // tex.Height = height as _;

    (front_texture, back_texture)
}

// fn update_texture(&mut self, mut part: OwnedBitmap) {
//     let texture = match self.texture.as_mut() {
//         Some(it) => it.as_texture_mut(),
//         _ => return,
//     };
//     let part = part.as_bitmap_mut();

//     // update uv's
//     texture.uv.U2 = part.width as f32 / TEXTURE_WIDTH as f32;
//     texture.uv.V2 = part.height as f32 / TEXTURE_HEIGHT as f32;

//     unsafe {
//         Gfx_UpdateTexturePart(texture.ID, 0, 0, part, 0);
//     }
// }

fn get_transform(entity: &Entity) -> Result<(Vec3, Vec3)> {
    let inner = entity.get_inner();

    let bubble_y = entity.get_model_name_y() + (1.0 / 32.0) * inner.NameTex.Height as f32;
    let position = Vec3::transform_y(bubble_y, inner.Transform);

    let rot = entity.get_rot();
    let rotation = Vec3::create(rot[0] + entity.get_head()[0], rot[1], rot[2]);

    Ok::<_, Error>((position, rotation))
}
