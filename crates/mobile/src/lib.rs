use bevy::prelude::*;

#[bevy_main]
fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            resizable: false,
            mode: bevy::window::WindowMode::BorderlessFullscreen(bevy::window::MonitorSelection::Primary),
            ..default()
        }),
        ..default()
    }))
    .add_plugins(ethertia::client::prelude::ClientGamePlugin);

    // MSAA is configured on the startup camera by ClientGamePlugin.

    app.run();
}
