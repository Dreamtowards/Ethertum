use std::{f32::consts::PI, hash::Hash, ops::Mul};

use bevy::{
    math::{ivec3, vec2, vec3},
    prelude::*,
    render::mesh::Indices,
    utils::HashMap,
};
use bevy_egui::egui::emath::inverse_lerp;

use crate::util::iter;
use super::chunk::*;

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
}

pub struct MeshGen {}

impl MeshGen {
    pub fn generate_chunk_mesh(vbuf: &mut VertexBuffer, chunk: &Chunk) {
        Self::sn_contouring(vbuf, chunk);

        for ly in 0..Chunk::LEN {
            for lz in 0..Chunk::LEN {
                for lx in 0..Chunk::LEN {
                    let lp = IVec3::new(lx, ly, lz);

                    let c = chunk.at_voxel(lp);

                    if c.tex_id != 0 && c.shape_id == VoxShape::Cube {
                        put_cube(vbuf, lp, chunk, c.tex_id);
                    }
                }
            }
        }
    }

    pub fn generate_chunk_mesh_foliage(vbuf: &mut VertexBuffer, chunk: &Chunk) {
        iter::iter_xzy(Chunk::LEN, |lp| {
            let c = chunk.at_voxel(lp);

            if c.tex_id != 0 {
                if c.shape_id == VoxShape::Leaves {
                    put_leaves(vbuf, lp.as_vec3(), c.tex_id);
                } else if c.shape_id == VoxShape::Grass {
                    put_grass(vbuf, lp.as_vec3(), c.tex_id);
                }
            }
        });
    }

    const AXES: [IVec3; 3] = [ivec3(1, 0, 0), ivec3(0, 1, 0), ivec3(0, 0, 1)];
    const ADJACENT: [[IVec3; 6]; 3] = [
        [
            ivec3(0, 0, 0),
            ivec3(0, -1, 0),
            ivec3(0, -1, -1),
            ivec3(0, -1, -1),
            ivec3(0, 0, -1),
            ivec3(0, 0, 0),
        ],
        [
            ivec3(0, 0, 0),
            ivec3(0, 0, -1),
            ivec3(-1, 0, -1),
            ivec3(-1, 0, -1),
            ivec3(-1, 0, 0),
            ivec3(0, 0, 0),
        ],
        [
            ivec3(0, 0, 0),
            ivec3(-1, 0, 0),
            ivec3(-1, -1, 0),
            ivec3(-1, -1, 0),
            ivec3(0, -1, 0),
            ivec3(0, 0, 0),
        ],
    ];

    const VERT: [IVec3; 8] = [
        ivec3(0, 0, 0), // 0
        ivec3(0, 0, 1),
        ivec3(0, 1, 0), // 2
        ivec3(0, 1, 1),
        ivec3(1, 0, 0), // 4
        ivec3(1, 0, 1),
        ivec3(1, 1, 0), // 6
        ivec3(1, 1, 1),
    ];
    // from min to max in each Edge.  axis order x y z.
    // Diagonal Edge in Cell is in-axis-flip-index edge.  i.e. diag of edge[axis*4 +i] is edge[axis*4 +(3-i)]
    /*     +--2--+    +-----+    +-----+
     *    /|    /|   /7    /6  11|    10
     *   +--3--+ |  +-----+ |  +-----+ |
     *   | +--0|-+  5 +---4-+  | +---|-+
     *   |/    |/   |/    |/   |9    |8
     *   +--1--+    +-----+    +-----+
     *   |3  2| winding. for each axis.
     *   |1  0|
     */
    const EDGE: [[usize; 2]; 12] = [
        [0, 4],
        [1, 5],
        [2, 6],
        [3, 7], // X
        [5, 7],
        [1, 3],
        [4, 6],
        [0, 2], // Y
        [4, 5],
        [0, 1],
        [6, 7],
        [2, 3], // Z
    ];

    fn sn_signchanged(c0: &Vox, c1: &Vox) -> bool {
        (c0.isovalue() > 0.) != (c1.isovalue() > 0.) // use .is_empty() ?
    }

