#[cfg(not(target_arch = "wasm32"))]
use wayland_client::{Connection, QueueHandle};

#[cfg(not(target_arch = "wasm32"))]
pub struct WaylandApp<State> {
    conn: Connection,
    queue_handle: QueueHandle<State>,
}

#[cfg(not(target_arch = "wasm32"))]
impl<State> WaylandApp<State> {}

#[cfg(target_arch = "wasm32")]
pub struct WaylandApp;
