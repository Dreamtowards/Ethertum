use bevy::{
    prelude::*,
    platform::collections::{HashMap, HashSet},
};
use bevy_renet::renet::ClientId;
use std::{net::SocketAddr, path::Path};
use anyhow::Context;

use crate::{
    net::{EntityId, ServerNetworkPlugin},
    voxel::ServerVoxelPlugin,
};

pub struct DedicatedServerPlugin;

impl Plugin for DedicatedServerPlugin {
    fn build(&self, app: &mut App) {
        // Load before installing network startup systems: an invalid file must not
        // accidentally bind the default game port or get overwritten on exit.
        let settings = match ServerSettings::load(Path::new(SERVER_SETTINGS_FILE)) {
            Ok(settings) => settings,
            Err(error) => {
                error!("Server startup failed: {error:#}");
                app.add_systems(Startup, |mut exit: MessageWriter<AppExit>| {
                    exit.write(AppExit::error());
                });
                return;
            }
        };
        app.insert_resource(ServerInfo::default());
        app.insert_resource(settings);

        // Network
        app.add_plugins(ServerNetworkPlugin);

        // ChunkSystem
        app.add_plugins(ServerVoxelPlugin);

        // Physics
        // app.add_plugins(PhysicsPlugins::default());

        app.add_systems(Last, on_exit); // save settings.
        app.add_systems(Startup, bind_status_endpoint);
        app.add_systems(Update, rcon::on_http_recv.run_if(resource_exists::<rcon::HttpServer>));
    }
}

const SERVER_SETTINGS_FILE: &str = "server.settings.json";

fn bind_status_endpoint(mut cmds: Commands, cfg: Res<ServerSettings>) {
    if let Some(addr) = cfg.status_addr {
        match tiny_http::Server::http(addr) {
            Ok(server) => {
                info!("Start status endpoint on {}", server.server_addr());
                cmds.insert_resource(rcon::HttpServer { server });
            }
            Err(error) => warn!("Status endpoint {addr} disabled: {error}"),
        }
    }
}

fn on_exit(mut exit_events: MessageReader<bevy::app::AppExit>, cfg: Res<ServerSettings>) {
    for _ in exit_events.read() {
        info!("Saving server settings to {SERVER_SETTINGS_FILE}");

        if let Err(error) = cfg.validate().and_then(|_| crate::util::settings::save(Path::new(SERVER_SETTINGS_FILE), &*cfg)) {
            error!("Cannot save server settings: {error:#}");
        }
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

    pub fn on_http_recv(http: Res<HttpServer>, serv: Res<ServerInfo>, cfg: Res<ServerSettings>) {
        if let Ok(Some(req)) = http.server.try_recv() {
            info!("Req URL: {}", req.url());
            let motd = Motd {
                motd: cfg.motd.clone(),
                num_player_limit: cfg.num_player_limit,
                num_player_online: serv.online_players.len() as u32,
                protocol_version: crate::net::PROTOCOL_ID,
                favicon_url: "".into(),
                game_addr: format!(":{}", cfg.port),
            };
            match serde_json::to_string(&motd) {
                Ok(body) => {
                    if let Err(error) = req.respond(tiny_http::Response::from_string(body)) {
                        warn!("Failed to respond to status request: {}", error);
                    }
                }
                Err(error) => warn!("Failed to serialize server status: {}", error),
            }
        }
    }
}

#[derive(Resource, serde::Deserialize, serde::Serialize, Asset, TypePath, Clone)]
#[serde(default)]
pub struct ServerSettings {
    pub port: u16,
    pub num_player_limit: u32,
    pub motd: String,
    /// None disables the optional HTTP status endpoint. Missing fields keep the old address.
    pub status_addr: Option<SocketAddr>,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            port: 4060,
            num_player_limit: crate::net::MAX_CLIENTS,
            motd: "An Ethertum Server".into(),
            status_addr: Some(SocketAddr::from(([0, 0, 0, 0], 8001))),
        }
    }
}

impl ServerSettings {
    fn load(path: &Path) -> anyhow::Result<Self> {
        let settings: Self = crate::util::settings::load(path)?.unwrap_or_default();
        settings.validate().with_context(|| format!("Invalid settings in {}", path.display()))?;
        Ok(settings)
    }

    pub(crate) fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(self.port != 0, "Game port must not be zero");
        anyhow::ensure!((1..=crate::net::MAX_CLIENTS).contains(&self.num_player_limit), "Player limit must be in 1..={}", crate::net::MAX_CLIENTS);
        anyhow::ensure!(self.status_addr.is_none_or(|addr| addr.port() != 0), "Status port must not be zero");
        Ok(())
    }
}

#[cfg(test)]
mod settings_tests {
    use super::*;

    #[test]
    fn reads_settings_file_and_rejects_invalid_configuration() {
        let dir = std::env::temp_dir().join(format!("ethertia-server-settings-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("server.settings.json");
        assert_eq!(ServerSettings::load(&path).unwrap().num_player_limit, crate::net::MAX_CLIENTS);
        std::fs::write(&path, r#"{"port":4567,"num_player_limit":12,"motd":"From disk"}"#).unwrap();
        let cfg = ServerSettings::load(&path).unwrap();
        assert_eq!((cfg.port, cfg.num_player_limit), (4567, 12));
        assert_eq!(cfg.motd, "From disk");
        std::fs::write(&path, r#"{"num_player_limit":80}"#).unwrap();
        assert!(ServerSettings::load(&path).is_err());
        std::fs::write(&path, "{").unwrap();
        assert!(ServerSettings::load(&path).is_err());
        assert!(ServerSettings::load(&dir).is_err());
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn custom_settings_and_legacy_defaults_are_validated() {
        let cfg: ServerSettings = serde_json::from_str(r#"{"port":4567,"num_player_limit":12,"motd":"Custom"}"#).unwrap();
        cfg.validate().unwrap();
        assert_eq!(cfg.port, 4567);
        assert_eq!(cfg.num_player_limit, 12);
        assert_eq!(cfg.motd, "Custom");
        assert_eq!(cfg.status_addr.unwrap().port(), 8001);
        let disabled: ServerSettings = serde_json::from_str(r#"{"status_addr":null}"#).unwrap();
        assert!(disabled.status_addr.is_none());
        for limit in [0, crate::net::MAX_CLIENTS + 1] {
            assert!(ServerSettings { num_player_limit: limit, ..default() }.validate().is_err());
        }
        assert!(ServerSettings { port: 0, ..default() }.validate().is_err());
        assert!(ServerSettings { status_addr: Some(SocketAddr::from(([127, 0, 0, 1], 0))), ..default() }.validate().is_err());
    }

    #[test]
    fn occupied_game_port_is_reported_without_panicking() {
        let socket = std::net::UdpSocket::bind("0.0.0.0:0").unwrap();
        let result = crate::net::new_netcode_server_transport(socket.local_addr().unwrap().port(), 4);
        let Err(error) = result else { panic!("occupied port unexpectedly bound"); };
        assert!(error.to_string().contains("Cannot bind game endpoint"));
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