    // Naive SurfaceNets Method of Evaluate FeaturePoint.
    // return in-cell point.
    fn sn_featurepoint(lp: IVec3, chunk: &Chunk) -> Vec3 {
        let mut sign_changes = 0;
        let mut fp_sum = Vec3::ZERO;

        for edge_i in 0..12 {
            let edge = Self::EDGE[edge_i];
            let v0 = Self::VERT[edge[0]];
            let v1 = Self::VERT[edge[1]];
            let c0 = chunk.get_voxel_rel(lp + v0);
            let c1 = chunk.get_voxel_rel(lp + v1);

            if Self::sn_signchanged(&c0, &c1) {
                if let Some(t) = inverse_lerp(c0.isovalue()..=c1.isovalue(), 0.0) {
                    // if !t.is_finite() {
                    //     continue;
                    // }
                    assert!(t.is_finite(), "t = {}", t);

                    let p = t * (v1 - v0).as_vec3() + v0.as_vec3(); // (v1-v0) must > 0. since every edge vert are min-to-max

                    fp_sum += p;
                    sign_changes += 1;
                }
            }
        }

        // assert_ne!(sign_changes, 0);
        if sign_changes == 0 {
            // 由于外力修改 eg Water，可能存在非法情况 此时还不至于panic
            return Vec3::ONE * 0.5;
        }
        assert!(fp_sum.is_finite());

        fp_sum / (sign_changes as f32)
    }

    // Evaluate Normal of a Cell FeaturePoint
    // via Approxiate Differental Gradient
    fn sn_grad(lp: IVec3, chunk: &Chunk) -> Vec3 {
        // let E = 1;  // Epsilon
        let val = chunk.get_voxel_rel(lp).isovalue();
        vec3(
            chunk.get_voxel_rel(lp + IVec3::X).isovalue() - val,
            chunk.get_voxel_rel(lp + IVec3::Y).isovalue() - val,
            chunk.get_voxel_rel(lp + IVec3::Z).isovalue() - val,
            // chunk.get_cell_rel(lp + IVec3::X).value - chunk.get_cell_rel(lp - IVec3::X).value,
            // chunk.get_cell_rel(lp + IVec3::Y).value - chunk.get_cell_rel(lp - IVec3::Y).value,
            // chunk.get_cell_rel(lp + IVec3::Z).value - chunk.get_cell_rel(lp - IVec3::Z).value,
        )
        // Normalize may fail since Isovalue may non-differenced e.g. all water isoval == 0.1
        .try_normalize()
        .unwrap_or(Vec3::NEG_Y) // NEG_Y will be Y after grad-to-normal flip.
    }

    fn sn_contouring(vbuf: &mut VertexBuffer, chunk: &Chunk) {
        for ly in 0..Chunk::LEN {
            for lz in 0..Chunk::LEN {
                for lx in 0..Chunk::LEN {
                    let lp = IVec3::new(lx, ly, lz);
                    let c0 = chunk.at_voxel(lp);

                    // for 3 axes edges, if sign-changed, connect adjacent 4 cells' vertices
                    for axis_i in 0..3 {
                        let c1 = match chunk.get_voxel_neib(lp + Self::AXES[axis_i]) {
                            None => continue, // do not generate face if it's a Nil Cell (non-loaded)
                            Some(c1) => c1,
                        };
                        if !Self::sn_signchanged(c0, &c1) {
                            continue;
                        }

                        let winding_flip = c0.is_isoval_empty();

                        for quadvert_i in 0..6 {
                            let winded_vi = if winding_flip { 5 - quadvert_i } else { quadvert_i };

                            let p = lp + Self::ADJACENT[axis_i][winded_vi];
                            let c = chunk.get_voxel_rel(p);

                            let fp = Self::sn_featurepoint(p, chunk);
                            let norm = -Self::sn_grad(p, chunk);

                            let mut nearest_val = f32::INFINITY;
                            let mut nearest_tex = c.tex_id;
                            for vert in Self::VERT {
                                let c = chunk.get_voxel_rel(p + vert);
                                if !c.is_isoval_empty() && c.isovalue() < nearest_val {
                                    nearest_val = c.isovalue();
                                    nearest_tex = c.tex_id;
                                    // assert(!c.is_tex_empty());  the nearest_tex shouldn't be Nil
                                }
                            }

                            vbuf.push_vertex(p.as_vec3() + fp + 0.5, vec2(nearest_tex as f32, -1.), norm);
                        }
                    }
                }
            }
        }
    }
}

#[rustfmt::skip]
static CUBE_POS: [f32; 6 * 6 * 3] = [
    0., 0., 1., 0., 1., 1., 0., 1., 0.,   0., 0., 1., 0., 1., 0., 0., 0., 0., // Left -X
    1., 0., 0., 1., 1., 0., 1., 1., 1.,   1., 0., 0., 1., 1., 1., 1., 0., 1., // Right +X
    0., 0., 1., 0., 0., 0., 1., 0., 0.,   0., 0., 1., 1., 0., 0., 1., 0., 1., // Bottom -Y
    0., 1., 1., 1., 1., 1., 1., 1., 0.,   0., 1., 1., 1., 1., 0., 0., 1., 0., // Bottom +Y
    0., 0., 0., 0., 1., 0., 1., 1., 0.,   0., 0., 0., 1., 1., 0., 1., 0., 0., // Front -Z
    1., 0., 1., 1., 1., 1., 0., 1., 1.,   1., 0., 1., 0., 1., 1., 0., 0., 1., // Back +Z
];

