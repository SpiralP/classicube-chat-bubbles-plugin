use std::os::raw::c_float;

use classicube_helpers::entities::Entity;
use classicube_sys::{MATH_DEG2RAD, Matrix, OwnedTexture, Vec3};
use tracing::warn;

use super::helpers::{create_textures, get_transform};

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
        let height = self.textures.0.as_texture().height;

        let ratio = BUBBLE_HEIGHT / height as f32;
        let scale = Vec3::create(ratio, ratio, 1.0);

        let translation = Matrix::translate(position.x, position.y + 0.5, position.z);
        let scale = Matrix::scale(scale.x, scale.y, scale.z);

        self.transform = scale
            * Matrix::rotate_z(180.0 * MATH_DEG2RAD as c_float)
            * Matrix::rotate_x(-rotation.x * MATH_DEG2RAD as c_float)
            * Matrix::rotate_y(-rotation.y * MATH_DEG2RAD as c_float)
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
