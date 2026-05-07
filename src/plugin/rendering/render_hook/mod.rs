pub mod renderable;

use std::{cell::Cell, os::raw::c_float};

use classicube_sys::{ENTITIES_SELF_ID, Entities, Entity, EntityVTABLE};

thread_local!(
    static ORIGINAL_FN: Cell<Option<unsafe extern "C" fn(*mut Entity, c_float, c_float)>> =
        Default::default();
);

thread_local!(
    static ORIGINAL_VTABLE: Cell<Option<*const EntityVTABLE>> = const { Cell::new(None) };
);

thread_local!(
    static VTABLE: Cell<Option<Box<EntityVTABLE>>> = Default::default();
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

    // Capture the original VTABLE pointer (not just RenderModel) so free()
    // can restore it. Reading me.VTABLE on the next init() would otherwise
    // see our patched vtable and treat it as "original" — installing a hook
    // on top of a hook → infinite recursion on the next render frame.
    ORIGINAL_VTABLE.set(Some(me.VTABLE));
    ORIGINAL_FN.set(v_table.RenderModel);

    let new_v_table = Box::new(EntityVTABLE {
        Tick: v_table.Tick,
        Despawn: v_table.Despawn,
        SetLocation: v_table.SetLocation,
        GetCol: v_table.GetCol,
        RenderModel: Some(hook),
        ShouldRenderName: v_table.ShouldRenderName,
    });
    me.VTABLE = &*new_v_table;

    VTABLE.with(|cell| {
        cell.set(Some(new_v_table));
    });
}

pub fn free() {
    // Restore the original VTABLE pointer BEFORE dropping the hooked box,
    // otherwise the entity briefly points at freed memory.
    if let Some(original_vtable) = ORIGINAL_VTABLE.take() {
        unsafe {
            let entity_ptr = Entities.List[ENTITIES_SELF_ID as usize];
            if !entity_ptr.is_null() {
                (*entity_ptr).VTABLE = original_vtable;
            }
        }
    }
    VTABLE.with(|cell| {
        cell.take();
    });
    ORIGINAL_FN.set(None);
}
