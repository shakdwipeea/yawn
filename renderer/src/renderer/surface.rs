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
}
