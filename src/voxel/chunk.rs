use std::{
    sync::{RwLock, Weak},
    usize,
};

use bevy::{math::ivec3, prelude::*};

// Voxel System

// enum CellShape {
//     Isosurface,
//     Cube,
//     Leaves,
//     Grass,
//     // CustomMesh {
//     //     mesh_id: u16,
//     // }
// }

#[derive(Clone, Copy)]
pub struct Cell {
    pub tex_id: u16,

    pub shape_id: u16,

    /// SDF value, used for Isosurface Extraction.
    /// 0 -> surface, +0 positive -> void, -0 negative -> solid.
    pub isoval: u8,
    // Cached FeaturePoint
    // pub cached_fp: Vec3,
    // pub cached_norm: Vec3,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            shape_id: 0,
            tex_id: 0,
            isoval: isoval_f32_u8(0.0),
            // cached_fp: Vec3::INFINITY,
            // cached_norm: Vec3::INFINITY,
        }
    }
}

// todo: bugfix 自从压缩成u8后，地下有一些小碎片三角形 可能是由于精度误差 -0.01 变成 0.01 之类的了。要
// 修复这种bug 应该是把接近0的数值扩大 再转成u8.
fn isoval_f32_u8(f: f32) -> u8 {
    ((f.clamp(-1.0, 1.0) + 1.0) / 2.0 * 255.0) as u8
}

fn isoval_u8_f32(u: u8) -> f32 {
    u as f32 / 255.0 * 2.0 - 1.0
}

impl Cell {
    pub fn new(tex_id: u16, shape_id: u16, isovalue: f32) -> Self {
        Self {
            tex_id,
            shape_id,
            isoval: isoval_f32_u8(isovalue),
            ..default()
        }
    }

    pub fn isovalue(&self) -> f32 {
        isoval_u8_f32(self.isoval)
    }
    pub fn set_isovalue(&mut self, val: f32) {
        self.isoval = isoval_f32_u8(val);
    }

    pub fn is_tex_empty(&self) -> bool {
        self.tex_id == 0
    }

    pub fn is_isoval_empty(&self) -> bool {
        self.isovalue() <= 0.0
    }

    pub fn is_obaque_cube(&self) -> bool {
        self.shape_id == 1 && !self.is_tex_empty()
    }
}

// Chunk is "Heavy" type (big size, stored a lot voxels). thus copy/clone are not allowed.
pub struct Chunk {
    // shoud Box?
    cells: [Cell; 16 * 16 * 16],

    pub chunkpos: IVec3,

    pub entity: Entity,
    pub mesh_handle: Handle<Mesh>, // solid terrain
    pub mesh_handle_foliage: Handle<Mesh>,

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
            mesh_handle_foliage: Handle::default(),
        }
    }

    pub fn get_cell(&self, localpos: IVec3) -> &Cell {
        &self.cells[Chunk::local_idx(localpos)]
    }

    pub fn get_cell_neighbor(&self, relpos: IVec3) -> Option<Cell> {
        if Chunk::is_localpos(relpos) {
            Some(*self.get_cell(relpos))
        } else {
            /*
            let neib_idx = Chunk::neighbor_idx(relpos)?;
            self.neighbor_chunks[neib_idx].as_ref()
                .and_then(|neib_weak| neib_weak.upgrade())
                .and_then(|neib_chunkptr| neib_chunkptr.read().unwrap())
                .and_then(|neib_chunk| {
                    // assert!(neib_chunk.chunkpos == self.chunkpos + Self::NEIGHBOR_DIR[neib_idx] * Chunk::SIZE, "self.chunkpos = {}, neib {} pos {}", self.chunkpos, neib_idx, neib_chunk.chunkpos);
                    Some(*neib_chunk.get_cell(Chunk::as_localpos(relpos)))
                })
            */

            let neib_idx = Chunk::neighbor_idx(relpos)?;
            if let Some(neib_weak) = &self.neighbor_chunks[neib_idx] {
                let neib_chunkptr = neib_weak.upgrade()?;
                let neib_chunk = neib_chunkptr.read().unwrap();
                // assert!(neib_chunk.chunkpos == self.chunkpos + Self::NEIGHBOR_DIR[neib_idx] * Chunk::SIZE, "self.chunkpos = {}, neib {} pos {}", self.chunkpos, neib_idx, neib_chunk.chunkpos);

                return Some(*neib_chunk.get_cell(Chunk::as_localpos(relpos)));
            }
            None
        }
    }

    pub fn get_cell_rel(&self, relpos: IVec3) -> Cell {
        self.get_cell_neighbor(relpos).unwrap_or(Cell::new(0, 0, 0.0))
    }

    pub fn get_cell_mut(&mut self, localpos: IVec3) -> &mut Cell {
        &mut self.cells[Chunk::local_idx(localpos)]
    }

    pub fn set_cell(&mut self, localpos: IVec3, cell: &Cell) {
        self.cells[Chunk::local_idx(localpos)] = *cell;
    }

    pub fn is_neighbors_complete(&self) -> bool {
        /*
        for e in self.neighbor_chunks.iter() {
            if e.is_none() {
                return false;
            }
        }
        true
        */
        !self.neighbor_chunks.iter().any(|e| e.is_none())
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
        p.x >= 0 && p.x < 16 && p.y >= 0 && p.y < 16 && p.z >= 0 && p.z < 16
    }

    pub const LOCAL_IDX_CAP: usize = 4096; // 16^3, 2^12 bits (12 = 3 axes * 4 bits)

    // the index range is [0, 16^3 or 4096)
    pub fn local_idx(localpos: IVec3) -> usize {
        assert!(Chunk::is_localpos(localpos), "localpos = {}", localpos);
        (localpos.x << 8 | localpos.y << 4 | localpos.z) as usize
    }
    pub fn local_idx_pos(idx: i32) -> IVec3 {
        IVec3::new((idx >> 8) & 15, (idx >> 4) & 15, idx & 15)
    }

    pub fn at_boundary_naive(localpos: IVec3) -> i32 {
        if localpos.x == 0 {
            return 0;
        }
        if localpos.x == 15 {
            return 1;
        }
        if localpos.y == 0 {
            return 2;
        }
        if localpos.y == 15 {
            return 3;
        }
        if localpos.z == 0 {
            return 4;
        }
        if localpos.z == 15 {
            return 5;
        }
        -1
        // localpos.x == 0 || localpos.x == 15 ||
        // localpos.y == 0 || localpos.y == 15 ||
        // localpos.z == 0 || localpos.z == 15
    }



    #[rustfmt::skip]
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
        assert!(!Chunk::is_localpos(relpos));
        (0..Chunk::NEIGHBOR_DIR.len()).find(|&i| Chunk::is_localpos(relpos - (Chunk::NEIGHBOR_DIR[i] * Chunk::SIZE)))
    }

    // assert!(Self::NEIGHBOR_DIR[idx] + Self::NEIGHBOR_DIR[opposite_idx] == IVec3::ZERO, "idx = {}, opposite = {}", idx, opposite_idx);
    pub fn neighbor_idx_opposite(idx: usize) -> usize {
        // idx MOD 2 + (idx+1) MOD 2
        idx / 2 * 2 + (idx + 1) % 2
    }
}
