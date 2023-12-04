
use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{prelude::*, input::mouse::MouseMotion};
use bevy_xpbd_3d::{
    components::{LinearVelocity, GravityScale}, 
};

pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
    fn build(&self, app: &mut App) {

        app.register_type::<CharacterController>();

        app.add_systems(Update, 
            (
                ctl_input,
                sync_camera,
            ));

    }
}

/// a tag, sync transform 
#[derive(Component)]
pub struct CharacterControllerCamera;

#[derive(Component, Reflect)]
pub struct CharacterController {
    // state
    pitch: f32,
    yaw: f32,

    flying: bool,
    // sprint: bool,
    // sneak: bool,
    // jump: bool,

    // Input

    pub enable_input: bool,
    // fly_speed: f32,
    // walk_speed: f32,


    // Tmp KeyConfig
    // key_forward: KeyCode,
    // key_back: KeyCode,
    // key_left: KeyCode,
    // key_right: KeyCode,
    // key_up: KeyCode,    // flymode
    // key_down: KeyCode,  // flymode
    // key_sprint: KeyCode,
    // key_sneak: KeyCode,
    // key_jump: KeyCode,

    // mouse_sensitivity: f32,
}

impl Default for CharacterController {
    fn default() -> Self {
        Self {
            yaw: 0.,
            pitch: 0.,
            flying: false,
            enable_input: true,
        }
    }
}


fn ctl_input(
    key_input: Res<Input<KeyCode>>,
    // phys_ctx: ,
    time: Res<Time>,
    mut mouse_events: EventReader<MouseMotion>,
    mut query: Query<(
        &mut Transform,
        &mut CharacterController,
        &mut LinearVelocity,
        &mut GravityScale
    )>,
) {
    let mut mouse_delta = Vec2::ZERO;
    for mouse_event in mouse_events.read() {
        mouse_delta += mouse_event.delta;
    }
    let dt_sec = time.delta_seconds();

    for (mut trans, 
        mut ctl, 
        mut linvel, 
        mut gravity_scale) in query.iter_mut() {
        if !ctl.enable_input {
            continue;
        }

        // View Rotation
        let mouse_delta  = mouse_delta * 0.003;//ctl.mouse_sensitivity;

        ctl.pitch = (ctl.pitch - mouse_delta.y).clamp(-FRAC_PI_2, FRAC_PI_2);
        ctl.yaw -= mouse_delta.x;
        if ctl.yaw.abs() > PI {
            ctl.yaw = ctl.yaw.rem_euclid(2. * PI);
        }

        // Disp Move
        let mut movement = Vec3::ZERO;
        if key_input.pressed(KeyCode::A)  { movement.x -= 1.; }
        if key_input.pressed(KeyCode::D) { movement.x += 1.; }
        if key_input.pressed(KeyCode::W) { movement.z -= 1.; }
        if key_input.pressed(KeyCode::S)  { movement.z += 1.; }
        if key_input.pressed(KeyCode::ShiftLeft)  { movement.y -= 1.; }
        if key_input.pressed(KeyCode::Space)    { movement.y += 1.; }
        
        let sprint = key_input.pressed(KeyCode::ControlLeft);
        let sneak = key_input.pressed(KeyCode::ShiftLeft);
        let jump = key_input.pressed(KeyCode::Space);
        
        if key_input.pressed(KeyCode::L) {
            ctl.flying = !ctl.flying;
        }
        
        gravity_scale.0 = if ctl.flying {0.} else {1.};

        trans.rotation = Quat::from_rotation_y(ctl.yaw);

        // apply Yaw
        let movement = Mat3::from_rotation_y(ctl.yaw) * movement;
        linvel.0 += movement * 0.2;

    }
}


fn sync_camera(
    mut cam_query: Query<&mut Transform, With<CharacterControllerCamera>>,
    char_query: Query<(&Transform, &CharacterController), Without<CharacterControllerCamera>>,
) {
    if let Ok((char_trans, ctl)) = char_query.get_single() {
        if let Ok(mut cam_trans) = cam_query.get_single_mut() {

            cam_trans.translation = char_trans.translation;
            cam_trans.rotation = Quat::from_euler(EulerRot::YXZ, ctl.yaw, ctl.pitch, 0.0);
        }
    }
}