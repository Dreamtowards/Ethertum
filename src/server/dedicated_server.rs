use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};
use bevy_renet::renet::ClientId;

use crate::{
    net::{EntityId, ServerNetworkPlugin},
    voxel::ServerVoxelPlugin,
};

pub struct DedicatedServerPlugin;

impl Plugin for DedicatedServerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ServerInfo::default());
        app.insert_resource(ServerSettings::default());

        // Network
        app.add_plugins(ServerNetworkPlugin);

        // ChunkSystem
        app.add_plugins(ServerVoxelPlugin);

        // Physics
        // app.add_plugins(PhysicsPlugins::default());

        app.add_systems(PreStartup, on_init); // load settings.
        app.add_systems(Last, on_exit); // save settings.

        let rcon_port = 8001;
        let http_server = tiny_http::Server::http(format!("0.0.0.0:{}", rcon_port)).unwrap();
        info!("Start RCON endpoint on {}", http_server.server_addr().to_ip().unwrap());
        app.insert_resource(rcon::HttpServer { server: http_server });
        app.add_systems(Update, rcon::on_http_recv);
    }
}

const SERVER_SETTINGS_FILE: &str = "server.settings.json";

fn on_init(mut cfg: ResMut<ServerSettings>) {
    info!("Loading server settings from {SERVER_SETTINGS_FILE}");

    if let Ok(c) = serde_json::from_str(&SERVER_SETTINGS_FILE) {
        *cfg = c;
    }
}

fn on_exit(mut exit_events: EventReader<bevy::app::AppExit>, cfg: Res<ServerSettings>) {
    for _ in exit_events.read() {
        info!("Saving server settings to {SERVER_SETTINGS_FILE}");

        std::fs::write(SERVER_SETTINGS_FILE, serde_json::to_string_pretty(&*cfg).unwrap()).unwrap();
    }
}

pub mod rcon {
    use super::*;

    #[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
    pub struct Motd {
        pub motd: String,
        pub game_addr: String,
        pub num_player_online: u32,
        pub num_player_limit: u32,
        pub protocol_version: u64,
        pub favicon_url: String,
    }

    #[derive(Resource)]
    pub struct HttpServer {
        pub server: tiny_http::Server,
    }

    pub fn on_http_recv(http: Res<HttpServer>, serv: Res<ServerInfo>) {
        if let Ok(Some(req)) = http.server.try_recv() {
            info!("Req URL: {}", req.url());
            let motd = Motd {
                motd: "An Ethertum Server".into(),
                num_player_limit: 80,
                num_player_online: 0,
                protocol_version: 0,
                favicon_url: "".into(),
                game_addr: "127.0.0.1:4000".into(),
            };
            req.respond(tiny_http::Response::from_string(serde_json::to_string(&motd).unwrap()))
                .unwrap();
        }
    }
}

#[derive(Resource, serde::Deserialize, serde::Serialize, Asset, TypePath, Clone)]
pub struct ServerSettings {
    pub port: u16,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self { port: 4060 }
    }
}

#[derive(Resource, Default)]
pub struct ServerInfo {
    // PlayerList
    pub online_players: HashMap<ClientId, PlayerInfo>,
}

pub struct PlayerInfo {
    pub username: String,
    pub user_id: u64,

    pub client_id: ClientId, // network client id. renet

    pub entity_id: EntityId,
    pub position: Vec3,
    pub ping_rtt: u32,

    pub chunks_load_distance: IVec2,

    pub chunks_loaded: HashSet<IVec3>,
}

impl PlayerInfo {
    // fn update(&self) {
    // }
}
