// #![rustfmt::skip]

use bevy::prelude::*;

fn main() {
    // std::env::set_var("RUST_BACKTRACE", "full");
    // std::env::set_var("RUST_LOG", "info");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: bevy::window::WindowResolution::new(1280., 720.),
                title: format!("Ethertia {} Items", ethertia::VERSION_NAME),
                prevent_default_event_handling: true, // web: avoid twice esc to pause problem.
                ..default()
            }),
            ..default()
        }))
        .add_plugins(ethertia::client::prelude::ClientGamePlugin)
        .run();
}
