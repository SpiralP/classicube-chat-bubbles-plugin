mod bubble;
mod context;
mod render_hook;

use self::{bubble::Bubble, render_hook::StartStopRendering};
use crate::plugin::events::StartStopListening;
use classicube_helpers::{
    entities::{Entities, Entity},
    WithBorrow,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};
use tracing::debug;

thread_local!(
    static ENTITIES: RefCell<Option<Entities>> = Default::default();
);

thread_local!(
    static BUBBLES: RefCell<HashMap<u8, Rc<RefCell<Bubble>>>> = Default::default();
);

pub fn initialize() {
    context::initialize();
    render_hook::initialize();

    ENTITIES.with_borrow_mut(|option| {
        let mut entities = Entities::new();

        fn add(id: u8, entity: Weak<Entity>) {
            debug!(?id, "add");

            BUBBLES.with_borrow_mut(move |map| {
                let bubble = Rc::new(RefCell::new(Bubble::new(entity)));
                bubble.start_rendering();
                bubble.start_listening(id);
                map.insert(id, bubble);
            });
        }

        entities.on_added(|(id, entity)| {
            add(*id, entity.clone());
        });

        entities.on_removed(|id| {
            debug!(?id, "remove");

            BUBBLES.with_borrow_mut(move |map| {
                if let Some(bubble) = map.remove(id) {
                    bubble.stop_rendering();
                }
            });
        });

        for (id, entity) in entities.get_all() {
            add(id, entity);
        }

        *option = Some(entities);
    });
}

pub fn free() {
    BUBBLES.with_borrow_mut(|map| {
        for o in map.drain() {
            drop(o)
        }
    });

    render_hook::free();
    context::free();
}
