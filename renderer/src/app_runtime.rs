use std::future::Future;
use std::sync::mpsc::{Receiver, Sender};

#[cfg(target_os = "linux")]
use std::ptr::NonNull;
#[cfg(target_os = "linux")]
use std::time::{Duration, Instant};
#[cfg(target_arch = "wasm32")]
use std::{cell::RefCell, rc::Rc};

#[cfg(target_os = "linux")]
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle, WindowHandle,
};
#[cfg(target_os = "linux")]
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_output, delegate_registry, delegate_xdg_shell,
    delegate_xdg_window,
    output::{OutputHandler, OutputState},
    reexports::{calloop::EventLoop, calloop_wayland_source::WaylandSource},
    registry::{ProvidesRegistryState, RegistryState as SctkRegistryState},
    registry_handlers,
    shell::{
        xdg::{
            window::{Window, WindowConfigure, WindowDecorations, WindowHandler},
            XdgShell,
        },
        WaylandSurface,
    },
};
#[cfg(target_os = "linux")]
use wayland_client::{
    globals::{registry_queue_init, GlobalError},
    protocol::{wl_output, wl_surface},
    ConnectError, Connection, EventQueue, QueueHandle,
};

#[cfg(target_arch = "wasm32")]
use std::sync::mpsc;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;
#[cfg(target_arch = "wasm32")]
use web_sys::{AddEventListenerOptions, DedicatedWorkerGlobalScope};

use crate::{
    app::App,
    events::{SyncWindowEvent, WindowEvent},
    renderer::SyncCommand,
    task,
};

#[cfg(target_os = "linux")]
use crate::events::ResizeMessage;
#[cfg(target_arch = "wasm32")]
use crate::platform::web;
#[cfg(target_arch = "wasm32")]
use crate::platform::web::worker::{wait_for_canvas_transfer, MainWorker};
#[cfg(any(target_arch = "wasm32", target_os = "linux"))]
use crate::renderer::surface::SurfaceContext;
#[cfg(target_os = "linux")]
use crate::renderer::surface::WindowDimension;
#[cfg(any(target_arch = "wasm32", target_os = "linux"))]
use crate::renderer::Renderer;

#[cfg(target_os = "linux")]
use wayland_client::Proxy;

#[cfg(target_arch = "wasm32")]
fn init_platform() {
    static LOGGER_INIT: std::sync::Once = std::sync::Once::new();

    console_error_panic_hook::set_once();
    LOGGER_INIT.call_once(|| wasm_logger::init(wasm_logger::Config::default()));
}

/// Helper struct to store event listener closures.
#[cfg(target_arch = "wasm32")]
#[derive(Default)]
pub struct EventListeners {
    pub resize_listener: Option<Closure<dyn FnMut()>>,
    pub mousemove_listener: Option<Closure<dyn FnMut(web_sys::MouseEvent)>>,
    pub mousedown_listener: Option<Closure<dyn FnMut(web_sys::MouseEvent)>>,
    pub wheel_listener: Option<Closure<dyn FnMut(web_sys::WheelEvent)>>,
    pub keyboard_listener: Option<Closure<dyn FnMut(web_sys::KeyboardEvent)>>,
}

