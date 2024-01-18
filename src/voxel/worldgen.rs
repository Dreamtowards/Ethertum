
use std::ops::Div;

use bevy::prelude::*;

use super::chunk::*;

use noise::{NoiseFn, Perlin};

pub struct WorldGen {

}
use super::material::mtl;


impl WorldGen {

   
    pub fn generate_chunk(chunk: &mut Chunk) {
        
        let seed = 100;
        let perlin = Perlin::new(seed);

        for ly in 0..Chunk::SIZE {
            for lz in 0..Chunk::SIZE {
                for lx in 0..Chunk::SIZE {
                    let lp = IVec3::new(lx, ly, lz);
                    let p = chunk.chunkpos + lp;

                    let f_terr = perlin.get(p.xz().as_dvec2().div(64.).to_array()) as f32;
                    let f_3d = perlin.get(p.as_dvec3().div(24.).to_array()) as f32;

                    let val = f_terr - (p.y as f32) / 18. + f_3d * 2.;
                    
                    let mtl = mtl::STONE;//(p.x / 2 % 24).abs() as u16;
                    chunk.set_cell(lp, &Cell::new(val, mtl));
                }
            }
        }


        // Self::populate_chunk(chunk);
    }

    fn populate_chunk(chunk: &mut Chunk) {

        for lx in 0..Chunk::SIZE {
            for lz in 0..Chunk::SIZE {
                let mut num_to_air = 0;

                for ly in (0..Chunk::SIZE).rev() {
                    let lp = IVec3::new(lx, ly, lz);
                    let p = chunk.chunkpos + lp;

                    let mut c = *chunk.get_cell(lp);

                    if c.is_empty() {
                        num_to_air = 0;
                    } else {
                        num_to_air += 1;
                    }

                    let mut replace = c.mtl;
                    if num_to_air == 1 {
                        replace = mtl::GRASS;
                    } else if num_to_air < 5 {
                        replace = mtl::DIRT;
                    }
                    c.mtl = replace;

                    chunk.set_cell(lp, &c);
                }
            }
        }
    }

}