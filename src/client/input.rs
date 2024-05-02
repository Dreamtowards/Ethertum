use bevy::window::*;
use leafwing_input_manager::action_state::ActionState;
use leafwing_input_manager::axislike::DualAxis;
use leafwing_input_manager::axislike::VirtualDPad;
use leafwing_input_manager::input_map::InputMap;
use leafwing_input_manager::InputManagerBundle;

use crate::client::prelude::*;
use crate::client::ui::*;
use crate::prelude::*;

pub fn init(app: &mut App) {

    app.add_systems(Startup, super::input::input_setup);
    app.add_systems(Update, super::input::input_handle);
    app.add_plugins(leafwing_input_manager::plugin::InputManagerPlugin::<InputAction>::default());
    // app.add_plugins((bevy_touch_stick::TouchStickPlugin::<InputStickId>::default());
}


#[derive(Default, Reflect, Hash, Clone, PartialEq, Eq)]
pub enum InputStickId {
    #[default]
    LeftMove,
    RightLook,
}

#[derive(leafwing_input_manager::Actionlike, PartialEq, Eq, Clone, Copy, Hash, Debug, Reflect)]
pub enum InputAction {
    Move,
    Look,

    Jump,
    Sprint,
    Sneak,

    Attack,  // or Break Block
    UseItem, // or Place Block

    // HUD
    ESC, // PauseMenu or MainMenu (not in game)
    Fullscreen,

    TabPlayerList,
    Hotbar1,
    Hotbar2,
    Hotbar3,
    Hotbar4,
    Hotbar5,
    Hotbar6,
    Hotbar7,
    Hotbar8,
    ToggleLook, // toggle Grab-Crosshair or UnGrab-Pointer
}

impl InputAction {
    pub fn default_input_map() -> InputMap<Self> {
        let mut input_map = InputMap::default();

        // Default gamepad input bindings
        input_map.insert(Self::Move, DualAxis::left_stick());
        input_map.insert(Self::Look, DualAxis::right_stick());

        // Default kbm input bindings
        input_map.insert(Self::Move, VirtualDPad::wasd());
        input_map.insert(Self::Move, VirtualDPad::arrow_keys());
        // input_map.insert(Self::Look, VirtualDPad::mouse_motion());  // Don't use MouseMotion for Look, the experimence is quite bad.
        input_map.insert(Self::Jump, KeyCode::Space);
        input_map.insert(Self::Sprint, KeyCode::ControlLeft);
        input_map.insert(Self::Sneak, KeyCode::ShiftLeft);

        input_map.insert(Self::Attack, MouseButton::Left);
        input_map.insert(Self::UseItem, MouseButton::Right);

        input_map.insert(Self::ESC, KeyCode::Escape);
        input_map.insert(Self::Fullscreen, KeyCode::F11);
        input_map.insert(Self::ToggleLook, KeyCode::Comma);

        input_map // .build()?
    }
}

pub fn input_setup(mut cmds: Commands) {
    cmds.spawn(InputManagerBundle::<InputAction>::with_map(InputAction::default_input_map()));
}

