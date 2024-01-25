// #![rustfmt::skip]

use bevy::{prelude::*, window::WindowResolution};

use ethertia::editor::EditorPlugin;
use ethertia::game::GamePlugin;


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
        .add_plugins(EditorPlugin)
        .add_plugins(GamePlugin)
        .run();
}
