#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use std::{
        convert::TryInto,
        time::{Duration, Instant},
    };

    use smithay_client_toolkit::reexports::calloop::EventLoop;
    use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
    use smithay_client_toolkit::{
        compositor::{CompositorHandler, CompositorState},
        delegate_compositor, delegate_output, delegate_registry, delegate_shm, delegate_xdg_shell,
        delegate_xdg_window,
        output::{OutputHandler, OutputState},
        registry::{ProvidesRegistryState, RegistryState},
        registry_handlers,
        shell::{
            xdg::{
                window::{Window, WindowConfigure, WindowDecorations, WindowHandler},
                XdgShell,
            },
            WaylandSurface,
        },
        shm::{
            slot::{Buffer, SlotPool},
            Shm, ShmHandler,
        },
    };
    use wayland_client::{
        globals::registry_queue_init,
        protocol::{wl_output, wl_shm, wl_surface},
        Connection, QueueHandle,
    };

    const INITIAL_SIZE: u32 = 320;
    const WINDOW_LIFETIME: Duration = Duration::from_secs(2);

    pub fn main() {
        let conn =
            Connection::connect_to_env().expect("failed to connect to the Wayland compositor");
        let (globals, event_queue) =
            registry_queue_init(&conn).expect("failed to initialize Wayland globals");
        let qh = event_queue.handle();

        let mut event_loop: EventLoop<App> =
            EventLoop::try_new().expect("failed to create event loop");
        WaylandSource::new(conn.clone(), event_queue)
            .insert(event_loop.handle())
            .expect("failed to insert Wayland source");

        let compositor = CompositorState::bind(&globals, &qh).expect("wl_compositor not available");
        let xdg_shell = XdgShell::bind(&globals, &qh).expect("xdg_wm_base not available");
        let shm = Shm::bind(&globals, &qh).expect("wl_shm not available");
        let pool = SlotPool::new((INITIAL_SIZE * INITIAL_SIZE * 4) as usize, &shm)
            .expect("failed to create shm pool");

        let surface = compositor.create_surface(&qh);
        let window = xdg_shell.create_window(surface, WindowDecorations::RequestServer, &qh);
        window.set_title("renderer sctk window");
        window.set_app_id("dev.yawn.renderer.sctk-window");
        window.set_min_size(Some((INITIAL_SIZE, INITIAL_SIZE)));
        window.commit();

        let mut app = App {
            registry_state: RegistryState::new(&globals),
            output_state: OutputState::new(&globals, &qh),
            shm,
            pool,
            buffer: None,
            width: INITIAL_SIZE,
            height: INITIAL_SIZE,
            exit: false,
            window,
        };

        let deadline = Instant::now() + WINDOW_LIFETIME;
        while !app.exit && Instant::now() < deadline {
            event_loop
                .dispatch(Duration::from_millis(50), &mut app)
                .expect("failed to dispatch Wayland events");
        }
    }

    struct App {
        registry_state: RegistryState,
        output_state: OutputState,
        shm: Shm,
        pool: SlotPool,
        buffer: Option<Buffer>,
        width: u32,
        height: u32,
        exit: bool,
        window: Window,
    }

    impl App {
        fn draw(&mut self) {
            let width = self.width;
            let height = self.height;
            let stride = self.width as i32 * 4;

            let buffer = self.buffer.get_or_insert_with(|| {
                self.pool
                    .create_buffer(
                        width as i32,
                        height as i32,
                        stride,
                        wl_shm::Format::Argb8888,
                    )
                    .expect("failed to create shared-memory buffer")
                    .0
            });

            let canvas = match self.pool.canvas(buffer) {
                Some(canvas) => canvas,
                None => {
                    let (next_buffer, canvas) = self
                        .pool
                        .create_buffer(
                            width as i32,
                            height as i32,
                            stride,
                            wl_shm::Format::Argb8888,
                        )
                        .expect("failed to create replacement shared-memory buffer");
                    *buffer = next_buffer;
                    canvas
                }
            };

            canvas
                .chunks_exact_mut(4)
                .enumerate()
                .for_each(|(index, pixel)| {
                    let x = (index % width as usize) as u32;
                    let y = (index / width as usize) as u32;
                    let r = ((x * 0xFF) / width) as u8;
                    let g = ((y * 0xFF) / height) as u8;
                    let b = 0x90;
                    let a = 0xFF;
                    let bytes: &mut [u8; 4] = pixel.try_into().expect("pixel should be four bytes");
                    *bytes = [b, g, r, a];
                });

            self.window
                .wl_surface()
                .damage_buffer(0, 0, self.width as i32, self.height as i32);
            buffer
                .attach_to(self.window.wl_surface())
                .expect("failed to attach buffer to surface");
            self.window.commit();
        }
    }

    impl CompositorHandler for App {
        fn scale_factor_changed(
            &mut self,
            _conn: &Connection,
            _qh: &QueueHandle<Self>,
            _surface: &wl_surface::WlSurface,
            _new_factor: i32,
        ) {
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

    impl OutputHandler for App {
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

    impl WindowHandler for App {
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
            self.buffer = None;
            self.width = configure
                .new_size
                .0
                .map(|value| value.get())
                .unwrap_or(INITIAL_SIZE);
            self.height = configure
                .new_size
                .1
                .map(|value| value.get())
                .unwrap_or(INITIAL_SIZE);

            self.draw();
        }
    }

    impl ShmHandler for App {
        fn shm_state(&mut self) -> &mut Shm {
            &mut self.shm
        }
    }

    impl ProvidesRegistryState for App {
        fn registry(&mut self) -> &mut RegistryState {
            &mut self.registry_state
        }

        registry_handlers![OutputState];
    }

    delegate_compositor!(App);
    delegate_output!(App);
    delegate_registry!(App);
    delegate_shm!(App);
    delegate_xdg_shell!(App);
    delegate_xdg_window!(App);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    imp::main();
}
