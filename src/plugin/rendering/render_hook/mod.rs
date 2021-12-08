pub mod renderable;

use classicube_sys::{Entities, Entity, EntityVTABLE, ENTITIES_SELF_ID};
use std::{
    cell::Cell,
    os::raw::{c_double, c_float},
    pin::Pin,
};

thread_local!(
    static ORIGINAL_FN: Cell<Option<unsafe extern "C" fn(*mut Entity, c_double, c_float)>> =
        Default::default();
);

thread_local!(
    static VTABLE: Cell<Option<Pin<Box<EntityVTABLE>>>> = Default::default();
);

/// This is called when `LocalPlayer_RenderModel` is called.
extern "C" fn hook(local_player_entity: *mut Entity, delta: c_double, t: c_float) {
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
        RenderName: v_table.RenderName,
    });
    me.VTABLE = new_v_table.as_ref().get_ref();

    VTABLE.with(|cell| {
        cell.set(Some(new_v_table));
    });
}

pub fn free() {
    // self entity doesn't exist during free; no need to cleanup
}
