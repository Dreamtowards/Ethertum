use std::net::ToSocketAddrs;

use bevy::{ecs::system::SystemParam, math::vec3, pbr::DirectionalLightShadowMap, prelude::*};
use bevy_renet::renet::RenetClient;
use avian3d::prelude::*;

#[cfg(feature = "target_native_os")]
use bevy_atmosphere::prelude::*;

use crate::client::prelude::*;
use crate::item::ItemPlugin;
use crate::net::{CPacket, ClientNetworkPlugin, RenetClientHelper};
use crate::server::prelude::IntegratedServerPlugin;
use crate::ui::prelude::*;
use crate::voxel::ClientVoxelPlugin;

pub struct ClientGamePlugin;

impl Plugin for ClientGamePlugin {
    fn build(&self, app: &mut App) {
        // Render
        {
            // Atmosphere
            #[cfg(feature = "target_native_os")]
            {
                app.add_plugins(AtmospherePlugin);
                app.insert_resource(AtmosphereModel::default());
            }

            // for SSR
            //app.insert_resource(Msaa::Off);
            app.insert_resource(bevy::pbr::DefaultOpaqueRendererMethod::deferred());

            // Billiboard
            // use bevy_mod_billboard::prelude::*;
            // app.add_plugins(BillboardPlugin);

            // ShadowMap sizes
            app.insert_resource(DirectionalLightShadowMap { size: 1024 });

            // SSAO
            // app.add_plugins(TemporalAntiAliasPlugin);
            // app.insert_resource(AmbientLight { brightness: 0.05, ..default() });
        }
        // .obj model loader.
        app.add_plugins(bevy_obj::ObjPlugin);
        app.insert_resource(GlobalVolume::new(bevy::audio::Volume::Linear(1.0))); // Audio GlobalVolume

        // Physics
        app.add_plugins(PhysicsPlugins::default());

        // UI
        app.add_plugins(crate::ui::UiPlugin);

        // Gameplay
        app.add_plugins(CharacterControllerPlugin); // CharacterController
        app.add_plugins(ClientVoxelPlugin); // Voxel
        app.add_plugins(ItemPlugin); // Items

        // Network
        app.add_plugins(ClientNetworkPlugin); // Client Network
        app.add_plugins(IntegratedServerPlugin);

        // ClientInfo
        app.insert_resource(ClientInfo::default());
        app.register_type::<ClientInfo>();

        super::settings::build_plugin(app); // Config
        super::input::init(app); // Input

        // World
        super::client_world::init(app);

        // Debug
        {
            // app.add_systems(Update, wfc_test);

            // Draw Basis
            app.add_systems(PostUpdate, debug_draw_gizmo.in_set(PhysicsSet::Sync).run_if(condition::in_world));

            // World Inspector
            app.add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new().run_if(|cli: Res<ClientInfo>| cli.dbg_inspector));
        }
    }
}

pub mod condition {
    use crate::client::prelude::*;
    use bevy::ecs::{change_detection::DetectChanges, schedule::common_conditions::resource_removed, system::Res};

    // a.k.a. loaded_world
    pub fn in_world(res: Option<Res<WorldInfo>>, res_vox: Option<Res<crate::voxel::ClientChunkSystem>>) -> bool {
        res.is_some() && res_vox.is_some()
    }
    pub fn load_world(res: Option<Res<WorldInfo>>) -> bool {
        res.is_some_and(|r| r.is_added())
    }
    pub fn unload_world() -> impl FnMut(Option<Res<WorldInfo>>, bevy::prelude::Local<bool>) -> bool + Clone {
        resource_removed::<WorldInfo>
    }
    pub fn manipulating(cli: Res<ClientInfo>) -> bool {
        cli.curr_ui == CurrentUI::None
    }
    pub fn in_ui(ui: CurrentUI) -> impl FnMut(Res<ClientInfo>) -> bool + Clone {
        move |cli: Res<ClientInfo>| cli.curr_ui == ui
    }
}

