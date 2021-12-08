use super::PlayerChatEvent;
use classicube_helpers::WithBorrow;
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

pub trait PlayerChatEventListener {
    fn handle_event(&mut self, event: &PlayerChatEvent);
}

pub trait StartStopListening {
    fn start_listening(&self, entity_id: u8);
    fn stop_listening(&self);
}

type Inner = Weak<RefCell<dyn PlayerChatEventListener>>;

thread_local!(
    static EVENT_LISTENERS: RefCell<HashMap<u8, Vec<Inner>>> = Default::default();
);

pub fn with_all_listeners<R, F: FnOnce(&mut HashMap<u8, Vec<Inner>>) -> R>(f: F) -> R {
    EVENT_LISTENERS.with_borrow_mut(|listeners| f(listeners))
}

impl<T> StartStopListening for Rc<RefCell<T>>
where
    T: PlayerChatEventListener,
    T: 'static,
{
    fn start_listening(&self, entity_id: u8) {
        // need to use cast here because ptr_eq will compare "fat pointers" which
        // will basically compare the inner type
        let weak = Rc::downgrade(self) as Weak<RefCell<dyn PlayerChatEventListener>>;
        with_all_listeners(move |map| {
            if let Some(listeners) = map.get_mut(&entity_id) {
                listeners.push(weak);
            } else {
                map.insert(entity_id, vec![weak]);
            }
        })
    }

    fn stop_listening(&self) {
        let weak = Rc::downgrade(self) as Weak<RefCell<dyn PlayerChatEventListener>>;
        with_all_listeners(move |map| {
            for listeners in map.values_mut() {
                let weak = weak.clone();
                listeners.retain(move |other| !other.ptr_eq(&weak));
            }
        })
    }
}
