use core::fmt;

#[derive(Debug)]
pub enum WindowEvent {
    Resize(ResizeMessage),
    PointerMove(MouseMessage),
    PointerClick(MouseMessage),
}

// Display for WindowEvent
impl fmt::Display for WindowEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WindowEvent::Resize(msg) => write!(f, "Resize: {:?}", msg),
            WindowEvent::PointerMove(msg) => write!(f, "PointerMove: {:?}", msg),
            WindowEvent::PointerClick(msg) => write!(f, "PointerClick: {:?}", msg),
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
    pub client_x: f64,
    pub client_y: f64,
    pub movement_x: f64,
    pub movement_y: f64,
    pub offset_x: f64,
    pub offset_y: f64,
}

impl MouseMessage {
    pub fn from_evt(event: web_sys::MouseEvent) -> Self {
        let window = web_sys::window().unwrap();
        Self {
            scale_factor: window.device_pixel_ratio(),
            button: event.button() as f64,
            client_x: event.client_x() as f64,
            client_y: event.client_y() as f64,
            movement_x: event.movement_x() as f64,
            movement_y: event.movement_y() as f64,
            offset_x: event.offset_x() as f64,
            offset_y: event.offset_y() as f64,
        }
    }
}
