
use bevy::{prelude::*, window::WindowResolution};

mod game;
mod voxel;

mod character_controller;
mod editor;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins
            .set(WindowPlugin {
               primary_window: Some(Window {
                   resolution: WindowResolution::new(1920., 1080.),
                   title: "Ethertia 0.1.0 2023.12a".into(),
                   ..default()
               }),
               ..default()
            })
        )
        .add_plugins(editor::EditorPlugin)
        .add_plugins(game::GamePlugin)
        .run();
}
