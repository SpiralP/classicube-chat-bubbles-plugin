#![allow(non_snake_case)]

use classicube_helpers::{WithBorrow, WithInner};
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

#[allow(clippy::missing_safety_doc)]
pub unsafe fn Texture_Render(tex: &mut Texture, front: bool) {
    Gfx_BindTexture(tex.ID);
    Gfx_Draw2DTexture(tex, PACKEDCOL_WHITE, front);
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
    let x1: f32 = tex.X as _;
    let x2: f32 = (tex.X as f32 + tex.Width as f32) as _;
    let y1: f32 = tex.Y as _;
    let y2: f32 = (tex.Y as f32 + tex.Height as f32) as _;

    if !clockwise {
        // counter-clockwise
        [
            VertexTextured {
                X: x1,
                Y: y1,
                Z: 0 as _,
                Col: color,
                U: tex.uv.U1,
                V: tex.uv.V1,
            },
            VertexTextured {
                X: x2,
                Y: y1,
                Z: 0 as _,
                Col: color,
                U: tex.uv.U2,
                V: tex.uv.V1,
            },
            VertexTextured {
                X: x2,
                Y: y2,
                Z: 0 as _,
                Col: color,
                U: tex.uv.U2,
                V: tex.uv.V2,
            },
            VertexTextured {
                X: x1,
                Y: y2,
                Z: 0 as _,
                Col: color,
                U: tex.uv.U1,
                V: tex.uv.V2,
            },
        ]
    } else {
        [
            VertexTextured {
                X: x1,
                Y: y1,
                Z: 0 as _,
                Col: color,
                U: tex.uv.U1,
                V: tex.uv.V1,
            },
            VertexTextured {
                X: x1,
                Y: y2,
                Z: 0 as _,
                Col: color,
                U: tex.uv.U1,
                V: tex.uv.V2,
            },
            VertexTextured {
                X: x2,
                Y: y2,
                Z: 0 as _,
                Col: color,
                U: tex.uv.U2,
                V: tex.uv.V2,
            },
            VertexTextured {
                X: x2,
                Y: y1,
                Z: 0 as _,
                Col: color,
                U: tex.uv.U2,
                V: tex.uv.V1,
            },
        ]
    }
}
