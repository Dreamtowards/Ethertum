use std::sync::Arc;

use crate::prelude::*;

/// An owned, read-only neighborhood. Its voxel arrays never change with the world.
///
/// 快照持有中心区块和最多 26 个已加载邻居的独立 Chunk 外壳；体素数组通过 Arc
/// 共享，创建时不会复制 27 份体素。世界编辑时由 Arc::make_mut 写时复制，后台
/// 任务继续读取捕获时的旧数据。这里只提供 Deref，不提供 DerefMut，限制为只读。
/// 当前仍有邻居 Box 分配及字段/句柄克隆的成本，并非完全零分配。
#[derive(Clone)]
pub struct ChunkSnapshot(Chunk);

impl ChunkSnapshot {
    pub(super) fn new(chunk: Chunk) -> Self {
        Self(chunk)
    }
    // 仅供 voxel 模块内的主线程邻域编辑：消费快照后编辑独立数据，再显式写回世界。
    pub(super) fn into_chunk(self) -> Chunk {
        self.0
    }
}

impl std::ops::Deref for ChunkSnapshot {
    type Target = Chunk;
    fn deref(&self) -> &Chunk {
        &self.0
    }
}

// World systems own chunks. Clones share immutable voxel storage until a write.
#[derive(Clone)]
pub struct Chunk {
    // Arc 保护共享存储的生命周期；并发读取安全来自只读快照和下面的写时复制。
    pub(super) voxel: Arc<[Vox; Self::LEN3]>,

    pub chunkpos: IVec3,

    pub is_populated: bool,
    pub(super) force_blocky: bool,

    pub entity: Entity,
    pub mesh_handle_terrain: Handle<Mesh>, // solid terrain
    pub mesh_handle_foliage: Handle<Mesh>,
    pub mesh_handle_liquid: Handle<Mesh>,

    // Detached neighbors, populated only when preparing meshing/population snapshots.
    pub(super) neighbor_chunks: [Option<Box<Chunk>>; Self::NEIGHBOR_DIR.len()],
}

impl Chunk {
    pub const LEN: i32 = 16; // i32 for xyz iter
    pub const LEN3: usize = (Self::LEN * Self::LEN * Self::LEN) as usize;
    // pub const LOCAL_IDX_CAP: usize = 4096;  // 16^3, 2^12 bits (12 = 3 axes * 4 bits)

    pub fn new(chunkpos: IVec3) -> Self {
        Self {
            voxel: Arc::new([Vox::default(); Self::LEN3]),
            chunkpos,
            is_populated: false,
            force_blocky: false,
            neighbor_chunks: Default::default(),
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

    pub fn at_voxel_mut(&mut self, localpos: IVec3) -> &mut Vox {
        // 若快照仍共享此数组，首次写入复制整个区块的 4096 个体素；独占时直接修改。
        // 在没有新的共享引用时，后续写入复用这份数组，旧快照中的数据保持不变。
        &mut Arc::make_mut(&mut self.voxel)[Chunk::local_idx(localpos)]
    }

    pub fn get_voxel_rel(&self, relpos: IVec3) -> Option<Vox> {
        if Chunk::is_localpos(relpos) {
            Some(*self.at_voxel(relpos))
        } else {
            let neib_chunkptr = self.get_chunk_rel(relpos)?;
            Some(*neib_chunkptr.at_voxel(Chunk::as_localpos(relpos)))
        }
    }

    pub fn set_voxel_rel(&mut self, relpos: IVec3, mut visitor: impl FnMut(&mut Vox)) -> Option<Vox> {
        let vox = if Chunk::is_localpos(relpos) {
            self.at_voxel_mut(relpos)
        } else {
            self.neighbor_chunks[Self::neighbor_idx(relpos)?]
                .as_mut()?
                .at_voxel_mut(Chunk::as_localpos(relpos))
        };
        visitor(vox);
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
            if v.tex_id == VoxTex::Log {
                // v.id.light_emission != 0
                visitor(v, i);
            }
        });
    }

    pub fn get_chunk_rel(&self, relpos: IVec3) -> Option<&Chunk> {
        if Chunk::is_localpos(relpos) {
            return Some(self);
        }
        self.get_chunk_neib(Chunk::neighbor_idx(relpos)?)
    }

    pub fn get_chunk_neib(&self, neib_idx: usize) -> Option<&Chunk> {
        self.neighbor_chunks.get(neib_idx)?.as_deref()
    }

    /// Convert a finite, i32-range world position to its aligned chunk origin.
    /// Floor before integer conversion so -0.1 belongs to -16, not 0.
    /// Network callers still validate position bounds before reaching this API.
    pub fn world_to_chunkpos(p: Vec3) -> IVec3 {
        Self::as_chunkpos(p.floor().as_ivec3())
    }

    pub fn as_chunkpos(p: IVec3) -> IVec3 {
        fn _floor16(x: i32) -> i32 {
            x & (!15)
        }
        IVec3::new(_floor16(p.x), _floor16(p.y), _floor16(p.z))
    }

    pub fn as_localpos(p: IVec3) -> IVec3 {
        fn _mod16(x: i32) -> i32 {
            x & 15
        }
        IVec3::new(_mod16(p.x), _mod16(p.y), _mod16(p.z))
    }

    /// Mesh sampling covers [-1, 16] on each axis. A boundary edit can affect
    /// face, edge and corner neighbors; an interior edit affects only its owner.
    pub fn mesh_dependents(p: IVec3) -> Vec<IVec3> {
        let ranges = [p.x, p.y, p.z].map(|v| {
            let first = v.saturating_sub(1) & !15;
            let last = v.saturating_add(1) & !15;
            (first, last)
        });
        let mut result = Vec::with_capacity(8);
        for x in (ranges[0].0..=ranges[0].1).step_by(16) {
            for y in (ranges[1].0..=ranges[1].1).step_by(16) {
                for z in (ranges[2].0..=ranges[2].1).step_by(16) {
                    result.push(ivec3(x, y, z));
                }
            }
        }
        result
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
