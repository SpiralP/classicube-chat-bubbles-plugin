#![allow(non_snake_case)]

use classicube_helpers::WithInner;
use classicube_sys::{
    Gfx_BindTexture, Gfx_SetVertexFormat, Gfx_UpdateDynamicVb_IndexedTris, OwnedGfxVertexBuffer,
    PackedCol, Texture, VertexFormat__VERTEX_FORMAT_TEXTURED, VertexTextured, PACKEDCOL_WHITE,
};
use std::cell::RefCell;
use tracing::warn;

thread_local!(
    static TEX_VB: RefCell<Option<OwnedGfxVertexBuffer>> = Default::default();
);

#[tracing::instrument(skip_all)]
unsafe fn Gfx_Draw2DTexture(tex: &mut Texture, col: PackedCol, front: bool) {
    unsafe {
        let mut vertices = Gfx_Make2DQuad(tex, col, front);

        Gfx_SetVertexFormat(VertexFormat__VERTEX_FORMAT_TEXTURED);
        TEX_VB
            .with_inner(|tex_vb| {
                Gfx_UpdateDynamicVb_IndexedTris(tex_vb.resource_id, vertices.as_mut_ptr() as _, 4);
            })
            .unwrap_or_else(|| {
                warn!("TEX_VB None");
            });
    }
}

#[allow(clippy::missing_safety_doc)]
pub unsafe fn Texture_Render(tex: &mut Texture, front: bool) {
    unsafe {
        Gfx_BindTexture(tex.ID);
        Gfx_Draw2DTexture(tex, PACKEDCOL_WHITE, front);
    }
}

pub fn context_recreated() {
    TEX_VB.with_borrow_mut(|tex_vb| {
        // create texture buffer
        *tex_vb = Some(OwnedGfxVertexBuffer::new(
            VertexFormat__VERTEX_FORMAT_TEXTURED,
            4,
        ));
    });
}

pub fn context_lost() {
    TEX_VB.with_borrow_mut(|tex_vb| drop(tex_vb.take()));
}

/// clockwise verts (for backface culling), differs from ClassiCube source!
pub fn Gfx_Make2DQuad(tex: &mut Texture, color: PackedCol, clockwise: bool) -> [VertexTextured; 4] {
    let x1: f32 = tex.x as _;
    let x2: f32 = (tex.x as f32 + tex.width as f32) as _;
    let y1: f32 = tex.y as _;
    let y2: f32 = (tex.y as f32 + tex.height as f32) as _;

    if !clockwise {
        // counter-clockwise
        [
            VertexTextured {
                x: x1,
                y: y1,
                z: 0 as _,
                Col: color,
                U: tex.uv.u1,
                V: tex.uv.v1,
            },
            VertexTextured {
                x: x2,
                y: y1,
                z: 0 as _,
                Col: color,
                U: tex.uv.u2,
                V: tex.uv.v1,
            },
            VertexTextured {
                x: x2,
                y: y2,
                z: 0 as _,
                Col: color,
                U: tex.uv.u2,
                V: tex.uv.v2,
            },
            VertexTextured {
                x: x1,
                y: y2,
                z: 0 as _,
                Col: color,
                U: tex.uv.u1,
                V: tex.uv.v2,
            },
        ]
    } else {
        [
            VertexTextured {
                x: x1,
                y: y1,
                z: 0 as _,
                Col: color,
                U: tex.uv.u1,
                V: tex.uv.v1,
            },
            VertexTextured {
                x: x1,
                y: y2,
                z: 0 as _,
                Col: color,
                U: tex.uv.u1,
                V: tex.uv.v2,
            },
            VertexTextured {
                x: x2,
                y: y2,
                z: 0 as _,
                Col: color,
                U: tex.uv.u2,
                V: tex.uv.v2,
            },
            VertexTextured {
                x: x2,
                y: y1,
                z: 0 as _,
                Col: color,
                U: tex.uv.u2,
                V: tex.uv.v1,
            },
        ]
    }
}
