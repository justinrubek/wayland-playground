use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_output, delegate_pointer, delegate_registry,
    delegate_seat, delegate_shm, delegate_xdg_shell, delegate_xdg_window,
    output::{OutputHandler, OutputState},
    reexports::calloop::LoopHandle,
    reexports::{
        client::{
            protocol::{wl_output, wl_seat, wl_shm, wl_surface},
            Connection,
        },
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Modifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::xdg::{
        window::{Window, WindowConfigure, WindowHandler, XdgWindowState},
        XdgShellState,
    },
    shm::ShmHandler,
    shm::{
        slot::{Buffer, SlotPool},
        ShmState,
    },
};
use wayland_client::{
    protocol::{wl_keyboard, wl_pointer},
    QueueHandle,
};

pub(crate) struct SimpleWindow {
    pub(crate) registry_state: RegistryState,
    pub(crate) seat_state: SeatState,
    pub(crate) output_state: OutputState,
    pub(crate) compositor_state: CompositorState,
    pub(crate) shm_state: ShmState,
    pub(crate) xdg_shell_state: XdgShellState,
    pub(crate) xdg_window_state: XdgWindowState,

    pub(crate) exit: bool,
    pub(crate) first_configure: bool,
    pub(crate) pool: Option<SlotPool>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) shift: Option<u32>,
    pub(crate) buffer: Option<Buffer>,
    pub(crate) window: Option<Window>,
    pub(crate) keyboard: Option<wl_keyboard::WlKeyboard>,
    pub(crate) keyboard_focus: bool,
    pub(crate) pointer: Option<wl_pointer::WlPointer>,
    pub(crate) loop_handle: LoopHandle<'static, SimpleWindow>,
}

impl SimpleWindow {
    pub fn draw(&mut self, _conn: &Connection, qh: &QueueHandle<Self>) {
        if let Some(window) = self.window.as_ref() {
            let width = self.width;
            let height = self.height;
            let stride = self.width as i32 * 4;
            let pool = self.pool.as_mut().unwrap();

            let buffer = self.buffer.get_or_insert_with(|| {
                pool.create_buffer(
                    width as i32,
                    height as i32,
                    stride,
                    wl_shm::Format::Argb8888,
                )
                .expect("create buffer")
                .0
            });

            let canvas = match pool.canvas(buffer) {
                Some(canvas) => canvas,
                None => {
                    // This should be rare, but if the compositor has not released the previous
                    // buffer, we need double-buffering.
                    let (second_buffer, canvas) = pool
                        .create_buffer(
                            self.width as i32,
                            self.height as i32,
                            stride,
                            wl_shm::Format::Argb8888,
                        )
                        .expect("create buffer");
                    *buffer = second_buffer;
                    canvas
                }
            };

            // Draw to the window:
            {
                let shift = self.shift.unwrap_or(0);
                canvas
                    .chunks_exact_mut(4)
                    .enumerate()
                    .for_each(|(index, chunk)| {
                        let x = ((index + shift as usize) % width as usize) as u32;
                        let y = (index / width as usize) as u32;

                        let a = 0xFF;
                        let r =
                            u32::min(((width - x) * 0xFF) / width, ((height - y) * 0xFF) / height);
                        let g = u32::min((x * 0xFF) / width, ((height - y) * 0xFF) / height);
                        let b = u32::min(((width - x) * 0xFF) / width, (y * 0xFF) / height);
                        let color = (a << 24) + (r << 16) + (g << 8) + b;

                        let array: &mut [u8; 4] = chunk.try_into().unwrap();
                        *array = color.to_le_bytes();
                    });

                if let Some(shift) = &mut self.shift {
                    *shift = (*shift + 1) % width;
                }
            }

            // Damage the entire window
            window
                .wl_surface()
                .damage_buffer(0, 0, self.width as i32, self.height as i32);

            // Request our next frame
            window.wl_surface().frame(qh, window.wl_surface().clone());

            // Attach and commit to present.
            buffer
                .attach_to(window.wl_surface())
                .expect("buffer attach");
            window.wl_surface().commit();
        }
    }
}