#[rustfmt::skip]
static CUBE_UV: [f32; 6 * 6 * 2] = [
    1., 1., 1., 0., 0., 0., 1., 1., 0., 0., 0., 1., // One Face.
    1., 1., 1., 0., 0., 0., 1., 1., 0., 0., 0., 1.,
    1., 1., 1., 0., 0., 0., 1., 1., 0., 0., 0., 1.,
    1., 1., 1., 0., 0., 0., 1., 1., 0., 0., 0., 1., 
    1., 1., 1., 0., 0., 0., 1., 1., 0., 0., 0., 1.,
    1., 1., 1., 0., 0., 0., 1., 1., 0., 0., 0., 1.,
];

#[rustfmt::skip]
static CUBE_NORM: [f32; 6 * 6 * 3] = [
    -1., 0., 0.,-1., 0., 0.,-1., 0., 0.,-1., 0., 0.,-1., 0., 0.,-1., 0., 0.,
     1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0.,
     0.,-1., 0., 0.,-1., 0., 0.,-1., 0., 0.,-1., 0., 0.,-1., 0., 0.,-1., 0.,
     0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0.,
     0., 0.,-1., 0., 0.,-1., 0., 0.,-1., 0., 0.,-1., 0., 0.,-1., 0., 0.,-1.,
     0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1.,
];

// static CUBE_IDX: [u32;6*6] = [
// ];

fn put_cube(vbuf: &mut VertexBuffer, lp: IVec3, chunk: &Chunk, tex_id: u16) {
    for face_i in 0..6 {
        let face_dir = Vec3::from_slice(&CUBE_NORM[face_i * 18..]); // 18: 3 scalar * 3 vertex * 2 triangle

        // skip the face if there's Obaque
        if chunk.get_voxel_rel(lp + face_dir.as_ivec3()).is_obaque_cube() {
            continue;
        }

        for vert_i in 0..6 {
            // let uv = Vec2::from_slice(&CUBE_UV[face_i * 12 + vert_i * 2..]);

            vbuf.push_vertex(
                Vec3::from_slice(&CUBE_POS[face_i * 18 + vert_i * 3..]) + lp.as_vec3(),
                Vec2::new(tex_id as f32, -1.),
                Vec3::from_slice(&CUBE_NORM[face_i * 18 + vert_i * 3..]),
            );
        }
    }
}

// put a -X face in middle of pos. for foliages.
fn put_face(vbuf: &mut VertexBuffer, tex_id: u16, pos: Vec3, rot: Quat, scale: Vec2) {
    // -X Face
    for i in 0..6 {
        // 6 verts
        let p = Vec3::from_slice(&CUBE_POS[i * 3..]) - vec3(0.0, 0.5, 0.5); // -0.5: centerized for proper rotation
        let p = (rot * (p * vec3(1.0, scale.y, scale.x))) + pos;

        let n = Vec3::from_slice(&CUBE_NORM[i * 3..]);
        let n = rot * n;

        let uv = Vec2::from_slice(&CUBE_UV[i * 2..]);
        let uv = VoxTex::map_uv(uv, tex_id);

        vbuf.push_vertex(p, uv, n);
    }
}

fn put_leaves(vbuf: &mut VertexBuffer, pos: Vec3, tex_id: u16) {
    let deg45 = PI / 4.;
    let siz = 1.4;

    put_face(vbuf, tex_id, pos + 0.5, Quat::from_axis_angle(Vec3::Y, deg45), vec2(1.4, 1.0) * siz);
    put_face(vbuf, tex_id, pos + 0.5, Quat::from_axis_angle(Vec3::Y, -deg45), vec2(1.4, 1.0) * siz);
    put_face(vbuf, tex_id, pos + 0.5, Quat::from_axis_angle(Vec3::Z, deg45), vec2(1.0, 1.4) * siz);
    put_face(vbuf, tex_id, pos + 0.5, Quat::from_axis_angle(Vec3::Z, -deg45), vec2(1.0, 1.4) * siz);
}

fn put_grass(vbuf: &mut VertexBuffer, pos: Vec3, tex_id: u16) {
    let ang = PI / 3.0;
    let siz = 1.4;

    put_face(vbuf, tex_id, pos + 0.5, Quat::from_axis_angle(Vec3::Y, ang), Vec2::ONE * siz);
    put_face(vbuf, tex_id, pos + 0.5, Quat::from_axis_angle(Vec3::Y, ang * 2.), Vec2::ONE * siz);
    put_face(vbuf, tex_id, pos + 0.5, Quat::from_axis_angle(Vec3::Y, ang * 3.), Vec2::ONE * siz);
}

// fn mat_model(pos: Vec3, rot: Mat3, scale: Vec3) {

// }
