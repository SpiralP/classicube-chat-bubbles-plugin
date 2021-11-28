mod vertex_buffer;

pub use self::vertex_buffer::Texture_Render;
use classicube_helpers::{
    events::gfx::{ContextLostEventHandler, ContextRecreatedEventHandler},
    WithBorrow,
};
use std::cell::RefCell;

thread_local!(
    static CONTEXT_RECREATED_HANDLER: RefCell<Option<ContextRecreatedEventHandler>> =
        Default::default();
);
thread_local!(
    static CONTEXT_LOST_HANDLER: RefCell<Option<ContextLostEventHandler>> = Default::default();
);

pub fn initialize() {
    CONTEXT_RECREATED_HANDLER.with_borrow_mut(|option| {
        let mut handler = ContextRecreatedEventHandler::new();
        handler.on(|_| {
            vertex_buffer::context_recreated();
        });

        *option = Some(handler);
    });

    CONTEXT_LOST_HANDLER.with_borrow_mut(|option| {
        let mut handler = ContextLostEventHandler::new();
        handler.on(|_| {
            vertex_buffer::context_lost();
        });

        *option = Some(handler);
    });

    // start with context created
    vertex_buffer::context_recreated();
}

pub fn free() {
    vertex_buffer::context_lost();
}
