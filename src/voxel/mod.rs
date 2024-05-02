mod chunk;
mod meshgen;
mod voxel_client;
mod voxel_server;
pub mod worldgen;

pub use chunk::{Vox, Chunk, VoxShape};
pub use voxel_client::{ClientChunkSystem, ClientVoxelPlugin, HitResult, VoxelBrush};
pub use voxel_server::{ServerChunkSystem, ServerVoxelPlugin};
pub use worldgen::WorldGen;

use crate::util::AsMutRef;
use bevy::{prelude::*, utils::HashMap};
use std::sync::Arc;

pub type ChunkPtr = Arc<Chunk>;

#[derive(Resource, Deref, Clone)]
struct ChannelTx<T>(crate::channel_impl::Sender<T>);

#[derive(Resource, Deref, Clone)]
struct ChannelRx<T>(crate::channel_impl::Receiver<T>);

pub fn is_chunk_in_load_distance(mid_cp: IVec3, cp: IVec3, vd: IVec2) -> bool {
    (mid_cp.x - cp.x).abs() <= vd.x * Chunk::LEN && (mid_cp.z - cp.z).abs() <= vd.x * Chunk::LEN && (mid_cp.y - cp.y).abs() <= vd.y * Chunk::LEN
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

    fn get_voxel(&self, p: IVec3) -> Option<&Vox> {
        let chunkptr = self.get_chunk(Chunk::as_chunkpos(p))?;

        Some(chunkptr.at_voxel(Chunk::as_localpos(p)))
    }

    fn get_voxel_mut(&self, p: IVec3) -> Option<&mut Vox> {
        self.get_voxel(p).map(|v| v.as_mut())
    }
}
