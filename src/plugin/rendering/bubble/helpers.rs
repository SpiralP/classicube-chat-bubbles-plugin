use std::{cell::RefCell, mem, os::raw::c_int};

use anyhow::{Error, Result};
use classicube_helpers::{entities::Entity, WithBorrow};
use classicube_sys::{
    cc_int16, Bitmap, Context2D, Context2D_DrawPixels, Context2D_DrawText, DrawTextArgs,
    Drawer2D_TextHeight, Drawer2D_TextWidth, FontDesc, Font_Make, OwnedContext2D, OwnedString,
    OwnedTexture, PackedCol, TextureRec, Vec3, FONT_FLAGS_FONT_FLAGS_NONE,
};
use tracing::{debug, warn};

use crate::{bubble_image_parts::*, plugin::rendering::bubble::inner::BUBBLE_HEIGHT};

thread_local!(
    static FONT: RefCell<FontDesc> = RefCell::new(unsafe {
        let mut font = mem::zeroed();
        Font_Make(&mut font, 8, FONT_FLAGS_FONT_FLAGS_NONE as _);
        font
    });
);

/// returns (front, back)
#[tracing::instrument]
pub fn create_textures(text: &str) -> (OwnedTexture, OwnedTexture) {
    debug!("");

    let (mut context_2d, width, height) = FONT.with_borrow_mut(|font| {
        let string = OwnedString::new(text);
        let (context_2d, width, height) = unsafe {
            let mut text_args = DrawTextArgs {
                text: string.get_cc_string(),
                font,
                useShadow: 0,
            };

            let text_width = Drawer2D_TextWidth(&mut text_args);
            let text_height = if text_width == 0 {
                0
            } else {
                Drawer2D_TextHeight(&mut text_args)
            };

            let width = text_width + (LEFT_WIDTH as c_int) * 2 + 2;
            let height =
                text_height.max(12) + (TOP_HEIGHT as c_int) + (BOTTOM_CENTER_HEIGHT as c_int) + 2;

            debug!(?text_width, ?text_height, ?width, ?height);

            let mut context_2d = OwnedContext2D::new_pow_of_2(width, height, FRONT_COLOR);

            if text_width != 0 && text_height != 0 {
                Context2D_DrawText(
                    context_2d.as_context_2d_mut(),
                    &mut text_args,
                    width / 2 - text_width / 2,
                    height / 2 - text_height / 2,
                );
            }

            draw_parts(context_2d.as_context_2d_mut(), width, height);

            (context_2d, width, height)
        };

        drop(string);

        (context_2d, width, height)
    });

    let u2 = width as f32 / context_2d.as_bitmap().width as f32;
    let v2 = height as f32 / context_2d.as_bitmap().height as f32;

    let front_texture = OwnedTexture::new(
        context_2d.as_bitmap_mut(),
        (-(width as cc_int16 / 2), -(height as cc_int16)),
        (width as _, height as _),
        TextureRec {
            U1: 0.0,
            V1: 0.0,
            U2: u2,
            V2: v2,
        },
    );

    let mut context_2d = OwnedContext2D::new(1, 1, BACK_COLOR);
    let back_texture = OwnedTexture::new(
        context_2d.as_bitmap_mut(),
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
    let bubble_y = entity.get_model_name_y() * entity.get_model_scale().Y - BUBBLE_HEIGHT;
    let mut position = entity.get_position();
    position.Y += bubble_y;

    let rot = entity.get_rot();
    let rotation = Vec3::create(rot[0] + entity.get_head()[0], rot[1], rot[2]);

    Ok::<_, Error>((position, rotation))
}

unsafe fn draw_parts(context: &mut Context2D, width: c_int, height: c_int) {
    // TOP_LEFT_CORNER
    let mut top_left_corner_pixels = TOP_LEFT_CORNER_PIXELS;
    Context2D_DrawPixels(
        context,
        0,
        0,
        &mut Bitmap {
            scan0: top_left_corner_pixels.as_mut_ptr(),
            width: TOP_LEFT_CORNER_WIDTH as c_int,
            height: TOP_LEFT_CORNER_HEIGHT as c_int,
        },
    );

    // TOP
    let mut top_pixels = TOP_PIXELS;
    for x in (TOP_LEFT_CORNER_WIDTH as c_int)..width {
        Context2D_DrawPixels(
            context,
            x,
            0,
            &mut Bitmap {
                scan0: top_pixels.as_mut_ptr(),
                width: TOP_WIDTH as c_int,
                height: TOP_HEIGHT as c_int,
            },
        );
    }

    // TOP_LEFT_CORNER flip x
    let mut top_right_corner_pixels = TOP_LEFT_CORNER_PIXELS;
    flip_x(
        &mut top_right_corner_pixels,
        TOP_LEFT_CORNER_WIDTH as usize,
        TOP_LEFT_CORNER_HEIGHT as usize,
    );
    Context2D_DrawPixels(
        context,
        width - TOP_LEFT_CORNER_WIDTH as c_int,
        0,
        &mut Bitmap {
            scan0: top_right_corner_pixels.as_mut_ptr(),
            width: TOP_LEFT_CORNER_WIDTH as c_int,
            height: TOP_LEFT_CORNER_HEIGHT as c_int,
        },
    );

    // LEFT
    let mut left_pixels = LEFT_PIXELS;
    for y in TOP_LEFT_CORNER_HEIGHT as usize..height as usize {
        Context2D_DrawPixels(
            context,
            0,
            y as c_int,
            &mut Bitmap {
                scan0: left_pixels.as_mut_ptr(),
                width: LEFT_WIDTH as c_int,
                height: LEFT_HEIGHT as c_int,
            },
        );
    }

    // LEFT flip x
    let mut right_pixels = LEFT_PIXELS;
    flip_x(&mut right_pixels, LEFT_WIDTH as usize, LEFT_HEIGHT as usize);
    for y in TOP_LEFT_CORNER_HEIGHT as usize..height as usize {
        Context2D_DrawPixels(
            context,
            width - LEFT_WIDTH as c_int,
            y as c_int,
            &mut Bitmap {
                scan0: right_pixels.as_mut_ptr(),
                width: LEFT_WIDTH as c_int,
                height: LEFT_HEIGHT as c_int,
            },
        );
    }

    // BOTTOM_LEFT_CORNER
    let mut bottom_left_corner_pixels = BOTTOM_LEFT_CORNER_PIXELS;
    Context2D_DrawPixels(
        context,
        0,
        height - BOTTOM_LEFT_CORNER_HEIGHT as c_int,
        &mut Bitmap {
            scan0: bottom_left_corner_pixels.as_mut_ptr(),
            width: BOTTOM_LEFT_CORNER_WIDTH as c_int,
            height: BOTTOM_LEFT_CORNER_HEIGHT as c_int,
        },
    );

    // BOTTOM
    let mut bottom_pixels = BOTTOM_PIXELS;
    for x in (BOTTOM_LEFT_CORNER_WIDTH as c_int)..width {
        Context2D_DrawPixels(
            context,
            x,
            height - BOTTOM_HEIGHT as c_int,
            &mut Bitmap {
                scan0: bottom_pixels.as_mut_ptr(),
                width: BOTTOM_WIDTH as c_int,
                height: BOTTOM_HEIGHT as c_int,
            },
        );
    }

    // BOTTOM_LEFT_CORNER flip x
    let mut bottom_right_corner_pixels = BOTTOM_LEFT_CORNER_PIXELS;
    flip_x(
        &mut bottom_right_corner_pixels,
        BOTTOM_LEFT_CORNER_WIDTH as usize,
        BOTTOM_LEFT_CORNER_HEIGHT as usize,
    );
    Context2D_DrawPixels(
        context,
        width - BOTTOM_LEFT_CORNER_WIDTH as c_int,
        height - BOTTOM_LEFT_CORNER_HEIGHT as c_int,
        &mut Bitmap {
            scan0: bottom_right_corner_pixels.as_mut_ptr(),
            width: BOTTOM_LEFT_CORNER_WIDTH as c_int,
            height: BOTTOM_LEFT_CORNER_HEIGHT as c_int,
        },
    );

    // BOTTOM_CENTER
    let mut bottom_center_pixels = BOTTOM_CENTER_PIXELS;
    flip_x(
        &mut bottom_center_pixels,
        BOTTOM_CENTER_WIDTH as usize,
        BOTTOM_CENTER_HEIGHT as usize,
    );
    Context2D_DrawPixels(
        context,
        width / 2 - BOTTOM_CENTER_WIDTH as c_int / 2,
        height - BOTTOM_CENTER_HEIGHT as c_int,
        &mut Bitmap {
            scan0: bottom_center_pixels.as_mut_ptr(),
            width: BOTTOM_CENTER_WIDTH as c_int,
            height: BOTTOM_CENTER_HEIGHT as c_int,
        },
    );
}

fn flip_x(c: &mut [PackedCol], w: usize, h: usize) {
    for x in 0..w / 2 {
        for y in 0..h {
            let i1 = y * w + x;
            let i2 = y * w + w - x - 1;
            c.swap(i1, i2);
        }
    }
}
