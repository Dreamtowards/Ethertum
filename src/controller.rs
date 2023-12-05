
use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{prelude::*, input::mouse::MouseMotion};
use bevy_xpbd_3d::{
    components::*, plugins::spatial_query::{ShapeHits, ShapeCaster}, parry::na::ComplexField, 
};

pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
    fn build(&self, app: &mut App) {

        app.register_type::<CharacterController>();

        app.add_systems(Update, 
            (
                input_move,
                sync_camera,
            ));

    }
}

#[derive(Bundle)]
pub struct CharacterControllerBundle {
    character_controller: CharacterController,
    rigid_body: RigidBody,
    collider: Collider,
    ground_caster: ShapeCaster,
    sleeping_disabled: SleepingDisabled,
    locked_axes: LockedAxes,
    gravity_scale: GravityScale,
    friction: Friction,
    restitution: Restitution,
}
impl CharacterControllerBundle {
    pub fn new(collider: Collider) -> Self {
        // Create shape caster as a slightly smaller version of collider
        let mut caster_shape = collider.clone();
        caster_shape.set_scale(Vec3::ONE * 0.99, 10);

        Self {
            character_controller: CharacterController::default(),
            rigid_body: RigidBody::Dynamic,
            collider,
            ground_caster: ShapeCaster::new(
                caster_shape, 
                Vec3::ZERO,
                Quat::default(),
                Vec3::NEG_Y
            ).with_max_time_of_impact(0.2),
            sleeping_disabled: SleepingDisabled,
            locked_axes: LockedAxes::ROTATION_LOCKED,
            gravity_scale: GravityScale(2.),
            friction: Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
            restitution: Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
        }
    }
}

/// a tag, sync transform 
#[derive(Component)]
pub struct CharacterControllerCamera;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct CharacterController {
    // State
    pitch: f32,
    yaw: f32,

    flying: bool,
    // sprint: bool,
    // sneak: bool,
    // jump: bool,

    // Readonly State
    is_grounded: bool,

    
    // Control Param
    max_slope_angle: f32,


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
            is_grounded: false,
            max_slope_angle: PI * 0.25
        }
    }
}


fn input_move(
    key_input: Res<Input<KeyCode>>,
    // phys_ctx: ,
    time: Res<Time>,
    mut mouse_events: EventReader<MouseMotion>,
    mut query: Query<(
        &mut Transform,
        &mut CharacterController,
        &mut LinearVelocity,
        &mut GravityScale,
        &ShapeHits,
        &Rotation,
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
        mut gravity_scale,
        hits,
        rotation,
    ) in query.iter_mut() {
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
        
        let is_sprinting = key_input.pressed(KeyCode::ControlLeft);
        let is_sneaking = key_input.pressed(KeyCode::ShiftLeft);

        if key_input.just_pressed(KeyCode::L) {
            ctl.flying = !ctl.flying;
        }


        // Flying
        gravity_scale.0 = if ctl.flying {0.} else {2.};

        if ctl.flying {
            if key_input.pressed(KeyCode::ShiftLeft)  { movement.y -= 1.; }
            if key_input.pressed(KeyCode::Space)    { movement.y += 1.; }
        }

        // Apply Yaw
        let movement = Mat3::from_rotation_y(ctl.yaw) * movement.normalize_or_zero();
        trans.rotation = Quat::from_rotation_y(ctl.yaw);


        // Is Grouned
        // The character is grounded if the shape caster has a hit with a normal
        // that isn't too steep.
        ctl.is_grounded = hits.iter().any(|hit| {
            // if ctl.max_slope_angle == 0. {
            //     true
            // } else {
                rotation.rotate(-hit.normal2).angle_between(Vec3::Y).abs() <= ctl.max_slope_angle
            // }
        });
        
        // Jump
        let jump = key_input.just_pressed(KeyCode::Space);
        if  jump && ctl.is_grounded && !ctl.flying {
            let jump_impulse = 8.;
            linvel.0.y += jump_impulse;
            info!("JMP");
        }
        
        // Movement
        let mut acceleration = 30.;
        if is_sprinting {
            acceleration *= 2.5;
        } else if is_sneaking {
            acceleration *= 0.3;
        }
        if ctl.flying {
            linvel.0 += movement * acceleration * dt_sec;
        } else {
            if !ctl.is_grounded {
                acceleration *= 0.1;  // LessMove on air
            }
            linvel.x += movement.x * acceleration * dt_sec;
            linvel.z += movement.z * acceleration * dt_sec;
        }

        // Damping
        // We could use `LinearDamping`, but we don't want to dampen movement along the Y axis
        if ctl.flying {
            let damping_factor = 0.05.powf(dt_sec);
            linvel.0 *= damping_factor;
        } else if ctl.is_grounded && !jump {
            let damping_factor = 0.005.powf(dt_sec);
            linvel.0 *= damping_factor;
        }
        // if ctl.flying {
        //     linvel.0 *= damping_factor;
        // } else if ctl.is_grounded {
        //     linvel.x *= damping_factor;
        //     linvel.z *= damping_factor;
        // }

    }
}


fn sync_camera(
    mut cam_query: Query<&mut Transform, With<CharacterControllerCamera>>,
    char_query: Query<(&Transform, &CharacterController), Without<CharacterControllerCamera>>,
) {
    if let Ok((char_trans, ctl)) = char_query.get_single() {
        if let Ok(mut cam_trans) = cam_query.get_single_mut() {

            cam_trans.translation = char_trans.translation + Vec3::new(0., 0.8, 0.);
            cam_trans.rotation = Quat::from_euler(EulerRot::YXZ, ctl.yaw, ctl.pitch, 0.0);
        }
    }
}