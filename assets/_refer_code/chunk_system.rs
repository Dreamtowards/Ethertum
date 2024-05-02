use bevy::{
    prelude::*,
    tasks::Task,
    utils::{HashMap, HashSet},
};
use bevy_xpbd_3d::components::Collider;
use std::sync::{Arc, RwLock};

use super::{chunk::*, TerrainMaterial};

// Box<Chunk>;         not supported for SharedPtr
pub type ChunkPtr = Arc<RwLock<Chunk>>;



#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct ChunkSystem {
    /// all loaded chunks.
    // linear-list of loaded chunks.
    #[reflect(ignore)]
    pub chunks: HashMap<IVec3, ChunkPtr>,

    // Spare Voxel Octree for Spatial lookup acceleration.
    // chunks_svo: SVO<Arc<RwLock<Chunk>>>,

    #[cfg(feature = "target_native_os")]
    #[reflect(ignore)]
    pub chunks_loading: HashMap<IVec3, Task<ChunkPtr>>,

    #[cfg(feature = "experimental_channel")]
    #[reflect(ignore)]
    pub chunks_loading: HashMap<IVec3, ()>,

    #[cfg(feature = "target_native_os")]
    #[reflect(ignore)]
    pub chunks_meshing: HashMap<IVec3, Task<(Mesh, Option<Collider>, Entity, Handle<Mesh>)>>,

    #[cfg(feature = "experimental_channel")]
    #[reflect(ignore)]
    pub chunks_meshing: HashMap<IVec3, ()>,

    pub chunks_remesh: HashSet<IVec3>, // marked as ReMesh

    pub view_distance: IVec2,

    pub entity: Entity,


    pub dbg_remesh_all_chunks: bool,

    pub max_concurrent_loading: usize,
    pub max_concurrent_meshing: usize,
}

impl Default for ChunkSystem {
    fn default() -> Self {
        Self {
            chunks: HashMap::default(), //Arc::new(RwLock::new(HashMap::new())),
            view_distance: IVec2::new(1, 1),
            entity: Entity::PLACEHOLDER,
            shader_terrain: Handle::default(),
            dbg_remesh_all_chunks: false,
            chunks_remesh: HashSet::default(),
            chunks_meshing: HashMap::default(),
            chunks_loading: HashMap::default(),
            max_concurrent_loading: 16,
            max_concurrent_meshing: 16,
        }
    }
}

impl ChunkSystem {
    pub fn new(view_distance: IVec2) -> Self {
        Self {
            view_distance,
            // chunks_loading: HashSet::new(),
            // chunks_meshing: HashMap::new(),
            ..default()
        }
    }

    pub fn get_chunk(&self, chunkpos: IVec3) -> Option<&ChunkPtr> {
        assert!(Chunk::is_chunkpos(chunkpos));
        self.chunks.get(&chunkpos)
        /*
        if let Some(chunk) = self.chunks.get(&chunkpos) {
            //.read().unwrap().get(&chunkpos) {
            Some(chunk)
        } else {
            None
        }
        */
    }

    pub fn has_chunk(&self, chunkpos: IVec3) -> bool {
        assert!(Chunk::is_chunkpos(chunkpos));

        self.chunks.contains_key(&chunkpos) //.read().unwrap().contains_key(&chunkpos)
    }

    pub fn num_chunks(&self) -> usize {
        self.chunks.len() //.read().unwrap().len()
    }

    pub fn get_cell(&self, p: IVec3) -> Option<Cell> {
        let chunk = self.get_chunk(Chunk::as_chunkpos(p))?.read().unwrap();
        Some(*chunk.get_cell(Chunk::as_localpos(p)))
    }

    pub fn spawn_chunk(&mut self, chunkptr: ChunkPtr) {
        let chunkpos;
        {
            let mut chunk = chunkptr.write().unwrap();
            chunkpos = chunk.chunkpos;

            let mut load = Vec::new();

            for neib_idx in 0..Chunk::NEIGHBOR_DIR.len() {
                let neib_dir = Chunk::NEIGHBOR_DIR[neib_idx];
                let neib_chunkpos = chunkpos + neib_dir * Chunk::LEN;

                // todo: delay remesh or only remesh full-neighbor complete chunks

                // set neighbor_chunks cache
                chunk.neighbor_chunks[neib_idx] = {
                    if let Some(neib_chunkptr) = self.get_chunk(neib_chunkpos) {
                        
                        let mut neib_chunk = neib_chunkptr.write().unwrap();

                        // update neighbor's `neighbor_chunk`
                        neib_chunk.neighbor_chunks[Chunk::neighbor_idx_opposite(neib_idx)] = Some(Arc::downgrade(&chunkptr));

                        if neib_chunk.is_neighbors_complete() {
                            load.push(neib_chunk.chunkpos);
                        }
                        

                        Some(Arc::downgrade(neib_chunkptr))
                    } else {
                        None
                    }
                }
            }

            if chunk.is_neighbors_complete() {
                self.mark_chunk_remesh(chunkpos);
            }
            for cp in load {
                self.mark_chunk_remesh(cp);
            }
        }

        self.chunks.insert(chunkpos, chunkptr);

        // // There is no need to cast shadows for chunks below the surface.
        // if chunkpos.y <= 64 {
        //     entity_commands.insert(NotShadowCaster);
        // }

        // self.set_chunk_meshing(chunkpos, ChunkMeshingState::Pending);
    }

    pub fn despawn_chunk(&mut self, chunkpos: IVec3) -> Option<ChunkPtr> {
        let chunk = self.chunks.remove(&chunkpos)?;

        //cmds.entity(chunk.entity).despawn_recursive();

        Some(chunk)
    }

    pub fn mark_chunk_remesh(&mut self, chunkpos: IVec3) {
        self.chunks_remesh.insert(chunkpos);
    }

    // pub fn set_chunk_meshing(&mut self, chunkpos: IVec3, stat: ChunkMeshingState) {
    //     self.chunks_meshing.insert(chunkpos, stat);
    // }
}
