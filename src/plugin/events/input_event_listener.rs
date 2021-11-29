use classicube_helpers::WithBorrow;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

pub enum InputEvent {
    ChatOpened,
    ChatClosed,
    InputTextChanged(String),
}

pub trait InputEventListener {
    fn handle_event(&mut self, event: &InputEvent);
}

pub trait StartStopListening {
    fn start_listening(&self);
    fn stop_listening(&self);
}

type Inner = Weak<RefCell<dyn InputEventListener>>;

thread_local!(
    static EVENT_LISTENERS: RefCell<Vec<Inner>> = Default::default();
);

fn with_listeners<R, F: FnOnce(&mut Vec<Inner>) -> R>(f: F) -> R {
    EVENT_LISTENERS.with_borrow_mut(|listeners| f(listeners))
}

impl<T> StartStopListening for Rc<RefCell<T>>
where
    T: InputEventListener,
    T: 'static,
{
    fn start_listening(&self) {
        // need to use cast here because ptr_eq will compare "fat pointers" which
        // will basically compare the inner type
        let weak = Rc::downgrade(self) as Weak<RefCell<dyn InputEventListener>>;
        with_listeners(move |listeners| {
            listeners.push(weak);
        });
    }

    fn stop_listening(&self) {
        let weak = Rc::downgrade(self) as Weak<RefCell<dyn InputEventListener>>;
        with_listeners(move |listeners| {
            listeners.retain(move |other| !other.ptr_eq(&weak));
        });
    }
}

pub fn emit_input_event(event: InputEvent) {
    with_listeners(|listeners| {
        listeners.retain(|listener| {
            if let Some(listener) = listener.upgrade() {
                listener.borrow_mut().handle_event(&event);
                true
            } else {
                false
            }
        })
    })
}
