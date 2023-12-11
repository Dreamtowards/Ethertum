use bevy::{
    prelude::*,
    render::{mesh::{Mesh, Indices}, render_resource::PrimitiveTopology},
};

use super::chunk::*;

#[derive(Default)]
pub struct VertexBuffer {
    pub pos: Vec<Vec3>,
    pub uv: Vec<Vec2>,
    pub norm: Vec<Vec3>,

    pub indices: Vec<u32>,
}

impl VertexBuffer {

    fn push_vertex(&mut self, pos: Vec3, uv: Vec2, norm: Vec3) {
        self.pos.push(pos);
        self.uv.push(uv);
        self.norm.push(norm);
    }

    fn into_mesh(self) -> Mesh {
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

    pub fn generate_chunk_mesh(chunk: &Chunk) -> Mesh {

        let mut vbuf = VertexBuffer::default();
        
        for y in 0..Chunk::SIZE {
            for z in 0..Chunk::SIZE {
                for x in 0..Chunk::SIZE {
                    let lp = IVec3::new(x, y, z);

                    let cell = chunk.get_cell(lp);

                    if !cell.is_empty() {


                        put_cube(&mut vbuf, lp);

                    }
                }
            }
        }

        vbuf.into_mesh()
    }

}




fn put_cube(vbuf: &mut VertexBuffer, lp: IVec3) {
    let vtx_base = vbuf.pos.len() as u32;
    let idx_base = vbuf.indices.len() as u32;

    info!("GenBlock at {:?}", lp);

    vbuf.pos.extend(vec![
        // top (facing towards +y)
        Vec3::new(-0.5, 0.5, -0.5) + lp.as_vec3(), // vertex with index 0
        Vec3::new(0.5, 0.5, -0.5) + lp.as_vec3(), // vertex with index 1
        Vec3::new(0.5, 0.5, 0.5) + lp.as_vec3(), // etc. until 23
        Vec3::new(-0.5, 0.5, 0.5) + lp.as_vec3(),
        // bottom   (-y)
        Vec3::new(-0.5, -0.5, -0.5) + lp.as_vec3(),
        Vec3::new(0.5, -0.5, -0.5) + lp.as_vec3(),
        Vec3::new(0.5, -0.5, 0.5) + lp.as_vec3(),
        Vec3::new(-0.5, -0.5, 0.5) + lp.as_vec3(),
        // right    (+x)
        Vec3::new(0.5, -0.5, -0.5) + lp.as_vec3(),
        Vec3::new(0.5, -0.5, 0.5) + lp.as_vec3(),
        Vec3::new(0.5, 0.5, 0.5) + lp.as_vec3(), // This vertex is at the same position as vertex with index 2, but they'll have different UV and normal
        Vec3::new(0.5, 0.5, -0.5) + lp.as_vec3(),
        // left     (-x)
        Vec3::new(-0.5, -0.5, -0.5) + lp.as_vec3(),
        Vec3::new(-0.5, -0.5, 0.5) + lp.as_vec3(),
        Vec3::new(-0.5, 0.5, 0.5) + lp.as_vec3(),
        Vec3::new(-0.5, 0.5, -0.5) + lp.as_vec3(),
        // back     (+z)
        Vec3::new(-0.5, -0.5, 0.5) + lp.as_vec3(),
        Vec3::new(-0.5, 0.5, 0.5) + lp.as_vec3(),
        Vec3::new(0.5, 0.5, 0.5) + lp.as_vec3(),
        Vec3::new(0.5, -0.5, 0.5) + lp.as_vec3(),
        // forward  (-z)
        Vec3::new(-0.5, -0.5, -0.5) + lp.as_vec3(),
        Vec3::new(-0.5, 0.5, -0.5) + lp.as_vec3(),
        Vec3::new(0.5, 0.5, -0.5) + lp.as_vec3(),
        Vec3::new(0.5, -0.5, -0.5) + lp.as_vec3(),
    ]);
    vbuf.norm.extend(vec![
        // Normals for the top side (towards +y)
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        // Normals for the bottom side (towards -y)
        Vec3::new(0.0, -1.0, 0.0),
        Vec3::new(0.0, -1.0, 0.0),
        Vec3::new(0.0, -1.0, 0.0),
        Vec3::new(0.0, -1.0, 0.0),
        // Normals for the right side (towards +x)
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        // Normals for the left side (towards -x)
        Vec3::new(-1.0, 0.0, 0.0),
        Vec3::new(-1.0, 0.0, 0.0),
        Vec3::new(-1.0, 0.0, 0.0),
        Vec3::new(-1.0, 0.0, 0.0),
        // Normals for the back side (towards +z)
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, 1.0),
        // Normals for the forward side (towards -z)
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 0.0, -1.0),
    ]);
    vbuf.uv.extend(vec![
        // Assigning the UV coords for the top side.
        Vec2::new(0.0, 0.2), Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0), Vec2::new(1.0, 0.25),
        // Assigning the UV coords for the bottom side.
        Vec2::new(0.0, 0.45), Vec2::new(0.0, 0.25), Vec2::new(1.0, 0.25), Vec2::new(1.0, 0.45),
        // Assigning the UV coords for the right side.
        Vec2::new(1.0, 0.45), Vec2::new(0.0, 0.45), Vec2::new(0.0, 0.2), Vec2::new(1.0, 0.2),
        // Assigning the UV coords for the left side.
        Vec2::new(1.0, 0.45), Vec2::new(0.0, 0.45), Vec2::new(0.0, 0.2), Vec2::new(1.0, 0.2),
        // Assigning the UV coords for the back side.
        Vec2::new(0.0, 0.45), Vec2::new(0.0, 0.2), Vec2::new(1.0, 0.2), Vec2::new(1.0, 0.45),
        // Assigning the UV coords for the forward side.
        Vec2::new(0.0, 0.45), Vec2::new(0.0, 0.2), Vec2::new(1.0, 0.2), Vec2::new(1.0, 0.45),
    ]);
    vbuf.indices.extend(vec![
        0,3,1 , 1,3,2, // triangles making up the top (+y) facing side.
        4,5,7 , 5,6,7, // bottom (-y)
        8,11,9 , 9,11,10, // right (+x)
        12,13,15 , 13,14,15, // left (-x)
        16,19,17 , 17,19,18, // back (+z)
        20,21,23 , 21,22,23, // forward (-z)
    ]);
    // apply rel idx
    for val in vbuf.indices[idx_base as usize..].iter_mut() {
        *val = vtx_base + *val;
    }

}