pub fn input_handle(
    key: Res<ButtonInput<KeyCode>>,
    query_input: Query<&ActionState<InputAction>>,

    mut mouse_wheel_events: EventReader<bevy::input::mouse::MouseWheel>,
    mut query_window: Query<&mut Window, With<bevy::window::PrimaryWindow>>,
    mut query_controller: Query<&mut CharacterController>,

    worldinfo: Option<ResMut<WorldInfo>>,
    player: Option<ResMut<ClientPlayerInfo>>,
    mut cli: ResMut<ClientInfo>,
    cfg: Res<ClientSettings>,
) {
    let action_state = query_input.single();

    let mut window = query_window.single_mut();

    // ESC
    if action_state.just_pressed(&InputAction::ESC) {
        if worldinfo.is_some() {
            cli.curr_ui = if cli.curr_ui == CurrentUI::None {
                CurrentUI::PauseMenu
            } else {
                CurrentUI::None
            };
        } else {
            cli.curr_ui = CurrentUI::MainMenu;
        }
    }
    // Toggle Game-Manipulating (grabbing mouse / character controlling) when CurrentUi!=None.
    let curr_manipulating = cli.curr_ui == CurrentUI::None;

    // Apply Cursor Grab
    let cursor_grab = curr_manipulating && cli.enable_cursor_look;
    window.cursor.grab_mode = if cursor_grab { CursorGrabMode::Locked } else { CursorGrabMode::None };
    window.cursor.visible = !cursor_grab;

    // Enable Character Controlling
    if let Ok(ctr) = &mut query_controller.get_single_mut() {
        ctr.enable_input = curr_manipulating;
        ctr.enable_input_cursor_look = cursor_grab;
    }

    // Toggle Cursor-Look
    if curr_manipulating && action_state.just_pressed(&InputAction::ToggleLook) {
        cli.enable_cursor_look = !cli.enable_cursor_look;
    }

    if curr_manipulating && !key.pressed(KeyCode::AltLeft) && player.is_some() {
        let wheel_delta = mouse_wheel_events.read().fold(0.0, |acc, v| acc + v.x + v.y);
        let mut player = player.unwrap();

        player.hotbar_index = (player.hotbar_index as i32 + -wheel_delta as i32).rem_euclid(ClientPlayerInfo::HOTBAR_SLOTS as i32) as u32;
    }

    // Temporary F4 Debug Settings
    if key.just_pressed(KeyCode::F4) {
        cli.curr_ui = CurrentUI::Settings;
    }

    // Temporary Toggle F9 Debug Inspector
    if key.just_pressed(KeyCode::F9) {
        cli.dbg_inspector = !cli.dbg_inspector;
    }

    // Toggle F3 Debug TextInfo
    if key.just_pressed(KeyCode::F3) {
        cli.dbg_text = !cli.dbg_text;
    }

    // Toggle F12 Debug MenuBar
    if key.just_pressed(KeyCode::F12) {
        cli.dbg_menubar = !cli.dbg_menubar;
    }

    // Toggle Fullscreen
    if action_state.just_pressed(&InputAction::Fullscreen) || (key.pressed(KeyCode::AltLeft) && key.just_pressed(KeyCode::Enter)) {
        window.mode = if window.mode != WindowMode::Fullscreen {
            WindowMode::Fullscreen
        } else {
            WindowMode::Windowed
        };
    }
    // Vsync
    window.present_mode = if cfg.vsync { PresentMode::AutoVsync } else { PresentMode::AutoNoVsync };

    unsafe {
        crate::ui::_WINDOW_SIZE = Vec2::new(window.resolution.width(), window.resolution.height());
    }
}

// // TouchStick  Move-Left
// cmds.spawn((
//     Name::new("InputStickMove"),
//     DespawnOnWorldUnload,
//     // map this stick as a left gamepad stick (through bevy_input)
//     // leafwing will register this as a normal gamepad
//     TouchStickGamepadMapping::LEFT_STICK,
//     TouchStickUiBundle {
//         stick: TouchStick {
//             id: InputStickId::LeftMove,
//             stick_type: TouchStickType::Fixed,
//             ..default()
//         },
//         // configure the interactable area through bevy_ui
//         style: Style {
//             width: Val::Px(150.),
//             height: Val::Px(150.),
//             position_type: PositionType::Absolute,
//             left: Val::Percent(15.),
//             bottom: Val::Percent(5.),
//             ..default()
//         },
//         ..default()
//     },
// ))
// .with_children(|parent| {
//     parent.spawn((
//         TouchStickUiKnob,
//         ImageBundle {
//             image: asset_server.load("knob.png").into(),
//             style: Style {
//                 width: Val::Px(75.),
//                 height: Val::Px(75.),
//                 ..default()
//             },
//             ..default()
//         },
//     ));
//     parent.spawn((
//         TouchStickUiOutline,
//         ImageBundle {
//             image: asset_server.load("outline.png").into(),
//             style: Style {
//                 width: Val::Px(150.),
//                 height: Val::Px(150.),
//                 ..default()
//             },
//             ..default()
//         },
//     ));
// });

// // spawn a look stick
// cmds.spawn((
//     Name::new("InputStickLook"),
//     DespawnOnWorldUnload,
//     // map this stick as a right gamepad stick (through bevy_input)
//     // leafwing will register this as a normal gamepad
//     TouchStickGamepadMapping::RIGHT_STICK,
//     TouchStickUiBundle {
//         stick: TouchStick {
//             id: InputStickId::RightLook,
//             stick_type: TouchStickType::Floating,
//             ..default()
//         },
//         // configure the interactable area through bevy_ui
//         style: Style {
//             width: Val::Px(150.),
//             height: Val::Px(150.),
//             position_type: PositionType::Absolute,
//             right: Val::Percent(15.),
//             bottom: Val::Percent(5.),
//             ..default()
//         },
//         ..default()
//     },
// ))
// .with_children(|parent| {
//     parent.spawn((
//         TouchStickUiKnob,
//         ImageBundle {
//             image: asset_server.load("knob.png").into(),
//             style: Style {
//                 width: Val::Px(75.),
//                 height: Val::Px(75.),
//                 ..default()
//             },
//             ..default()
//         },
//     ));
//     parent.spawn((
//         TouchStickUiOutline,
//         ImageBundle {
//             image: asset_server.load("outline.png").into(),
//             style: Style {
//                 width: Val::Px(150.),
//                 height: Val::Px(150.),
//                 ..default()
//             },
//             ..default()
//         },
//     ));
// });
