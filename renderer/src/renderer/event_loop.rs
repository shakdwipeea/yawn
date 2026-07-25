/// Event loop driver and dispatch for the renderer.
///
/// Contains the per-frame `tick` that drains the event channel, coalesces
/// pointer-move events, dispatches sync commands, and finally calls `render`.
/// All handler methods live here to keep `mod.rs` focused on GPU resource
/// management and frame submission.
#[cfg(target_arch = "wasm32")]
use std::{cell::RefCell, rc::Rc};

use log::info;

use crate::{
    events::{MouseMessage, ResizeMessage, SyncWindowEvent, WheelMessage, WindowEvent},
    renderer::{surface::WindowDimension, CommandContext, Renderer},
};

impl Renderer {
    #[cfg(target_arch = "wasm32")]
    pub fn tick(
        renderer: &Rc<RefCell<Renderer>>,
        events_chan: &std::sync::mpsc::Receiver<WindowEvent>,
        time: f32,
    ) {
        if let Ok(mut r) = renderer.try_borrow_mut() {
            let mut events = Vec::new();
            while let Ok(event) = events_chan.try_recv() {
                events.push(event);
            }

            let mut coalesced_move: Option<MouseMessage> = None;

            for event in events {
                match event {
                    WindowEvent::Sync(SyncWindowEvent::PointerMove(msg)) => {
                        if let Some(ref mut prev) = coalesced_move {
                            prev.movement_x += msg.movement_x;
                            prev.movement_y += msg.movement_y;
                            prev.client_x = msg.client_x;
                            prev.client_y = msg.client_y;
                            prev.offset_x = msg.offset_x;
                            prev.offset_y = msg.offset_y;
                            prev.buttons = msg.buttons;
                        } else {
                            coalesced_move = Some(msg);
                        }
                    }
                    other => r.handle_event(other),
                }
            }

            if let Some(msg) = coalesced_move {
                let evt = WindowEvent::Sync(SyncWindowEvent::PointerMove(msg));
                r.handle_event(evt);
            }

            r.render(time);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn tick_native(&mut self, events_chan: &std::sync::mpsc::Receiver<WindowEvent>, time: f32) {
        let mut events = Vec::new();
        while let Ok(event) = events_chan.try_recv() {
            events.push(event);
        }

        let mut coalesced_move: Option<MouseMessage> = None;

        for event in events {
            match event {
                WindowEvent::Sync(SyncWindowEvent::PointerMove(msg)) => {
                    if let Some(ref mut prev) = coalesced_move {
                        prev.movement_x += msg.movement_x;
                        prev.movement_y += msg.movement_y;
                        prev.client_x = msg.client_x;
                        prev.client_y = msg.client_y;
                        prev.offset_x = msg.offset_x;
                        prev.offset_y = msg.offset_y;
                        prev.buttons = msg.buttons;
                    } else {
                        coalesced_move = Some(msg);
                    }
                }
                other => self.handle_event(other),
            }
        }

        if let Some(msg) = coalesced_move {
            let evt = WindowEvent::Sync(SyncWindowEvent::PointerMove(msg));
            self.handle_event(evt);
        }

        self.render(time);
    }

    fn handle_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::Sync(sync_event) => {
                self.handle_sync_event(sync_event);
            }
        }
    }

    fn handle_sync_event(&mut self, event: SyncWindowEvent) {
        match event {
            SyncWindowEvent::PointerMove(msg) => {
                self.mouse_move(msg);
            }
            SyncWindowEvent::Resize(msg) => {
                self.resize(msg);
            }
            SyncWindowEvent::PointerClick(msg) => {
                let x = (msg.offset_x * msg.scale_factor) as f32;
                let y = (msg.offset_y * msg.scale_factor) as f32;
                self.scene.frame_metadata.mouse_click = [x, y];
                log::info!("clicked");
            }
            SyncWindowEvent::PointerWheel(msg) => {
                let wheel_msg = WheelMessage {
                    scale_factor: 1.0,
                    delta_x: 0.0,
                    delta_y: msg.delta_y as f64,
                    delta_z: 0.0,
                    delta_mode: 1, // DOM_DELTA_LINE
                    client_x: 0.0,
                    client_y: 0.0,
                };
                self.scene.cam.zoom(&wheel_msg);
            }
            SyncWindowEvent::Keyboard(msg) => {
                log::info!("Key event received: {:?}", msg);
            }
            SyncWindowEvent::AppCommand(cmd) => {
                let (scene, resources, context) =
                    (&mut self.scene, &mut self.resources, &self.context);
                let mut cx = CommandContext::new(scene, resources, context);
                cmd.run(&mut cx);
            }
        }
    }

    fn resize(&mut self, msg: ResizeMessage) {
        let new_width = (msg.width * msg.scale_factor) as u32;
        let new_height = (msg.height * msg.scale_factor) as u32;
        if new_width != self.surface_size.width || new_height != self.surface_size.height {
            self.surface_size = WindowDimension::new(new_width, new_height);
            self.context.surface_config.width = new_width;
            self.context.surface_config.height = new_height;
            self.context
                .surface
                .configure(&self.context.device, &self.context.surface_config);
            self.recreate_depth_texture();

            self.scene.resize(
                new_width as f64,
                new_height as f64,
                msg.scale_factor,
                &self.context.queue,
            );

            info!(
                "Resized: ({}, {}), scale: {}",
                new_width, new_height, msg.scale_factor
            );
        }
    }

    pub fn mouse_move(&mut self, msg: MouseMessage) {
        if (msg.buttons & 0x04) != 0 {
            let delta_x = (msg.movement_x * msg.scale_factor) as f32;
            let delta_y = (msg.movement_y * msg.scale_factor) as f32;
            self.scene.cam.orbit(delta_x, delta_y);
        }
    }
}
