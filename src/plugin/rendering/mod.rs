mod bubble;
mod context;
mod render_hook;

use self::{bubble::Bubble, render_hook::StartStopRendering};
use classicube_helpers::{entities::ENTITY_SELF_ID, WithBorrow};
use std::{cell::RefCell, rc::Rc};

thread_local!(
    static THING: RefCell<Option<Rc<RefCell<Bubble>>>> = Default::default();
);

pub fn initialize() {
    context::initialize();
    render_hook::initialize();

    THING.with_borrow_mut(|option| {
        let bubble = Rc::new(RefCell::new(Bubble::new(ENTITY_SELF_ID)));
        bubble.start_rendering();
        *option = Some(bubble);
    });
}

pub fn free() {
    THING.with_borrow_mut(|option| {
        drop(option.take());
    });

    render_hook::free();
    context::free();
}
