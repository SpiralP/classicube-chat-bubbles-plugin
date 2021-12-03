use classicube_helpers::WithBorrow;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputEvent {
    ChatOpened,
    ChatClosed,
    InputTextChanged(String),
}

pub trait InputEventListener {
    fn handle_event(&mut self, event: &InputEvent);
}

pub trait StartStopListening {
    fn start_listening(&self, entity_id: u8);
    fn stop_listening(&self);
}

type Inner = Weak<RefCell<dyn InputEventListener>>;

thread_local!(
    static EVENT_LISTENERS: RefCell<HashMap<u8, Vec<Inner>>> = Default::default();
);

fn with_all_listeners<R, F: FnOnce(&mut HashMap<u8, Vec<Inner>>) -> R>(f: F) -> R {
    EVENT_LISTENERS.with_borrow_mut(|listeners| f(listeners))
}

impl<T> StartStopListening for Rc<RefCell<T>>
where
    T: InputEventListener,
    T: 'static,
{
    fn start_listening(&self, entity_id: u8) {
        // need to use cast here because ptr_eq will compare "fat pointers" which
        // will basically compare the inner type
        let weak = Rc::downgrade(self) as Weak<RefCell<dyn InputEventListener>>;
        with_all_listeners(move |map| {
            if let Some(listeners) = map.get_mut(&entity_id) {
                listeners.push(weak);
            } else {
                map.insert(entity_id, vec![weak]);
            }
        })
    }

    fn stop_listening(&self) {
        let weak = Rc::downgrade(self) as Weak<RefCell<dyn InputEventListener>>;
        with_all_listeners(move |map| {
            for listeners in map.values_mut() {
                let weak = weak.clone();
                listeners.retain(move |other| !other.ptr_eq(&weak));
            }
        })
    }
}

pub fn emit_input_event(entity_id: u8, event: InputEvent) {
    with_all_listeners(|map| {
        if let Some(listeners) = map.get_mut(&entity_id) {
            listeners.retain(|listener| {
                if let Some(listener) = listener.upgrade() {
                    listener.borrow_mut().handle_event(&event);
                    true
                } else {
                    false
                }
            })
        }
    })
}
