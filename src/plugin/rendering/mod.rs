mod render_hook;

use self::render_hook::{Renderer, StartStopRendering};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug)]
struct Bubble {
    player_id: usize,
}

impl Renderer for Bubble {
    fn render(&mut self) {
        todo!()
    }
}

pub fn initialize() {
    render_hook::initialize();

    let bubble = Rc::new(RefCell::new(Bubble { player_id: 0 }));
    bubble.start_rendering();
}
