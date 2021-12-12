use anyhow::{Error, Result};
use classicube_helpers::{entities::Entity, WithBorrow};
use classicube_sys::{
    cc_int16, DrawTextArgs, Drawer2D_DrawText, Drawer2D_MakeFont, Drawer2D_TextHeight,
    Drawer2D_TextWidth, FontDesc, OwnedBitmap, OwnedString, OwnedTexture, TextureRec, Vec3,
    FONT_FLAGS_FONT_FLAGS_NONE,
};
use std::{cell::RefCell, mem};
use tracing::{debug, warn};

thread_local!(
    static FONT: RefCell<FontDesc> = RefCell::new(unsafe {
        let mut font = mem::zeroed();
        Drawer2D_MakeFont(&mut font, 16, FONT_FLAGS_FONT_FLAGS_NONE as _);
        font
    });
);

/// returns (front, back)
#[tracing::instrument]
pub fn create_textures(text: &str) -> (OwnedTexture, OwnedTexture) {
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
            U1: 0.0,
            V1: 0.0,
            U2: u2,
            V2: v2,
        },
    );

    let mut bitmap = OwnedBitmap::new(1, 1, 0xFFFF_00FF);
    let back_texture = OwnedTexture::new(
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

    (front_texture, back_texture)
}

pub fn get_transform(entity: &Entity) -> Result<(Vec3, Vec3)> {
    let inner = entity.get_inner();

    let bubble_y = entity.get_model_name_y() + (1.0 / 32.0) * inner.NameTex.Height as f32;
    let position = Vec3::transform_y(bubble_y, inner.Transform);

    let rot = entity.get_rot();
    let rotation = Vec3::create(rot[0] + entity.get_head()[0], rot[1], rot[2]);

    Ok::<_, Error>((position, rotation))
}
