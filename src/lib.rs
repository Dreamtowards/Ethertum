
// Client
pub mod game;
pub mod ui;
pub mod character_controller;

#[cfg(feature = "target_native_os")]
pub mod editor;

pub mod util;
pub mod voxel;
pub mod net;
pub mod item;

pub mod game_server;


use crossbeam_channel as channel_impl;

use bevy::prelude::*;

#[cfg(target_os = "android")]
#[bevy_main]
fn main() {
    //std::env::set_var("RUST_BACKTRACE", "full");

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            present_mode: bevy::window::PresentMode::AutoNoVsync,
            resolution: bevy::window::WindowResolution::new(1280., 720.),
            title: "Ethertia 0.2.6 2024.02e Items".into(),
            fit_canvas_to_parent: true,  // web: full-window
            prevent_default_event_handling: true,  // web: avoid twice esc to pause problem.
            ..default()
        }),
        ..default()
    }))
    .add_plugins(game::GamePlugin);
    // .add_plugins(ethertia::editor::EditorPlugin)
    // .add_plugins(ethertia::net::NetworkServerPlugin)

    app.insert_resource(Msaa::Off);

    app.run();
}
