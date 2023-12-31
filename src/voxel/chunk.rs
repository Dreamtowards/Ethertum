
use std::sync::{RwLock, Weak, Arc};

use bevy::prelude::*;

use super::chunk_system::ChunkPtr;



// Voxel System

#[derive(Clone, Copy)]
pub struct Cell {
	/// SDF value, used for Isosurface Extraction.
	/// 0 -> surface, +0 positive -> void, -0 negative -> solid.
    pub value: f32,

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
    cells: [Cell; 16*16*16],

    pub chunkpos: IVec3,

    pub entity: Entity,

    // cached neighbor chunks (if they are not empty even if they are loaded)
    // for Quick Access neighbor voxel, without global find neighbor chunk by chunkpos
    neighbor_chunks: [Option<Weak<Arc<ChunkPtr>>>; 6],



}

impl Chunk {

    pub const SIZE: i32 = 16;


    pub fn new(chunkpos: IVec3) -> Self {
        Self {
            cells: [Cell::default(); 16*16*16],
            chunkpos,
            neighbor_chunks: [None, None, None, None, None, None],
            entity: Entity::PLACEHOLDER,
        }
    }

    #[inline]
    pub fn get_cell(&self, localpos: IVec3) -> &Cell {
        &self.cells[Chunk::local_cell_idx(localpos)]
    }

    pub fn get_cell_neighbor(&self, relpos: IVec3) -> Option<&Cell> {
        if Chunk::is_localpos(relpos) {
            Some(self.get_cell(relpos))
        } else {
            // if let Some(neib) = Chunk::neighbor_idx(relpos) {
            //     if let Some(neib_chunk_weak) = self.neighbor_chunks[neib] {
            //         if let Some(neib_chunk) = neib_chunk_weak.upgrade() {

            //             return Some(neib_chunk.get_cell(Chunk::as_localpos(relpos)));
            //         }
            //     }
            // }
            None
        }
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

    fn _floor16(x: i32) -> i32 { x & (!15) }
    fn _mod16(x: i32) -> i32 { x & 15 }

    pub fn as_chunkpos(p: IVec3) -> IVec3 {
        IVec3::new(Self::_floor16(p.x), Self::_floor16(p.y), Self::_floor16(p.z))
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
        p.x >= 0 && p.x < 16 &&
        p.y >= 0 && p.y < 16 &&
        p.z >= 0 && p.z < 16
    }

    fn local_cell_idx(localpos: IVec3) -> usize {
        assert!(Chunk::is_localpos(localpos));
        (localpos.x << 8 | localpos.y << 4 | localpos.z) as usize
    }

    const NEIGHBOR_DIR: [IVec3; 6] = [
        IVec3::new(-1, 0, 0),
        IVec3::new( 1, 0, 0),
        IVec3::new( 0,-1, 0),
        IVec3::new( 0, 1, 0),
        IVec3::new( 0, 0,-1),
        IVec3::new( 0, 0, 1),
    ];

    fn neighbor_idx(relpos: IVec3) -> Option<usize> {
        for i in 0..Chunk::NEIGHBOR_DIR.len() {
            if Chunk::is_localpos(relpos - (Chunk::NEIGHBOR_DIR[i] * Chunk::SIZE)) {
                return Some(i);
            }
        }
        None
    }

}






