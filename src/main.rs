// #![rustfmt::skip] 

use bevy::prelude::*;

fn main() {
    //std::env::set_var("RUST_BACKTRACE", "full");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: bevy::window::PresentMode::AutoNoVsync,
                resolution: bevy::window::WindowResolution::new(1920., 1080.),
                title: "Ethertia 0.2.1 2024.01f Network Basic".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(ethertia::game::GamePlugin)
        // .add_plugins(ethertia::editor::EditorPlugin)
        // .add_plugins(ethertia::net::NetworkServerPlugin)
        .run();
}
