#[cfg(target_os = "linux")]
use smithay_client_toolkit::shell::xdg::window::Window;
#[cfg(target_os = "linux")]
use wayland_client::Connection;

#[cfg(target_os = "linux")]
use crate::platform::native::wayland::WaylandWindowHandleSource;

/// Physical window or surface dimensions used for swapchain configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowDimension {
    pub width: u32,
    pub height: u32,
}

impl WindowDimension {
    /// Create a new physical size in pixels.
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// Platform surface bundle passed into renderer initialization.
pub struct SurfaceContext {
    pub target: wgpu::SurfaceTarget<'static>,
    pub size: WindowDimension,
}

impl SurfaceContext {
    /// Create a surface context from a target and current surface metrics.
    pub fn new(target: wgpu::SurfaceTarget<'static>, size: WindowDimension) -> Self {
        Self { target, size }
    }

    /// Create a surface context from a transferred `OffscreenCanvas`.
    #[cfg(target_arch = "wasm32")]
    pub fn from_offscreen_canvas(canvas: web_sys::OffscreenCanvas) -> Self {
        let width = canvas.width();
        let height = canvas.height();

        Self::new(
            wgpu::SurfaceTarget::OffscreenCanvas(canvas),
            WindowDimension::new(width, height),
        )
    }

    /// Create a surface context for an SCTK Wayland toplevel.
    #[cfg(target_os = "linux")]
    pub fn from_wayland_window(conn: Connection, window: Window, size: WindowDimension) -> Self {
        // We move the connection and window into the surface target so wgpu can
        // keep borrowing valid raw Wayland handles for as long as the surface
        // exists.
        Self::new(WaylandWindowHandleSource::new(conn, window).into(), size)
    }
}
