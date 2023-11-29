
use bevy::prelude::*;

mod editor;
mod world;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(editor::EditorPlugin)
        .add_plugins(world::WorldPlugin)
        .run();
}
