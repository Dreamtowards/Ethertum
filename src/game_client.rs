use std::f32::consts::{PI, TAU};

use bevy::{
    app::AppExit, ecs::{reflect, system::{CommandQueue, SystemParam}}, 
    input::mouse::MouseWheel, math::vec3, pbr::DirectionalLightShadowMap, prelude::*, 
    utils::HashSet, window::{CursorGrabMode, PresentMode, PrimaryWindow, WindowMode}
};
use bevy_renet::renet::{transport::NetcodeClientTransport, RenetClient};
use bevy_touch_stick::*;
use bevy_xpbd_3d::prelude::*;

#[cfg(feature = "target_native_os")]
use bevy_atmosphere::prelude::*;

use crate::{character_controller::{CharacterController, CharacterControllerCamera, CharacterControllerPlugin}, util::TimeIntervals};
use crate::item::{Inventory, ItemPlugin};
use crate::net::{CPacket, ClientNetworkPlugin, RenetClientHelper};
use crate::{ui::CurrentUI, voxel::ClientVoxelPlugin};



pub struct GameClientPlugin;

impl Plugin for GameClientPlugin {
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
            // use bevy_mod_billboard::prelude::*;
            // app.add_plugins(BillboardPlugin);
    
            // ShadowMap sizes
            app.insert_resource(DirectionalLightShadowMap { size: 512 });
    
            // SSAO
            // app.add_plugins(TemporalAntiAliasPlugin);
            // app.insert_resource(AmbientLight { brightness: 0.05, ..default() });
        }
        // .obj model loader.
        app.add_plugins(bevy_obj::ObjPlugin);
        app.insert_resource(GlobalVolume::new(1.0));  // Audio GlobalVolume

        // Physics
        app.add_plugins(PhysicsPlugins::default());

        // UI
        app.add_plugins(crate::ui::UiPlugin);

        // CharacterController
        app.add_plugins(CharacterControllerPlugin);

        // Voxel
        app.add_plugins(ClientVoxelPlugin);

        // Items
        app.add_plugins(ItemPlugin);

        // Network Client
        app.add_plugins(ClientNetworkPlugin);

        // ClientInfo
        app.insert_resource(ClientInfo::default());
        app.register_type::<ClientInfo>();
        app.register_type::<WorldInfo>();

        // World Setup/Cleanup, Tick
        app.add_systems(First, on_world_init.run_if(condition::load_world));  // Camera, Player, Sun
        app.add_systems(Last, on_world_exit.run_if(condition::unload_world()));
        app.add_systems(Update, tick_world.run_if(condition::in_world));  // Sun, World Timing.
        
        app.add_systems(Update, handle_inputs); // toggle: PauseGameControl, Fullscreen

        // App Init/Exit
        app.add_systems(PreStartup, on_app_init);  // load settings
        app.add_systems(Last, on_app_exit);  // save settings


        // Debug
        {
            // Draw Basis
            app.add_systems(PostUpdate, debug_draw_gizmo.after(PhysicsSet::Sync).run_if(condition::in_world));
            
            // World Inspector
            app.add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new().run_if(|cli: Res<ClientInfo>| cli.dbg_inspector));
        }

        // Input
        {
            app.add_plugins((
                bevy_touch_stick::TouchStickPlugin::<InputStickId>::default(),
                leafwing_input_manager::plugin::InputManagerPlugin::<InputAction>::default(),
            ));
        }
    }
}










fn on_app_init(
    mut cli: ResMut<ClientInfo>,
) {
    info!("Loading {CLIENT_SETTINGS_FILE}");
    if let Ok(str) = std::fs::read_to_string(CLIENT_SETTINGS_FILE) {
        if let Ok(val) = serde_json::from_str(&str) {
            cli.cfg = val;
        }
    }
}

fn on_app_exit(
    mut exit_events: EventReader<AppExit>,
    cli: Res<ClientInfo>,
) {
    for _ in exit_events.read() {
        info!("Program Terminate");
        
        info!("Saving {CLIENT_SETTINGS_FILE}");
        std::fs::write(CLIENT_SETTINGS_FILE, serde_json::to_string(&cli.cfg).unwrap()).unwrap();   
    }
}



pub mod condition {
    use bevy::ecs::{change_detection::DetectChanges, schedule::{common_conditions::resource_removed, State}, system::Res};
    use crate::ui::CurrentUI;
    use super::{ClientInfo, WorldInfo};

