
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

                    let val = perlin.get(p.as_dvec3().div(24.).to_array());

                    if val < 0. {
                        chunk.set_cell(lp, &Cell::new(1., 1));
                    }
                }
            }
        }


    }

    fn populate_chunk(chunk: &mut Chunk) {

    }

}