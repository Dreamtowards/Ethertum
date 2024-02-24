use bevy::prelude::*;

use ethertia::game::GamePlugin;

#[bevy_main]
fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            resizable: false,
            mode: bevy::window::WindowMode::BorderlessFullscreen,
            ..default()
        }),
        ..default()
    }))
    .add_plugins(GamePlugin);

    #[cfg(target_os = "android")]
    app.insert_resource(Msaa::Off);

    app.run();
}