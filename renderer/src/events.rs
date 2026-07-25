use core::fmt;

use crate::renderer::{CommandContext, SyncCommand};

#[derive(Debug)]
pub enum WindowEvent {
    Sync(SyncWindowEvent),
}

#[derive(Debug)]
pub enum SyncWindowEvent {
    Resize(ResizeMessage),
    PointerMove(MouseMessage),
    PointerClick(MouseMessage),
    PointerWheel(WheelMessage),
    Keyboard(KeyboardMessage),
    AppCommand(Box<dyn SyncCommand>),
}

#[derive(Debug)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub model_matrix: [f32; 16],
    pub shader_source: String,
    pub pipeline_key: String,
}

#[derive(Debug)]
pub enum SceneCommand {
    Clear,
    AddMesh(MeshData),
}

impl SyncCommand for SceneCommand {
    fn run(self: Box<Self>, cx: &mut CommandContext<'_>) {
        match *self {
            SceneCommand::Clear => cx.scene.clear_meshes(),
            SceneCommand::AddMesh(mesh_data) => {
                cx.add_mesh(mesh_data);
            }
        }
    }
}

impl fmt::Display for WindowEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WindowEvent::Sync(evt) => write!(f, "Sync({:?})", evt),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResizeMessage {
    pub scale_factor: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone)]
pub struct MouseMessage {
    pub scale_factor: f64,
    pub button: f64,
    pub buttons: u16,
    pub client_x: f64,
    pub client_y: f64,
    pub movement_x: f64,
    pub movement_y: f64,
    pub offset_x: f64,
    pub offset_y: f64,
}

impl MouseMessage {
    #[cfg(target_arch = "wasm32")]
    pub fn from_evt(event: web_sys::MouseEvent) -> Self {
        let window = web_sys::window().unwrap();
        Self {
            scale_factor: window.device_pixel_ratio(),
            button: event.button() as f64,
            buttons: event.buttons(),
            client_x: event.client_x() as f64,
            client_y: event.client_y() as f64,
            movement_x: event.movement_x() as f64,
            movement_y: event.movement_y() as f64,
            offset_x: event.offset_x() as f64,
            offset_y: event.offset_y() as f64,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WheelMessage {
    pub scale_factor: f64,
    pub delta_x: f64,
    pub delta_y: f64,
    pub delta_z: f64,
    pub delta_mode: u32,
    pub client_x: f64,
    pub client_y: f64,
}

impl WheelMessage {
    #[cfg(target_arch = "wasm32")]
    pub fn from_evt(event: web_sys::WheelEvent) -> Self {
        let window = web_sys::window().unwrap();
        Self {
            scale_factor: window.device_pixel_ratio(),
            delta_x: event.delta_x(),
            delta_y: event.delta_y(),
            delta_z: event.delta_z(),
            delta_mode: event.delta_mode(),
            client_x: event.client_x() as f64,
            client_y: event.client_y() as f64,
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeyboardMessage {
    pub key: String,
    pub code: String,
    pub alt_key: bool,
    pub ctrl_key: bool,
    pub meta_key: bool,
    pub shift_key: bool,
    pub location: u32,
    pub repeat: bool,
}

impl KeyboardMessage {
    #[cfg(target_arch = "wasm32")]
    pub fn from_evt(event: web_sys::KeyboardEvent) -> Self {
        Self {
            key: event.key(),
            code: event.code(),
            alt_key: event.alt_key(),
            ctrl_key: event.ctrl_key(),
            meta_key: event.meta_key(),
            shift_key: event.shift_key(),
            location: event.location(),
            repeat: event.repeat(),
        }
    }
}
