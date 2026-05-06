pub mod renderable;

use std::{cell::Cell, os::raw::c_float, pin::Pin};

use classicube_sys::{ENTITIES_SELF_ID, Entities, Entity, EntityVTABLE};

thread_local!(
    static ORIGINAL_FN: Cell<Option<unsafe extern "C" fn(*mut Entity, c_float, c_float)>> =
        Default::default();
);

thread_local!(
    static VTABLE: Cell<Option<Pin<Box<EntityVTABLE>>>> = Default::default();
);

/// This is called when `LocalPlayer_RenderModel` is called.
extern "C" fn hook(local_player_entity: *mut Entity, delta: c_float, t: c_float) {
    ORIGINAL_FN.with(|cell| {
        if let Some(f) = cell.get() {
            unsafe {
                f(local_player_entity, delta, t);
            }
        }
    });

    renderable::render_all();
}

pub fn initialize() {
    let me = unsafe { &mut *Entities.List[ENTITIES_SELF_ID as usize] };
    let v_table = unsafe { &*me.VTABLE };

    ORIGINAL_FN.with(|cell| {
        cell.set(v_table.RenderModel);
    });

    let new_v_table = Box::pin(EntityVTABLE {
        Tick: v_table.Tick,
        Despawn: v_table.Despawn,
        SetLocation: v_table.SetLocation,
        GetCol: v_table.GetCol,
        RenderModel: Some(hook),
        ShouldRenderName: v_table.ShouldRenderName,
    });
    me.VTABLE = new_v_table.as_ref().get_ref();

    VTABLE.with(|cell| {
        cell.set(Some(new_v_table));
    });
}

pub fn free() {
    // self entity doesn't exist during free; no need to cleanup
}