/// Setup default window event listeners that forward events to the worker thread.
#[cfg(target_arch = "wasm32")]
fn setup_event_listeners(worker_chan: &Sender<WindowEvent>) -> Result<EventListeners, JsValue> {
    let window = web_sys::window().unwrap();
    let resize_worker_chan = worker_chan.clone();

    let resize_listener: Closure<dyn FnMut()> = Closure::new(move || {
        use crate::events::ResizeMessage;

        let window = web_sys::window().unwrap();
        let width = window.inner_width().ok().unwrap().as_f64().unwrap();
        let height = window.inner_height().ok().unwrap().as_f64().unwrap();

        resize_worker_chan
            .send(WindowEvent::Sync(SyncWindowEvent::Resize(ResizeMessage {
                width,
                height,
                scale_factor: window.device_pixel_ratio(),
            })))
            .unwrap();
    });

    window.add_event_listener_with_callback("resize", resize_listener.as_ref().unchecked_ref())?;

    let mousemove_worker_chan = worker_chan.clone();
    let mousemove_listener: Closure<dyn FnMut(web_sys::MouseEvent)> =
        Closure::new(move |event: web_sys::MouseEvent| {
            use crate::events::MouseMessage;
            if event.buttons() & 0x04 != 0 {
                event.prevent_default();
            }
            let mouse_event_data = MouseMessage::from_evt(event.clone());

            let mut event_data =
                WindowEvent::Sync(SyncWindowEvent::PointerMove(mouse_event_data.clone()));
            if event.type_() == "click" {
                event_data =
                    WindowEvent::Sync(SyncWindowEvent::PointerClick(mouse_event_data.clone()));
            }

            mousemove_worker_chan.clone().send(event_data).unwrap();
        });

    window.add_event_listener_with_callback(
        "mousemove",
        mousemove_listener.as_ref().unchecked_ref(),
    )?;

    window
        .add_event_listener_with_callback("click", mousemove_listener.as_ref().unchecked_ref())?;

    let mousedown_listener: Closure<dyn FnMut(web_sys::MouseEvent)> =
        Closure::new(move |event: web_sys::MouseEvent| {
            if event.button() == 1 {
                event.prevent_default();
            }
        });

    window.add_event_listener_with_callback(
        "mousedown",
        mousedown_listener.as_ref().unchecked_ref(),
    )?;

    let wheel_worker_chan = worker_chan.clone();
    let wheel_listener: Closure<dyn FnMut(web_sys::WheelEvent)> =
        Closure::new(move |event: web_sys::WheelEvent| {
            use crate::events::WheelMessage;

            event.prevent_default();
            let wheel_event_data = WheelMessage::from_evt(event);

            wheel_worker_chan
                .send(WindowEvent::Sync(SyncWindowEvent::PointerWheel(
                    wheel_event_data,
                )))
                .unwrap();
        });

    let wheel_options = {
        let options = AddEventListenerOptions::new();
        options.set_passive(false);
        options
    };

    window.add_event_listener_with_callback_and_add_event_listener_options(
        "wheel",
        wheel_listener.as_ref().unchecked_ref(),
        &wheel_options,
    )?;

    let keyboard_worker_chan = worker_chan.clone();
    let keyboard_listener: Closure<dyn FnMut(web_sys::KeyboardEvent)> =
        Closure::new(move |event: web_sys::KeyboardEvent| {
            use crate::events::KeyboardMessage;

            let keyboard_event_data = KeyboardMessage::from_evt(event);

            keyboard_worker_chan
                .send(WindowEvent::Sync(SyncWindowEvent::Keyboard(
                    keyboard_event_data,
                )))
                .unwrap();
        });

    window
        .add_event_listener_with_callback("keydown", keyboard_listener.as_ref().unchecked_ref())?;

    Ok(EventListeners {
        resize_listener: Some(resize_listener),
        mousemove_listener: Some(mousemove_listener),
        mousedown_listener: Some(mousedown_listener),
        wheel_listener: Some(wheel_listener),
        keyboard_listener: Some(keyboard_listener),
    })
}

/// Web-specific runtime that manages the renderer worker thread and event loop.
///
/// This struct handles:
/// - Web worker lifecycle (spawning, canvas transfer)
/// - Window event listener setup (resize, mouse, wheel, keyboard)
/// - Animation frame loop
/// - Sync command dispatch and async task result delivery
///
/// It implements the [`App`] trait to provide a platform-agnostic interface
/// for application code.
pub struct WebAppRuntime {
    sender: Sender<WindowEvent>,
    #[cfg(target_arch = "wasm32")]
    _worker: MainWorker,
    #[cfg(target_arch = "wasm32")]
    _event_listeners: EventListeners,
}

impl WebAppRuntime {
    #[cfg(target_arch = "wasm32")]
    fn start_loop(renderer: &Rc<RefCell<Renderer>>, events_chan: Rc<Receiver<WindowEvent>>) {
        let renderer = renderer.clone();

        // use request animation frame for the loop
        let render_frame: Closure<dyn FnMut(f32)> = Closure::new(move |time: f32| {
            Renderer::tick(&renderer, events_chan.as_ref(), time);

            Self::start_loop(&renderer, events_chan.clone());
        });

        let global = js_sys::global().unchecked_into::<DedicatedWorkerGlobalScope>();

        global
            .request_animation_frame(render_frame.as_ref().unchecked_ref())
            .unwrap();

        render_frame.forget();
    }

