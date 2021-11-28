use super::{context::Texture_Render, render_hook::Renderable};
use classicube_helpers::WithBorrow;
use classicube_sys::{
    cc_int16, DrawTextArgs, Drawer2D_DrawText, Drawer2D_MakeFont, Drawer2D_TextHeight,
    Drawer2D_TextWidth, Entities, FontDesc, Gfx, Gfx_LoadMatrix, Gfx_SetAlphaTest,
    Gfx_SetTexturing, Matrix, MatrixType__MATRIX_VIEW, OwnedBitmap, OwnedString, OwnedTexture,
    TextureRec, Vec3, FONT_FLAGS_FONT_FLAGS_NONE, MATH_DEG2RAD,
};
use std::{cell::RefCell, mem, os::raw::c_float};
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
    entity_id: u8,
    texture: Option<OwnedTexture>,
    transform: Matrix,
}

impl Bubble {
    pub fn new(entity_id: u8) -> Self {
        Self {
            entity_id,
            texture: Default::default(),
            transform: Matrix::IDENTITY,
        }
    }

    fn update(&mut self) {
        if self.texture.is_none() {
            // TODO or if text changed
            self.texture = Some(create_texture());
        }

        self.update_transform();
    }

    fn update_transform(&mut self) {
        let p = unsafe { Entities.List[self.entity_id as usize] };
        if p.is_null() {
            warn!("player {} is null!", self.entity_id);
            return;
        }
        let e = unsafe { &mut *p };

        let scale = Vec3::create(1.0, 1.0, 1.0);
        self.transform = Matrix::scale(scale.X, scale.Y, scale.Z)
            * Matrix::rotate_z((-e.RotZ + 180.0) * MATH_DEG2RAD as c_float)
            * Matrix::rotate_x(-e.RotX * MATH_DEG2RAD as c_float)
            * Matrix::rotate_y(-e.RotY * MATH_DEG2RAD as c_float)
            * Matrix::translate(e.Position.X, e.Position.Y, e.Position.Z);
        /* return rotZ * rotX * rotY * scale * translate; */
    }
}

impl Renderable for Bubble {
    fn render(&mut self) {
        self.update();

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

#[tracing::instrument]
fn create_texture() -> OwnedTexture {
    debug!("");
    let text = "hello";

    let mut bitmap = FONT.with_borrow_mut(|font| {
        let string = OwnedString::new(text);
        let bitmap = unsafe {
            let mut text_args = DrawTextArgs {
                text: string.get_cc_string(),
                font,
                useShadow: 1,
            };

            let text_width = Drawer2D_TextWidth(&mut text_args);
            let text_height = Drawer2D_TextHeight(&mut text_args);

            let mut bitmap = OwnedBitmap::new_pow_of_2(text_width, text_height, 0xFFFF_00FF);
            Drawer2D_DrawText(bitmap.as_bitmap_mut(), &mut text_args, 0, 0);

            bitmap
        };

        drop(string);

        bitmap
    });

    let texture = OwnedTexture::new(
        bitmap.as_bitmap_mut(),
        (
            -(BUBBLE_WIDTH as cc_int16 / 2),
            -(BUBBLE_HEIGHT as cc_int16),
        ),
        (BUBBLE_WIDTH as _, BUBBLE_HEIGHT as _),
        TextureRec {
            U1: 0.0,
            V1: 0.0,
            U2: 1.0,
            V2: 1.0,
        },
    );

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
