use bevy::{
    prelude::*,
    render::{mesh::{Mesh, Indices}, render_resource::PrimitiveTopology},
};

use super::{chunk::*, chunk_system::ChunkPtr};

#[derive(Default)]
pub struct VertexBuffer {
    pub pos: Vec<Vec3>,
    pub uv: Vec<Vec2>,
    pub norm: Vec<Vec3>,

    pub indices: Vec<u32>,
}

impl VertexBuffer {

    pub fn with_capacity(num_vert: usize) -> Self {
        let mut vtx = VertexBuffer::default();
        vtx.pos.reserve(num_vert * 3);
        vtx.uv.reserve(num_vert * 2);
        vtx.norm.reserve(num_vert * 3);
        vtx
    }

    pub fn push_vertex(&mut self, pos: Vec3, uv: Vec2, norm: Vec3) {
        self.pos.push(pos);
        self.uv.push(uv);
        self.norm.push(norm);
    }

    // fn is_indexed(&self) -> bool {
    //     !self.indices.is_empty()
    // }

    pub fn into_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.pos)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uv)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.norm);

        if !self.indices.is_empty() {
            mesh.set_indices(Some(Indices::U32(self.indices)));
        }

        mesh
    }

}


pub struct MeshGen {

}

impl MeshGen {

    pub fn generate_chunk_mesh(vbuf: &mut VertexBuffer, chunk: &ChunkPtr) {

        for y in 0..Chunk::SIZE {
            for z in 0..Chunk::SIZE {
                for x in 0..Chunk::SIZE {
                    let lp = IVec3::new(x, y, z);

                    let cell = chunk.get_cell(lp);

                    if !cell.is_empty() {


                        put_cube(vbuf, lp, chunk);

                    }
                }
            }
        }
    }

}




static CUBE_POS: [f32;6*6*3] = [
    0., 0., 1., 0., 1., 1., 0., 1., 0.,  // Left -X
    0., 0., 1., 0., 1., 0., 0., 0., 0.,
    1., 0., 0., 1., 1., 0., 1., 1., 1.,  // Right +X
    1., 0., 0., 1., 1., 1., 1., 0., 1.,
    0., 0., 1., 0., 0., 0., 1., 0., 0.,  // Bottom -Y
    0., 0., 1., 1., 0., 0., 1., 0., 1.,
    0., 1., 1., 1., 1., 1., 1., 1., 0.,  // Bottom +Y
    0., 1., 1., 1., 1., 0., 0., 1., 0.,
    0., 0., 0., 0., 1., 0., 1., 1., 0.,  // Front -Z
    0., 0., 0., 1., 1., 0., 1., 0., 0.,
    1., 0., 1., 1., 1., 1., 0., 1., 1.,  // Back +Z
    1., 0., 1., 0., 1., 1., 0., 0., 1.,
];

static CUBE_UV: [f32;6*6*2] = [
    1., 0., 1., 1., 0., 1., 1., 0., 0., 1., 0., 0.,  // One Face.
    1., 0., 1., 1., 0., 1., 1., 0., 0., 1., 0., 0.,
    1., 0., 1., 1., 0., 1., 1., 0., 0., 1., 0., 0.,
    1., 0., 1., 1., 0., 1., 1., 0., 0., 1., 0., 0.,
    1., 0., 1., 1., 0., 1., 1., 0., 0., 1., 0., 0.,
    1., 0., 1., 1., 0., 1., 1., 0., 0., 1., 0., 0.,
];

static CUBE_NORM: [f32;6*6*3] = [
    -1., 0., 0.,-1., 0., 0.,-1., 0., 0.,-1., 0., 0.,-1., 0., 0.,-1., 0., 0.,
    1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0.,
    0.,-1., 0., 0.,-1., 0., 0.,-1., 0., 0.,-1., 0., 0.,-1., 0., 0.,-1., 0.,
    0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0.,
    0., 0.,-1., 0., 0.,-1., 0., 0.,-1., 0., 0.,-1., 0., 0.,-1., 0., 0.,-1.,
    0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1.,
];

// static CUBE_IDX: [u32;6*6] = [
// ];


fn put_cube(vbuf: &mut VertexBuffer, lp: IVec3, chunk: &ChunkPtr) {
    
    for face_i in 0..6 {
        let face_dir = Vec3::from_slice(&CUBE_NORM[face_i*18..]);  // 18: 3 scalar * 3 vertex * 2 triangle

        if let Some(neib) = chunk.get_cell_neighbor(lp + face_dir.as_ivec3()) {
            if !neib.is_empty() {
                continue;
            }
        } 

        for vert_i in 0..6 {
            vbuf.push_vertex(
                Vec3::from_slice(&CUBE_POS[face_i*18 + vert_i*3..]) + lp.as_vec3(), 
                Vec2::from_slice(&CUBE_UV[face_i*12 + vert_i*2..]), 
                Vec3::from_slice(&CUBE_NORM[face_i*18 + vert_i*3..]), 
            );
        }
    }

}