    #[cfg(target_arch = "wasm32")]
    /// Initialize the web worker, canvas ownership, and event listeners.
    pub fn new(worker_name: &str, canvas_selector: &str) -> Result<Self, JsValue> {
        init_platform();

        let (sender, receiver) = mpsc::channel::<WindowEvent>();

        let canvas = web::get_canvas_element(canvas_selector);
        let worker = MainWorker::spawn(worker_name, 1, move || {
            spawn_local(async move {
                let canvas = wait_for_canvas_transfer().await;
                let surface_context = SurfaceContext::from_offscreen_canvas(canvas);
                let renderer = Renderer::new(surface_context).await;
                let renderer = Rc::new(RefCell::new(renderer));
                let receiver = Rc::new(receiver);
                Self::start_loop(&renderer, receiver);
            });
        })?;

        worker.transfer_ownership(&canvas);

        let event_listeners = setup_event_listeners(&sender)?;

        Ok(Self {
            _worker: worker,
            sender,
            _event_listeners: event_listeners,
        })
    }
}

impl App for WebAppRuntime {
    type Error = std::sync::mpsc::SendError<WindowEvent>;

    fn send_command<C>(&self, cmd: C) -> Result<(), Self::Error>
    where
        C: SyncCommand,
    {
        self.sender
            .send(WindowEvent::Sync(SyncWindowEvent::AppCommand(Box::new(
                cmd,
            ))))
    }

