use std::{hash::Hash};

use bevy::{
    math::{ivec3, vec2, vec3},
    prelude::*,
    render::{
        mesh::{Indices, Mesh},
        render_resource::PrimitiveTopology,
    }, utils::{HashMap, hashbrown::hash_map::{OccupiedEntry, VacantEntry}, Entry},
};
use bevy_egui::egui::emath::inverse_lerp;

use super::{chunk::*, chunk_system::ChunkPtr};


#[derive(PartialEq)]
struct HashVec3(Vec3);
impl Hash for HashVec3 {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.x.to_bits().hash(state);
        self.0.y.to_bits().hash(state);
        self.0.z.to_bits().hash(state);
    }
}
impl Eq for HashVec3 {
}


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
        vtx.pos.reserve(num_vert);
        vtx.uv.reserve(num_vert);
        vtx.norm.reserve(num_vert);
        vtx
    }

    pub fn push_vertex(&mut self, pos: Vec3, uv: Vec2, norm: Vec3) {
        self.pos.push(pos);
        self.uv.push(uv);
        self.norm.push(norm);
    }

    fn is_indexed(&self) -> bool {
        !self.indices.is_empty()
    }

    fn vertex_count(&self) -> usize {
        if self.is_indexed() {
            self.indices.len()
        } else {
            self.pos.len()
        }
    }

    fn triangles_count(&self) -> usize {
        self.vertex_count() / 3
    }

    pub fn clear(&mut self) {
        self.pos.clear();
        self.norm.clear();
        self.uv.clear();
        self.indices.clear();
    }

    pub fn compute_flat_normals(&mut self) {
        assert!(!self.is_indexed());
        self.norm.clear();
        self.norm.reserve(self.pos.len());

        let pos = &self.pos;
        for tri_i in 0..self.triangles_count() {
            let p0 = pos[tri_i*3];
            let p1 = pos[tri_i*3+1];
            let p2 = pos[tri_i*3+2];

            let n = (p1 - p0).cross(p2 - p0).normalize();

            self.norm.push(n);
            self.norm.push(n);
            self.norm.push(n);
        }
    }

    pub fn compute_smooth_normals(&mut self) {
        assert!(!self.is_indexed());
        
        let mut pos2norm = HashMap::<HashVec3, Vec3>::new();

        let pos = &self.pos;
        for tri_i in 0..self.triangles_count() {
            let p0 = pos[tri_i*3];
            let p1 = pos[tri_i*3+1];
            let p2 = pos[tri_i*3+2];

            let n = (p1 - p0).cross(p2 - p0);

            let a0 = (p1 - p0).angle_between(p2 - p0);
            let a1 = (p2 - p1).angle_between(p0 - p1);
            let a2 = (p0 - p2).angle_between(p1 - p2);

            *pos2norm.entry(HashVec3(p0)).or_default() += n * a0;
            *pos2norm.entry(HashVec3(p1)).or_default() += n * a1;
            *pos2norm.entry(HashVec3(p2)).or_default() += n * a2;
        }

        for norm in pos2norm.values_mut() {
            *norm = norm.normalize();
        }

        self.norm.clear();
        self.norm.reserve(self.pos.len());
        for pos in self.pos.iter() {
            self.norm.push(*pos2norm.get(&HashVec3(*pos)).unwrap());
        }

    }

    pub fn compute_indexed(&mut self) {
        assert!(!self.is_indexed());
        self.indices.clear();
        self.indices.reserve(self.vertex_count());
        
        let mut vert2idx = HashMap::<HashVec3, u32>::new();
        let mut verts = Vec::new();

        for vert in self.pos.iter() {
            
            match vert2idx.entry(HashVec3(*vert)) {
                Entry::Occupied(e) => {
                    let idx = *e.get();
                    self.indices.push(idx);
                },
                Entry::Vacant(e) => {
                    let idx = verts.len() as u32;
                    e.insert(idx);
                    verts.push(vert);
                    self.indices.push(idx);
                }
            }
            todo!("Sth");

        }
    }

    pub fn into_mesh(self) -> Mesh {
        let has_idx = self.is_indexed();

        Mesh::new(PrimitiveTopology::TriangleList)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.pos)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.norm)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uv)
            .with_indices(if has_idx {
                Some(Indices::U32(self.indices))
            } else {
                None
            })
    }

    // pub fn to_mesh(&self, mesh: &mut Mesh) {
    //     mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.pos.clone());
    //     mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.norm.clone());
    //     mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uv.clone());
    //     mesh.set_indices(if self.is_indexed() {
    //         Some(Indices::U32(self.indices.clone()))
    //     } else {
    //         None
    //     })
    // }
}