impl CompositorHandler for SimpleWindow {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
        // Not needed for this example.
    }

    fn frame(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.draw(conn, qh);
    }
}

impl OutputHandler for SimpleWindow {
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

impl ShmHandler for SimpleWindow {
    fn shm_state(&mut self) -> &mut ShmState {
        &mut self.shm_state
    }
}

impl SeatHandler for SimpleWindow {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            println!("Set keyboard capability");
            let (keyboard, source) = self
                .seat_state
                .get_keyboard_with_repeat(qh, &seat, None)
                .expect("Failed to create keyboard");
            self.loop_handle
                .insert_source(source, |e, _, _state| {
                    dbg!(e);
                })
                .expect("Failed to insert the repeating keyboard into the event loop");
            self.keyboard = Some(keyboard);
        }

        if capability == Capability::Pointer && self.pointer.is_none() {
            println!("Set pointer capability");
            let pointer = self
                .seat_state
                .get_pointer(qh, &seat)
                .expect("Failed to create pointer");
            self.pointer = Some(pointer);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_some() {
            println!("Unset keyboard capability");
            self.keyboard.take().unwrap().release();
        }

        if capability == Capability::Pointer && self.pointer.is_some() {
            println!("Unset pointer capability");
            self.pointer.take().unwrap().release();
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl KeyboardHandler for SimpleWindow {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        keysyms: &[u32],
    ) {
        if self.window.as_ref().map(Window::wl_surface) == Some(surface) {
            println!("Keyboard focus on window with pressed syms: {keysyms:?}");
            self.keyboard_focus = true;
        }
    }

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _: u32,
    ) {
        if self.window.as_ref().map(Window::wl_surface) == Some(surface) {
            println!("Release keyboard focus on window");
            self.keyboard_focus = false;
        }
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        println!("Key press: {event:?}");
    }

    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        println!("Key release: {event:?}");
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _serial: u32,
        modifiers: Modifiers,
    ) {
        println!("Update modifiers: {modifiers:?}");
    }
}

impl PointerHandler for SimpleWindow {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        use PointerEventKind::*;
        for event in events {
            // Ignore events for other surfaces
            if Some(&event.surface) != self.window.as_ref().map(Window::wl_surface) {
                continue;
            }

            match event.kind {
                Enter { .. } => {
                    println!("Pointer entered @{:?}", event.position);
                }
                Leave { .. } => {
                    println!("Pointer left");
                }
                Motion { .. } => {}
                Press { button, .. } => {
                    println!("Press {:x} @ {:?}", button, event.position);
                    self.shift = self.shift.xor(Some(0));
                }
                Release { button, .. } => {
                    println!("Release {:x} @ {:?}", button, event.position);
                }
                Axis {
                    horizontal,
                    vertical,
                    ..
                } => {
                    println!("Scroll H:{horizontal:?}, V:{vertical:?}");
                }
            }
        }
    }
}

impl WindowHandler for SimpleWindow {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &Window) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        _window: &Window,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        match configure.new_size {
            Some(size) => {
                self.width = size.0;
                self.height = size.1;
                self.buffer = None;
            }
            None => {
                self.width = 256;
                self.height = 256;
                self.buffer = None;
            }
        }

        // Initiate the first draw.
        if self.first_configure {
            self.first_configure = false;
            self.draw(conn, qh);
        }
    }
}

delegate_compositor!(SimpleWindow);
delegate_output!(SimpleWindow);
delegate_shm!(SimpleWindow);

delegate_seat!(SimpleWindow);
delegate_keyboard!(SimpleWindow);
delegate_pointer!(SimpleWindow);

delegate_xdg_shell!(SimpleWindow);
delegate_xdg_window!(SimpleWindow);

delegate_registry!(SimpleWindow);

impl ProvidesRegistryState for SimpleWindow {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState,];
}
