
use bevy::prelude::*;
use bevy_xpbd_3d::plugins::PhysicsPlugins;

use crate::{net::ServerNetworkPlugin, voxel::ServerVoxelPlugin};



pub struct GameServerPlugin;

impl Plugin for GameServerPlugin {
    fn build(&self, app: &mut App) {

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