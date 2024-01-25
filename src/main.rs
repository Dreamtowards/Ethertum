// #![rustfmt::skip]

use bevy::{prelude::*, window::WindowResolution};

mod game;
mod util;
mod voxel;
mod net;

mod character_controller;
mod editor;


fn main() {
    //std::env::set_var("RUST_BACKTRACE", "full");

    App::new()
        .add_plugins(DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: bevy::window::PresentMode::AutoNoVsync,
                    resolution: WindowResolution::new(1920., 1080.),
                    title: "Ethertia 0.1.5 2024.01e EnhancedVoxel Part 3. The Dig.".into(),
                    ..default()
                }),   
                ..default()
            })
        )
        .add_plugins(editor::EditorPlugin)
        .add_plugins(game::GamePlugin)
        .run();
}
