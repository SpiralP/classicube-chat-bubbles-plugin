use super::helpers::{create_textures, get_transform};
use classicube_helpers::entities::Entity;
use classicube_sys::{Matrix, OwnedTexture, Vec3, MATH_DEG2RAD};
use std::os::raw::c_float;
use tracing::warn;

// pub const BUBBLE_WIDTH: u8 = 4;
pub const BUBBLE_HEIGHT: f32 = 0.5;

pub struct InnerBubble {
    /// (front, back)
    pub textures: (OwnedTexture, OwnedTexture),
    pub transform: Matrix,
}
impl InnerBubble {
    pub fn new(text: &str) -> InnerBubble {
        InnerBubble {
            textures: create_textures(text),
            transform: Matrix::IDENTITY,
        }
    }

    pub fn update_transform(&mut self, position: Vec3, rotation: Vec3) {
        let height = self.textures.0.as_texture().Height;

        let ratio = BUBBLE_HEIGHT / height as f32;
        let scale = Vec3::create(ratio, ratio, 1.0);

        let translation = Matrix::translate(position.X, position.Y + 0.5, position.Z);
        let scale = Matrix::scale(scale.X, scale.Y, scale.Z);

        self.transform = scale
            * Matrix::rotate_z(180.0 * MATH_DEG2RAD as c_float)
            * Matrix::rotate_x(-rotation.X * MATH_DEG2RAD as c_float)
            * Matrix::rotate_y(-rotation.Y * MATH_DEG2RAD as c_float)
            * translation;
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
