use rand::Rng;

use crate::{net::ServerNetworkPlugin, prelude::*, voxel::ServerVoxelPlugin};

use super::prelude::{ServerInfo, ServerSettings};

pub struct IntegratedServerPlugin;

impl Plugin for IntegratedServerPlugin {
    fn build(&self, app: &mut App) {

        app.insert_resource(ServerInfo::default());
        app.insert_resource(ServerSettings {
            port: 6000 + rand::thread_rng().gen_range(0..6000),
            ..default()
        });

        // Network
        app.add_plugins(ServerNetworkPlugin);

        // ChunkSystem
        app.add_plugins(ServerVoxelPlugin);


    }
}
