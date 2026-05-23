use std::future::Future;

use crate::camera::CameraCommand;
use crate::events::{MeshData, SceneCommand};
use crate::renderer::SyncCommand;

/// Trait defining the interface for an application that can interact with the renderer.
///
/// This trait encapsulates the requirements from `level-editor/src/lib.rs` and provides
/// a clean interface for sending commands to the renderer without depending on
/// concrete implementation details.
///
/// Applications can use the `WebAppRuntime` struct (WASM) or implement this
/// trait directly for custom runtime behavior.
pub trait App {
    /// The error type returned by fallible operations on this app.
    type Error: std::error::Error;

    /// Send a sync command to the renderer thread.
    fn send_command<C>(&self, cmd: C) -> Result<(), Self::Error>
    where
        C: SyncCommand;

    /// Spawn async work outside the renderer thread and enqueue its result as a
    /// sync command once it is ready.
    #[cfg(target_arch = "wasm32")]
    fn spawn_async<F, C>(&self, future: F)
    where
        F: Future<Output = Option<C>> + 'static,
        C: SyncCommand;

    /// Native async work may hop to a background thread, so the future must be `Send`.
    #[cfg(not(target_arch = "wasm32"))]
    fn spawn_async<F, C>(&self, future: F)
    where
        F: Future<Output = Option<C>> + Send + 'static,
        C: SyncCommand;

    /// Add a mesh to the scene.
    fn add_mesh(&self, mesh: MeshData) -> Result<(), Self::Error> {
        self.send_command(SceneCommand::AddMesh(mesh))
    }

    /// Clear all meshes from the scene.
    fn clear_scene(&self) -> Result<(), Self::Error> {
        self.send_command(SceneCommand::Clear)
    }

    /// Set the camera position and look-at target.
    fn set_camera_look_at(&self, eye: [f32; 3], target: [f32; 3]) -> Result<(), Self::Error> {
        self.send_command(CameraCommand::LookAt { eye, target })
    }

    /// Orbit the camera by the given pixel deltas.
    fn orbit_camera(&self, dx: f32, dy: f32) -> Result<(), Self::Error> {
        self.send_command(CameraCommand::Orbit {
            delta_x: dx,
            delta_y: dy,
        })
    }

    /// Zoom the camera by the given delta (negative = zoom in, positive = zoom out).
    fn zoom_camera(&self, delta: f32) -> Result<(), Self::Error> {
        self.send_command(CameraCommand::Zoom { delta })
    }
}
