use std::ops::Div;

use bevy::prelude::*;

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

                    let mut tex = mtl::STONE; //(p.x / 2 % 24).abs() as u16;
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
    }
}
