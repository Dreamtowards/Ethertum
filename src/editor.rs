
use bevy::prelude::*;

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        use bevy_editor_pls::prelude::*;

        // Editor
        app.add_plugins(EditorPlugin::default());

    }
}
