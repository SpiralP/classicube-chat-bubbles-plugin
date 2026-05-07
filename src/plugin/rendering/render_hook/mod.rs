pub mod renderable;

use std::{cell::Cell, ffi::c_void};

use classicube_sys::{
    Game, Gfx, Gfx_LoadMatrix, Gfx_SetAlphaBlending, Gfx_SetAlphaTest, Gfx_SetDepthTest,
    Gfx_SetDepthWrite, Matrix, MatrixType__MATRIX_PROJ, MatrixType__MATRIX_VIEW, OwnedScreen,
    screen::Priority,
};

/// Called from `Gui_RenderGui` between `Gfx_Begin2D` and the HUD screen's
/// render. Switch to 3D-style state, draw bubbles, then restore 2D state so
/// the HUD (and any later screens) see the state they expect.
unsafe extern "C" fn render(_: *mut c_void, _: f32) {
    unsafe {
        Gfx_SetDepthTest(1);
        Gfx_SetDepthWrite(1);
        Gfx_SetAlphaBlending(0);
        Gfx_LoadMatrix(MatrixType__MATRIX_PROJ, &raw const Gfx.Projection);

        renderable::render_all();

        // Reconstruct the 2D ortho the engine's `Gfx_Begin2D` had loaded.
        // `Matrix::orthographic(0, w, 0, h, -100, 1000)` matches the GL backend's
        // `Gfx_CalcOrthoMatrix`. Other backends (D3D9/Soft) use a slightly
        // different z mapping, but depth test is off in 2D mode so the z
        // difference has no visible effect.
        let width = Game.Width as f32;
        let height = Game.Height as f32;
        let ortho = Matrix::orthographic(0.0, width, 0.0, height, -100.0, 1000.0);
        Gfx_LoadMatrix(MatrixType__MATRIX_PROJ, &ortho);
        Gfx_LoadMatrix(MatrixType__MATRIX_VIEW, &Matrix::IDENTITY);

        Gfx_SetAlphaBlending(1);
        Gfx_SetDepthWrite(0);
        Gfx_SetDepthTest(0);
        // `bubble::render_inner` enables alpha-test for the bubble texture but
        // doesn't reset it. ClassiCube's convention (matching `EntityNames_Render`)
        // is to leave alpha-test off; otherwise translucent HUD gradients (chat
        // backdrop, escape menu backdrop) get their <128-alpha pixels discarded.
        Gfx_SetAlphaTest(0);
    }
}

thread_local!(
    static SCREEN: Cell<Option<OwnedScreen>> = const { Cell::new(None) };
);

pub fn initialize() {
    let mut screen = OwnedScreen::new();
    screen.on_render(render);
    // Render below the HUD (priority 10) so the chatbox / hotbar / crosshair
    // still draw on top of bubbles. Above us nameplates already happened in the
    // 3D phase, so bubbles land above nameplates and under the HUD.
    screen.add(Priority::UnderHud);
    SCREEN.set(Some(screen));
}

pub fn free() {
    // Dropping the OwnedScreen calls Gui_Remove and frees the screen + vtable boxes.
    SCREEN.take();
}
