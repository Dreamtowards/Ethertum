use std::ops::Div;

use bevy::{math::ivec3, prelude::*};

use crate::util::{hash, iter};

use super::chunk::*;

use noise::{Fbm, NoiseFn, Perlin};

pub struct WorldGen {}
use super::material::mtl;

impl WorldGen {
    pub fn generate_chunk(chunk: &mut Chunk) {
        let seed = 100;
        // let perlin = Perlin::new(seed);
        let mut fbm = Fbm::<Perlin>::new(seed);
        // fbm.frequency = 0.2;
        // fbm.lacunarity = 0.2;
        fbm.octaves = 4;
        // fbm.persistence = 2;

        for ly in 0..Chunk::SIZE {
            for lz in 0..Chunk::SIZE {
                for lx in 0..Chunk::SIZE {
                    let lp = IVec3::new(lx, ly, lz);
                    let p = chunk.chunkpos + lp;

                    let f_terr = fbm.get(p.xz().as_dvec2().div(129.).to_array()) as f32;
                    let f_3d = fbm.get(p.as_dvec3().div(70.).to_array()) as f32;

                    let mut val = f_terr - (p.y as f32) / 12. + f_3d * 2.5;

                    let mut tex = mtl::NIL; //(p.x / 2 % 24).abs() as u16;
                    if val > 0.0 {
                        tex = mtl::STONE;
                    } else if p.y < 0 && val < 0. {
                        val = 0.1;
                        tex = mtl::WATER;
                    }
                    chunk.set_cell(lp, &Cell::new(tex, 0, val));
                }
            }
        }

        Self::populate_chunk(chunk);
    }

    fn populate_chunk(chunk: &mut Chunk) {
        let chunkpos = chunk.chunkpos;
        let perlin = Perlin::new(123);

        for lx in 0..Chunk::SIZE {
            for lz in 0..Chunk::SIZE {
                let mut air_dist = 0;

                for ly in (0..Chunk::SIZE).rev() {
                    let lp = IVec3::new(lx, ly, lz);
                    let p = chunk.chunkpos + lp;

                    let mut c = *chunk.get_cell(lp);

                    if c.is_tex_empty() {
                        air_dist = 0;
                    } else {
                        air_dist += 1;
                    }

                    if c.tex_id == mtl::STONE {
                        let mut replace = c.tex_id;
                        if p.y < 2 && air_dist <= 2 && perlin.get([p.x as f64 / 32., p.z as f64 / 32.]) > 0.1 {
                            replace = mtl::SAND;
                        } else if air_dist <= 1 {
                            replace = mtl::GRASS;
                        } else if air_dist < 3 {
                            replace = mtl::DIRT;
                        }
                        c.tex_id = replace;
                    }

                    chunk.set_cell(lp, &c);
                }
            }
        }
        
        for lx in 0..Chunk::SIZE {
            for lz in 0..Chunk::SIZE {
                let x = chunkpos.x + lx;
                let z = chunkpos.z + lz;

                // Grass
                // hash(x * z * 100) < 0.23
                if perlin.get([x as f64 / 18., z as f64 / 18.]) > 0.3 {
                    for ly in 0..Chunk::SIZE-1 {
                        let lp = ivec3(lx,ly,lz);

                        if chunk.get_cell(lp).tex_id == mtl::GRASS &&
                           chunk.get_cell(lp + IVec3::Y).tex_id == 0 {

                            let c = chunk.get_cell_mut(lp + IVec3::Y);
                            c.tex_id = mtl::FERN;
                            c.shape_id = 3;
                            break;
                        }
                    }
                }

                // Vines
                if hash(x ^ z * 7384) < (24.0 / 256.0) {
                    for ly in 0..Chunk::SIZE-1 {
                        let lp = ivec3(lx,ly,lz);

                        if chunk.get_cell(lp).tex_id == 0 && 
                           chunk.get_cell(lp + IVec3::Y).tex_id == mtl::STONE {

                            for i in 0..(8.0 * hash(x*z*5321)) as i32 {
                                let lp = lp + IVec3::NEG_Y * i;
                                if lp.y < 0 {
                                    break;
                                }
                                let c = chunk.get_cell_mut(lp);
                                if c.tex_id != 0 {
                                    break;
                                } 
                                c.tex_id = mtl::LEAVES;
                                c.shape_id = 2;
                            }
                            break;
                        }
                    }
                }

                // Trees
                if hash(x ^ z * 9572) < (3.0 / 256.0) {
                    
                    for ly in 0..Chunk::SIZE {
                        let lp = ivec3(lx,ly,lz);

                        if chunk.get_cell(lp).tex_id != mtl::GRASS {
                            continue;
                        }

                        let siz = hash(x ^ ly ^ z);
                        let trunk_height = 3 + (siz * 6.0) as i32;
                        let leaves_rad = 2 + (siz * 5.0) as i32;

                        // Leaves
                        iter::iter_aabb(leaves_rad, leaves_rad, |rp| {
                            let rp = *rp;
                            if rp.length_squared() >= leaves_rad*leaves_rad {
                                return;
                            }
                            let lp = lp + IVec3::Y * trunk_height + rp;
                            if !Chunk::is_localpos(lp) {
                                return;
                            }
                            let c = chunk.get_cell_mut(lp);
                            c.tex_id = mtl::LEAVES;
                            c.shape_id = 2;
                        });

                        // Trunk
                        for i in 0..trunk_height {
                            if i + lp.y > 15 {
                                break;
                            }
                            let c = chunk.get_cell_mut(lp + IVec3::Y * i);
                            c.tex_id = mtl::LOG;
                            c.shape_id = 0;
                            c.set_isovalue(2.0 * (1.2 - i as f32 / trunk_height as f32));
                        }
                        break;
                    }
                }
            }
        }
    }
}
