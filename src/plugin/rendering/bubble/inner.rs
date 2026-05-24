use std::os::raw::c_float;

use classicube_helpers::entities::Entity;
use classicube_sys::{MATH_DEG2RAD, Matrix, OwnedTexture, Vec3};
use tracing::warn;

use super::helpers::{SINGLE_LINE_CANVAS_HEIGHT, create_textures, get_transform};

// pub const BUBBLE_WIDTH: u8 = 4;
pub const BUBBLE_HEIGHT: f32 = 0.5;

/// Fixed pixel-to-world scale. A single-line bubble fills `BUBBLE_HEIGHT`;
/// multi-line bubbles grow taller proportionally. Without this, the original
/// `BUBBLE_HEIGHT / texture_height` ratio shrank each line as more were added.
const SCALE_RATIO: f32 = BUBBLE_HEIGHT / SINGLE_LINE_CANVAS_HEIGHT as f32;

pub struct InnerBubble {
    /// (front, back)
    pub textures: (OwnedTexture, OwnedTexture),
    pub transform: Matrix,
}
impl InnerBubble {
    pub fn new(lines: &[String]) -> Option<InnerBubble> {
        Some(InnerBubble {
            textures: create_textures(lines)?,
            transform: Matrix::IDENTITY,
        })
    }

    /// World-space height of the rendered bubble. The stacker uses this to
    /// advance each older bubble by its own height (minus a small overlap),
    /// keeping the visual gap between bubbles constant regardless of how
    /// many text lines each one contains.
    pub fn height_world(&self) -> f32 {
        self.textures.0.as_texture().height as f32 * SCALE_RATIO
    }

    /// `position` is the eye world position; `y_offset` is applied in the
    /// bubble's local frame (after the rotate_z flip), so head pitch rotates
    /// it along with the head. Callers should fold the eye-to-nameplate
    /// distance (from `get_transform`'s third return value) into `y_offset`
    /// so the bubble's resting position sits on top of the head.
    pub fn update_transform(&mut self, position: Vec3, rotation: Vec3, y_offset: f32) {
        let scale = Vec3::create(SCALE_RATIO, SCALE_RATIO, 1.0);

        let translation = Matrix::translate(position.x, position.y, position.z);
        let scale = Matrix::scale(scale.x, scale.y, scale.z);
        let local_up_translation = Matrix::translate(0.0, y_offset, 0.0);

        self.transform = scale
            * Matrix::rotate_z(180.0 * MATH_DEG2RAD as c_float)
            * local_up_translation
            * Matrix::rotate_x(-rotation.x * MATH_DEG2RAD as c_float)
            * Matrix::rotate_y(-rotation.y * MATH_DEG2RAD as c_float)
            * translation;
    }

    /// `animation_y` is the spawn/fly/stack offset; this helper adds the
    /// head-top offset on top so the resting bubble sits on the head.
    pub fn update_transform_entity(&mut self, entity: &Entity, animation_y: f32) {
        let (position, rotation, head_top_offset) = match get_transform(entity) {
            Ok(ok) => ok,
            Err(e) => {
                warn!("get_transform: {:?}", e);
                return;
            }
        };
        self.update_transform(position, rotation, animation_y + head_top_offset);
    }
}
