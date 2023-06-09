use std::time::Duration;

use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::reexports::calloop::EventLoop;
use smithay_client_toolkit::reexports::client::{
    globals::registry_queue_init, Connection, WaylandSource,
};
use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell};
use smithay_client_toolkit::shell::xdg::window::WindowDecorations;
use smithay_client_toolkit::shell::xdg::XdgShell;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shm::slot::SlotPool;
use smithay_client_toolkit::shm::Shm;

mod error;
mod layer;
mod window;

use crate::error::AppResult;
use crate::layer::SimpleLayer;
use crate::window::SimpleWindow;

#[allow(dead_code)]
fn simple_window() -> AppResult<()> {
    let connection = Connection::connect_to_env()?;

    let (globals, queue) = registry_queue_init(&connection)?;
    let qh = queue.handle();
    let mut event_loop = EventLoop::<SimpleWindow>::try_new().expect("Failed to create event loop");
    let loop_handle = event_loop.handle();
    WaylandSource::new(queue)
        .unwrap()
        .insert(loop_handle)
        .unwrap();

    let compositor = CompositorState::bind(&globals, &qh).expect("wl_compositor not available");
    let xdg_shell = XdgShell::bind(&globals, &qh).expect("xdg shell not available");
    let shm = Shm::bind(&globals, &qh).expect("wl_shm not available");

    let surface = compositor.create_surface(&qh);
    let window = xdg_shell.create_window(surface, WindowDecorations::RequestServer, &qh);

    window.set_title("a wayland window");
    window.set_app_id("dev.rubek.experiments.wayland.SimpleWindow");
    window.set_min_size(Some((256, 256)));

    window.commit();

    let pool = SlotPool::new(256 * 256 * 4, &shm).expect("Failed to create pool");
    let mut simple_window =
        SimpleWindow::init(&globals, &qh, event_loop.handle(), shm, pool, window);

    loop {
        event_loop.dispatch(Duration::from_millis(16), &mut simple_window)?;

        if simple_window.exit {
            break;
        }
    }

    Ok(())
}

fn simple_layer() -> AppResult<()> {
    let connection = Connection::connect_to_env()?;

    let (globals, mut queue) = registry_queue_init(&connection)?;
    let qh = queue.handle();

    let compositor = CompositorState::bind(&globals, &qh).expect("wl_compositor not available");

    let layer_shell = LayerShell::bind(&globals, &qh).expect("layer shell not available");

    let shm = Shm::bind(&globals, &qh).expect("wl_shm not available");

    let surface = compositor.create_surface(&qh);

    let layer =
        layer_shell.create_layer_surface(&qh, surface, Layer::Top, Some("simple_layer"), None);

    layer.set_anchor(Anchor::LEFT | Anchor::TOP | Anchor::RIGHT);
    layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
    layer.set_exclusive_zone(32);
    layer.set_size(2560, 32);

    layer.commit();

    let pool = SlotPool::new(256 * 256 * 4, &shm).expect("Failed to create pool");

    let mut simple_layer = SimpleLayer::init(&globals, &qh, shm, pool, layer);

    loop {
        queue.blocking_dispatch(&mut simple_layer)?;

        if simple_layer.exit {
            break;
        }
    }

    Ok(())
}

fn main() -> AppResult<()> {
    tracing_subscriber::fmt::init();

    // simple_window()
    simple_layer()
}