    // a.k.a. loaded_world
    pub fn in_world(res: Option<Res<WorldInfo>>, res_vox: Option<Res<crate::voxel::ClientChunkSystem>>) -> bool {
        res.is_some() && res_vox.is_some()
    }
    pub fn load_world(res: Option<Res<WorldInfo>>) -> bool {
        res.is_some_and(|r|r.is_added())
    }
    pub fn unload_world() -> impl FnMut(Option<Res<WorldInfo>>) -> bool + Clone {
        resource_removed::<WorldInfo>()
    }
    pub fn manipulating(cli: Res<ClientInfo>) -> bool {
        cli.curr_ui == CurrentUI::None
    }
    pub fn in_ui(ui: CurrentUI) -> impl FnMut(Res<ClientInfo>) -> bool + Clone {
        move |cli: Res<ClientInfo>| {
            cli.curr_ui == ui
        }
    }
}





/// Marker: Despawn the Entity on World Unload.
#[derive(Component)]
pub struct DespawnOnWorldUnload;

// Marker: Sun
#[derive(Component)]
struct Sun;

fn on_world_init(
    mut cmds: Commands,
    asset_server: Res<AssetServer>,
    // mut materials: ResMut<Assets<StandardMaterial>>,
    // mut meshes: ResMut<Assets<Mesh>>,
    mut cli: ResMut<ClientInfo>,
) {
    info!("Load World. setup Player, Camera, Sun.");

    // Camera
    cmds.spawn((
        Camera3dBundle {
            projection: Projection::Perspective(PerspectiveProjection { fov: TAU / 4.6, ..default() }),
            camera: Camera { hdr: true, ..default() },
            ..default()
        },

        #[cfg(feature = "target_native_os")]
        AtmosphereCamera::default(), // Marks camera as having a skybox, by default it doesn't specify the render layers the skybox can be seen on

        FogSettings {
            // color, falloff shoud be set in ClientInfo.sky_fog_visibility, etc. due to dynamic debug reason.
            // falloff: FogFalloff::Atmospheric { extinction: Vec3::ZERO, inscattering:  Vec3::ZERO },  // mark as Atmospheric. value will be re-set by ClientInfo.sky_fog...
            ..default()
        },

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
                shadows_enabled: cli.skylight_shadow,
                ..default()
            },
            ..default()
        },
        Sun, // Marks the light as Sun
        Name::new("Sun"),
        DespawnOnWorldUnload,
    ));


    // TouchStick  Move-Left
    cmds.spawn((
        Name::new("InputStickMove"),
        DespawnOnWorldUnload,
        // map this stick as a left gamepad stick (through bevy_input)
        // leafwing will register this as a normal gamepad
        TouchStickGamepadMapping::LEFT_STICK,
        TouchStickUiBundle {
            stick: TouchStick {
                id: InputStickId::LeftMove,
                stick_type: TouchStickType::Fixed,
                ..default()
            },
            // configure the interactable area through bevy_ui
            style: Style {
                width: Val::Px(150.),
                height: Val::Px(150.),
                position_type: PositionType::Absolute,
                left: Val::Percent(15.),
                bottom: Val::Percent(5.),
                ..default()
            },
            ..default()
        },
    )).with_children(|parent| {
        parent.spawn((
            TouchStickUiKnob,
            ImageBundle {
                image: asset_server.load("knob.png").into(),
                style: Style {
                    width: Val::Px(75.),
                    height: Val::Px(75.),
                    ..default()
                },
                ..default()
            },
        ));
        parent.spawn((
            TouchStickUiOutline,
            ImageBundle {
                image: asset_server.load("outline.png").into(),
                style: Style {
                    width: Val::Px(150.),
                    height: Val::Px(150.),
                    ..default()
                },
                ..default()
            },
        ));
    });

    // spawn a look stick
    cmds.spawn((
        Name::new("InputStickLook"),
        DespawnOnWorldUnload,
        // map this stick as a right gamepad stick (through bevy_input)
        // leafwing will register this as a normal gamepad
        TouchStickGamepadMapping::RIGHT_STICK,
        TouchStickUiBundle {
            stick: TouchStick {
                id: InputStickId::RightLook,
                stick_type: TouchStickType::Floating,
                ..default()
            },
            // configure the interactable area through bevy_ui
            style: Style {
                width: Val::Px(150.),
                height: Val::Px(150.),
                position_type: PositionType::Absolute,
                right: Val::Percent(15.),
                bottom: Val::Percent(5.),
                ..default()
            },
            ..default()
        },
    )).with_children(|parent| {
        parent.spawn((
            TouchStickUiKnob,
            ImageBundle {
                image: asset_server.load("knob.png").into(),
                style: Style {
                    width: Val::Px(75.),
                    height: Val::Px(75.),
                    ..default()
                },
                ..default()
            },
        ));
        parent.spawn((
            TouchStickUiOutline,
            ImageBundle {
                image: asset_server.load("outline.png").into(),
                style: Style {
                    width: Val::Px(150.),
                    height: Val::Px(150.),
                    ..default()
                },
                ..default()
            },
        ));
    });

}

