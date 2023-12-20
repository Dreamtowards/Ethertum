
use bevy::{prelude::*, diagnostic::{FrameTimeDiagnosticsPlugin, EntityCountDiagnosticsPlugin, SystemInformationDiagnosticsPlugin}};

use bevy_egui::{egui, EguiContexts, EguiPlugin};

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {

        //app.add_plugins(EguiPlugin);
        //app.add_systems(Update, ui_example_system);

        // Editor
        use bevy_editor_pls::prelude::*;
        app.add_plugins(EditorPlugin::default()
            // .in_new_window(Window {
            //     title: "Editor".into(),
            //     ..default()
            // })
        );

        app.add_plugins((FrameTimeDiagnosticsPlugin, EntityCountDiagnosticsPlugin, SystemInformationDiagnosticsPlugin));
        
        // Setup Controls
        app.insert_resource(editor_controls());
        app.add_systems(Startup, setup_editor_camera_controls);

    }
}


fn editor_controls() -> bevy_editor_pls::controls::EditorControls {
    use bevy_editor_pls::controls::*;
    let mut editor_controls = EditorControls::default_bindings();
    editor_controls.unbind(Action::PlayPauseEditor);

    editor_controls.insert(
        Action::PlayPauseEditor,
        Binding {
            input: UserInput::Single(Button::Keyboard(KeyCode::Escape)),
            conditions: vec![BindingCondition::ListeningForText(false)],
        },
    );

    editor_controls
}

fn setup_editor_camera_controls(
    mut query: Query<&mut bevy_editor_pls::default_windows::cameras::camera_3d_free::FlycamControls>,
) {
    let mut controls = query.single_mut();
    controls.key_up = KeyCode::E;
    controls.key_down = KeyCode::Q;
}

// fn ui_example_system(mut ctx: EguiContexts) {
//     egui::Window::new("Hello").show(ctx.ctx_mut(), |ui| {
//         ui.label("world");
        
//         if ui.button("text").clicked() {
            
//         }
//     });
// }