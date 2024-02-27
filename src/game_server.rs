
use bevy::{prelude::*, utils::{HashMap, HashSet}};
use bevy_renet::renet::ClientId;
use bevy_xpbd_3d::plugins::PhysicsPlugins;

use crate::{net::{EntityId, ServerNetworkPlugin}, voxel::ServerVoxelPlugin};



pub struct GameServerPlugin;

impl Plugin for GameServerPlugin {
    fn build(&self, app: &mut App) {

        app.insert_resource(ServerInfo::default());

        // Network
        app.add_plugins(ServerNetworkPlugin);

        // ChunkSystem
        app.add_plugins(ServerVoxelPlugin);
        
        // Physics
        // app.add_plugins(PhysicsPlugins::default());

        
        app.add_systems(PreStartup, on_init);  // load settings.
        app.add_systems(Last, on_exit);  // save settings.
        
    }
}


fn on_init(
    mut serv: ResMut<ServerInfo>,
) {
    if let Err(err) = serv.load() {
        panic!("{}", err)
    }
}

fn on_exit(
    mut exit_events: EventReader<bevy::app::AppExit>,
    serv: ResMut<ServerInfo>,
) {
    for _ in exit_events.read() {

    }
}



const SERVER_SETTINGS_FILE: &str = "server.settings.json";

#[derive(serde::Deserialize, serde::Serialize, Asset, TypePath, Clone)]
pub struct ServerSettings {
    addr: String,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            addr: "0.0.0.0:4000".into(),
        }
    }
}

impl ServerSettings {
    pub fn addr(&self) -> &str {
        &self.addr
    }
}

#[derive(Resource, Default)]
pub struct ServerInfo {
    // PlayerList
    pub online_players: HashMap<ClientId, PlayerInfo>,
    cfg: ServerSettings,
}

impl ServerInfo {
    pub fn cfg(&self) -> &ServerSettings {
        &self.cfg
    }

    fn load(&mut self) -> anyhow::Result<()> {
        Ok(match std::fs::read_to_string(SERVER_SETTINGS_FILE) {
            Ok(s) => {
                info!("Loading server settings from {SERVER_SETTINGS_FILE}");
                
                self.cfg = serde_json::from_str(&s)?
            },
            Err(_) => {
                info!("Saving server settings to {SERVER_SETTINGS_FILE}");

                let s = serde_json::to_string_pretty(&self.cfg)?;
                std::fs::write(SERVER_SETTINGS_FILE, s)?;
            },
        })
    }
}


pub struct PlayerInfo {
    pub username: String,
    pub user_id: u64,

    pub client_id: ClientId,  // network client id. renet

    pub entity_id: EntityId,
    pub position: Vec3,

    pub chunks_load_distance: IVec2,

    pub chunks_loaded: HashSet<IVec3>,
}

impl PlayerInfo {

    fn update(&self) {



    }

}