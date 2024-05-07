

use super::*;
use bevy::math::ivec3;

pub type VoxLightQueue = Vec<(ChunkPtr, u16, VoxLight)>;


pub fn compute_skylight(chunk: &ChunkPtr, queue_add: &mut VoxLightQueue) {

    for lx in 0..Chunk::LEN {
        for lz in 0..Chunk::LEN {
            let mut lightlevel = 15;

            for ly in (0..Chunk::LEN).rev() {
                let lp = ivec3(lx, ly, lz);
                
                let v = chunk.at_voxel_mut(lp);
                if !v.is_nil() {
                    lightlevel = 0;
                }
                v.light.set_red(lightlevel);

                if lightlevel != 0 {
                    
                    queue_add.push((chunk.clone(), Chunk::local_idx(lp) as u16, v.light));
                }
            }
        }
    }
}

pub fn collect_chunk_lights(chunk: &ChunkPtr, queue_add: &mut VoxLightQueue) {
    chunk.for_voxel_lights(|v, local_idx| {
        queue_add.push((chunk.clone(), local_idx as u16, v.light));
    });
}

pub fn compute_voxel_light(queue_add: &mut VoxLightQueue, queue_del: &mut VoxLightQueue) {

    while let Some((chunkptr, local_idx, light)) = queue_add.pop() {
        let lp = Chunk::local_idx_pos(local_idx as i32);
        let lightlevel = light.red();

        for i in 0..6 {
            let dir = Chunk::NEIGHBOR_DIR[i];
            try_spread_light(&chunkptr, lp+dir, lightlevel, queue_add, queue_del);
        }
    }

}

fn try_spread_light(chunk: &ChunkPtr, lp: IVec3, lightlevel: u16, queue_add: &mut VoxLightQueue, queue_del: &mut VoxLightQueue) {
    let chunk = if Chunk::is_localpos(lp) {chunk} else { &chunk.get_chunk_rel(lp).unwrap() };
    let lp = Chunk::as_localpos(lp);

    let vox = chunk.at_voxel_mut(lp);
    
    if  lightlevel > 0 && vox.light.red() < lightlevel-1 && !vox.is_obaque_cube() {
        vox.light.set_red(lightlevel-1);

        queue_add.push((chunk.clone(), Chunk::local_idx(lp) as u16, vox.light));
    }

    // if removal
    // let old_light = vox.light;
    // if  vox.light.red() != 0 && vox.light.red() < lightlevel {
    //     vox.light.set_red(0);

    //     queue_del.push((chunk.clone(), Chunk::local_idx(lp) as u16, old_light));
    // } else if vox.light.red() >= lightlevel {

    //     queue_add.push((chunk.clone(), Chunk::local_idx(lp) as u16, vox.light));
    // }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vox_light() {

        let mut l = VoxLight::new(5, 6, 7, 8);
        println!("{}", l);
        
        for i in 0..18 {
            l.set_sky(i);
            l.set_red(i+1);
            l.set_green(i+2);
            l.set_blue(i+3);
            println!("{}", l);
        }
    }
}