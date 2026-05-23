#[cfg(target_os = "linux")]
use std::ptr::NonNull;

#[cfg(target_os = "linux")]
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle, WindowHandle,
};
#[cfg(target_os = "linux")]
use smithay_client_toolkit::shell::{xdg::window::Window, WaylandSurface};
#[cfg(target_os = "linux")]
use wayland_client::{Connection, Proxy};

/// Owns the Wayland objects that wgpu must keep alive for the entire lifetime
/// of the surface.
///
/// SCTK does not expose `raw-window-handle` traits on its `Window`, so we wrap
/// the connection and surface together and borrow the raw `wl_display` and
/// `wl_surface` pointers on demand. This mirrors the manual raw-handle setup in
/// `renderer/src/bin/sctk_window.rs`, but packages it behind the safe traits
/// that `wgpu::Instance::create_surface` expects.
#[cfg(target_os = "linux")]
#[derive(Debug, Clone)]
pub struct WaylandWindowHandleSource {
    conn: Connection,
    window: Window,
}

#[cfg(target_os = "linux")]
impl WaylandWindowHandleSource {
    /// Build a new handle source from the live Wayland connection and SCTK
    /// toplevel window.
    pub fn new(conn: Connection, window: Window) -> Self {
        Self { conn, window }
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    pub fn window(&self) -> &Window {
        &self.window
    }
}

#[cfg(target_os = "linux")]
impl HasDisplayHandle for WaylandWindowHandleSource {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        let display = NonNull::new(self.conn.backend().display_ptr() as *mut _)
            .ok_or(HandleError::Unavailable)?;
        let raw = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(display));

        // SAFETY: `display_ptr` comes from the live `Connection` owned by `self`,
        // so the borrowed handle cannot outlive the underlying `wl_display`.
        Ok(unsafe { DisplayHandle::borrow_raw(raw) })
    }
}

#[cfg(target_os = "linux")]
impl HasWindowHandle for WaylandWindowHandleSource {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let surface = NonNull::new(self.window.wl_surface().id().as_ptr() as *mut _)
            .ok_or(HandleError::Unavailable)?;
        let raw = RawWindowHandle::Wayland(WaylandWindowHandle::new(surface));

        // SAFETY: the `Window` owns the Wayland surface proxy for at least as
        // long as `self` is alive, so the borrowed handle remains valid for the
        // returned lifetime.
        Ok(unsafe { WindowHandle::borrow_raw(raw) })
    }
}

#[cfg(not(target_os = "linux"))]
pub struct WaylandWindowHandleSource;
