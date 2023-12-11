
use std::borrow::BorrowMut;
use std::path::Path;
use std::sync::{Arc, RwLockReadGuard, RwLock, Weak, RwLockWriteGuard};

use bevy::prelude::*;
use bevy::render::primitives::Aabb;
use bevy::tasks::Task;
use bevy::utils::{HashMap, HashSet};

use crate::world::chunk;


// Voxel System

#[derive(Clone, Copy)]
struct Cell {
	/// SDF value, used for Isosurface Extraction.
	/// 0 -> surface, +0 positive -> void, -0 negative -> solid.
    value: f32,

    /// Material Id
    mtl: u16,

    /// Cached FeaturePoint
    cached_fp: Vec3,
    cached_norm: Vec3
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            value: 0.,
            mtl: 0,
            cached_fp: Vec3::INFINITY,
            cached_norm: Vec3::INFINITY
        }
    }
}

impl Cell {

    fn new(value: f32, mtl: u16) -> Self {
        Self {
            value,
            mtl,
            ..default()
        }
    }

}



// Chunk is "Heavy" type (big size, stored a lot voxels). thus copy/clone are not allowed.
pub struct Chunk {

    // shoud Box?
    cells: [Cell; 16*16*16],

    chunkpos: IVec3,

    entity: Entity,

    // cached neighbor chunks (if they are not empty even if they are loaded)
    // for Quick Access neighbor voxel, without global find neighbor chunk by chunkpos
    neighbors: [Option<Weak<RwLock<Chunk>>>; 6],



}

impl Chunk {

    pub const SIZE: i32 = 16;


    pub fn new(chunkpos: IVec3) -> Self {
        Self {
            cells: [Cell::default(); 16*16*16],
            chunkpos,
            neighbors: [None, None, None, None, None, None],
            entity: Entity::PLACEHOLDER,
        }
    }

    #[inline]
    fn get_cell(&self, localpos: IVec3) -> Cell {
        self.cells[Chunk::local_cell_idx(localpos) as usize]
    }

    fn get_cell_mut(&mut self, localpos: IVec3) -> &mut Cell {
        &mut self.cells[Chunk::local_cell_idx(localpos) as usize]
    }

    fn set_cell(&mut self, localpos: IVec3, cell: Cell) {
        self.cells[Chunk::local_cell_idx(localpos) as usize] = cell;
    }


    pub fn neighbor_chunk(&self, i: i32) -> Option<ChunkPtr> {

        if let Some(chunk) = &self.neighbors[i as usize] {
            chunk.upgrade()
        } else {
            None
        }
    }

    fn _floor16(x: i32) -> i32 { x & (!15) }
    fn _mod16(x: i32) -> i32 { x & 15 }

    pub fn as_chunkpos(p: IVec3) -> IVec3 {
        IVec3::new(Self::_floor16(p.x), Self::_floor16(p.y), Self::_floor16(p.z))
    }

    pub fn as_localpos(p: IVec3) -> IVec3 {
        IVec3::new(Self::_mod16(p.x), Self::_mod16(p.y), Self::_mod16(p.z))
    }

    /// mod(p, 16) == 0
    fn is_chunkpos(p: IVec3) -> bool {
        p % 16 == IVec3::ZERO
    }
    // [0, 16)
    fn is_localpos(p: IVec3) -> bool {
        p.x >= 0 && p.x < 16 &&
        p.y >= 0 && p.y < 16 &&
        p.z >= 0 && p.z < 16
    }

    fn local_cell_idx(localpos: IVec3) -> i32 {
        assert!(Chunk::is_localpos(localpos));
        localpos.x << 8 | localpos.y << 4 | localpos.z
    }

}


enum SVO<T> {

    Octree {
        children: [Box<SVO<T>>; 8],
    },
    Leaf {
        chunk: T
    }

}

#[derive(Component)]
pub struct ChunkComponent {
    chunkpos: IVec3,
}

impl ChunkComponent {
    fn new(chunkpos: IVec3) -> Self {
        Self {
            chunkpos,
        }
    }
}


pub enum ChunkMeshingState {
    Pending,
    Meshing(Task<Mesh>),
    Completed,
}

#[derive(Resource)]
pub struct ChunkSystem {

    /// all loaded chunks.
    /// ChunkList can be read (by multiple threads) at the same time, but only one can be writing at the same time and no other can be reading at this time.
    // use RW-Lock.

