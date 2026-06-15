pub mod renderable;

use std::{cell::Cell, ffi::c_void};

use classicube_sys::{
    Game, Gfx, Gfx_LoadMatrix, Gfx_SetAlphaBlending, Gfx_SetAlphaTest, Gfx_SetDepthTest,
    Gfx_SetDepthWrite, Matrix, MatrixType__MATRIX_PROJ, MatrixType__MATRIX_VIEW, OwnedScreen,
    screen::Priority,
};

/// Mirror ClassiCube's per-backend `Gfx_CalcOrthoMatrix`, picking the formula
/// at compile time. `Matrix::orthographic` is GL-flavored (clip-space z `[-1, 1]`)
/// — feeding it to D3D9/D3D11 (clip-space z `[0, 1]`) puts every 2D vertex
/// outside the clip range and the rasterizer culls the entire HUD.
///
/// `Gfx_CalcOrthoMatrix` itself is not `CC_API` and so isn't exported from
/// `ClassiCube.dll` on Windows, so we replicate it here.
fn calc_ortho_matrix(width: f32, height: f32, z_near: f32, z_far: f32) -> Matrix {
    let mut m = Matrix::IDENTITY;
    m.row1.x = 2.0 / width;
    m.row2.y = -2.0 / height;

    if cfg!(target_os = "windows") {
        // D3D9 / D3D11: clip-space z is [0, 1]; D3D9 also wants a half-pixel
        // x/y nudge. Mirrors `Graphics_D3D9.c:756` (z math is identical to
        // `Graphics_D3D11.c:507`; the half-pixel nudge is harmless on D3D11).
        let adjust_x = 0.5 * (2.0 / width);
        let adjust_y = 0.5 * (-2.0 / height);
        m.row3.z = 1.0 / (z_near - z_far);
        m.row4.x = -1.0 - adjust_x;
        m.row4.y = 1.0 - adjust_y;
        m.row4.z = z_near / (z_near - z_far);
    } else {
        // GL clip-space z is [-1, 1]. Mirrors `_GLShared.h:289`.
        m.row3.z = -2.0 / (z_far - z_near);
        m.row4.x = -1.0;
        m.row4.y = 1.0;
        m.row4.z = -(z_far + z_near) / (z_far - z_near);
    }
    m
}

/// Called from `Gui_RenderGui` between `Gfx_Begin2D` and the HUD screen's
/// render. Switch to 3D-style state, draw bubbles, then restore 2D state so
/// the HUD (and any later screens) see the state they expect.
unsafe extern "C" fn render(_: *mut c_void, _: f32) {
    crate::plugin::events::local_presence::poll();
    unsafe {
        Gfx_SetDepthTest(1);
        // Depth-write OFF: bubbles are translucent quads. Leaving depth-write
        // on would have each bubble's depth occlude later-drawn bubbles at the
        // same depth, causing z-fighting when stacks momentarily overlap (e.g.
        // during the spawn-ease-up tween). With depth-write off, later bubbles
        // always overdraw earlier ones — matching the iteration order
        // (oldest → newest), so the newest message stays visually on top.
        Gfx_SetDepthWrite(0);
        Gfx_SetAlphaBlending(0);
        Gfx_LoadMatrix(MatrixType__MATRIX_PROJ, &raw const Gfx.Projection);

        renderable::render_all();

        // Reconstruct the 2D ortho the engine's `Gfx_Begin2D` had loaded.
        // Must use a backend-correct formula: `Matrix::orthographic` is
        // GL-only and breaks clip-space culling on D3D9/D3D11.
        let width = Game.Width as f32;
        let height = Game.Height as f32;
        let ortho = calc_ortho_matrix(width, height, -100.0, 1000.0);
        Gfx_LoadMatrix(MatrixType__MATRIX_PROJ, &ortho);
        Gfx_LoadMatrix(MatrixType__MATRIX_VIEW, &Matrix::IDENTITY);

        Gfx_SetAlphaBlending(1);
        Gfx_SetDepthWrite(0);
        Gfx_SetDepthTest(0);
        // Defensive: ClassiCube's convention (matching `EntityNames_Render`)
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
