
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
    info!("Loading server settings from {SERVER_SETTINGS_FILE}");
    if let Ok(str) = std::fs::read_to_string(SERVER_SETTINGS_FILE) {
        if let Ok(val) = serde_json::from_str(&str) {
            serv.cfg = val;
        }
    }
}

fn on_exit(
    mut exit_events: EventReader<bevy::app::AppExit>,
    mut serv: ResMut<ServerInfo>,
) {
    for _ in exit_events.read() {
        
        info!("Saving server settings to {SERVER_SETTINGS_FILE}");
        std::fs::write(SERVER_SETTINGS_FILE, serde_json::to_string(&serv.cfg).unwrap()).unwrap();   
    }
}



const SERVER_SETTINGS_FILE: &str = "./server.json";

#[derive(Deserialize, Serialize, Asset, TypePath, Clone)]
pub struct ServerSettings {
    
    pub addr: String,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            addr: "0.0.0.0:4000".into(),
        }
    }
}


#[derive(Resource, Default)]
pub struct ServerInfo {
    // PlayerList
    pub online_players: HashMap<ClientId, PlayerInfo>,

    pub cfg: ServerSettings,
}

impl ServerInfo {


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