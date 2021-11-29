use crate::plugin::events::{InputEvent, InputEventListener};

use super::{context::Texture_Render, render_hook::Renderable};
use anyhow::{Error, Result};
use classicube_helpers::{entities::Entity, WithBorrow};
use classicube_sys::{
    cc_int16, DrawTextArgs, Drawer2D_DrawText, Drawer2D_MakeFont, Drawer2D_TextHeight,
    Drawer2D_TextWidth, FontDesc, Gfx, Gfx_LoadMatrix, Gfx_SetAlphaTest, Gfx_SetTexturing, Matrix,
    MatrixType__MATRIX_VIEW, OwnedBitmap, OwnedString, OwnedTexture, TextureRec, Vec3,
    FONT_FLAGS_FONT_FLAGS_NONE, MATH_DEG2RAD,
};
use std::{cell::RefCell, mem, os::raw::c_float, rc::Weak};
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

pub struct Bubble {
    entity: Weak<Entity>,
    texture: Option<OwnedTexture>,
    transform: Matrix,
}

impl Bubble {
    pub fn new(entity: Weak<Entity>) -> Self {
        Self {
            entity,
            texture: Default::default(),
            transform: Matrix::IDENTITY,
        }
    }

    fn update_transform(&mut self) {
        let entity = if let Some(entity) = self.entity.upgrade() {
            entity
        } else {
            warn!("entity Rc Weak dropped?");
            return;
        };

        let (position, rotation) = match get_transform(entity.as_ref()) {
            Ok(ok) => ok,
            Err(e) => {
                warn!("get_transform: {:?}", e);
                return;
            }
        };

        let (width, height) = match self.texture.as_mut() {
            Some(t) => {
                let t = t.as_texture();
                (t.Width as f32, t.Height as f32)
            }
            _ => return,
        };

        // let ratio = width as f32 / height as f32;
        let width = BUBBLE_WIDTH as f32 / width as f32;
        // let height = ratio * width;
        let scale = Vec3::create(width, width, 1.0);
        self.transform = Matrix::scale(scale.X, scale.Y, scale.Z)
            * Matrix::rotate_z((-rotation.Z + 180.0) * MATH_DEG2RAD as c_float)
            * Matrix::rotate_x(-rotation.X * MATH_DEG2RAD as c_float)
            * Matrix::rotate_y(-rotation.Y * MATH_DEG2RAD as c_float)
            * Matrix::translate(position.X, position.Y, position.Z);
        /* return rotZ * rotX * rotY * scale * translate; */
    }
}

impl Renderable for Bubble {
    fn render(&mut self) {
        if self.texture.is_some() {
            self.update_transform();
        }

        let texture = match self.texture.as_mut() {
            Some(it) => it.as_texture_mut(),
            _ => return,
        };

        unsafe {
            let m = self.transform * Gfx.View;
            Gfx_LoadMatrix(MatrixType__MATRIX_VIEW, &m);
        }

        unsafe {
            Gfx_SetAlphaTest(1);
            Gfx_SetTexturing(1);

            Texture_Render(texture);
        }

        unsafe {
            Gfx_LoadMatrix(MatrixType__MATRIX_VIEW, &Gfx.View);
        }
    }
}

impl InputEventListener for Bubble {
    fn handle_event(&mut self, event: &InputEvent) {
        match event {
            InputEvent::ChatOpened => {
                self.texture = Some(create_texture(""));
            }

            InputEvent::ChatClosed => {
                self.texture = None;
            }

            InputEvent::InputTextChanged(text) => {
                self.texture = Some(create_texture(text));
            }
        }
    }
}

#[tracing::instrument]
fn create_texture(text: &str) -> OwnedTexture {
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
            let text_height = Drawer2D_TextHeight(&mut text_args);
            let width = text_width;
            let height = text_height;

            let mut bitmap = OwnedBitmap::new_pow_of_2(width, height, 0xFFFF_00FF);
            Drawer2D_DrawText(bitmap.as_bitmap_mut(), &mut text_args, 0, 0);

            (bitmap, width, height)
        };

        drop(string);

        (bitmap, width, height)
    });

    let u2 = width as f32 / bitmap.as_bitmap().width as f32;
    let v2 = height as f32 / bitmap.as_bitmap().height as f32;

    let texture = OwnedTexture::new(
        bitmap.as_bitmap_mut(),
        (-(width as cc_int16 / 2), -(height as cc_int16)),
        (width as _, height as _),
        TextureRec {
            U1: 0.0,
            V1: 0.0,
            U2: u2,
            V2: v2,
        },
    );

    // let tex = texture.as_texture_mut();
    // tex.Width = width as _;
    // tex.Height = height as _;

    texture
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
