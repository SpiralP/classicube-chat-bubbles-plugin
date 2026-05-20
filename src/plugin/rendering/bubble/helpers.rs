use std::{cell::RefCell, mem, os::raw::c_int, slice};

use anyhow::{Error, Result};
use classicube_helpers::entities::Entity;
use classicube_sys::{
    Bitmap, Context2D, Context2D_DrawPixels, Context2D_DrawText, DrawTextArgs, Drawer2D_TextHeight,
    Drawer2D_TextWidth, FONT_FLAGS_FONT_FLAGS_NONE, Font_Free, Font_Make, FontDesc, OwnedContext2D,
    OwnedString, OwnedTexture, PackedCol, TextureRec, Vec3, cc_int16,
};
use tracing::{debug, warn};

use crate::bubble_image_parts::*;

const BACK_FILL: PackedCol = 0;

thread_local!(
    static FONT: RefCell<Option<FontDesc>> = const { RefCell::new(None) };
);

fn with_font<R>(f: impl FnOnce(&mut FontDesc) -> R) -> R {
    FONT.with_borrow_mut(|slot| {
        let font = slot.get_or_insert_with(|| unsafe {
            let mut font = mem::zeroed();
            Font_Make(&mut font, 8, FONT_FLAGS_FONT_FLAGS_NONE as _);
            font
        });
        f(font)
    })
}

pub fn free() {
    FONT.with_borrow_mut(|slot| {
        if let Some(mut font) = slot.take() {
            unsafe { Font_Free(&mut font) };
        }
    });
}

/// Body height a single line of text occupies (before borders) when its
/// rendered width is non-zero. Matches `text_height.max(12)` for single-line
/// inputs and lets the world-height scale stay constant across line counts.
pub const SINGLE_LINE_TEXT_HEIGHT: c_int = 12;

/// Total canvas height for a single-line bubble, in pixels. Used to derive the
/// fixed world-space scale ratio so multi-line bubbles don't squish vertically.
pub const SINGLE_LINE_CANVAS_HEIGHT: c_int =
    SINGLE_LINE_TEXT_HEIGHT + TOP_HEIGHT as c_int + BOTTOM_CENTER_HEIGHT as c_int + 2;

/// returns (front, back)
#[tracing::instrument]
pub fn create_textures(lines: &[String]) -> (OwnedTexture, OwnedTexture) {
    debug!("");

    let (mut front_context, mut back_context, width, height) = with_font(|font| {
        let strings: Vec<OwnedString> = lines.iter().map(|l| OwnedString::new(l.clone())).collect();

        unsafe {
            let metrics: Vec<(c_int, c_int)> = strings
                .iter()
                .map(|s| {
                    let mut args = DrawTextArgs {
                        text: s.get_cc_string(),
                        font,
                        useShadow: 0,
                    };
                    let w = Drawer2D_TextWidth(&mut args);
                    let h = if w == 0 {
                        0
                    } else {
                        Drawer2D_TextHeight(&mut args)
                    };
                    (w, h)
                })
                .collect();

            let max_w = metrics.iter().map(|(w, _)| *w).max().unwrap_or(0);
            let total_h: c_int = metrics.iter().map(|(_, h)| *h).sum();
            let body_height = total_h.max(SINGLE_LINE_TEXT_HEIGHT);

            let width = max_w + (LEFT_WIDTH as c_int) * 2 + 2;
            let height = body_height + (TOP_HEIGHT as c_int) + (BOTTOM_CENTER_HEIGHT as c_int) + 2;

            debug!(?max_w, ?total_h, ?width, ?height);

            let mut front_context = OwnedContext2D::new_pow_of_2(width, height, FRONT_COLOR);
            let mut back_context = OwnedContext2D::new_pow_of_2(width, height, BACK_FILL);

            // Center the text block vertically, then stack lines downward.
            // Lines are left-aligned within the text area; the bubble itself
            // stays centered on the player's head because its width tracks the
            // widest line and its texture origin is `-(width / 2)`.
            let text_x = LEFT_WIDTH as c_int + 1;
            let mut y = height / 2 - total_h / 2;
            for (s, (w, h)) in strings.iter().zip(metrics.iter()) {
                if *w != 0 && *h != 0 {
                    let mut args = DrawTextArgs {
                        text: s.get_cc_string(),
                        font,
                        useShadow: 0,
                    };
                    Context2D_DrawText(front_context.as_context_2d_mut(), &mut args, text_x, y);
                }
                y += *h;
            }

            draw_parts(front_context.as_context_2d_mut(), width, height);
            draw_parts(back_context.as_context_2d_mut(), width, height);

            // Border PNGs include FRONT_COLOR pixels next to the antialias edge
            // that blend invisibly into the front canvas's fill. On the
            // transparent back canvas they'd render as an opaque stripe, so
            // strip them out, leaving just the antialias outline.
            let back_bitmap = back_context.as_bitmap_mut();
            let total = (back_bitmap.width * back_bitmap.height) as usize;
            for px in slice::from_raw_parts_mut(back_bitmap.scan0, total) {
                if *px == FRONT_COLOR {
                    *px = BACK_FILL;
                }
            }

            (front_context, back_context, width, height)
        }
    });

    let u2 = width as f32 / front_context.as_bitmap().width as f32;
    let v2 = height as f32 / front_context.as_bitmap().height as f32;

    let position = (-(width as cc_int16 / 2), -(height as cc_int16));
    let size = (width as _, height as _);
    let rec = TextureRec {
        u1: 0.0,
        v1: 0.0,
        u2,
        v2,
    };

    let front_texture = OwnedTexture::new(front_context.as_bitmap_mut(), position, size, rec);
    let back_texture = OwnedTexture::new(back_context.as_bitmap_mut(), position, size, rec);

    (front_texture, back_texture)
}

/// Returns `(eye_world_position, rotation, eye_to_nameplate_offset)`.
///
/// The anchor is the entity's eye (head's pivot of rotation), so head pitch
/// doesn't move the anchor. The third value is the distance from eye to
/// nameplate in model-space scaled units — feed it into the rotation chain's
/// local-up translation so head pitch rotates the bubble along with the head,
/// instead of leaving it parked directly above the body.
pub fn get_transform(entity: &Entity) -> Result<(Vec3, Vec3, f32)> {
    let scale_y = entity.get_model_scale().y;
    let eye_y = entity.get_model_eye_y() * scale_y;
    let head_top_offset = entity.get_model_name_y() * scale_y - eye_y;

    let mut position = entity.get_position();
    position.y += eye_y;

    let rot = entity.get_rot();
    let rotation = Vec3::create(rot[0] + entity.get_head()[0], rot[1], rot[2]);

    Ok::<_, Error>((position, rotation, head_top_offset))
}

unsafe fn draw_parts(context: &mut Context2D, width: c_int, height: c_int) {
    unsafe {
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
