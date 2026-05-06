use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

pub trait Renderable {
    fn render(&mut self);
}

pub trait StartStopRendering {
    fn start_rendering(&self);
    fn stop_rendering(&self);
}

type Inner = Weak<RefCell<dyn Renderable>>;

thread_local!(
    static RENDERABLES: RefCell<Vec<Inner>> = Default::default();
);

fn with_renderables<R, F: FnOnce(&mut Vec<Inner>) -> R>(f: F) -> R {
    RENDERABLES.with_borrow_mut(|renderables| f(renderables))
}

impl<T> StartStopRendering for Rc<RefCell<T>>
where
    T: Renderable,
    T: 'static,
{
    fn start_rendering(&self) {
        // need to use cast here because ptr_eq will compare "fat pointers" which
        // will basically compare the inner type
        let weak = Rc::downgrade(self) as Weak<RefCell<dyn Renderable>>;
        with_renderables(move |renderables| {
            renderables.push(weak);
        });
    }

    fn stop_rendering(&self) {
        let weak = Rc::downgrade(self) as Weak<RefCell<dyn Renderable>>;
        with_renderables(move |renderables| {
            renderables.retain(move |other| !other.ptr_eq(&weak));
        });
    }
}

pub fn render_all() {
    with_renderables(|renderables| {
        renderables.retain(|renderable| {
            if let Some(renderable) = renderable.upgrade() {
                renderable.borrow_mut().render();
                true
            } else {
                false
            }
        })
    })
}

#[test]
fn test_renderable() {
    #[derive(Debug)]
    struct Struct {}
    impl Renderable for Struct {
        fn render(&mut self) {
            todo!()
        }
    }

    {
        let renderable = Rc::new(RefCell::new(Struct {}));
        with_renderables(|renderables| assert!(renderables.is_empty()));
        renderable.start_rendering();
        with_renderables(|renderables| assert!(!renderables.is_empty()));
        renderable.stop_rendering();
        with_renderables(|renderables| assert!(renderables.is_empty()));
    }

    // test weak cleanup
    {
        let renderable = Rc::new(RefCell::new(Struct {}));
        with_renderables(|renderables| assert!(renderables.is_empty()));
        renderable.start_rendering();
        with_renderables(|renderables| assert!(!renderables.is_empty()));

        drop(renderable);
        render_all();
        with_renderables(|renderables| assert!(renderables.is_empty()));
    }
}
