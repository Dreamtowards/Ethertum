
use bevy::prelude::*;

use super::chunk::*;

pub struct WorldGen {

}


impl WorldGen {

   
    pub fn generate_chunk(chunk: &mut Chunk) {

        // for y in 0..Chunk::SIZE {
        //     for z in 0..Chunk::SIZE {
        //         for x in 0..Chunk::SIZE {
        //             let lp = IVec3::new(x, y, z);

        //         }
        //     }
        // }

        chunk.set_cell(IVec3::new(0,0,0), &Cell::new(1., 1));
        chunk.set_cell(IVec3::new(0,0,1), &Cell::new(1., 1));
        chunk.set_cell(IVec3::new(0,0,2), &Cell::new(1., 1));
        chunk.set_cell(IVec3::new(0,2,0), &Cell::new(1., 1));

    }

    fn populate_chunk(chunk: &mut Chunk) {

    }

}