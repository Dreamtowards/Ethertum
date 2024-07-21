use bevy::math::ivec3;
use std::sync::{Arc, Weak};

use crate::{prelude::*, util::{as_mut, AsMutRef}};
use super::{ChunkPtr, vox::*};


// Chunk is "Heavy" type (big size, stored a lot voxels). thus copy/clone are not allowed.
pub struct Chunk {
    // shoud Box?
    voxel: [Vox; Self::LEN3],

    pub chunkpos: IVec3,

    pub is_populated: bool,

    pub entity: Entity,
    pub mesh_handle_terrain: Handle<Mesh>, // solid terrain
    pub mesh_handle_foliage: Handle<Mesh>,
    pub mesh_handle_liquid: Handle<Mesh>,

    // cached neighbor chunks that loaded to the ChunkSystem.
    // for Quick Access without global find neighbor chunk by chunkpos
    pub neighbor_chunks: [Option<Weak<Chunk>>; Self::NEIGHBOR_DIR.len()],

    // Self Arc. for export self Arc in get_chunk_neib().  assigned by ChunkSystem::spawn_chunk()
    pub chunkptr_weak: Weak<Chunk>,  
}

impl Chunk {
    pub const LEN: i32 = 16; // i32 for xyz iter
    pub const LEN3: usize = (Self::LEN * Self::LEN * Self::LEN) as usize;
    // pub const LOCAL_IDX_CAP: usize = 4096;  // 16^3, 2^12 bits (12 = 3 axes * 4 bits)

    pub fn new(chunkpos: IVec3) -> Self {
        Self {
            voxel: [Vox::default(); Self::LEN3],
            chunkpos,
            is_populated: false,
            neighbor_chunks: Default::default(),
            chunkptr_weak: Weak::default(),
            entity: Entity::PLACEHOLDER,
            mesh_handle_terrain: Handle::default(),
            mesh_handle_foliage: Handle::default(),
            mesh_handle_liquid: Handle::default(),
        }
    }

    // Voxel Cell

    // pub fn ax_voxel(&self, local_idx: usize) -> &Vox {
    //     &self.voxel[local_idx]
    // }

    pub fn at_voxel(&self, localpos: IVec3) -> &Vox {
        &self.voxel[Chunk::local_idx(localpos)]
    }

    pub fn at_voxel_mut(&self, localpos: IVec3) -> &mut Vox {
        self.at_voxel(localpos).as_mut()
    }

    pub fn get_voxel_rel(&self, relpos: IVec3) -> Option<Vox> {
        if Chunk::is_localpos(relpos) {
            Some(*self.at_voxel(relpos))
        } else {
            let neib_chunkptr = self.get_chunk_rel(relpos)?;
            Some(*neib_chunkptr.at_voxel(Chunk::as_localpos(relpos)))
        }
    }

    pub fn set_voxel_rel(&self, relpos: IVec3, mut visitor: impl FnMut(&mut Vox)) -> Option<Vox> {
        let vox;
        let neib_chunkptr;
        if Chunk::is_localpos(relpos) {
            vox = self.at_voxel(relpos);
        } else {
            neib_chunkptr = self.get_chunk_rel(relpos)?;
            vox = neib_chunkptr.at_voxel(Chunk::as_localpos(relpos));
        }
        visitor(vox.as_mut());
        Some(*vox)
    }
    pub fn get_voxel_rel_or_default(&self, relpos: IVec3) -> Vox {
        self.get_voxel_rel(relpos).unwrap_or(Vox::default())
    }

    pub fn for_voxels(&self, mut visitor: impl FnMut(&Vox, usize)) {
        for i in 0..Self::LEN3 {
            visitor(&self.voxel[i], i);
        }
    }

    // light sources
    pub fn for_voxel_lights(&self, mut visitor: impl FnMut(&Vox, usize)) {
        self.for_voxels(|v, i| {
            if v.tex_id == VoxTex::Log {  // v.id.light_emission != 0
                visitor(v, i);
            }
        });
    }

    pub fn get_chunk_rel(&self, relpos: IVec3) -> Option<ChunkPtr> {
        if Chunk::is_localpos(relpos) {
            return self.chunkptr_weak.upgrade();
        }
        self.get_chunk_neib(Chunk::neighbor_idx(relpos)?)
    }

    pub fn get_chunk_neib(&self, neib_idx: usize) -> Option<ChunkPtr> {
        // if neib_idx == usize::MAX {
        //     return Some(self.chunkptr_weak.upgrade()?);
        // }
        if let Some(neib_weak) = &self.neighbor_chunks[neib_idx] {
            // assert!(neib_chunk.chunkpos == self.chunkpos + Self::NEIGHBOR_DIR[neib_idx] * Chunk::LEN, "self.chunkpos = {}, neib {} pos {}", self.chunkpos, neib_idx, neib_chunk.chunkpos);
            return Some(neib_weak.upgrade()?);
        }
        None
    }



    pub fn as_chunkpos(p: IVec3) -> IVec3 {
        fn _floor16(x: i32) -> i32 { x & (!15) }
        IVec3::new(_floor16(p.x), _floor16(p.y), _floor16(p.z))
    }

    pub fn as_localpos(p: IVec3) -> IVec3 {
        fn _mod16(x: i32) -> i32 { x & 15 }
        IVec3::new(_mod16(p.x), _mod16(p.y), _mod16(p.z))
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

    pub fn is_neighbors_all_loaded(&self) -> bool {
        !self.neighbor_chunks.iter().any(|e| e.is_none())
    }

    fn neighbor_idx(relpos: IVec3) -> Option<usize> {
        assert!(!Chunk::is_localpos(relpos));
        // if Chunk::is_localpos(relpos) {
        //     return Some(usize::MAX);
        // }
        (0..Chunk::NEIGHBOR_DIR.len()).find(|&i| Chunk::is_localpos(relpos - (Chunk::NEIGHBOR_DIR[i] * Chunk::LEN as i32)))
    }

    // assert!(Self::NEIGHBOR_DIR[idx] + Self::NEIGHBOR_DIR[opposite_idx] == IVec3::ZERO, "idx = {}, opposite = {}", idx, opposite_idx);
    pub fn neighbor_idx_opposite(idx: usize) -> usize {
        // idx MOD 2 + (idx+1) MOD 2
        idx / 2 * 2 + (idx + 1) % 2
    }







    // Voxel Light

    // pub fn at_lights(&self, localpos: IVec3) -> &VoxLight {
    //     &self.light[Chunk::local_idx(localpos)]
    // }
    // pub fn at_lights_mut(&self, localpos: IVec3) -> &mut VoxLight {
    //     as_mut(self.at_lights(localpos))
    // }

    // pub fn reset_lights(&mut self) {
    //     self.light = [VoxLight::default(); Self::LEN3];
    // }

    // pub fn at_light(&self, localpos: IVec3, chan: u8) -> u16 {
    //     self.at_lights(Chunk::local_idx(localpos)).get(chan)
    // }

    // pub fn set_light(&mut self, local_idx: u16, chan: u8, val: u8) {
    //     self.at_lights_mut(local_idx).set(chan, val);
    // }

}
