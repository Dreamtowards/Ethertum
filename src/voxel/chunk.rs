use std::sync::Weak;
use bevy::math::ivec3;

use crate::{prelude::*, util::AsMutRef};

use super::ChunkPtr;


#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize, Default, Reflect)]
pub enum VoxShape {
    #[default]
    Isosurface,

    Cube,
    Leaves,
    Grass,

    SlabYMin,
    SlabYMax,
    SlabXMin,
    SlabXMax,
    SlabZMin,
    SlabZMax,

    Fence,

    // CustomMesh {
    //     mesh_id: u16,
    // }
}




// naming VoxelUnit?: BlockState, Cell, Voxel or Vox?
#[derive(Clone, Copy)]
pub struct Vox {
    pub tex_id: u16,

    pub shape_id: VoxShape,

    /// SDF value, used for Isosurface Extraction.
    /// 0 -> surface, +0 positive -> void, -0 negative -> solid.
    pub isoval: u8,
    // Cached FeaturePoint
    // pub cached_fp: Vec3,
    // pub cached_norm: Vec3,
}

impl Default for Vox {
    fn default() -> Self {
        Vox {
            shape_id: VoxShape::Isosurface,
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

impl Vox {
    pub fn new(tex_id: u16, shape_id: VoxShape, isovalue: f32) -> Self {
        Self {
            tex_id,
            shape_id,
            isoval: isoval_f32_u8(isovalue),
            ..default()
        }
    }

    pub fn isovalue(&self) -> f32 {
        if self.shape_id != VoxShape::Isosurface {
            return 0.0;
        }
        isoval_u8_f32(self.isoval)
    }
    pub fn set_isovalue(&mut self, val: f32) {
        self.isoval = isoval_f32_u8(val);
    }

    pub fn is_tex_empty(&self) -> bool {
        self.tex_id == 0
    }
    pub fn is_nil(&self) -> bool {
        self.is_tex_empty()
    }

    pub fn is_isoval_empty(&self) -> bool {
        self.isovalue() <= 0.0
    }

    pub fn is_obaque_cube(&self) -> bool {
        self.shape_id == VoxShape::Cube && !self.is_tex_empty()
    }
}

#[derive(Default, Clone, Copy)]
struct VoxLight {
    // SkyLight, R, G, B u4
    light: u16,
}

#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
pub mod VoxTex {

    pub const Nil: u16 = 0;
    pub const Stone: u16 = 22;
    pub const Dirt: u16 = 1;
    pub const Grass: u16 = 12; // 7 11
    pub const Water: u16 = 24;
    pub const Sand: u16 = 19;
    pub const Log: u16 = 13;

    pub const ShortGrass: u16 = 13;
    pub const Bush: u16 = 14;
    pub const Rose: u16 = 15;
    pub const Fern: u16 = 16;
    pub const Leaves: u16 = 23;

    use bevy::math::Vec2;
    pub fn map_uv(uv: Vec2, tex_id: u16) -> Vec2 {
        const TEX_CAP: f32 = 24.;
        let tex = tex_id - 1; // -1: offset the 0 Nil
        Vec2::new(uv.x / TEX_CAP + tex as f32 / TEX_CAP, uv.y)
    }
}

// Chunk is "Heavy" type (big size, stored a lot voxels). thus copy/clone are not allowed.
pub struct Chunk {
    // shoud Box?
    voxel: [Vox; Self::LEN3],
    light: [VoxLight; Self::LEN3],

    pub chunkpos: IVec3,

    pub is_populated: bool,

    pub entity: Entity,
    pub mesh_handle: Handle<Mesh>, // solid terrain
    pub mesh_handle_foliage: Handle<Mesh>,

    // cached neighbor chunks (if they are not empty even if they are loaded)
    // for Quick Access neighbor voxel, without global find neighbor chunk by chunkpos
    pub neighbor_chunks: [Option<Weak<Chunk>>; Self::NEIGHBOR_DIR.len()],
}

impl Chunk {
    pub const LEN: i32 = 16;  // i32 for xyz iter
    pub const LEN3: usize = (Self::LEN * Self::LEN * Self::LEN) as usize;
    // pub const LOCAL_IDX_CAP: usize = 4096;  // 16^3, 2^12 bits (12 = 3 axes * 4 bits)

    pub fn new(chunkpos: IVec3) -> Self {
        Self {
            voxel: [Vox::default(); Self::LEN3],
            light: [VoxLight::default(); Self::LEN3],
            chunkpos,
            is_populated: false,
            neighbor_chunks: Default::default(),
            entity: Entity::PLACEHOLDER,
            mesh_handle: Handle::default(),
            mesh_handle_foliage: Handle::default(),
        }
    }

    pub fn at_voxel(&self, localpos: IVec3) -> &Vox {
        &self.voxel[Chunk::local_idx(localpos)]
    }

    pub fn at_voxel_mut(&self, localpos: IVec3) -> &mut Vox {
        self.at_voxel(localpos).as_mut()
    }

    pub fn get_voxel_neib(&self, relpos: IVec3) -> Option<Vox> {
        if Chunk::is_localpos(relpos) {
            Some(*self.at_voxel(relpos))
        } else {
            if let Some(neib_chunkptr) = self.get_chunk_neib(Chunk::neighbor_idx(relpos)?) {
                let neib_chunk = neib_chunkptr.as_ref();

                return Some(*neib_chunk.at_voxel(Chunk::as_localpos(relpos)));
            }
            None
        }
    }

    pub fn get_chunk_neib(&self, neib_idx: usize) -> Option<ChunkPtr> {
        if let Some(neib_weak) = &self.neighbor_chunks[neib_idx] {
            // assert!(neib_chunk.chunkpos == self.chunkpos + Self::NEIGHBOR_DIR[neib_idx] * Chunk::LEN, "self.chunkpos = {}, neib {} pos {}", self.chunkpos, neib_idx, neib_chunk.chunkpos);
            return Some(neib_weak.upgrade()?);
        }
        None
    }

    pub fn get_voxel_rel(&self, relpos: IVec3) -> Vox {
        self.get_voxel_neib(relpos).unwrap_or(Vox::default())
    }

    pub fn is_neighbors_complete(&self) -> bool {
        !self.neighbor_chunks.iter().any(|e| e.is_none())
    }

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
        (0..Chunk::NEIGHBOR_DIR.len()).find(|&i| Chunk::is_localpos(relpos - (Chunk::NEIGHBOR_DIR[i] * Chunk::LEN as i32)))
    }

    // assert!(Self::NEIGHBOR_DIR[idx] + Self::NEIGHBOR_DIR[opposite_idx] == IVec3::ZERO, "idx = {}, opposite = {}", idx, opposite_idx);
    pub fn neighbor_idx_opposite(idx: usize) -> usize {
        // idx MOD 2 + (idx+1) MOD 2
        idx / 2 * 2 + (idx + 1) % 2
    }
}
