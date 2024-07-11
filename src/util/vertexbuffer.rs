
// Temporary Solution. since i want make Vec3 as HashMap's key but glam Vec3 doesn't support trait of Hash, Eq,
// #[derive(PartialEq)]
// struct HashVec3(Vec3);
// impl Hash for HashVec3 {
//     fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
//         self.0.x.to_bits().hash(state);
//         self.0.y.to_bits().hash(state);
//         self.0.z.to_bits().hash(state);
//     }
// }
// impl Eq for HashVec3 {}

use std::{fmt::Error, hash::Hash};
use std::ops::Mul;

use bevy::{prelude::*, render::mesh::Indices, utils::HashMap};

#[derive(Clone, Copy)]
pub struct Vertex {
    pub pos: Vec3,
    pub uv: Vec2,
    pub norm: Vec3,
}

impl Hash for Vertex {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.pos.mul(100.).as_ivec3().hash(state);
        self.norm.mul(100.).as_ivec3().hash(state);
        self.uv.mul(100.).as_ivec2().hash(state);
        // self.pos.x.to_bits().hash(state);
        // self.pos.y.to_bits().hash(state);
        // self.pos.z.to_bits().hash(state);
        // self.norm.x.to_bits().hash(state);
        // self.norm.y.to_bits().hash(state);
        // self.norm.z.to_bits().hash(state);
        // self.uv.x.to_bits().hash(state);
        // self.uv.y.to_bits().hash(state);
    }
}
impl PartialEq for Vertex {
    fn eq(&self, other: &Self) -> bool {
        self.pos.mul(100.).as_ivec3() == other.pos.mul(100.).as_ivec3()
            && self.norm.mul(100.).as_ivec3() == other.norm.mul(100.).as_ivec3()
            && self.uv.mul(100.).as_ivec2() == other.uv.mul(100.).as_ivec2()
    }
}

impl Eq for Vertex {}

#[derive(Default)]
pub struct VertexBuffer {
    pub vertices: Vec<Vertex>,

    pub indices: Vec<u32>,
}

impl VertexBuffer {
    // pub fn with_capacity(num_vert: usize) -> Self {
    //     let mut vtx = VertexBuffer::default();
    //     vtx.vertices.reserve(num_vert);
    //     vtx
    // }

    pub fn push_vertex(&mut self, pos: Vec3, uv: Vec2, norm: Vec3) {
        self.vertices.push(Vertex { pos, uv, norm });
    }

    pub fn is_indexed(&self) -> bool {
        !self.indices.is_empty()
    }

    pub fn vertex_count(&self) -> usize {
        if self.is_indexed() {
            self.indices.len()
        } else {
            self.vertices.len()
        }
    }

    // len_triangles
    fn triangle_count(&self) -> u32 {
        (self.vertex_count() / 3) as u32
    }

    fn vert(&self, idx: u32) -> &Vertex {
        if self.is_indexed() {
            &self.vertices[self.indices[idx as usize] as usize]
        } else {
            &self.vertices[idx as usize]
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    // pub fn compute_flat_normals(&mut self) {
    //     assert!(!self.is_indexed());

    //     for tri_i in 0..self.triangle_count() {
    //         let v = &mut self.vertices[tri_i as usize * 3..];
    //         let p0 = v[0].pos;
    //         let p1 = v[1].pos;
    //         let p2 = v[2].pos;

    //         let n = (p1 - p0).cross(p2 - p0).normalize();

    //         v[0].norm = n;
    //         v[1].norm = n;
    //         v[2].norm = n;
    //     }
    // }

    pub fn compute_smooth_normals(&mut self) {
        const SCALE: f32 = 100.;

        let mut pos2norm = HashMap::<IVec3, Vec3>::new();

        for tri_i in 0..self.triangle_count() {
            let p0 = self.vert(tri_i * 3).pos;
            let p1 = self.vert(tri_i * 3 + 1).pos;
            let p2 = self.vert(tri_i * 3 + 2).pos;

            let n = (p1 - p0).cross(p2 - p0);

            let a0 = (p1 - p0).angle_between(p2 - p0);
            let a1 = (p2 - p1).angle_between(p0 - p1);
            let a2 = (p0 - p2).angle_between(p1 - p2);

            *pos2norm.entry(p0.mul(SCALE).as_ivec3()).or_default() += n * a0;
            *pos2norm.entry(p1.mul(SCALE).as_ivec3()).or_default() += n * a1;
            *pos2norm.entry(p2.mul(SCALE).as_ivec3()).or_default() += n * a2;
        }

        for norm in pos2norm.values_mut() {
            *norm = norm.normalize();
        }

        for v in &mut self.vertices {
            v.norm = *pos2norm.get(&v.pos.mul(SCALE).as_ivec3()).unwrap();
        }
    }

    pub fn compute_indexed_naive(&mut self) {
        assert!(!self.is_indexed());
        self.indices.clear();
        self.indices.reserve(self.vertex_count());

        for i in 0..self.vertex_count() {
            self.indices.push(i as u32);
        }
    }

    // pub fn compute_indexed(&mut self) {
    //     assert!(!self.is_indexed());
    //     self.indices.clear();
    //     self.indices.reserve(self.vertex_count());

    //     let mut vert2idx = HashMap::<Vertex, u32>::new();

    //     let mut vertices = Vec::new();

    //     for vert in self.vertices.iter() {
    //         // if let Some(idx) = vert2idx.get(vert) {
    //         //     self.indices.push(*idx);
    //         // } else {
    //         //     let idx = vertices.len() as u32;
    //         //     vert2idx.insert(*vert, idx);
    //         //     vertices.push(*vert);
    //         //     self.indices.push(idx);
    //         // }

    //         match vert2idx.entry(*vert) {
    //             Entry::Occupied(e) => {
    //                 let idx = *e.get();
    //                 self.indices.push(idx);
    //             }
    //             Entry::Vacant(e) => {
    //                 let idx = vertices.len() as u32;
    //                 e.insert(idx);
    //                 vertices.push(*vert);
    //                 self.indices.push(idx);
    //             }
    //         }
    //     }

    //     self.vertices = vertices;
    // }

    pub fn to_mesh(&self, mesh: &mut Mesh) {
        let pos: Vec<Vec3> = self.vertices.iter().map(|v| v.pos).collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);

        let uv: Vec<Vec2> = self.vertices.iter().map(|v| v.uv).collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);

        let norm: Vec<Vec3> = self.vertices.iter().map(|v| v.norm).collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, norm);

        if self.is_indexed() {
            mesh.insert_indices(Indices::U32(self.indices.clone()));
        }
    }

    pub fn export_obj(&self) -> Result<String, Error> {
        use std::fmt::Write;
        let mut buf = String::new();
        let num_verts = self.vertex_count() as u32;

        for i in 0..num_verts {
            let v = self.vert(i);
            writeln!(buf, "v {} {} {}", v.pos.x, v.pos.y, v.pos.z)?;
        }
        for i in 0..num_verts {
            let v = self.vert(i);
            writeln!(&mut buf, "vt {} {}", v.uv.x, v.uv.y)?;
        }
        for i in 0..num_verts {
            let v = self.vert(i);
            writeln!(&mut buf, "vn {} {} {}", v.norm.x, v.norm.y, v.norm.z)?;
        }

        for _ti in 0..num_verts/3 {
            let i = _ti*3 + 1; // global index offset 1, obj spec.
            writeln!(&mut buf, "f {}/{}/{} {}/{}/{} {}/{}/{}", i,i,i,i+1,i+1,i+1,i+2,i+2,i+2)?;
        }

        Ok(buf)
    }
}