    // 设计一个高性能区块系统，这两个区块列表 及每个区块 都有RwLock特性，即 可同时可被多处读，但只能被互斥写

    // linear-list of loaded chunks.
    chunks: Arc<RwLock<HashMap<IVec3, Arc<RwLock<Chunk>>>>>, 

    // Spare Voxel Octree for Spatial lookup acceleration.
    // chunks_svo: SVO<Arc<RwLock<Chunk>>>,

    pub chunks_loading: HashSet<IVec3>,
    pub chunks_meshing: HashMap<IVec3, ChunkMeshingState>,

    pub view_distance: IVec2,

}

pub type ChunkPtr = Arc<RwLock<Chunk>>;

impl ChunkSystem {

    pub fn new() -> Self {
        Self { 
            chunks: Arc::new(RwLock::new(HashMap::new())), 
            view_distance: IVec2::new(2, 2),
            chunks_loading: HashSet::new(),
            chunks_meshing: HashMap::new(),
        }
    }

    pub fn get_chunk(&self, chunkpos: IVec3) -> Option<ChunkPtr> {
        assert!(Chunk::is_chunkpos(chunkpos));

        if let Some(chunk) = self.chunks.read().unwrap().get(&chunkpos) {
            Some(chunk.clone())
        } else {
            None
        }
    }

    pub fn has_chunk(&self, chunkpos: IVec3) -> bool {
        assert!(Chunk::is_chunkpos(chunkpos));

         self.chunks.read().unwrap().contains_key(&chunkpos)
    }

    pub fn num_chunks(&self) -> usize {

        self.chunks.read().unwrap().len()
    }

    pub fn provide_chunk(&self, chunkpos: IVec3) -> ChunkPtr {
        assert!(!self.has_chunk(chunkpos));

        let mut chunk = Arc::new(RwLock::new(Chunk::new(chunkpos)));

        let load = false;  // chunk_loader.load_chunk(chunk);

        if !load {

            ChunkGenerator::generate_chunk(chunk.write().unwrap().borrow_mut());
        }

        chunk
    }


    pub fn spawn_chunk(&self, chunkptr: ChunkPtr, cmds: &mut Commands) {
        let chunkpos;

        {
            let mut chunk = chunkptr.write().unwrap();
            chunkpos = chunk.chunkpos;
    
            let entity = cmds.spawn((
                ChunkComponent::new(chunkpos),
                PbrBundle {
                    // mesh: meshes.add(generate_chunk_mesh()),
                    transform: Transform::from_translation(chunkpos.as_vec3()),
                    // visibility: Visibility::Hidden,
                    ..default()
                },
                Aabb::from_min_max(Vec3::ZERO, Vec3::ONE * (Chunk::SIZE as f32)),
                
                // AsyncCollider(ComputedCollider::TriMesh),
                // RigidBody::Static,
            )).id();

            chunk.entity = entity;
        }
        

        self.chunks.write().unwrap().insert(chunkpos, chunkptr);
        // // There is no need to cast shadows for chunks below the surface.
        // if chunkpos.y <= 64 {
        //     entity_commands.insert(NotShadowCaster);
        // }

    }

    pub fn despawn_chunk(&self, chunkpos: IVec3, cmds: &mut Commands) -> Option<ChunkPtr> {

        if let Some(chunk_) = self.chunks.write().unwrap().remove(&chunkpos) {
            {
                let chunk = chunk_.read().unwrap();

                cmds.entity(chunk.entity).despawn_recursive();
            }
            Some(chunk_)
        } else {
            None
        }
    }

    pub fn set_chunk_meshing(&mut self, chunkpos: IVec3, stat: ChunkMeshingState) {
        self.chunks_meshing.insert(chunkpos, stat);
    }

}





struct ChunkGenerator {

}

impl ChunkGenerator {

    fn generate_chunk(chunk: &mut Chunk) {

        // for y in 0..Chunk::SIZE {
        //     for z in 0..Chunk::SIZE {
        //         for x in 0..Chunk::SIZE {
        //             let lp = IVec3::new(x, y, z);

        //         }
        //     }
        // }

        chunk.set_cell(IVec3::new(0,0,0), Cell::new(1., 1))

    }

    fn populate_chunk(chunk: &mut Chunk) {

    }

}



struct ChunkLoader {

    save_dir: Path,

}

impl ChunkLoader {

    fn load_chunk(chunk: &mut Chunk) {

    }

    fn save_chunk(chunk: &mut Chunk) {

    }

}