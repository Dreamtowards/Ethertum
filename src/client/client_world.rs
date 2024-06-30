use std::f32::consts::PI;

use bevy_renet::renet::transport::NetcodeClientTransport;
use bevy_renet::renet::RenetClient;

use crate::client::prelude::*;
use crate::net::{CPacket, RenetClientHelper};
use crate::prelude::*;
use crate::util::TimeIntervals;

pub fn init(app: &mut App) {
    app.register_type::<WorldInfo>();

    app.insert_resource(ClientPlayerInfo::default());
    app.register_type::<ClientPlayerInfo>();

    // World Setup/Cleanup, Tick
    app.add_systems(First, on_world_init.run_if(condition::load_world)); // Camera, Player, Sun
    app.add_systems(Last, on_world_exit.run_if(condition::unload_world()));
    app.add_systems(Update, tick_world.run_if(condition::in_world)); // Sun, World Timing.
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct ClientPlayerInfo {
    #[reflect(ignore)]
    pub inventory: Inventory,

    pub hotbar_index: u32,

    pub health: u32,
    pub health_max: u32,
}

impl ClientPlayerInfo {
    pub const HOTBAR_SLOTS: u32 = 9;
}

impl Default for ClientPlayerInfo {
    fn default() -> Self {
        Self {
            inventory: Inventory::new(36),
            hotbar_index: 0,
            health: 20,
            health_max: 20,
        }
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

/// Marker: Despawn the Entity on World Unload.
#[derive(Component)]
pub struct DespawnOnWorldUnload;

// Marker: Sun
#[derive(Component)]
struct Sun;

fn on_world_init(
    mut cmds: Commands,
    // asset_server: Res<AssetServer>,
    // materials: ResMut<Assets<StandardMaterial>>,
    // meshes: ResMut<Assets<Mesh>>,
    // cli: ResMut<ClientInfo>,
) {
    info!("Load World. setup Player, Camera, Sun.");

    // crate::net::netproc_client::spawn_player(
    //     &mut cmds.spawn_empty(),
    //     true,
    //     &cli.cfg.username, &asset_server, &mut meshes, &mut materials);

    // Camera
    cmds.spawn((
        Camera3dBundle {
            // projection: Projection::Perspective(PerspectiveProjection { fov: TAU / 4.6, ..default() }),
            // camera: Camera { hdr: true, ..default() },
            ..default()
        },
        #[cfg(feature = "target_native_os")]
        bevy_atmosphere::plugin::AtmosphereCamera::default(), // Marks camera as having a skybox, by default it doesn't specify the render layers the skybox can be seen on
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
            directional_light: DirectionalLight { ..default() },
            ..default()
        },
        Sun, // Marks the light as Sun
        Name::new("Sun"),
        DespawnOnWorldUnload,
    ));
}

fn on_world_exit(mut cmds: Commands, query_despawn: Query<Entity, With<DespawnOnWorldUnload>>) {
    info!("Unload World");

    for entity in query_despawn.iter() {
        cmds.entity(entity).despawn_recursive();
    }

    // todo: net_client.disconnect();  即时断开 否则服务器会觉得你假死 对其他用户体验不太好
    cmds.remove_resource::<RenetClient>();
    cmds.remove_resource::<NetcodeClientTransport>();
}

fn tick_world(
    #[cfg(feature = "target_native_os")] mut atmosphere: bevy_atmosphere::system_param::AtmosphereMut<bevy_atmosphere::prelude::Nishita>,
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

        if player_pos.distance_squared(*last_player_pos) > 0.01 * 0.01 {
            *last_player_pos = player_pos;
            net_client.send_packet(&CPacket::PlayerPos { position: player_pos });
        }
    }
    // net_client.send_packet(&CPacket::LoadDistance {
    //     load_distance: cli.chunks_load_distance,
    // }); // todo: Only Send after Edit Dist Config

    // Ping Network
    if time.at_interval(1.0) {
        net_client.send_packet(&CPacket::Ping {
            client_time: crate::util::current_timestamp_millis(),
            last_rtt: cli.ping.0 as u32,
        });
    }

    // Fog
    let mut fog = query_fog.single_mut();
    fog.color = cli.sky_fog_color;
    if cli.sky_fog_is_atomspheric {
        // let FogFalloff::Atmospheric { .. } = fog.falloff {
        fog.falloff = FogFalloff::from_visibility_colors(cli.sky_fog_visibility, cli.sky_extinction_color, cli.sky_inscattering_color);
    } else {
        fog.falloff = FogFalloff::from_visibility_squared(cli.sky_fog_visibility / 4.0);
    }

    // Sun Pos
    let sun_angle = worldinfo.daytime * PI * 2.;

    // if !time.at_interval(0.5) {
    //     return;
    // }
    #[cfg(feature = "target_native_os")]
    {
        atmosphere.sun_position = Vec3::new(sun_angle.cos(), sun_angle.sin(), 0.);
    }

    if let Some((mut light_trans, mut directional)) = query_sun.single_mut().into() {
        directional.illuminance = sun_angle.sin().max(0.0).powf(2.0) * cli.skylight_illuminance * 1000.0;
        directional.shadows_enabled = cli.skylight_shadow;

        // or from000.looking_at()
        light_trans.rotation = Quat::from_rotation_z(sun_angle) * Quat::from_rotation_y(PI / 2.3);
    }
}
