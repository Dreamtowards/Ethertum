
use std::ops::Div;

use bevy::prelude::*;

use super::chunk::*;

use noise::{NoiseFn, Perlin};

pub struct WorldGen {

}


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
                    
                    chunk.set_cell(lp, &Cell::new(val, 1));
                }
            }
        }


    }

    fn populate_chunk(chunk: &mut Chunk) {

    }

}