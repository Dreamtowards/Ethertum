// #![rustfmt::skip] 

use bevy::prelude::*;

fn main() {
    //std::env::set_var("RUST_BACKTRACE", "full");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: bevy::window::PresentMode::AutoNoVsync,
                resolution: bevy::window::WindowResolution::new(1280., 720.),
                title: "Ethertia 0.2.5 2024.02d Foliage".into(),
                prevent_default_event_handling: true,  // web: avoid twice esc to pause problem.
                ..default()
            }),
            ..default()
        }))
        .add_plugins(ethertia::game::GamePlugin)
        // .add_plugins(ethertia::editor::EditorPlugin)
        // .add_plugins(ethertia::net::NetworkServerPlugin)
        .run();
}
