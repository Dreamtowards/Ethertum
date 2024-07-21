use std::ops::Div;

use bevy::{math::ivec3, prelude::*};
use noise::{Fbm, NoiseFn, Perlin};

use super::*;
use crate::util::{hash, iter};

pub fn generate_chunk(chunk: &mut Chunk) {
    let seed = 100;
    // let perlin = Perlin::new(seed);
    let mut fbm = Fbm::<Perlin>::new(seed);
    // fbm.frequency = 0.2;
    // fbm.lacunarity = 0.2;
    fbm.octaves = 5;
    // fbm.persistence = 2;

    for ly in 0..Chunk::LEN {
        for lz in 0..Chunk::LEN {
            for lx in 0..Chunk::LEN {
                let lp = IVec3::new(lx, ly, lz);
                let p = chunk.chunkpos + lp;

                let f_terr = fbm.get(p.xz().as_dvec2().div(130.).to_array()) as f32;
                let f_3d = fbm.get(p.as_dvec3().div(90.).to_array()) as f32;

                let mut val = f_terr - (p.y as f32) / 18. + f_3d * 4.5;
                // val = (-p.y as f32 - 1.) / 18.;  // super flat

                let mut tex = VoxTex::Nil; //(p.x / 2 % 24).abs() as u16;
                if val > 0.0 {
                    tex = VoxTex::Stone;
                }
                else if p.y < 0 && val < 0. {
                    val = -0.1;
                    tex = VoxTex::Water;
                }
                *chunk.at_voxel_mut(lp) = Vox::new(tex, VoxShape::Isosurface, val);
            }
        }
    }
}

pub fn populate_chunk(chunk: &mut Chunk) {
    let chunkpos = chunk.chunkpos;
    let perlin = Perlin::new(123);

    for lx in 0..Chunk::LEN {
        for lz in 0..Chunk::LEN {
            // distance to air in top direction.
            let mut air_dist = 0;

            // check top air_dist. for CubicChunk system, otherwise the chunk-top will be surface/grass
            for i in 0..3 {
                if !chunk.get_voxel_rel_or_default(ivec3(lx, Chunk::LEN + i, lz)).is_nil() {
                    air_dist += 1;
                }
            }

            for ly in (0..Chunk::LEN).rev() {
                let lp = IVec3::new(lx, ly, lz);
                let c = chunk.at_voxel_mut(lp);

                if c.is_nil() {
                    air_dist = 0;
                } else {
                    air_dist += 1;
                }

                let p = chunk.chunkpos + lp;
                if c.tex_id == VoxTex::Stone {
                    let mut replace = c.tex_id;
                    if p.y < 2 && air_dist <= 2 && perlin.get([p.x as f64 / 32., p.z as f64 / 32.]) > 0.1 {
                        replace = VoxTex::Sand;
                    } else if air_dist <= 1 {
                        replace = VoxTex::Grass;
                    } else if air_dist < 3 {
                        replace = VoxTex::Dirt;
                    }
                    c.tex_id = replace;
                }
            }
        }
    }

    for lx in 0..Chunk::LEN {
        for lz in 0..Chunk::LEN {
            let x = chunkpos.x + lx;
            let z = chunkpos.z + lz;

            // TallGrass
            // hash(x * z * 100) < 0.23
            let g = perlin.get([x as f64 / 18., z as f64 / 18.]);
            if g > 0.0 {
                for ly in 0..Chunk::LEN - 1 {
                    let lp = ivec3(lx, ly, lz);

                    if chunk.at_voxel(lp).tex_id == VoxTex::Grass && chunk.at_voxel(lp + IVec3::Y).is_nil() {
                        let c = chunk.at_voxel_mut(lp + IVec3::Y);
                        c.tex_id = if g > 0.94 {
                            VoxTex::Rose
                        } else if g > 0.8 {
                            VoxTex::Fern
                        } else if g > 0.24 {
                            VoxTex::Bush
                        } else {
                            VoxTex::ShortGrass
                        };
                        c.shape_id = VoxShape::Grass;
                        break;
                    }
                }
            }

            // Vines
            if hash(x ^ (z * 7384)) < (18.0 / 256.0) {
                for ly in 0..Chunk::LEN - 1 {
                    let lp = ivec3(lx, ly, lz);

                    if chunk.at_voxel(lp).is_nil() && chunk.at_voxel(lp + IVec3::Y).tex_id == VoxTex::Stone {
                        for i in 0..(12.0 * hash(x ^ (z * 121))) as i32 {
                            let lp = lp + IVec3::NEG_Y * i;
                            if lp.y < 0 {
                                break;
                            }
                            let c = chunk.at_voxel_mut(lp);
                            if !c.is_nil() {
                                break;
                            }
                            c.tex_id = VoxTex::Leaves;
                            c.shape_id = VoxShape::Leaves;
                        }
                        break;
                    }
                }
            }

            // Trees
            if hash(x ^ (z * 9572)) < (3.0 / 256.0) {
                for ly in 0..Chunk::LEN {
                    let lp = ivec3(lx, ly, lz);

                    if chunk.at_voxel(lp).tex_id != VoxTex::Grass {
                        continue;
                    }
                    let siz = hash(x ^ ly ^ z);
                    gen_tree(chunk, lp, siz);
                }
            }
        }
    }
}

pub fn gen_tree(chunk: &mut Chunk, lp: IVec3, siz: f32) {
    let trunk_height = 3 + (siz * 6.0) as i32;
    let leaves_rad = 2 + (siz * 5.0) as i32;

    // Leaves
    iter::iter_aabb(leaves_rad, leaves_rad, |rp| {
        if rp.length_squared() >= leaves_rad * leaves_rad {
            return;
        }
        let lp = lp + IVec3::Y * trunk_height + rp;

        // if let Some(chunkptr) = chunk.get_chunk_rel(lp) {
        //     let vox = chunkptr.at_voxel_mut(Chunk::as_localpos(lp));
        //     vox .tex_id =VoxTex::Leaves;
        //     vox.shape_id = VoxShape::Leaves;
        // }

        chunk.set_voxel_rel(lp, |vox| {
            vox.tex_id =VoxTex::Leaves;
            vox.shape_id = VoxShape::Leaves;
        });
    });

    // Trunk
    for i in 0..trunk_height {
        if i + lp.y > 15 {
            break;
        }
        let c = chunk.at_voxel_mut(lp + IVec3::Y * i);
        c.tex_id = VoxTex::Log;
        c.shape_id = VoxShape::Isosurface;
        c.set_isovalue(2.0 * (1.2 - i as f32 / trunk_height as f32));
    }
}
