#![allow(non_snake_case)]

use classicube_helpers::{WithBorrow, WithInner};
use classicube_sys::{
    Gfx_BindTexture, Gfx_Make2DQuad, Gfx_SetVertexFormat, Gfx_UpdateDynamicVb_IndexedTris,
    OwnedGfxVertexBuffer, PackedCol, Texture, VertexFormat__VERTEX_FORMAT_TEXTURED,
    PACKEDCOL_WHITE,
};
use std::cell::RefCell;
use tracing::warn;

thread_local!(
    static TEX_VB: RefCell<Option<OwnedGfxVertexBuffer>> = Default::default();
);

#[tracing::instrument(skip_all)]
unsafe fn Gfx_Draw2DTexture(tex: &mut Texture, col: PackedCol) {
    let mut vertices = Gfx_Make2DQuad(tex, col);

    Gfx_SetVertexFormat(VertexFormat__VERTEX_FORMAT_TEXTURED);
    TEX_VB
        .with_inner(|tex_vb| {
            Gfx_UpdateDynamicVb_IndexedTris(tex_vb.resource_id, vertices.as_mut_ptr() as _, 4);
        })
        .unwrap_or_else(|| {
            warn!("TEX_VB None");
        });
}

pub unsafe fn Texture_Render(tex: &mut Texture) {
    Gfx_BindTexture(tex.ID);
    Gfx_Draw2DTexture(tex, PACKEDCOL_WHITE);
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
