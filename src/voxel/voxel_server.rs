
use std::sync::{Arc, RwLock};
use bevy::{prelude::*, utils::{HashMap}};

use crate::util::iter;
use super::{Chunk, ChunkPtr, ChunkSystem, MpscRx, MpscTx, WorldGen};


type ChunkLoadingData = (IVec3, ChunkPtr);


pub struct ServerVoxelPlugin;

impl Plugin for ServerVoxelPlugin {
    fn build(&self, app: &mut App) {
        
        app.insert_resource(ServerChunkSystem::new());

        // {
        //     let (tx, rx) = crate::channel_impl::unbounded::<ChunkLoadingData>();
        //     app.insert_resource(MpscTx(tx));
        //     app.insert_resource(MpscRx(rx));
        // }

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

impl ChunkSystem for ServerChunkSystem {
    fn get_chunks(&self) -> &HashMap<IVec3, ChunkPtr> {
        &self.chunks
    }
}

impl ServerChunkSystem {
    fn new() -> Self {
        Self {
            chunks: HashMap::default(),
        }
    }

    fn spawn_chunk(&mut self, chunkptr: ChunkPtr) {
        let cp = chunkptr.read().unwrap().chunkpos;
        self.chunks.insert(cp, chunkptr);
    }
}