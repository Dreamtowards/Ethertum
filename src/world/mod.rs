
use std::f32::consts::PI;

use bevy::{prelude::*, utils::HashMap};

use bevy_atmosphere::prelude::*;

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

        app.add_systems(Startup, startup);
        app.add_systems(Update, tick_world);
        
    }
}


mod chunk;
use chunk::Chunk;

use crate::controller::{self, CharacterControllerCamera};


#[derive(Resource)]
struct ChunkSystem {

    // ChunkSystem
    chunks: HashMap<IVec3, Chunk>,


}

impl ChunkSystem {
    fn new() -> Self {
        Self { 
            chunks: HashMap::new(), 
        }
    }
}

#[derive(Reflect, Resource, Default, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
struct WorldInfo {
    
    seed: u64,

    name: String,

    #[inspector(min = 0.0, max = 1.0)]
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

    // Logical Player
    let logical_player = commands.spawn((
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
        Collider::capsule(1., 0.4),
        // Friction, Restitution
        SleepingDisabled,
        LockedAxes::ROTATION_LOCKED,
        GravityScale(1.),
        // Ccd, Mass
        // LogicPlayerTag

        controller::CharacterController::default(),
    )).id();

    // Camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(5., 0., 5.),
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
        PbrBundle {
            mesh: meshes.add(shape::Box::new(40., 0.001, 40.).into()),
            material: materials.add(Color::WHITE.into()),
            //transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            ..default()
        },
        RigidBody::Static,
        Collider::cuboid(40., 0.001, 40.)
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
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
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
        if (worldinfo.paused_steps > 0) {
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
        
        light_trans.rotation = Quat::from_rotation_z(sun_ang) * Quat::from_rotation_y(PI / 2.);
    }
}
