// #![rustfmt::skip] 

use bevy::{log::LogPlugin, prelude::*};

fn main() {
    std::env::set_var("RUST_BACKTRACE", "full");
    std::env::set_var("RUST_LOG", "info");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: bevy::window::WindowResolution::new(1280., 720.),
                title: format!("Ethertia {} 2024.02e Items", std::env!("CARGO_PKG_VERSION")),
                prevent_default_event_handling: true,  // web: avoid twice esc to pause problem.
                ..default()
            }),
            ..default()
        }).set(LogPlugin {
            filter: String::new(),
            ..default()
        }))
        .add_plugins(ethertia::game_client::GameClientPlugin)
        // .add_plugins(ethertia::editor::EditorPlugin)
        .run();
}
