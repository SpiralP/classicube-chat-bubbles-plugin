mod events;
mod rendering;

use tracing::debug;

pub fn initialize() {
    debug!("plugin initialize");

    rendering::initialize();
    events::initialize();
}

pub fn on_new_map() {
    debug!("plugin on_new_map");
}

pub fn on_new_map_loaded() {
    debug!("plugin on_new_map_loaded");
}

pub fn reset() {
    debug!("plugin reset");
}

pub fn free() {
    debug!("plugin free");

    events::free();
    rendering::free();
}
