use raw_window_handle::{HasRawWindowHandle, HasRawDisplayHandle, RawWindowHandle, RawDisplayHandle, WaylandDisplayHandle, WaylandWindowHandle};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::seat::SeatState;
use smithay_client_toolkit::shell::xdg::XdgShellState;
use smithay_client_toolkit::reexports::client::{
    globals::registry_queue_init,
    Connection,
    Proxy, QueueHandle,
};
use smithay_client_toolkit::shell::xdg::window::{Window, XdgWindowState};

mod error;
mod wgpu_app;

use crate::error::AppResult;
use crate::wgpu_app::Wgpu;

fn main() -> AppResult<()> {
    let connection = Connection::connect_to_env()?;

    let (globals, mut event_queue) = registry_queue_init(&connection)?;
    let qh: QueueHandle<Wgpu> = event_queue.handle();

    let compositor_state = CompositorState::bind(&globals, &qh)?;
    let xdg_shell_state = XdgShellState::bind(&globals, &qh)?;
    let mut xdg_window_state = XdgWindowState::bind(&globals, &qh);

    let surface = compositor_state.create_surface(&qh);

    let window = Window::builder()
        .title("WGPU wayland example")
        .app_id("dev.rubek.experiments.wayland.Wgpu")
        .min_size((256, 256))
        .map(&qh, &xdg_shell_state, &mut xdg_window_state, surface)
        .expect("Failed to create window");

    let instance = wgpu::Instance::new(wgpu::Backends::all());

    let handle = {
        let mut window_handle = WaylandWindowHandle::empty();
        window_handle.surface = window.wl_surface().id().as_ptr() as *mut _;
        let mut display_handle = WaylandDisplayHandle::empty();
        display_handle.display = connection.backend().display_ptr() as *mut _;

        let window_handle = RawWindowHandle::Wayland(window_handle);
        let display_handle = RawDisplayHandle::Wayland(display_handle);

        /// https://github.com/rust-windowing/raw-window-handle/issues/49
        struct YesRawWindowHandleImplementingHasRawWindowHandleIsUnsound(RawWindowHandle, RawDisplayHandle);

        unsafe impl HasRawWindowHandle for YesRawWindowHandleImplementingHasRawWindowHandleIsUnsound {
            fn raw_window_handle(&self) -> RawWindowHandle {
                self.0
            }
        }

        unsafe impl HasRawDisplayHandle for YesRawWindowHandleImplementingHasRawWindowHandleIsUnsound {
            fn raw_display_handle(&self) -> RawDisplayHandle {
                self.1
            }
        }

        YesRawWindowHandleImplementingHasRawWindowHandleIsUnsound(window_handle, display_handle)
    };

    let surface = unsafe { instance.create_surface(&handle) };

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: Some(&surface),
        ..Default::default()
    })).expect("Failed to find an appropriate adapter");

    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default(), None))
        .expect("Failed to request device");

    let mut wgpu = Wgpu {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),

        exit: false,
        width: 0,
        height: 0,
        window,
        device,
        surface,
        adapter,
        queue,
    };

    loop {
        event_queue.blocking_dispatch(&mut wgpu).unwrap();

        if wgpu.exit {
            break;
        }
    }

    drop(wgpu.surface);
    drop(wgpu.window);

    Ok(())
}
