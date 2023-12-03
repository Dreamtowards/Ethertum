
use bevy::prelude::*;

use bevy_egui::{egui, EguiContexts, EguiPlugin};

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {

        app.add_plugins(EguiPlugin);
        //app.add_systems(Update, ui_example_system);

        // Editor
        use bevy_editor_pls::prelude::*;
        app.add_plugins(EditorPlugin::default());

    }
}


fn ui_example_system(mut ctx: EguiContexts) {
    egui::Window::new("Hello").show(ctx.ctx_mut(), |ui| {
        ui.label("world");
        
        if ui.button("text").clicked() {
            
        }
    });
}