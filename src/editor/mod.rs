use bevy::{
    diagnostic::{DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin},
    prelude::*,
    render::{renderer::RenderAdapterInfo, view::VisibleEntities},
    window::{CursorGrabMode, PrimaryWindow, WindowMode},
};
use bevy_editor_pls::editor::EditorEvent;
use bevy_egui::{
    egui::{style::HandleShape, FontData, FontDefinitions, FontFamily, Rounding},
    EguiContexts, EguiPlugin, EguiSettings,
};


pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {

        // Editor
        use bevy_editor_pls::prelude::*;
        app.add_plugins(
            EditorPlugin::default(), // .in_new_window(Window {
                                     //     title: "Editor".into(),
                                     //     ..default()
                                     // })
        );

        // Setup Controls
        app.insert_resource(res_editor_controls());
        app.add_systems(Startup, setup_editor_camera_controls);
        // app.add_systems(Update, handle_inputs);

    }
}

// fn handle_inputs(
//     mut editor_events: EventReader<bevy_editor_pls::editor::EditorEvent>,
//     mut window_query: Query<&mut Window, With<PrimaryWindow>>,
//     mut controller_query: Query<&mut CharacterController>,
//     key: Res<Input<KeyCode>>,
//     // mouse_input: Res<Input<MouseButton>>,
// ) {
//     let mut window = window_query.single_mut();

//     // Toggle MouseGrab
//     for event in editor_events.read() {
//         if let EditorEvent::Toggle { now_active } = *event {
//             let playing = !now_active;
//             window.cursor.grab_mode = if playing { CursorGrabMode::Locked } else { CursorGrabMode::None };
//             window.cursor.visible = !playing;
//             for mut controller in &mut controller_query {
//                 controller.enable_input = playing;
//             }
//         }
//     }

//     // Toggle Fullscreen
//     if key.just_pressed(KeyCode::F11) || (key.pressed(KeyCode::AltLeft) && key.just_pressed(KeyCode::Return)) {
//         window.mode = if window.mode != WindowMode::Fullscreen {
//             WindowMode::Fullscreen
//         } else {
//             WindowMode::Windowed
//         };
//     }
// }
fn res_editor_controls() -> bevy_editor_pls::controls::EditorControls {
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

fn setup_editor_camera_controls(mut query: Query<&mut bevy_editor_pls::default_windows::cameras::camera_3d_free::FlycamControls>) {
    let mut controls = query.single_mut();
    controls.key_up = KeyCode::KeyE;
    controls.key_down = KeyCode::KeyQ;
}
