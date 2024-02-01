use std::f32::consts::{PI, TAU};

use bevy::{
    math::vec3,
    pbr::DirectionalLightShadowMap,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow, WindowMode},
};
use bevy_atmosphere::prelude::*;
use bevy_xpbd_3d::prelude::*;

use crate::{
    character_controller::{CharacterController, CharacterControllerBundle, CharacterControllerCamera, CharacterControllerPlugin},
    net::NetworkClientPlugin,
};

use crate::voxel::VoxelPlugin;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {

        // Render
        {
            // Atmosphere
            app.insert_resource(AtmosphereModel::default());
            app.add_plugins(AtmospherePlugin);
    
            // ShadowMap sizes
            app.insert_resource(DirectionalLightShadowMap { size: 512 });
    
            // SSAO
            // app.add_plugins(TemporalAntiAliasPlugin);
            // app.insert_resource(AmbientLight {
            //         brightness: 0.05,
            //         ..default()
            //     });
        }

        // Physics
        app.add_plugins(PhysicsPlugins::default());

        // CharacterController
        app.add_plugins(CharacterControllerPlugin);

        // WorldInfo
        app.insert_resource(WorldInfo::new());
        app.register_type::<WorldInfo>();

        // ChunkSystem
        app.add_plugins(VoxelPlugin);

        app.add_systems(OnEnter(AppState::InGame), startup);  // Camera, Player, Sun
        app.add_systems(OnExit(AppState::InGame), cleanup);
        app.add_systems(Update, tick_world.run_if(in_state(AppState::InGame)));  // Sun, World Timing.

        // Debug Draw Gizmos
        app.add_systems(PostUpdate, gizmo_sys.after(PhysicsSet::Sync).run_if(in_state(AppState::InGame)));

        // Network Client
        app.add_plugins(NetworkClientPlugin);

        app.add_state::<AppState>();

        app.add_systems(Update, handle_inputs); // toggle: PauseGameControl, Fullscreen

        app.add_plugins(crate::ui::UiPlugin);

        app.add_state::<GameInput>();
        app.add_systems(OnEnter(GameInput::Controlling), ingame_toggle);
        app.add_systems(OnExit(GameInput::Controlling), ingame_toggle);
    }
}

// #[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
// pub enum SystemSet {
//     UI,
// }

// !!!!!!!!这里要大重构，去掉AppState 多余了 思路有问题，直接一个bool放在Res ClientInfo即可的。就是不能in_state()这么方便condition

// 这个有点问题 他应该是一个bool的状态, 用于判断世界逻辑systems是否该被执行 清理/初始化, 而不应该有多种可能
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum AppState {
    #[default]
    MainMenu,
    InGame,      // InGameWorld
    WtfSettings, // 这个是乱加的，因为Settings应该可以和InGame共存，也就是InGame的同时有Settings
    WtfServerList,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum GameInput {
    #[default]
    Paused,
    // Is Manipulating/Controlling game e.g. WSAD
    Controlling,
}

// #[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
// pub enum WtfSettingsUIs {
//     #[default]
//     Settings,  // for MainMenu, InGame
//     Inventory,  // only for InGame
// }

fn ingame_toggle(
    next_state: Res<State<GameInput>>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    mut controller_query: Query<&mut CharacterController>,
) {
    let mut window = window_query.single_mut();

    let to_play = *next_state == GameInput::Controlling;

    window.cursor.grab_mode = if to_play { CursorGrabMode::Locked } else { CursorGrabMode::None };
    window.cursor.visible = !to_play;

    for mut controller in &mut controller_query {
        controller.enable_input = to_play;
    }
}

fn startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut next_state: ResMut<NextState<GameInput>>,
) {
    next_state.set(GameInput::Controlling);

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
        CharacterControllerBundle::new(
            Collider::capsule(1., 0.4),
            CharacterController {
                is_flying: true,
                enable_input: true,
                ..default()
            },
        ),
        Name::new("Player"),
    ));

    // Camera
    commands.spawn((
        Camera3dBundle {
            projection: Projection::Perspective(PerspectiveProjection { fov: TAU / 4.6, ..default() }),
            camera: Camera { hdr: true, ..default() },
            ..default()
        },
        AtmosphereCamera::default(), // Marks camera as having a skybox, by default it doesn't specify the render layers the skybox can be seen on
        CharacterControllerCamera,
        Name::new("Camera"),
    ));
    // .insert(ScreenSpaceAmbientOcclusionBundle::default())
    // .insert(TemporalAntiAliasBundle::default());

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
        Name::new("Sun"),
    ));

    // commands.spawn((
    //     SceneBundle {
    //         scene: assets.load("spaceship.glb#Scene0"),
    //         transform: Transform::from_xyz(0., 0., -10.),
    //         ..default()
    //     },
    //     AsyncSceneCollider::new(Some(ComputedCollider::TriMesh)),
    //     RigidBody::Static,
    // ));

    // // Floor
    // commands.spawn((
    //     SceneBundle {
    //         scene: assets.load("playground.glb#Scene0"),
    //         transform: Transform::from_xyz(0.5, -5.5, 0.5),
    //         ..default()
    //     },
    //     AsyncSceneCollider::new(Some(ComputedCollider::TriMesh)),
    //     RigidBody::Static,
    // ));

    // // Cube
    // commands.spawn((
    //     RigidBody::Dynamic,
    //     AngularVelocity(Vec3::new(2.5, 3.4, 1.6)),
    //     Collider::cuboid(1.0, 1.0, 1.0),
    //     PbrBundle {
    //         mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
    //         material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
    //         transform: Transform::from_xyz(0.0, 4.0, 0.0),
    //         ..default()
    //     },
    // ));
}

