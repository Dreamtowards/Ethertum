
use bevy::{prelude::*, window::WindowResolution};

mod game;
mod voxel;
mod util;

mod character_controller;
mod editor;


fn main() {
    //std::env::set_var("RUST_BACKTRACE", "full");

    App::new()
        .add_plugins(DefaultPlugins
            .set(WindowPlugin {
               primary_window: Some(Window {
                   resolution: WindowResolution::new(1920., 1080.),
                   title: "Ethertia 0.1.2 2023.12c Voxel Blocky".into(),
                   ..default()
               }),
               ..default()
            })
        )
        .add_plugins(editor::EditorPlugin)
        .add_plugins(game::GamePlugin)
        .run();
}
