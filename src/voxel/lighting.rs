use super::*;
use bevy::math::ivec3;

// Positions avoid retaining live chunks across unloads.
pub type VoxLightQueue = Vec<(IVec3, VoxLight)>;

pub fn compute_skylight(world: &mut impl ChunkSystem, cp: IVec3, queue: &mut VoxLightQueue) {
    for x in 0..Chunk::LEN {
        for z in 0..Chunk::LEN {
            let mut level = 15;
            for y in (0..Chunk::LEN).rev() {
                let p = cp + ivec3(x, y, z);
                let Some(voxel) = world.get_voxel_mut(p) else {
                    continue;
                };
                if !voxel.is_nil() {
                    level = 0;
                }
                voxel.light.set_red(level);
                if level != 0 {
                    queue.push((p, voxel.light));
                }
            }
        }
    }
}

pub fn collect_chunk_lights(chunk: &Chunk, queue: &mut VoxLightQueue) {
    chunk.for_voxel_lights(|v, index| {
        queue.push((chunk.chunkpos + Chunk::local_idx_pos(index as i32), v.light));
    });
}

pub fn compute_voxel_light(world: &mut impl ChunkSystem, queue: &mut VoxLightQueue) {
    while let Some((p, light)) = queue.pop() {
        let level = light.red();
        if level <= 1 {
            continue;
        }
        for dir in &Chunk::NEIGHBOR_DIR[..6] {
            let neighbor = p + *dir;
            let Some(voxel) = world.get_voxel(neighbor) else {
                continue;
            };
            if voxel.light.red() < level - 1 && !voxel.is_obaque_cube() {
                let voxel = world.get_voxel_mut(neighbor).unwrap();
                voxel.light.set_red(level - 1);
                queue.push((neighbor, voxel.light));
            }
        }
    }
}
