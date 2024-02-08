
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

        // ServerInfo
        // app.insert_resource(ServerInfo::default());
        
    }
}




#[derive(Resource, Default)]
pub struct ServerInfo {
    // PlayerList
    pub online_players: HashMap<ClientId, PlayerInfo>,
}

impl ServerInfo {


}


pub struct PlayerInfo {
    pub username: String,
    pub user_id: u64,

    pub entity_id: EntityId,
    pub position: Vec3,

    pub chunks_load_distance: IVec2,

    pub chunks_loaded: HashSet<IVec3>,
}

impl PlayerInfo {

    fn update(&self) {



    }

}