fn debug_draw_gizmo(
    mut gizmo: Gizmos,
    // mut gizmo_config: ResMut<GizmoConfigStore>,
    query_cam: Query<&Transform, With<CharacterControllerCamera>>,
) {
    // gizmo.config.depth_bias = -1.; // always in front

    // World Basis Axes
    let n = 5;
    gizmo.line(Vec3::ZERO, Vec3::X * 2. * n as f32, Srgba::RED);
    gizmo.line(Vec3::ZERO, Vec3::Y * 2. * n as f32, Srgba::GREEN);
    gizmo.line(Vec3::ZERO, Vec3::Z * 2. * n as f32, Srgba::BLUE);

    let color = Srgba::gray(0.4);
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
        gizmo.ray(p + rot * offset, Vec3::X * n, Srgba::RED);
        gizmo.ray(p + rot * offset, Vec3::Y * n, Srgba::GREEN);
        gizmo.ray(p + rot * offset, Vec3::Z * n, Srgba::BLUE);
    }
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct ClientInfo {
    // Networking
    pub server_addr: String, // just a record
    pub disconnected_reason: String,
    pub ping: (u64, i64, i64, u64),     // ping. (rtt, c2s, ping-begin) in ms.
    pub playerlist: Vec<(String, u32)>, // as same as SPacket::PlayerList. username, ping.

    // Debug Draw
    pub dbg_text: bool,
    pub dbg_menubar: bool,
    pub dbg_inspector: bool,
    pub dbg_gizmo_remesh_chunks: bool,
    pub dbg_gizmo_curr_chunk: bool,
    pub dbg_gizmo_all_loaded_chunks: bool,

    // Render Sky
    pub sky_fog_color: Color,
    pub sky_fog_visibility: f32,
    pub sky_inscattering_color: Color,
    pub sky_extinction_color: Color,
    pub sky_fog_is_atomspheric: bool,
    pub skylight_shadow: bool,
    pub skylight_illuminance: f32,

    // Control
    pub enable_cursor_look: bool,

    // UI
    #[reflect(ignore)]
    pub curr_ui: CurrentUI,
}

impl Default for ClientInfo {
    fn default() -> Self {
        Self {
            disconnected_reason: String::new(),
            ping: (0, 0, 0, 0),
            playerlist: Vec::new(),
            server_addr: String::new(),

            dbg_text: false,
            dbg_menubar: true,
            dbg_inspector: false,
            dbg_gizmo_remesh_chunks: true,
            dbg_gizmo_curr_chunk: false,
            dbg_gizmo_all_loaded_chunks: false,

            sky_fog_color: Color::srgba(0.0, 0.666, 1.0, 1.0),
            sky_fog_visibility: 1200.0, // 280 for ExpSq, 1200 for Atmo
            sky_fog_is_atomspheric: true,
            sky_inscattering_color: Color::srgb(110.0 / 255.0, 230.0 / 255.0, 1.0), // bevy demo: Color::rgb(0.7, 0.844, 1.0),
            sky_extinction_color: Color::srgb(0.35, 0.5, 0.66),

            skylight_shadow: true,
            skylight_illuminance: 20.,

            enable_cursor_look: true,

            curr_ui: CurrentUI::MainMenu,
        }
    }
}

// A helper on Client

#[derive(SystemParam)]
pub struct EthertiaClient<'w, 's> {
    clientinfo: ResMut<'w, ClientInfo>,
    pub cfg: ResMut<'w, ClientSettings>,

    cmds: Commands<'w, 's>,
}

impl<'w, 's> EthertiaClient<'w, 's> {
    /// for Singleplayer
    // pub fn load_world(&mut self, cmds: &mut Commands, server_addr: String)

    pub fn data(&mut self) -> &mut ClientInfo {
        self.clientinfo.as_mut()
    }

    pub fn connect_server(&mut self, server_addr: String) {
        info!("Connecting to {}", server_addr);

        let mut addrs = match server_addr.trim().to_socket_addrs() {
            Ok(addrs) => addrs.collect::<Vec<_>>(),
            Err(err) => {
                error!("Failed to resolve DNS of server_addr: {}", err);
                self.data().curr_ui = CurrentUI::DisconnectedReason;
                return;
            }
        };
        let addr = match addrs.pop() {
            Some(addr) => addr,
            None => {
                self.data().curr_ui = CurrentUI::DisconnectedReason;
                return;
            }
        };

        self.data().curr_ui = CurrentUI::ConnectingServer;
        self.clientinfo.server_addr.clone_from(&server_addr);

        let mut net_client = RenetClient::new(bevy_renet::renet::ConnectionConfig::default());

        let username = &self.cfg.username;
        net_client.send_packet(&CPacket::Login {
            uuid: crate::util::hashcode(username),
            access_token: 123,
            username: username.clone(),
        });

        self.cmds.insert_resource(net_client);
        self.cmds.insert_resource(crate::net::new_netcode_client_transport(
            addr,
            Some("userData123".to_string().into_bytes()),
        ));

        // clear DisconnectReason on new connect, to prevents display old invalid reason.
        self.clientinfo.disconnected_reason.clear();

        // 提前初始化世界 以防用资源时 发现没有被初始化
        self.cmds.insert_resource(WorldInfo::default());
    }

    pub fn enter_world(&mut self) {
        self.cmds.insert_resource(WorldInfo::default());
        self.data().curr_ui = CurrentUI::None;
    }

    pub fn exit_world(&mut self) {
        self.cmds.remove_resource::<WorldInfo>();
        self.data().curr_ui = CurrentUI::MainMenu;
    }
}
