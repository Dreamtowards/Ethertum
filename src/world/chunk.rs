
use bevy::prelude::*;

// Voxel System

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




pub struct Chunk {

    cells: [Cell; 16*16*16],

    chunkpos: IVec3



}

impl Chunk {


    // fn local_cell(self, localpos: IVec3) -> &Cell {
    //     &self.cells[Chunk::local_cell_idx(localpos) as usize]
    // }


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

