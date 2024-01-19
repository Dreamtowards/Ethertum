use std::sync::{Arc, RwLock, Weak};

use bevy::{math::ivec3, prelude::*};

use super::chunk_system::ChunkPtr;

// Voxel System

#[derive(Clone, Copy)]
pub struct Cell {
    /// SDF value, used for Isosurface Extraction.
    /// 0 -> surface, +0 positive -> void, -0 negative -> solid.
    pub value: f32,

    /// Material Id
    pub mtl: u16,

    /// Cached FeaturePoint
    pub cached_fp: Vec3,
    pub cached_norm: Vec3,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            value: 0.,
            mtl: 0,
            cached_fp: Vec3::INFINITY,
            cached_norm: Vec3::INFINITY,
        }
    }
}

impl Cell {
    pub fn new(value: f32, mtl: u16) -> Self {
        Self {
            value,
            mtl,
            ..default()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.value <= 0.
    }

    pub fn is_solid(&self) -> bool {
        self.value > 0.
    }
}

// Chunk is "Heavy" type (big size, stored a lot voxels). thus copy/clone are not allowed.
pub struct Chunk {
    // shoud Box?
    cells: [Cell; 16 * 16 * 16],

    pub chunkpos: IVec3,

    pub entity: Entity,
    pub mesh_handle: Handle<Mesh>,

    // cached neighbor chunks (if they are not empty even if they are loaded)
    // for Quick Access neighbor voxel, without global find neighbor chunk by chunkpos
    pub neighbor_chunks: [Option<Weak<RwLock<Chunk>>>; Self::NEIGHBOR_DIR.len()],
}

impl Chunk {
    pub const SIZE: i32 = 16;

    pub fn new(chunkpos: IVec3) -> Self {
        Self {
            cells: [Cell::default(); 16 * 16 * 16],
            chunkpos,
            neighbor_chunks: Default::default(),
            entity: Entity::PLACEHOLDER,
            mesh_handle: Handle::default(),
        }
    }

    #[inline]
    pub fn get_cell(&self, localpos: IVec3) -> &Cell {
        &self.cells[Chunk::local_cell_idx(localpos)]
    }

    pub fn get_cell_neighbor(&self, relpos: IVec3) -> Option<Cell> {
        if Chunk::is_localpos(relpos) {
            Some(*self.get_cell(relpos))
        } else {
            if let Some(neib_idx) = Chunk::neighbor_idx(relpos) {
                if let Some(neib_weak) = &self.neighbor_chunks[neib_idx] {
                    if let Some(neib_chunkptr) = neib_weak.upgrade() {
                        let neib_chunk = neib_chunkptr.read().unwrap();
                        // assert!(neib_chunk.chunkpos == self.chunkpos + Self::NEIGHBOR_DIR[neib_idx] * Chunk::SIZE, "self.chunkpos = {}, neib {} pos {}", self.chunkpos, neib_idx, neib_chunk.chunkpos);

                        return Some(*neib_chunk.get_cell(Chunk::as_localpos(relpos)));
                    }
                }
            }
            None
        }
    }

    pub fn get_cell_rel(&self, relpos: IVec3) -> Cell {
        self.get_cell_neighbor(relpos)
            .unwrap_or(Cell::new(f32::INFINITY, 0))
    }

    pub fn get_cell_mut(&mut self, localpos: IVec3) -> &mut Cell {
        &mut self.cells[Chunk::local_cell_idx(localpos)]
    }

    pub fn set_cell(&mut self, localpos: IVec3, cell: &Cell) {
        self.cells[Chunk::local_cell_idx(localpos)] = *cell;
    }

    // pub fn neighbor_chunk(&self, i: i32) -> Option<ChunkPtr> {
    //     if let Some(chunk) = &self.neighbors[i as usize] {
    //         chunk.upgrade()
    //     } else {
    //         None
    //     }
    // }

    fn _floor16(x: i32) -> i32 {
        x & (!15)
    }
    fn _mod16(x: i32) -> i32 {
        x & 15
    }

    pub fn as_chunkpos(p: IVec3) -> IVec3 {
        IVec3::new(
            Self::_floor16(p.x),
            Self::_floor16(p.y),
            Self::_floor16(p.z),
        )
    }

    pub fn as_localpos(p: IVec3) -> IVec3 {
        IVec3::new(Self::_mod16(p.x), Self::_mod16(p.y), Self::_mod16(p.z))
    }

    /// mod(p, 16) == 0
    pub fn is_chunkpos(p: IVec3) -> bool {
        p % 16 == IVec3::ZERO
    }
    // [0, 16)
    pub fn is_localpos(p: IVec3) -> bool {
        p.x >= 0 && p.x < 16 && p.y >= 0 && p.y < 16 && p.z >= 0 && p.z < 16
    }

    fn local_cell_idx(localpos: IVec3) -> usize {
        assert!(Chunk::is_localpos(localpos), "localpos = {}", localpos);
        (localpos.x << 8 | localpos.y << 4 | localpos.z) as usize
    }

    pub const NEIGHBOR_DIR: [IVec3; 6 + 12 + 8] = [
        // 6 Faces
        ivec3(-1, 0, 0),
        ivec3(1, 0, 0),
        ivec3(0, -1, 0),
        ivec3(0, 1, 0),
        ivec3(0, 0, -1),
        ivec3(0, 0, 1),
        // 12 Edges
        ivec3(0, -1, -1), // X
        ivec3(0, 1, 1),
        ivec3(0, 1, -1),
        ivec3(0, -1, 1),
        ivec3(-1, 0, -1), // Y
        ivec3(1, 0, 1),
        ivec3(1, 0, -1),
        ivec3(-1, 0, 1),
        ivec3(-1, -1, 0), // Z
        ivec3(1, 1, 0),
        ivec3(-1, 1, 0),
        ivec3(1, -1, 0),
        // 8 Vertices
        ivec3(-1, -1, -1),
        ivec3(1, 1, 1),
        ivec3(1, -1, -1),
        ivec3(-1, 1, 1),
        ivec3(-1, -1, 1),
        ivec3(1, 1, -1),
        ivec3(1, -1, 1),
        ivec3(-1, 1, -1),
    ];

    fn neighbor_idx(relpos: IVec3) -> Option<usize> {
        for i in 0..Chunk::NEIGHBOR_DIR.len() {
            if Chunk::is_localpos(relpos - (Chunk::NEIGHBOR_DIR[i] * Chunk::SIZE)) {
                return Some(i);
            }
        }
        None
    }

    // assert!(Self::NEIGHBOR_DIR[idx] + Self::NEIGHBOR_DIR[opposite_idx] == IVec3::ZERO, "idx = {}, opposite = {}", idx, opposite_idx);
    pub fn neighbor_idx_opposite(idx: usize) -> usize {
        idx / 2 * 2 + (idx + 1) % 2
    }
}
