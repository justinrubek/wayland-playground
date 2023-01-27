use wayland_client::{
    protocol::wl_registry,
    Connection, Dispatch, QueueHandle, globals::{GlobalListContents, registry_queue_init},
};

struct AppState;

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for AppState {
    fn event(
        _state: &mut Self,
        _: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        data: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<AppState>,
    ) {
        println!("data: {data:?}");
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for AppState {
    fn event(
        _state: &mut Self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _: &Connection,
        _: &QueueHandle<AppState>,
    ) {
        // print characteristics of `global` event
        if let wl_registry::Event::Global { name, interface, version } = event {
            println!("[{name}] {interface} (v{version})");
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let connection = Connection::connect_to_env()?;

    let display = connection.display();
    println!("Display: {display:?}");

    let (globals, mut queue) = registry_queue_init::<AppState>(&connection)?;
    let qh = queue.handle();

    let _registry = display.get_registry(&qh, ());

    println!("received globals:");
    queue.roundtrip(&mut AppState)?;

    println!("Globals: {:?}", globals.contents());
    // let compositor: wl_compositor::WlCompositor = globals.bind(&qh, 4..=5, ()).unwrap();

    Ok(())
}
