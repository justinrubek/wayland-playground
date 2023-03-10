use std::time::Duration;

use smithay_client_toolkit::reexports::calloop::EventLoop;
use smithay_client_toolkit::reexports::client::{
    globals::registry_queue_init,
    Connection, WaylandSource,
};
use smithay_client_toolkit::shell::xdg::window::Window;
use smithay_client_toolkit::shm::slot::SlotPool;

mod error;
mod window;

use crate::error::AppResult;
use crate::window::SimpleWindow;

fn main() -> AppResult<()> {
    let connection = Connection::connect_to_env()?;

    let (globals, queue) = registry_queue_init(&connection)?;
    let qh = queue.handle();
    let mut event_loop = EventLoop::try_new().expect("Failed to create event loop");
    let loop_handle = event_loop.handle();
    WaylandSource::new(queue)
        .unwrap()
        .insert(loop_handle)
        .unwrap();

    println!("Globals: {:?}", globals.contents());
    // let compositor: wl_compositor::WlCompositor = globals.bind(&qh, 4..=5, ()).unwrap();
    let mut simple_window = SimpleWindow::init(
        &globals,
        &qh,
        event_loop.handle(),
    );

    let pool = SlotPool::new(
        simple_window.width as usize * simple_window.height as usize * 4,
        &simple_window.shm_state,
    ).expect("Failed to create pool");
    simple_window.pool = Some(pool);

    let surface = simple_window.compositor_state.create_surface(&qh);

    let window = Window::builder()
        .title("a window")
        .app_id("dev.rubek.experiments.wayland.SimpleWindow")
        .min_size((256, 256))
        .map(&qh, &simple_window.xdg_shell_state, &mut simple_window.xdg_window_state, surface)
        .expect("Failed to create window");

    simple_window.window = Some(window);

    loop {
        event_loop.dispatch(Duration::from_millis(16), &mut simple_window)?;

        if simple_window.exit {
            break;
        }
    }

    Ok(())
}
