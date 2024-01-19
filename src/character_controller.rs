use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{input::mouse::MouseMotion, prelude::*};
use bevy_xpbd_3d::{
    components::*,
    parry::na::ComplexField,
    plugins::spatial_query::{ShapeCaster, ShapeHits},
    PhysicsSet,
};

pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<CharacterController>();

        app.add_systems(Update, input_move);

        app.add_systems(PostUpdate, sync_camera.in_set(PhysicsSet::Sync));
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
    pub fn new(collider: Collider, character_controller: CharacterController) -> Self {
        // Create shape caster as a slightly smaller version of collider
        let mut caster_shape = collider.clone();
        caster_shape.set_scale(Vec3::ONE * 0.99, 10);

        Self {
            character_controller,
            rigid_body: RigidBody::Dynamic,
            collider,
            ground_caster: ShapeCaster::new(caster_shape, Vec3::ZERO, Quat::default(), Vec3::NEG_Y)
                .with_max_time_of_impact(0.2),
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
    pub pitch: f32,
    pub yaw: f32,

    pub is_flying: bool,
    // sprint: bool,
    // sneak: bool,
    // jump: bool,

    // Readonly State
    pub is_grounded: bool,

    pub is_sprinting: bool,
    pub is_sneaking: bool,

    // Control Param
    pub jump_impulse: f32,
    pub acceleration: f32,
    pub max_slope_angle: f32,

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
            is_flying: false,
            enable_input: true,
            is_grounded: false,
            is_sprinting: false,
            is_sneaking: false,
            jump_impulse: 7.,
            acceleration: 50.,
            max_slope_angle: PI * 0.25,
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

    for (mut trans, mut ctl, mut linvel, mut gravity_scale, hits, rotation) in query.iter_mut() {
        if !ctl.enable_input {
            continue;
        }

        // View Rotation
        let mouse_delta = mouse_delta * 0.003; //ctl.mouse_sensitivity;

        ctl.pitch = (ctl.pitch - mouse_delta.y).clamp(-FRAC_PI_2, FRAC_PI_2);
        ctl.yaw -= mouse_delta.x;
        if ctl.yaw.abs() > PI {
            ctl.yaw = ctl.yaw.rem_euclid(2. * PI);
        }

        // Disp Move
        let mut movement = Vec3::ZERO;
        if key_input.pressed(KeyCode::A) {
            movement.x -= 1.;
        }
        if key_input.pressed(KeyCode::D) {
            movement.x += 1.;
        }
        if key_input.pressed(KeyCode::W) {
            movement.z -= 1.;
        }
        if key_input.pressed(KeyCode::S) {
            movement.z += 1.;
        }

        // Input Sprint
        if key_input.pressed(KeyCode::W) {
            if key_input.pressed(KeyCode::ControlLeft) {
                ctl.is_sprinting = true;
            }
        } else {
            ctl.is_sprinting = false;
        }

        ctl.is_sneaking = key_input.pressed(KeyCode::ShiftLeft);

        if key_input.just_pressed(KeyCode::L) {
            ctl.is_flying = !ctl.is_flying;
        }

        // Flying
        gravity_scale.0 = if ctl.is_flying { 0. } else { 2. };

        if ctl.is_flying {
            if key_input.pressed(KeyCode::ShiftLeft) {
                movement.y -= 1.;
            }
            if key_input.pressed(KeyCode::Space) {
                movement.y += 1.;
            }
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

        let time_now = time.elapsed_seconds();

        // Jump
        let jump = key_input.pressed(KeyCode::Space);

        // Input: Fly: DoubleJump
        if key_input.just_pressed(KeyCode::Space) {
            static mut LAST_FLY_JUMP: f32 = 0.;
            if time_now - unsafe { LAST_FLY_JUMP } < 0.3 {
                ctl.is_flying = !ctl.is_flying;
            }
            unsafe {
                LAST_FLY_JUMP = time_now;
            }
        }
        if ctl.is_grounded && ctl.is_flying {
            ctl.is_flying = false;
        }

        // Sprint DoubleW
        if key_input.just_pressed(KeyCode::W) {
            static mut LAST_W: f32 = 0.;
            if time_now - unsafe { LAST_W } < 0.3 {
                ctl.is_sprinting = true;
            }
            unsafe {
                LAST_W = time_now;
            }
        }

        static mut LAST_JUMP: f32 = 0.;
        if jump && ctl.is_grounded && !ctl.is_flying && time_now - unsafe { LAST_JUMP } > 0.3 {
            linvel.0.y = ctl.jump_impulse;

            // info!("JMP {:?}", linvel.0);
            unsafe {
                LAST_JUMP = time_now;
            }
        }

        // Movement
        let mut acceleration = ctl.acceleration;
        if ctl.is_sprinting {
            acceleration *= 2.;
        }

        if ctl.is_flying {
            linvel.0 += movement * acceleration * dt_sec;
        } else {
            if ctl.is_sneaking {
                // !Minecraft [Sneak] * 0.3
                acceleration *= 0.3;
            } // else if using item: // Minecraft [UsingItem] * 0.2

            if !ctl.is_grounded {
                acceleration *= 0.2; // LessMove on air MC-Like 0.2
            }

            linvel.x += movement.x * acceleration * dt_sec;
            linvel.z += movement.z * acceleration * dt_sec;
        }

        // Damping
        if ctl.is_flying {
            linvel.0 *= 0.01.powf(dt_sec);
        } else {
            let mut damping_factor = 0.0001.powf(dt_sec);
            if !ctl.is_grounded {
                damping_factor = 0.07.powf(dt_sec);
            }

            // We could use `LinearDamping`, but we don't want to dampen movement along the Y axis
            linvel.x *= damping_factor;
            linvel.z *= damping_factor;
        }
        // if ctl.flying {
        //     linvel.0 *= damping_factor;
        // } else if ctl.is_grounded {
        //     linvel.x *= damping_factor;
        //     linvel.z *= damping_factor;
        // }
    }
}

#[derive(Default)]
struct SmoothValue {
    pub target: f32,
    current: f32,
}

impl SmoothValue {
    fn tick(&mut self, dt: f32) {
        self.current += dt * (self.target - self.current);
    }
}

fn sync_camera(
    mut query_cam: Query<(&mut Transform, &mut Projection), With<CharacterControllerCamera>>,
    query_char: Query<(&Position, &CharacterController), Without<CharacterControllerCamera>>,
    mut fov_val: Local<SmoothValue>,
    time: Res<Time>,
) {
    if let Ok((char_pos, ctl)) = query_char.get_single() {
        if let Ok((mut cam_trans, mut proj)) = query_cam.get_single_mut() {
            cam_trans.translation = char_pos.0 + Vec3::new(0., 0.8, 0.);
            cam_trans.rotation = Quat::from_euler(EulerRot::YXZ, ctl.yaw, ctl.pitch, 0.0);

            fov_val.target = if ctl.is_sprinting { 90. } else { 70. };
            fov_val.tick(time.delta_seconds() * 16.);

            if let Projection::Perspective(pp) = proj.as_mut() {
                pp.fov = fov_val.current.to_radians();
            }
        }
    }
}
