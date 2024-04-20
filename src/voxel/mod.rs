mod chunk;
pub use chunk::{Cell, Chunk};

// mod chunk_system;
// pub use chunk_system::{ChunkPtr};

mod voxel_client;
mod voxel_server;
pub use voxel_client::{ClientChunkSystem, ClientVoxelPlugin, HitResult};
pub use voxel_server::{ServerChunkSystem, ServerVoxelPlugin};

mod material;
mod meshgen;
pub mod worldgen;
pub use worldgen::WorldGen;

use std::sync::Arc;
pub type ChunkPtr = Arc<Chunk>; // Box<Chunk>;         not supported for SharedPtr

use bevy::{prelude::*, utils::HashMap};

use crate::util::AsRefMut;

#[derive(Resource, Deref, Clone)]
struct ChannelTx<T>(crate::channel_impl::Sender<T>);

#[derive(Resource, Deref, Clone)]
struct ChannelRx<T>(crate::channel_impl::Receiver<T>);

#[derive(Component)]
pub struct ChunkComponent {
    pub chunkpos: IVec3,
}

impl ChunkComponent {
    pub fn new(chunkpos: IVec3) -> Self {
        Self { chunkpos }
    }
}

pub trait ChunkSystem {
    fn get_chunks(&self) -> &HashMap<IVec3, ChunkPtr>;

    fn get_chunk(&self, chunkpos: IVec3) -> Option<&ChunkPtr> {
        assert!(Chunk::is_chunkpos(chunkpos));
        self.get_chunks().get(&chunkpos)
    }

    fn has_chunk(&self, chunkpos: IVec3) -> bool {
        assert!(Chunk::is_chunkpos(chunkpos));
        self.get_chunks().contains_key(&chunkpos)
    }

    fn num_chunks(&self) -> usize {
        self.get_chunks().len()
    }

    fn get_cell(&self, p: IVec3) -> Option<Cell> {
        let chunkptr = self.get_chunk(Chunk::as_chunkpos(p))?;

        Some(*chunkptr.get_cell(Chunk::as_localpos(p)))
    }

    fn get_voxel(&self, p: IVec3) -> Option<&Cell> {
        let chunkptr = self.get_chunk(Chunk::as_chunkpos(p))?;

        Some(chunkptr.get_cell(Chunk::as_localpos(p)))
    }

    fn set_voxel(&mut self, p: IVec3, v: &Cell) -> Option<()> {
        let chunkptr = self.get_chunk(Chunk::as_chunkpos(p))?;

        chunkptr.as_ref_mut().set_cell(Chunk::as_localpos(p), v);
        Some(())
    }
}
