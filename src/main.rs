// #![rustfmt::skip]

use bevy::{prelude::*, window::WindowResolution};

use ethertia::game::GamePlugin;
use ethertia::editor::EditorPlugin;
use ethertia::net::NetworkServerPlugin;

fn main() {
    //std::env::set_var("RUST_BACKTRACE", "full");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: bevy::window::PresentMode::AutoNoVsync,
                resolution: WindowResolution::new(1920., 1080.),
                title: "Ethertia 0.2.1 2024.01f Network Basic".into(),
                ..default()
            }),
            ..default()
        }))
        // .add_plugins(EditorPlugin)
        .add_plugins(GamePlugin)
        // .add_plugins(NetworkServerPlugin)
        .run();
}
