// #![rustfmt::skip] 

use bevy::prelude::*;

fn main() {

    //std::env::set_var("RUST_BACKTRACE", "full");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: bevy::window::WindowResolution::new(1280., 720.),
                title: format!("Ethertia {} 2024.02e Items", std::env!("CARGO_PKG_VERSION")),
                fit_canvas_to_parent: true,  // web: full-window
                prevent_default_event_handling: true,  // web: avoid twice esc to pause problem.
                ..default()
            }),
            ..default()
        }))
        .add_plugins(ethertia::game_client::GameClientPlugin)
        // .add_plugins(ethertia::editor::EditorPlugin)
        .run();
}
