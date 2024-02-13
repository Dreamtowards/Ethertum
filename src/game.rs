use std::f32::consts::{PI, TAU};

use bevy::{ecs::system::{CommandQueue, SystemParam}, math::vec3, pbr::DirectionalLightShadowMap, prelude::*, window::{CursorGrabMode, PrimaryWindow, WindowMode}
};

#[cfg(feature = "target_native_os")]
use bevy_atmosphere::prelude::*;

use bevy_renet::renet::RenetClient;
use bevy_xpbd_3d::prelude::*;

use crate::{
    character_controller::{CharacterController, CharacterControllerCamera, CharacterControllerPlugin}, net::{CPacket, ClientNetworkPlugin, RenetClientHelper}, ui::CurrentUI
};

use crate::voxel::ClientVoxelPlugin;

pub mod condition {
    use bevy::ecs::{schedule::{common_conditions::{resource_added, resource_exists, resource_removed}, State}, system::Res};
    use crate::ui::CurrentUI;
    use super::WorldInfo;

    pub fn in_world() -> impl FnMut(Option<Res<WorldInfo>>) -> bool + Clone {
        resource_exists::<WorldInfo>()
    }
    pub fn load_world() -> impl FnMut(Option<Res<WorldInfo>>) -> bool + Clone {
        resource_added::<WorldInfo>()
    }
    pub fn unload_world() -> impl FnMut(Option<Res<WorldInfo>>) -> bool + Clone {
        resource_removed::<WorldInfo>()
    }
    pub fn manipulating() -> impl FnMut(Res<State<CurrentUI>>) -> bool + Clone {
        |curr_ui: Res<State<CurrentUI>>| *curr_ui == CurrentUI::None
    }
}

/// Despawn the Entity on World Unload.
#[derive(Component)]
pub struct DespawnOnWorldUnload;



pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {

        // Render
        {
            // Atmosphere
            #[cfg(feature = "target_native_os")]
            {
                app.add_plugins(AtmospherePlugin);
                app.insert_resource(AtmosphereModel::default());
            }

            // Billiboard
            use bevy_mod_billboard::prelude::*;
            app.add_plugins(BillboardPlugin);
    
            // ShadowMap sizes
            app.insert_resource(DirectionalLightShadowMap { size: 512 });
    
            // SSAO
            // app.add_plugins(TemporalAntiAliasPlugin);
            // app.insert_resource(AmbientLight { brightness: 0.05, ..default() });
        }

        // Physics
        app.add_plugins(PhysicsPlugins::default());

        // CharacterController
        app.add_plugins(CharacterControllerPlugin);

        // UI
        app.add_plugins(crate::ui::UiPlugin);

        // ChunkSystem
        app.add_plugins(ClientVoxelPlugin);

        // ClientInfo
        app.insert_resource(ClientInfo::default());

        // WorldInfo
        app.register_type::<WorldInfo>();

        // World Setup/Cleanup, Tick
        app.add_systems(First, startup.run_if(condition::load_world()));  // Camera, Player, Sun
        app.add_systems(Last, cleanup.run_if(condition::unload_world()));
        app.add_systems(Update, tick_world.run_if(condition::in_world()));  // Sun, World Timing.

        // Debug Draw Basis
        app.add_systems(PostUpdate, gizmo_sys.after(PhysicsSet::Sync).run_if(condition::in_world()));

        app.add_systems(Update, handle_inputs); // toggle: PauseGameControl, Fullscreen

        // Network Client
        app.add_plugins(ClientNetworkPlugin);


    }
}



#[derive(SystemParam)]
pub struct EthertiaClient<'w,'s> {

    clientinfo: ResMut<'w, ClientInfo>,

    cmds: Commands<'w,'s>,
}

impl<'w,'s> EthertiaClient<'w,'s> {

    /// for Singleplayer
    // pub fn load_world(&mut self, cmds: &mut Commands, server_addr: String)