fn on_world_exit(
    mut cmds: Commands,
    query_despawn: Query<Entity, With<DespawnOnWorldUnload>>,
) {
    info!("Unload World");

    for entity in query_despawn.iter() {
        cmds.entity(entity).despawn_recursive();
    }
    
    cmds.remove_resource::<RenetClient>();
    cmds.remove_resource::<NetcodeClientTransport>();
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
}




fn handle_inputs(
    key: Res<ButtonInput<KeyCode>>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut query_window: Query<&mut Window, With<PrimaryWindow>>,
    mut query_controller: Query<&mut CharacterController>,

    mut worldinfo: Option<ResMut<WorldInfo>>,
    mut cli: ResMut<ClientInfo>,
) {
    let mut window = query_window.single_mut();

    if key.just_pressed(KeyCode::Escape) {
        if worldinfo.is_some() {
            cli.curr_ui = if cli.curr_ui == CurrentUI::None { CurrentUI::PauseMenu } else { CurrentUI::None };
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
    if curr_manipulating && key.just_pressed(KeyCode::Comma) {
        cli.enable_cursor_look = !cli.enable_cursor_look;
    }

    if curr_manipulating {
        let wheel_delta = mouse_wheel_events.read().fold(0.0, |acc, v| acc+v.x+v.y);

        cli.hotbar_index = (cli.hotbar_index as i32 + -wheel_delta as i32).rem_euclid(HOTBAR_SLOTS as i32) as u32;
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
    if key.just_pressed(KeyCode::F11) || (key.pressed(KeyCode::AltLeft) && key.just_pressed(KeyCode::Enter)) {
        window.mode = if window.mode != WindowMode::Fullscreen {
            WindowMode::Fullscreen
        } else {
            WindowMode::Windowed
        };
    }
    // Vsync
    window.present_mode = if cli.vsync { PresentMode::AutoVsync } else { PresentMode::AutoNoVsync };

    unsafe {
        crate::ui::_WINDOW_SIZE = Vec2::new(window.resolution.width(), window.resolution.height()); 
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

    mut query_fog: Query<&mut FogSettings>,
    cli: Res<ClientInfo>,
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

    // Ping Network
    if time.at_interval(1.0) {
        net_client.send_packet(&CPacket::Ping { client_time: crate::util::current_timestamp_millis(), last_rtt: cli.ping.0 as u32 });
    }

    // Fog
    let mut fog = query_fog.single_mut();
    fog.color = cli.sky_fog_color;
    if cli.sky_fog_is_atomspheric {  // let FogFalloff::Atmospheric { .. } = fog.falloff {
        fog.falloff = FogFalloff::from_visibility_colors(cli.sky_fog_visibility, cli.sky_extinction_color, cli.sky_inscattering_color);
    } else {
        fog.falloff = FogFalloff::from_visibility_squared(cli.sky_fog_visibility / 4.0);
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

fn debug_draw_gizmo(
    mut gizmo: Gizmos, 
    // mut gizmo_config: ResMut<GizmoConfigStore>, 
    query_cam: Query<&Transform, With<CharacterControllerCamera>>
) {
    // gizmo.config.depth_bias = -1.; // always in front

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








/// the resource only exixts when world is loaded

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



// ClientSettings Configs


#[derive(serde::Deserialize, serde::Serialize, Clone, Default)]
pub struct ServerListItem {
    pub name: String,
    pub addr: String,
    
    #[serde(skip)]
    pub motd: String,
    #[serde(skip)]
    pub num_players_online: u32,
    #[serde(skip)]
    pub num_players_limit: u32,
    #[serde(skip)]
    pub ping: u32,
}

const CLIENT_SETTINGS_FILE: &str = "client.settings.json";

#[derive(serde::Deserialize, serde::Serialize, Asset, TypePath, Clone)]
pub struct ClientSettings {
    // Name, Addr
    pub serverlist: Vec<ServerListItem>,
    pub fov: f32,
    pub username: String,
    pub hud_padding: f32,
}

impl Default for ClientSettings {
    fn default() -> Self {
        Self {
            serverlist: Vec::default(),
            fov: 85.,
            username: crate::util::generate_simple_user_name(),
            hud_padding: 24.,
        }
    }
}






pub const HOTBAR_SLOTS: u32 = 9;


#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct ClientInfo {
    // Networking
    pub server_addr: String,  // just a record
    pub disconnected_reason: String,
    pub ping: (u64, i64, i64, u64),  // ping. (rtt, c2s, ping-begin) in ms.
    pub playerlist: Vec<(String, u32)>,  // as same as SPacket::PlayerList. username, ping.

    // Debug Draw
    pub dbg_text: bool,
    pub dbg_menubar: bool,
    pub dbg_inspector: bool,
    pub dbg_gizmo_remesh_chunks: bool,
    pub dbg_gizmo_curr_chunk: bool,
    pub dbg_gizmo_all_loaded_chunks: bool,

    // Voxel Brush
    pub brush_size: f32,
    pub brush_strength: f32,
    pub brush_shape: u16,
    pub brush_tex: u16,

    pub max_concurrent_meshing: usize,
    pub chunks_meshing: HashSet<IVec3>,


    // Render Sky
    pub sky_fog_color: Color,
    pub sky_fog_visibility: f32,  
    pub sky_inscattering_color: Color,
    pub sky_extinction_color: Color,
    pub sky_fog_is_atomspheric: bool,
    pub skylight_shadow: bool,

    pub vsync: bool,

    #[reflect(ignore)]
    pub cfg: ClientSettings,

    // Control
    pub enable_cursor_look: bool,
    
    // ClientPlayerInfo
    #[reflect(ignore)]
    pub inventory: Inventory,

    pub hotbar_index: u32,

    pub health: u32,
    pub health_max: u32,

    // UI
    #[reflect(ignore)]
    pub curr_ui: CurrentUI,

    pub ui_scale: f32,
}

impl Default for ClientInfo {
    fn default() -> Self {
        Self {
            disconnected_reason: String::new(),
            ping: (0,0,0,0),
            playerlist: Vec::new(),
            server_addr: String::new(),

            dbg_text: false,
            dbg_menubar: true,
            dbg_inspector: false,
            dbg_gizmo_remesh_chunks: true,
            dbg_gizmo_curr_chunk: false,
            dbg_gizmo_all_loaded_chunks: false,

            brush_size: 4.,
            brush_strength: 0.8,
            brush_shape: 0,
            brush_tex: 21,

            max_concurrent_meshing: 8,
            chunks_meshing: HashSet::default(),

            vsync: true,

            sky_fog_color: Color::rgba(0.0, 0.666, 1.0, 1.0),
            sky_fog_visibility: 1200.0,  // 280 for ExpSq, 1200 for Atmo
            sky_fog_is_atomspheric: true,
            sky_inscattering_color: Color::rgb(110.0 / 255.0, 230.0 / 255.0, 1.0),  // bevy demo: Color::rgb(0.7, 0.844, 1.0),
            sky_extinction_color: Color::rgb(0.35, 0.5, 0.66),

            skylight_shadow: false,

            cfg: ClientSettings::default(),
            
            enable_cursor_look: true,

            inventory: Inventory::new(36),
            hotbar_index: 0,
            health: 20,
            health_max: 20,

            curr_ui: CurrentUI::MainMenu,
            ui_scale: 1.0,
        }
    }
}













// A helper on Client

#[derive(SystemParam)]
pub struct EthertiaClient<'w,'s> {

    clientinfo: ResMut<'w, ClientInfo>,

    cmds: Commands<'w,'s>,
}

impl<'w,'s> EthertiaClient<'w,'s> {

    /// for Singleplayer
    // pub fn load_world(&mut self, cmds: &mut Commands, server_addr: String)

    pub fn data(&mut self) -> &mut ClientInfo {
        self.clientinfo.as_mut()
    }

    pub fn connect_server(&mut self, server_addr: String) {
        info!("Connecting to {}", server_addr);
        self.clientinfo.server_addr = server_addr.clone();

        let mut net_client = RenetClient::new(bevy_renet::renet::ConnectionConfig::default());
        
        let username = &self.clientinfo.cfg.username;
        net_client.send_packet(&CPacket::Login { uuid: crate::util::hashcode(username), access_token: 123, username: username.clone() });

        self.cmds.insert_resource(net_client);
        self.cmds.insert_resource(crate::net::new_netcode_client_transport(server_addr.trim().parse().unwrap(), Some("userData123".to_string().into_bytes())));
        
        // clear DisconnectReason on new connect, to prevents display old invalid reason.
        self.clientinfo.disconnected_reason.clear();

        // 提前初始化世界 以防用资源时 发现没有被初始化
        self.cmds.insert_resource(WorldInfo::default());

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
