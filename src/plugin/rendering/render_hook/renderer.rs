use classicube_helpers::WithBorrow;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

thread_local!(
    static RENDERERS: RefCell<Vec<Weak<RefCell<dyn Renderer>>>> = Default::default();
);

fn with_renderers<R, F: FnOnce(&mut Vec<Weak<RefCell<dyn Renderer>>>) -> R>(f: F) -> R {
    RENDERERS.with_borrow_mut(|renderers| f(renderers))
}

pub fn render_all() {
    with_renderers(|renderers| {
        renderers.retain(|renderer| {
            if let Some(renderer) = renderer.upgrade() {
                renderer.borrow_mut().render();
                true
            } else {
                false
            }
        })
    })
}

pub trait Renderer {
    fn render(&mut self);
}

pub trait StartStopRendering {
    fn start_rendering(&self);
    fn stop_rendering(&self);
}

impl<T> StartStopRendering for Rc<RefCell<T>>
where
    T: Renderer,
    T: Sized + 'static,
{
    fn start_rendering(&self) {
        let weak = Rc::downgrade(self);
        with_renderers(|renderers| {
            renderers.push(weak);
        });
    }

    fn stop_rendering(&self) {
        let weak = Rc::downgrade(self) as _;
        with_renderers(|renderers| {
            renderers.retain(|other| !other.ptr_eq(&weak));
        });
    }
}

#[test]
fn test_renderer() {
    struct Struct {}
    impl Renderer for Struct {
        fn render(&mut self) {
            todo!()
        }
    }

    {
        let renderer = Rc::new(RefCell::new(Struct {}));
        with_renderers(|renderers| assert!(renderers.is_empty()));
        renderer.start_rendering();
        with_renderers(|renderers| assert!(!renderers.is_empty()));
        renderer.stop_rendering();
        with_renderers(|renderers| assert!(renderers.is_empty()));
    }

    // test weak cleanup
    {
        let renderer = Rc::new(RefCell::new(Struct {}));
        with_renderers(|renderers| assert!(renderers.is_empty()));
        renderer.start_rendering();
        with_renderers(|renderers| assert!(!renderers.is_empty()));

        drop(renderer);
        render_all();
        with_renderers(|renderers| assert!(renderers.is_empty()));
    }
}