    #[cfg(target_arch = "wasm32")]
    fn spawn_async<F, C>(&self, future: F)
    where
        F: Future<Output = Option<C>> + 'static,
        C: SyncCommand,
    {
        let sender = self.sender.clone();
        task::spawn(async move {
            if let Some(command) = future.await {
                let _ = sender.send(WindowEvent::Sync(SyncWindowEvent::AppCommand(Box::new(
                    command,
                ))));
            }
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn spawn_async<F, C>(&self, future: F)
    where
        F: Future<Output = Option<C>> + Send + 'static,
        C: SyncCommand,
    {
        let sender = self.sender.clone();
        task::spawn(async move {
            if let Some(command) = future.await {
                let _ = sender.send(WindowEvent::Sync(SyncWindowEvent::AppCommand(Box::new(
                    command,
                ))));
            }
        });
    }
}

#[cfg(target_os = "linux")]
use thiserror::Error;

#[cfg(target_os = "linux")]
#[derive(Error, Debug)]
pub enum WaylandError {
    #[error("failed to send Wayland window event")]
    Send(#[from] std::sync::mpsc::SendError<WindowEvent>),
    #[error("could not connect to wayland environment")]
    Connection(#[from] ConnectError),
    #[error("failed to initialize Wayland globals")]
    GlobalRegistry(#[from] GlobalError),
    #[error("Wayland protocol error")]
    Protocol(#[from] wayland_client::backend::WaylandError),
    #[error("Wayland event loop error: {0}")]
    EventLoop(String),
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
struct WaylandDispatchState {
    /// SCTK's registry helpers own the global cache used by the delegate macros.
    registry_state: SctkRegistryState,
    /// Surface enter/leave events are routed through `OutputHandler`, even when
    /// this runtime only needs the toplevel for wgpu surface creation.
    output_state: OutputState,
    /// Lets Wayland configure events resize the renderer through the same
    /// command path used by other platform events.
    sender: Sender<WindowEvent>,
    /// Set by the compositor close request so the native loop can terminate.
    exit: bool,
    /// Integer buffer scale of the output the surface currently lives on.
    ///
    /// Updated whenever the compositor calls
    /// `CompositorHandler::scale_factor_changed` (e.g. when the window enters a
    /// HiDPI output). Defaults to 1 — the protocol-level default — so a
    /// compositor that never reports a scale still produces a 1:1 buffer.
    current_scale: i32,
    /// Most recent logical-pixel size reported by `xdg_toplevel.configure`.
    ///
    /// Cached so a later `scale_factor_changed` can re-emit a `Resize` event
    /// with the correct physical buffer dimensions without waiting for the
    /// compositor to send a fresh configure. Initialised to the toplevel's
    /// initial size so the very first resize event is well-defined.
    last_logical_size: (u32, u32),
}

#[cfg(target_os = "linux")]
impl CompositorHandler for WaylandDispatchState {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        new_factor: i32,
    ) {
        // Ignore redundant notifications; the compositor may re-emit the same
        // scale when the surface re-enters an equivalent output.
        if new_factor == self.current_scale {
            return;
        }
        self.current_scale = new_factor;

        // Tell the compositor that subsequent buffers are supplied at this
        // integer scale. Without this, a HiDPI compositor would upscale the
        // logical-sized buffer to physical pixels and produce a blurry image.
        surface.set_buffer_scale(new_factor);

        // Re-emit a Resize so the renderer reconfigures its swapchain to the
        // new physical-pixel dimensions. We pair the cached logical size with
        // the freshly received scale so the dispatcher computes
        // `logical × scale` consistently with the configure path.
        let (width, height) = self.last_logical_size;
        let _ = self
            .sender
            .send(WindowEvent::Sync(SyncWindowEvent::Resize(ResizeMessage {
                width: width as f64,
                height: height as f64,
                scale_factor: new_factor as f64,
            })));
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

#[cfg(target_os = "linux")]
impl OutputHandler for WaylandDispatchState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

#[cfg(target_os = "linux")]
impl WindowHandler for WaylandDispatchState {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &Window) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _window: &Window,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        // Treat a missing size from the compositor as "keep current size":
        // some compositors send 0 for the unconstrained dimension on the
        // initial configure and expect the client to pick a sensible value.
        // Falling back to `last_logical_size` preserves a stable baseline
        // instead of collapsing the swapchain to zero.
        let width = configure
            .new_size
            .0
            .map(|v| v.get())
            .unwrap_or(self.last_logical_size.0);
        let height = configure
            .new_size
            .1
            .map(|v| v.get())
            .unwrap_or(self.last_logical_size.1);
        self.last_logical_size = (width, height);

        // The scale_factor here is the *current* integer buffer scale (set by
        // `CompositorHandler::scale_factor_changed`). Carrying it through the
        // Resize message lets `Renderer::resize` compute the physical buffer
        // size as `logical × scale`, matching what we promised the compositor
        // via `set_buffer_scale`.
        let _ = self
            .sender
            .send(WindowEvent::Sync(SyncWindowEvent::Resize(ResizeMessage {
                width: width as f64,
                height: height as f64,
                scale_factor: self.current_scale as f64,
            })));
    }
}

#[cfg(target_os = "linux")]
impl ProvidesRegistryState for WaylandDispatchState {
    fn registry(&mut self) -> &mut SctkRegistryState {
        &mut self.registry_state
    }

    registry_handlers![OutputState];
}

#[cfg(target_os = "linux")]
delegate_compositor!(WaylandDispatchState);
#[cfg(target_os = "linux")]
delegate_output!(WaylandDispatchState);
#[cfg(target_os = "linux")]
delegate_registry!(WaylandDispatchState);
#[cfg(target_os = "linux")]
delegate_xdg_shell!(WaylandDispatchState);
#[cfg(target_os = "linux")]
delegate_xdg_window!(WaylandDispatchState);

#[cfg(target_os = "linux")]
pub struct WaylandAppRuntime {
    sender: Sender<WindowEvent>,
    receiver: Receiver<WindowEvent>,
    conn: Connection,
    window: Window,
    event_queue: EventQueue<WaylandDispatchState>,
    dispatch_state: WaylandDispatchState,
}

#[cfg(target_os = "linux")]
const INITIAL_SIZE: u32 = 320;

#[cfg(target_os = "linux")]
impl WaylandAppRuntime {
    /// Store the Wayland objects that back the wgpu surface so raw-window-handle
    /// borrows stay valid for as long as the runtime exists.
    pub fn new() -> Result<Self, WaylandError> {
        let conn = Connection::connect_to_env()?;
        let (globals, event_queue) = registry_queue_init::<WaylandDispatchState>(&conn)?;
        let qh = event_queue.handle();

        let (sender, receiver) = std::sync::mpsc::channel::<WindowEvent>();

        // Wire up the SCTK delegates so the window, compositor, and registry
        // objects all agree on the state type used for dispatch. The scale and
        // size fields start at neutral values; the compositor will overwrite
        // them via `scale_factor_changed` and `configure` once the surface is
        // mapped.
        let dispatch_state = WaylandDispatchState {
            registry_state: SctkRegistryState::new(&globals),
            output_state: OutputState::new(&globals, &qh),
            sender: sender.clone(),
            exit: false,
            current_scale: 1,
            last_logical_size: (INITIAL_SIZE, INITIAL_SIZE),
        };

        let compositor = CompositorState::bind(&globals, &qh).expect("wl_compositor not available");
        let xdg_shell = XdgShell::bind(&globals, &qh).expect("xdg_wm_base not available");

        let surface = compositor.create_surface(&qh);
        let window = xdg_shell.create_window(surface, WindowDecorations::RequestServer, &qh);
        window.set_title("renderer sctk window");
        window.set_app_id("dev.yawn.renderer.sctk-window");
        window.set_min_size(Some((INITIAL_SIZE, INITIAL_SIZE)));
        window.commit();

        Ok(Self {
            sender,
            receiver,
            conn,
            window,
            event_queue,
            dispatch_state,
        })
    }

    /// Drive the native Wayland event loop and render frames until the window
    /// receives a compositor close request.
    pub fn run(self) -> Result<(), WaylandError> {
        let Self {
            sender: _,
            receiver,
            conn,
            window,
            event_queue,
            mut dispatch_state,
        } = self;

        let surface_context = SurfaceContext::from_wayland_window(
            conn.clone(),
            window.clone(),
            WindowDimension::new(INITIAL_SIZE, INITIAL_SIZE),
        );
        let mut renderer = futures::executor::block_on(Renderer::new(surface_context));

        let mut event_loop: EventLoop<WaylandDispatchState> =
            EventLoop::try_new().map_err(|err| WaylandError::EventLoop(err.to_string()))?;
        WaylandSource::new(conn, event_queue)
            .insert(event_loop.handle())
            .map_err(|err| WaylandError::EventLoop(err.to_string()))?;

        let start = Instant::now();
        while !dispatch_state.exit {
            event_loop
                .dispatch(Duration::from_millis(16), &mut dispatch_state)
                .map_err(|err| WaylandError::EventLoop(err.to_string()))?;
            renderer.tick_native(&receiver, start.elapsed().as_secs_f32() * 1000.0);
        }

        Ok(())
    }
}

#[cfg(target_os = "linux")]
impl HasDisplayHandle for WaylandAppRuntime {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        // WGPU only needs a borrowed `wl_display` pointer here. Keeping the
        // connection inside the runtime ensures that pointer outlives the
        // returned handle.
        let display = NonNull::new(self.conn.backend().display_ptr() as *mut _)
            .ok_or(HandleError::Unavailable)?;
        let raw = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(display));

        Ok(unsafe { DisplayHandle::borrow_raw(raw) })
    }
}

#[cfg(target_os = "linux")]
impl HasWindowHandle for WaylandAppRuntime {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // The wl_surface is owned by the SCTK window. Borrowing the raw surface
        // handle from it lets `wgpu::SurfaceTarget` use safe surface creation.
        let surface = NonNull::new(self.window.wl_surface().id().as_ptr() as *mut _)
            .ok_or(HandleError::Unavailable)?;
        let raw = RawWindowHandle::Wayland(WaylandWindowHandle::new(surface));

        Ok(unsafe { WindowHandle::borrow_raw(raw) })
    }
}

#[cfg(target_os = "linux")]
impl App for WaylandAppRuntime {
    type Error = WaylandError;

    fn send_command<C>(&self, cmd: C) -> Result<(), Self::Error>
    where
        C: SyncCommand,
    {
        self.sender
            .send(WindowEvent::Sync(SyncWindowEvent::AppCommand(Box::new(
                cmd,
            ))))
            .map_err(Into::into)
    }

    fn spawn_async<F, C>(&self, future: F)
    where
        F: Future<Output = Option<C>> + Send + 'static,
        C: SyncCommand,
    {
        let sender = self.sender.clone();
        task::spawn(async move {
            if let Some(command) = future.await {
                let _ = sender.send(WindowEvent::Sync(SyncWindowEvent::AppCommand(Box::new(
                    command,
                ))));
            }
        });
    }
}
