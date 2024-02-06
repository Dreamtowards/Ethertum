
use std::sync::{Arc, RwLock};
use bevy::{prelude::*, utils::{HashMap}};

use crate::util::iter;
use super::{Chunk, ChunkPtr, WorldGen};

pub struct ServerVoxelPlugin;

impl Plugin for ServerVoxelPlugin {
    fn build(&self, app: &mut App) {
        
        app.insert_resource(ServerChunkSystem::new());


        app.add_systems(Update, chunk_loadance);

    }
}



fn chunk_loadance(
    mut chunk_sys: ResMut<ServerChunkSystem>,

) {

    iter::iter_aabb(3, 2, |p| {
        let chunkpos = *p * Chunk::SIZE;

        if chunk_sys.has_chunk(chunkpos) {
            return;
        }

        let mut chunk = Chunk::new(chunkpos);
        WorldGen::generate_chunk(&mut chunk);

        let chunkptr = Arc::new(RwLock::new(chunk));
        chunk_sys.spawn_chunk(chunkptr);
    });

}






#[derive(Resource)]
pub struct ServerChunkSystem {

    pub chunks: HashMap<IVec3, ChunkPtr>,
}

impl ServerChunkSystem {
    fn new() -> Self {
        Self {
            chunks: HashMap::default(),
        }
    }

    pub fn get_chunk(&self, chunkpos: IVec3) -> Option<&ChunkPtr> {
        assert!(Chunk::is_chunkpos(chunkpos));
        self.chunks.get(&chunkpos)
    }
    
    pub fn has_chunk(&self, chunkpos: IVec3) -> bool {
        assert!(Chunk::is_chunkpos(chunkpos));
        self.chunks.contains_key(&chunkpos) //.read().unwrap().contains_key(&chunkpos)
    }


    fn spawn_chunk(&mut self, chunkptr: ChunkPtr) {
        let cp = chunkptr.read().unwrap().chunkpos;
        self.chunks.insert(cp, chunkptr);
    }
}