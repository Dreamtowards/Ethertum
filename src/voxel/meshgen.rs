use bevy::{
    prelude::*,
    render::{mesh::{Mesh, Indices}, render_resource::PrimitiveTopology}, math::{vec3, ivec3, vec2},
};
use bevy_egui::egui::emath::inverse_lerp;

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
        if self.is_indexed() {self.indices.len()} else {self.pos.len()}
    }

    pub fn make_indexed(&mut self) {

        self.indices.clear();

        for i in 0..self.pos.len() {
            self.indices.push(i as u32);
        }
    }

    pub fn into_mesh(self) -> Mesh {
        let has_idx = self.is_indexed();

        let mut tmp = Vec::new();
        tmp.resize(self.vertex_count(), Vec4::default());
        
        Mesh::new(PrimitiveTopology::TriangleList)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, tmp)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.pos)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, self.uv)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.norm)
            .with_indices(if has_idx {Some(Indices::U32(self.indices))} else {None})
    }

}


pub struct MeshGen {

}

impl MeshGen {

    pub fn generate_chunk_mesh(vbuf: &mut VertexBuffer, chunk: &Chunk) {

        Self::sn_contouring(vbuf, chunk);
        vbuf.make_indexed();
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





    
    const AXES: [IVec3; 3] = [
        ivec3(1, 0, 0),
        ivec3(0, 1, 0),
        ivec3(0, 0, 1),
    ];
    const ADJACENT: [[IVec3; 6]; 3] = [
        [ivec3(0,0,0), ivec3(0,-1,0), ivec3(0,-1,-1), ivec3(0,-1,-1), ivec3(0,0,-1), ivec3(0,0,0)],
        [ivec3(0,0,0), ivec3(0,0,-1), ivec3(-1,0,-1), ivec3(-1,0,-1), ivec3(-1,0,0), ivec3(0,0,0)],
        [ivec3(0,0,0), ivec3(-1,0,0), ivec3(-1,-1,0), ivec3(-1,-1,0), ivec3(0,-1,0), ivec3(0,0,0)]
    ];

    const VERT: [IVec3; 8] = [
        ivec3(0, 0, 0),  // 0
        ivec3(0, 0, 1),
        ivec3(0, 1, 0),  // 2
        ivec3(0, 1, 1),
        ivec3(1, 0, 0),  // 4
        ivec3(1, 0, 1),
        ivec3(1, 1, 0),  // 6
        ivec3(1, 1, 1)
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
        [0,4], [1,5], [2,6], [3,7],  // X
        [5,7], [1,3], [4,6], [0,2],  // Y
        [4,5], [0,1], [6,7], [2,3]   // Z
    ];

    fn sn_signchanged(c0: &Cell, c1: &Cell) -> bool {
        (c0.value > 0.) != (c1.value > 0.)  // use .is_empty() ?
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
            let c0 = chunk.get_cell(lp + v0);
            let c1 = chunk.get_cell(lp + v1);

            if Self::sn_signchanged(c0, c1) {
                let t = inverse_lerp(c0.value..=c1.value, 0.0).unwrap();

                // t maybe -INF if accessing a Nil Cell.
                if t.is_finite() {

                    let p = t * (v1 - v0).as_vec3() + v0.as_vec3();  // (v1-v0) must > 0. since every edge vert are min-to-max

                    fp_sum += p;
                    sign_changes += 1;
                }
            }
        }

        assert_ne!(sign_changes, 0);
        assert!(fp_sum.is_finite());

        fp_sum / (sign_changes as f32)
    }

    // Evaluate Normal of a Cell FeaturePoint
    // via Approxiate Differental Gradient  
    // DEL: WARN: may produce NaN Normal Value if the Cell's value is NaN (Nil Cell in the Context)
    fn sn_grad(lp: IVec3, chunk: &Chunk) -> Vec3 {
        // let E = 1;  // Epsilon
        let val = chunk.get_cell(lp).value;
        vec3(
            chunk.get_cell(lp + IVec3::X).value - val,//chunk.get_cell(lp - IVec3::X).value,
            chunk.get_cell(lp + IVec3::Y).value - val,//chunk.get_cell(lp - IVec3::Y).value,
            chunk.get_cell(lp + IVec3::Z).value - val,//chunk.get_cell(lp - IVec3::Z).value
        ).normalize()
    }

    fn sn_contouring(vbuf: &mut VertexBuffer, chunk: &Chunk) {

        for ly in 1..Chunk::SIZE-1 {
            for lz in 1..Chunk::SIZE-1 {
                for lx in 1..Chunk::SIZE-1 {
                    let lp = IVec3::new(lx, ly, lz);
                    let c0 = chunk.get_cell(lp);

                    // for 3 axes edges, if sign-changed, connect adjacent 4 cells' vertices
                    for axis_i in 0..3 {
                        // !OutBound
                        let c1 = chunk.get_cell(lp + Self::AXES[axis_i]);

                        if !Self::sn_signchanged(c0, c1) {
                            continue;
                        }

                        let winding_flip = c0.is_empty();

                        for quadvert_i in 0..6 {
                            let winded_vi = if winding_flip {5 - quadvert_i} else {quadvert_i};

                            let p = lp + Self::ADJACENT[axis_i][winded_vi];
                            //let c = chunk.get_cell(p);

                            let fp = Self::sn_featurepoint(p, chunk);//vec3(0.5, 0.5, 0.5);
                            let norm = -Self::sn_grad(p, chunk);

                            vbuf.push_vertex(
                                p.as_vec3() + fp, 
                                vec2(0., 0.), 
                                norm
                            );
                        }
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


fn put_cube(vbuf: &mut VertexBuffer, lp: IVec3, chunk: &Chunk) {
    
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
