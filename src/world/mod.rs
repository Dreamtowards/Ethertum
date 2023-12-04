
use std::f32::consts::{PI, TAU};

use bevy::{prelude::*, utils::HashMap, window::{CursorGrabMode, PrimaryWindow}};

use bevy_atmosphere::prelude::*;

use bevy_editor_pls::editor::EditorEvent;
use bevy_inspector_egui::prelude::*;
use bevy_inspector_egui::quick::ResourceInspectorPlugin;

use bevy_xpbd_3d::parry::mass_properties::MassProperties;
use bevy_xpbd_3d::prelude::*;



pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {

        // Atmosphere
        app.insert_resource(AtmosphereModel::default());
        app.add_plugins(AtmospherePlugin);
        
        // Physics
        app.add_plugins(PhysicsPlugins::default());

        app.add_plugins(controller::CharacterControllerPlugin);

        app.insert_resource(WorldInfo::new());
        app.register_type::<WorldInfo>();
        
        app.insert_resource(ClientInfo::default());
        app.register_type::<ClientInfo>();
        app.add_systems(Update, (editor_pause));

        app.add_systems(Startup, startup);
        app.add_systems(Update, tick_world);
        
    }
}


#[derive(Resource, Reflect, Default)]
struct ClientInfo {

    /// Is Controlling Game, InGame
    is_playing: bool,

}

impl ClientInfo {

    fn set_playing(self, playing: bool) {

    }

}

fn editor_pause(
    mut editor_events: EventReader<bevy_editor_pls::editor::EditorEvent>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    mut controller_query: Query<&mut CharacterController>,
) {
    let mut window = window_query.single_mut();

    for event in editor_events.read() {
        match *event {
            EditorEvent::Toggle { now_active } => {
                let playing = !now_active;
                window.cursor.grab_mode = if playing {CursorGrabMode::Locked} else {CursorGrabMode::None};
                window.cursor.visible = !playing;
                for mut controller in &mut controller_query {
                    controller.enable_input = playing;
                }
            },
            _ => ()
        }
    }
}






use crate::controller::{self, CharacterControllerCamera, CharacterController};


#[derive(Reflect, Resource, Default)]
struct WorldInfo {
    
    seed: u64,

    name: String,

    daytime: f32,

    // seconds a day time long
    daytime_length: f32,  

    // seconds
    time_inhabited: f32,

    time_created: u64,
    time_modified: u64,
    
    tick_timer: Timer,

    is_paused: bool,
    paused_steps: i32,
}

impl WorldInfo {
    fn new() -> Self {
        WorldInfo {
            seed: 0,
            name: "None Name".into(),
            daytime: 0.,
            daytime_length: 60. * 2.,

            time_inhabited: 0.,
            time_created: 0,
            time_modified: 0,
            
            tick_timer: Timer::new(
                bevy::utils::Duration::from_secs_f32(1. / 20.), // Update our atmosphere every 50ms (in a real game, this would be much slower, but for the sake of an example we use a faster update)
                TimerMode::Repeating,
            ),

            is_paused: false,
            paused_steps: 0,
        }
    }
}


#[derive(Component)]
struct Sun;



// Simple environment
fn startup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {

    let collider = Collider::capsule(1., 0.4);
    // Create shape caster as a slightly smaller version of collider
    let mut caster_shape = collider.clone();
    caster_shape.set_scale(Vec3::ONE * 0.99, 10);

    // Logical Player
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Capsule {
                radius: 0.4,
                depth: 1.0,
                ..default()
            })),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_xyz(0.0, 1.5, 0.0),
            ..default()
        },
        RigidBody::Dynamic,
        collider,
        // Friction, Restitution
        SleepingDisabled,
        LockedAxes::ROTATION_LOCKED,
        GravityScale(2.),
        Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
        Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
        // Ccd, Mass
        // LogicPlayerTag
        ShapeCaster::new(
            caster_shape, 
            Vec3::ZERO,
            Quat::default(),
            Vec3::NEG_Y
        ).with_max_time_of_impact(0.2),

        controller::CharacterController::default(),
    )).with_children(|p| {
        p.spawn(SpotLightBundle {
            spot_light: SpotLight {
                intensity: 1500.0,
                shadows_enabled: true,
                ..default()
            },
            transform: Transform::from_xyz(0., 0., -0.5),
            ..default()
        });
    });

    // Camera
    commands.spawn((
        Camera3dBundle {
            projection: Projection::Perspective(PerspectiveProjection {
                fov: TAU / 4.6,
                ..default()
            }),
            ..default()
        },
        AtmosphereCamera::default(), // Marks camera as having a skybox, by default it doesn't specify the render layers the skybox can be seen on
        CharacterControllerCamera,
    ));

    // Sun
    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                shadows_enabled: true,
                ..default()
            },
            ..default()
        },
        Sun, // Marks the light as Sun
    ));


    commands.spawn((
        SceneBundle {
            scene: assets.load("spaceship.glb#Scene0"),
            transform: Transform::from_xyz(0., 0., -10.),
            ..default()
        },
        AsyncSceneCollider::new(Some(ComputedCollider::TriMesh)),
        RigidBody::Static,
    ));

    // Floor
    commands.spawn((
        SceneBundle {
            scene: assets.load("playground.glb#Scene0"),
            transform: Transform::from_xyz(0., 0., -10.),
            ..default()
        },
        AsyncSceneCollider::new(Some(ComputedCollider::TriMesh)),
        RigidBody::Static,
    ));

    // Cube
    commands.spawn((
        RigidBody::Dynamic,
        AngularVelocity(Vec3::new(2.5, 3.4, 1.6)),
        Collider::cuboid(1.0, 1.0, 1.0),
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_xyz(0.0, 4.0, 0.0),
            ..default()
        },
    ));
    // Light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(0., 0.0, 0.0),
        ..default()
    });
}



fn tick_world(
    mut atmosphere: AtmosphereMut<Nishita>,
    mut query: Query<(&mut Transform, &mut DirectionalLight), With<Sun>>,
    mut worldinfo: ResMut<WorldInfo>,
    time: Res<Time>,
) {
    worldinfo.tick_timer.tick(time.delta());
    if !worldinfo.tick_timer.just_finished() {
        return;
    }

    // Pause & Steps
    if worldinfo.is_paused {
        if  worldinfo.paused_steps > 0 {
            worldinfo.paused_steps -= 1;
        } else {
            return;
        }
    }
    

    let dt_sec = worldinfo.tick_timer.duration().as_secs_f32();  // constant time step?
    worldinfo.time_inhabited += dt_sec;
    
    // DayTime
    worldinfo.daytime += dt_sec / worldinfo.daytime_length;
    worldinfo.daytime -= worldinfo.daytime.trunc();  // trunc to [0-1]

    // SunPos
    let sun_ang = worldinfo.daytime * PI*2.;

    // Atmosphere SunPos
    atmosphere.sun_position = Vec3::new(sun_ang.cos(), sun_ang.sin(), 0.);

    if let Some((mut light_trans, mut directional)) = query.single_mut().into() {
        directional.illuminance = sun_ang.sin().max(0.0).powf(2.0) * 100000.0;
        
        // or from000.looking_at()
        light_trans.rotation = Quat::from_rotation_z(sun_ang) * Quat::from_rotation_y(PI / 2.);
    }
}