fn cleanup(
    mut commands: Commands,
    cam_query: Query<Entity, With<CharacterControllerCamera>>,
    player_query: Query<Entity, With<CharacterController>>,
    sun_query: Query<Entity, With<Sun>>,
) {
    commands.entity(cam_query.single()).despawn_recursive();
    commands.entity(player_query.single()).despawn_recursive();
    commands.entity(sun_query.single()).despawn_recursive();
}

fn handle_inputs(
    key: Res<Input<KeyCode>>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,

    mut state: ResMut<State<GameInput>>,
    mut next_state: ResMut<NextState<GameInput>>,
) {
    let mut window = window_query.single_mut();

    if key.just_pressed(KeyCode::Escape) {
        next_state.set(if *state == GameInput::Paused {
            GameInput::Controlling
        } else {
            GameInput::Paused
        });
    }

    // Toggle Fullscreen
    if key.just_pressed(KeyCode::F11) || (key.pressed(KeyCode::AltLeft) && key.just_pressed(KeyCode::Return)) {
        window.mode = if window.mode != WindowMode::Fullscreen {
            WindowMode::Fullscreen
        } else {
            WindowMode::Windowed
        };
    }
}

fn tick_world(
    mut atmosphere: AtmosphereMut<Nishita>,
    mut query: Query<(&mut Transform, &mut DirectionalLight), With<Sun>>,
    mut worldinfo: ResMut<WorldInfo>,
    time: Res<Time>,
) {
    // worldinfo.tick_timer.tick(time.delta());
    // if !worldinfo.tick_timer.just_finished() {
    //     return;
    // }
    // let dt_sec = worldinfo.tick_timer.duration().as_secs_f32();  // constant time step?

    // // Pause & Steps
    // if worldinfo.is_paused {
    //     if  worldinfo.paused_steps > 0 {
    //         worldinfo.paused_steps -= 1;
    //     } else {
    //         return;
    //     }
    // }
    let dt_sec = time.delta_seconds();

    worldinfo.time_inhabited += dt_sec;

    // DayTime
    if worldinfo.daytime_length != 0. {
        worldinfo.daytime += dt_sec / worldinfo.daytime_length;
        worldinfo.daytime -= worldinfo.daytime.trunc(); // trunc to [0-1]
    }

    // Atmosphere SunPos
    let sun_ang = worldinfo.daytime * PI * 2.;
    atmosphere.sun_position = Vec3::new(sun_ang.cos(), sun_ang.sin(), 0.);

    if let Some((mut light_trans, mut directional)) = query.single_mut().into() {
        directional.illuminance = sun_ang.sin().max(0.0).powf(2.0) * 100000.0;

        // or from000.looking_at()
        light_trans.rotation = Quat::from_rotation_z(sun_ang) * Quat::from_rotation_y(PI / 2.3);
    }
}

fn gizmo_sys(mut gizmo: Gizmos, mut gizmo_config: ResMut<GizmoConfig>, query_cam: Query<&Transform, With<CharacterControllerCamera>>) {
    gizmo_config.depth_bias = -1.; // always in front

    // World Basis Axes
    let n = 5;
    gizmo.line(Vec3::ZERO, Vec3::X * 2. * n as f32, Color::RED);
    gizmo.line(Vec3::ZERO, Vec3::Y * 2. * n as f32, Color::GREEN);
    gizmo.line(Vec3::ZERO, Vec3::Z * 2. * n as f32, Color::BLUE);

    let color = Color::GRAY;
    for x in -n..=n {
        gizmo.ray(vec3(x as f32, 0., -n as f32), Vec3::Z * n as f32 * 2., color);
    }
    for z in -n..=n {
        gizmo.ray(vec3(-n as f32, 0., z as f32), Vec3::X * n as f32 * 2., color);
    }

    // View Basis
    let cam_trans = query_cam.single();
    let p = cam_trans.translation;
    let rot = cam_trans.rotation;
    let n = 0.03;
    let offset = vec3(0., 0., -0.5);
    gizmo.ray(p + rot * offset, Vec3::X * n, Color::RED);
    gizmo.ray(p + rot * offset, Vec3::Y * n, Color::GREEN);
    gizmo.ray(p + rot * offset, Vec3::Z * n, Color::BLUE);
}

#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct WorldInfo {
    pub seed: u64,

    pub name: String,

    pub daytime: f32,

    // seconds a day time long
    pub daytime_length: f32,

    // seconds
    pub time_inhabited: f32,

    time_created: u64,
    time_modified: u64,

    tick_timer: Timer,

    pub is_paused: bool,
    pub paused_steps: i32,

    pub dbg_text: bool,
}

impl WorldInfo {
    fn new() -> Self {
        WorldInfo {
            seed: 0,
            name: "None Name".into(),
            daytime: 0.15,
            daytime_length: 60. * 24.,

            time_inhabited: 0.,
            time_created: 0,
            time_modified: 0,

            tick_timer: Timer::new(bevy::utils::Duration::from_secs_f32(1. / 20.), TimerMode::Repeating),

            is_paused: false,
            paused_steps: 0,

            dbg_text: true,
        }
    }
}

// struct ClientSettings {
//     fov: f32,
//     server_list: Vec<>,
// }

struct ClientInfo {

}

#[derive(Component)]
struct Sun; // marker