    pub fn connect_server(&mut self, server_addr: String) {
        info!("Connecting to {}", server_addr);

        let mut net_client = RenetClient::new(bevy_renet::renet::ConnectionConfig::default());
        
        let username = &self.clientinfo.username;
        net_client.send_packet(&CPacket::Login { uuid: crate::util::hashcode(username), access_token: 123, username: username.clone() });

        self.cmds.insert_resource(net_client);
        self.cmds.insert_resource(crate::net::new_netcode_client_transport(server_addr.parse().unwrap(), Some("userData123".to_string().into_bytes())));
        
        // clear Disconnect Reason prevents mis-display.
        self.clientinfo.disconnected_reason.clear();

        // let mut cmd = CommandQueue::default();
        // cmd.push(move |world: &mut World| {
        //     world.insert_resource(crate::net::new_netcode_client_transport(server_addr.parse().unwrap()));
        //     world.insert_resource(RenetClient::new(bevy_renet::renet::ConnectionConfig::default()));
            
        //     let mut net_client = world.resource_mut::<RenetClient>();

        //     net_client.send_packet(&CPacket::Login { uuid: 1, access_token: 123 });
        // });
    }

    pub fn enter_world() {

    }
}




fn startup(
    mut cmds: Commands,
    // mut meshes: ResMut<Assets<Mesh>>,
    // mut materials: ResMut<Assets<StandardMaterial>>,
    // cli: Res<ClientInfo>,
) {
    info!("Load World. setup Player, Camera, Sun.");

    // Logical Player
    // crate::net::spawn_player(Entity::from_raw(1000), true, &cli.username, &mut cmds, &mut meshes, &mut materials);
    // commands.spawn((
    //     PbrBundle {
    //         mesh: meshes.add(Mesh::from(shape::Capsule {
    //             radius: 0.3,
    //             depth: 1.3,
    //             ..default()
    //         })),
    //         material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
    //         transform: Transform::from_xyz(0.0, 0.0, 0.0),
    //         ..default()
    //     },
    //     CharacterControllerBundle::new(
    //         Collider::capsule(1.3, 0.3),
    //         CharacterController {
    //             is_flying: true,
    //             enable_input: false,
    //             ..default()
    //         },
    //     ),
    //     Name::new("Player"),
    //     DespawnOnWorldUnload,
    // ));

    // Camera
    cmds.spawn((
        Camera3dBundle {
            projection: Projection::Perspective(PerspectiveProjection { fov: TAU / 4.6, ..default() }),
            camera: Camera { hdr: true, ..default() },
            ..default()
        },

        #[cfg(feature = "target_native_os")]
        AtmosphereCamera::default(), // Marks camera as having a skybox, by default it doesn't specify the render layers the skybox can be seen on

        CharacterControllerCamera,
        Name::new("Camera"),
        DespawnOnWorldUnload,
    ));
    // .insert(ScreenSpaceAmbientOcclusionBundle::default())
    // .insert(TemporalAntiAliasBundle::default());

    // Sun
    cmds.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                shadows_enabled: true,
                ..default()
            },
            ..default()
        },
        Sun, // Marks the light as Sun
        Name::new("Sun"),
        DespawnOnWorldUnload,
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
    //         scene: asset_server.load("playground.glb#Scene0"),
    //         transform: Transform::from_xyz(0.5, -5.5, 0.5),
    //         ..default()
    //     },
    //     AsyncSceneCollider::new(Some(ComputedCollider::TriMesh)),
    //     RigidBody::Static,
    //     DespawnOnWorldUnload,
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
    query_despawn: Query<Entity, With<DespawnOnWorldUnload>>,
) {
    info!("Unload World");

    for entity in query_despawn.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn handle_inputs(
    key: Res<Input<KeyCode>>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    mut controller_query: Query<&mut CharacterController>,

    mut worldinfo: Option<ResMut<WorldInfo>>,
    mut last_is_manipulating: Local<bool>,
    mut curr_ui: ResMut<State<CurrentUI>>,
    mut next_ui: ResMut<NextState<CurrentUI>>,

    mut clientinfo: ResMut<ClientInfo>,
) {
    let mut window = window_query.single_mut();

    if key.just_pressed(KeyCode::Escape) {
        if worldinfo.is_some() {
            next_ui.set(if *curr_ui == CurrentUI::None { CurrentUI::PauseMenu } else { CurrentUI::None });
        } else {
            next_ui.set(CurrentUI::MainMenu);
        }
    }
    // Toggle Game-Manipulating (grabbing mouse / character controlling) when UI set. 
    let curr_manipulating = *curr_ui == CurrentUI::None;
    if *last_is_manipulating != curr_manipulating {
        *last_is_manipulating = curr_manipulating;

        window.cursor.grab_mode = if curr_manipulating { CursorGrabMode::Locked } else { CursorGrabMode::None };
        window.cursor.visible = !curr_manipulating;

        if let Ok(ctr) = &mut controller_query.get_single_mut() {
            ctr.enable_input = curr_manipulating;
        }
    }

    // Toggle F3 Debug TextInfo
    if key.just_pressed(KeyCode::F3) {
        clientinfo.dbg_text = !clientinfo.dbg_text;
    }

    // Toggle F12 Debug MenuBar
    if key.just_pressed(KeyCode::F12) {
        clientinfo.dbg_menubar = !clientinfo.dbg_menubar;
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
    #[cfg(feature = "target_native_os")]
    mut atmosphere: AtmosphereMut<Nishita>,

    mut query_sun: Query<(&mut Transform, &mut DirectionalLight), With<Sun>>,
    mut worldinfo: ResMut<WorldInfo>,
    time: Res<Time>,

    query_player: Query<&Transform, (With<CharacterController>, Without<Sun>)>,
    mut net_client: ResMut<RenetClient>,
    mut last_player_pos: Local<Vec3>,
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

    // Send PlayerPos
    if let Ok(player_loc) = query_player.get_single() {
        let player_pos = player_loc.translation;

        if player_pos.distance_squared(*last_player_pos) > 0.01*0.01 {
            *last_player_pos = player_pos;
            net_client.send_packet(&CPacket::PlayerPos { position: player_pos });
        }
    }


    // Sun Pos
    let sun_angle = worldinfo.daytime * PI * 2.;
    
    #[cfg(feature = "target_native_os")]
    {
        atmosphere.sun_position = Vec3::new(sun_angle.cos(), sun_angle.sin(), 0.);
    }

    if let Some((mut light_trans, mut directional)) = query_sun.single_mut().into() {
        directional.illuminance = sun_angle.sin().max(0.0).powf(2.0) * 100000.0;

        // or from000.looking_at()
        light_trans.rotation = Quat::from_rotation_z(sun_angle) * Quat::from_rotation_y(PI / 2.3);
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
    if let Ok(cam_trans) = query_cam.get_single() {
        // let cam_trans = query_cam.single();
        let p = cam_trans.translation;
        let rot = cam_trans.rotation;
        let n = 0.03;
        let offset = vec3(0., 0., -0.5);
        gizmo.ray(p + rot * offset, Vec3::X * n, Color::RED);
        gizmo.ray(p + rot * offset, Vec3::Y * n, Color::GREEN);
        gizmo.ray(p + rot * offset, Vec3::Z * n, Color::BLUE);
    }

}

#[derive(Resource, Reflect)]
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

    // pub is_manipulating: bool,

}

impl Default for WorldInfo {
    fn default() -> Self {
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

            // is_manipulating: true,

        }
    }
}

// struct ClientSettings {
//     fov: f32,
//     server_list: Vec<>,
// }

#[derive(Resource)]
pub struct ClientInfo {
    pub disconnected_reason: String,  // todo: Clean at Connect. prevents re-show old reason.

    pub fov: f32,

    pub username: String,
    
    pub dbg_text: bool,
    pub dbg_menubar: bool,
    pub dbg_gizmo_all_loaded_chunks: bool,

    // ping. (full, client-time, server-time, client-time) in ms.
    pub ping: (u32, u64, u64, u64),

    // as same as SPacket::PlayerList. username, ping.
    pub playerlist: Vec<(String, u32)>,

    pub hud_padding: f32,

    pub brush_size: f32,
    pub brush_strength: f32,
    pub brush_shape: u16,
}

impl Default for ClientInfo {
    fn default() -> Self {
        Self {
            disconnected_reason: "none".into(),
            username: "User1".into(),
            ping: (0,0,0,0),
            fov: 85.,
            playerlist: Vec::new(),
            dbg_text: false,
            dbg_menubar: true,
            dbg_gizmo_all_loaded_chunks: false,

            hud_padding: 32.,

            brush_size: 4.,
            brush_strength: 0.8,
            brush_shape: 0,
        }
    }
}

#[derive(Component)]
struct Sun; // marker
