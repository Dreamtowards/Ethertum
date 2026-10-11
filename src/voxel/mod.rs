mod chunk;
mod chunk_mesh;
pub mod lighting;
pub mod meshgen;
mod vox;
mod voxel_client;
mod voxel_server;
pub mod worldgen;

mod render;

pub use chunk::{Chunk, ChunkSnapshot};
pub use render::TerrainMaterial;
pub use vox::{Vox, VoxLight, VoxShape, VoxTex};
pub use voxel_client::{ClientChunkSystem, ClientVoxelPlugin, HitResult, VoxelBrush};
pub use voxel_server::{ServerChunkSystem, ServerVoxelPlugin};

use bevy::{platform::collections::HashMap, prelude::*};

#[derive(Resource, Deref, Clone)]
struct ChannelTx<T>(crate::channel_impl::Sender<T>);

#[derive(Resource, Deref, Clone)]
struct ChannelRx<T>(crate::channel_impl::Receiver<T>);

pub fn is_chunk_in_load_distance(mid_cp: IVec3, cp: IVec3, vd: IVec2) -> bool {
    (mid_cp.x - cp.x).abs() <= vd.x * Chunk::LEN && (mid_cp.z - cp.z).abs() <= vd.x * Chunk::LEN && (mid_cp.y - cp.y).abs() <= vd.y * Chunk::LEN
}

// can_sustain_plant()

pub trait ChunkSystem {
    fn get_chunks(&self) -> &HashMap<IVec3, Chunk>;

    fn get_chunk(&self, chunkpos: IVec3) -> Option<&Chunk> {
        assert!(Chunk::is_chunkpos(chunkpos));
        self.get_chunks().get(&chunkpos)
    }

    fn get_chunk_mut(&mut self, chunkpos: IVec3) -> Option<&mut Chunk>;

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

    fn get_voxel_mut(&mut self, p: IVec3) -> Option<&mut Vox> {
        self.get_chunk_mut(Chunk::as_chunkpos(p))
            .map(|chunk| chunk.at_voxel_mut(Chunk::as_localpos(p)))
    }

    /// Capture the full sampling neighborhood without keeping pointers to live chunks.
    /// 在主线程提交网格任务前或开始邻域编辑前调用。包含面、边、角邻居，以便
    /// 采样区块边界；缺失邻居保持 None。clone 共享 Arc 体素数组，不立即复制体素。
    fn snapshot_chunk(&self, chunkpos: IVec3) -> Option<ChunkSnapshot> {
        let mut snapshot = self.get_chunk(chunkpos)?.clone();
        snapshot.neighbor_chunks = std::array::from_fn(|i| {
            self.get_chunk(chunkpos + Chunk::NEIGHBOR_DIR[i] * Chunk::LEN)
                .map(|chunk| Box::new(chunk.clone()))
        });
        Some(ChunkSnapshot::new(snapshot))
    }
}