impl Clone for VertexBuffer {
    fn clone(&self) -> Self {
        Self {
            pos: self.pos.clone(),
            norm: self.norm.clone(),
            uv: self.uv.clone(),
            indices: self.indices.clone(),
        }        
    }
}


pub struct MeshGen {}

impl MeshGen {
    pub fn generate_chunk_mesh(vbuf: &mut VertexBuffer, chunk: &Chunk) {
        Self::sn_contouring(vbuf, chunk);
        return;

        // for ly in 0..Chunk::SIZE {
        //     for lz in 0..Chunk::SIZE {
        //         for lx in 0..Chunk::SIZE {
        //             let lp = IVec3::new(lx, ly, lz);

        //             let cell = chunk.get_cell(lp);

        //             if !cell.is_empty() {

        //                 put_cube(vbuf, lp, chunk);

        //             }
        //         }
        //     }
        // }

        // vbuf.make_indexed();
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

    fn sn_signchanged(c0: &Cell, c1: &Cell) -> bool {
        (c0.value > 0.) != (c1.value > 0.) // use .is_empty() ?
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
            let c0 = chunk.get_cell_rel(lp + v0);
            let c1 = chunk.get_cell_rel(lp + v1);

            if Self::sn_signchanged(&c0, &c1) {
                // t maybe -INF if accessing a Nil Cell.
                if let Some(t) = inverse_lerp(c0.value..=c1.value, 0.0) {
                    if !t.is_finite() {
                        continue;
                    }
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
    // DEL: WARN: may produce NaN Normal Value if the Cell's value is NaN (Nil Cell in the Context)
    fn sn_grad(lp: IVec3, chunk: &Chunk) -> Vec3 {
        // let E = 1;  // Epsilon
        let val = chunk.get_cell_rel(lp).value;
        vec3(
            chunk.get_cell_rel(lp + IVec3::X).value - val,
            chunk.get_cell_rel(lp + IVec3::Y).value - val,
            chunk.get_cell_rel(lp + IVec3::Z).value - val,
            // chunk.get_cell_rel(lp + IVec3::X).value - chunk.get_cell_rel(lp - IVec3::X).value,
            // chunk.get_cell_rel(lp + IVec3::Y).value - chunk.get_cell_rel(lp - IVec3::Y).value,
            // chunk.get_cell_rel(lp + IVec3::Z).value - chunk.get_cell_rel(lp - IVec3::Z).value,
        )
        .normalize()
    }

    fn sn_contouring(vbuf: &mut VertexBuffer, chunk: &Chunk) {
        for ly in 0..Chunk::SIZE {
            for lz in 0..Chunk::SIZE {
                for lx in 0..Chunk::SIZE {
                    let lp = IVec3::new(lx, ly, lz);
                    let c0 = chunk.get_cell(lp);

                    // for 3 axes edges, if sign-changed, connect adjacent 4 cells' vertices
                    for axis_i in 0..3 {
                        // !OutBound
                        let c1 = chunk.get_cell_rel(lp + Self::AXES[axis_i]);

                        if !c1.value.is_finite() {
                            continue;
                        }
                        if !Self::sn_signchanged(&c0, &c1) {
                            continue;
                        }

                        let winding_flip = c0.is_empty();

                        for quadvert_i in 0..6 {
                            let winded_vi = if winding_flip {
                                5 - quadvert_i
                            } else {
                                quadvert_i
                            };

                            let p = lp + Self::ADJACENT[axis_i][winded_vi];
                            let c = chunk.get_cell_rel(p);

                            let fp = Self::sn_featurepoint(p, chunk);
                            let norm = -Self::sn_grad(p, chunk);

                            let mut nearest_val = f32::INFINITY;
                            let mut nearest_mtl = c.mtl;
                            for vert in Self::VERT {
                                let c = chunk.get_cell_rel(p + vert);
                                if !c.is_empty() && c.value < nearest_val {
                                    nearest_val = c.value;
                                    nearest_mtl = c.mtl;
                                }
                            }

                            vbuf.push_vertex(p.as_vec3() + fp, vec2(nearest_mtl as f32, 0.), norm);
                        }
                    }
                }
            }
        }
    }
}

static CUBE_POS: [f32; 6 * 6 * 3] = [
    0., 0., 1., 0., 1., 1., 0., 1., 0., // Left -X
    0., 0., 1., 0., 1., 0., 0., 0., 0., 1., 0., 0., 1., 1., 0., 1., 1., 1., // Right +X
    1., 0., 0., 1., 1., 1., 1., 0., 1., 0., 0., 1., 0., 0., 0., 1., 0., 0., // Bottom -Y
    0., 0., 1., 1., 0., 0., 1., 0., 1., 0., 1., 1., 1., 1., 1., 1., 1., 0., // Bottom +Y
    0., 1., 1., 1., 1., 0., 0., 1., 0., 0., 0., 0., 0., 1., 0., 1., 1., 0., // Front -Z
    0., 0., 0., 1., 1., 0., 1., 0., 0., 1., 0., 1., 1., 1., 1., 0., 1., 1., // Back +Z
    1., 0., 1., 0., 1., 1., 0., 0., 1.,
];

static CUBE_UV: [f32; 6 * 6 * 2] = [
    1., 0., 1., 1., 0., 1., 1., 0., 0., 1., 0., 0., // One Face.
    1., 0., 1., 1., 0., 1., 1., 0., 0., 1., 0., 0., 1., 0., 1., 1., 0., 1., 1., 0., 0., 1., 0., 0.,
    1., 0., 1., 1., 0., 1., 1., 0., 0., 1., 0., 0., 1., 0., 1., 1., 0., 1., 1., 0., 0., 1., 0., 0.,
    1., 0., 1., 1., 0., 1., 1., 0., 0., 1., 0., 0.,
];

static CUBE_NORM: [f32; 6 * 6 * 3] = [
    -1., 0., 0., -1., 0., 0., -1., 0., 0., -1., 0., 0., -1., 0., 0., -1., 0., 0., 1., 0., 0., 1.,
    0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 0., -1., 0., 0., -1., 0., 0., -1., 0.,
    0., -1., 0., 0., -1., 0., 0., -1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1.,
    0., 0., 1., 0., 0., 0., -1., 0., 0., -1., 0., 0., -1., 0., 0., -1., 0., 0., -1., 0., 0., -1.,
    0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1.,
];

// static CUBE_IDX: [u32;6*6] = [
// ];

fn put_cube(vbuf: &mut VertexBuffer, lp: IVec3, chunk: &Chunk) {
    for face_i in 0..6 {
        let face_dir = Vec3::from_slice(&CUBE_NORM[face_i * 18..]); // 18: 3 scalar * 3 vertex * 2 triangle

        if let Some(neib) = chunk.get_cell_neighbor(lp + face_dir.as_ivec3()) {
            if !neib.is_empty() {
                continue;
            }
        }

        for vert_i in 0..6 {
            vbuf.push_vertex(
                Vec3::from_slice(&CUBE_POS[face_i * 18 + vert_i * 3..]) + lp.as_vec3(),
                Vec2::from_slice(&CUBE_UV[face_i * 12 + vert_i * 2..]),
                Vec3::from_slice(&CUBE_NORM[face_i * 18 + vert_i * 3..]),
            );
        }
    }
}
