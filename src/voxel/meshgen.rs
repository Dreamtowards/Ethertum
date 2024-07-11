use std::f32::consts::PI;

use bevy::{
    math::{vec2, vec3},
    prelude::*,
};

use super::*;
use crate::util::{iter, vtx::VertexBuffer};

pub static mut DBG_FORCE_BLOCKY: bool = false;

pub fn generate_chunk_mesh(vbuf: &mut VertexBuffer, chunk: &Chunk) {
    if unsafe{!DBG_FORCE_BLOCKY} {

        sn::sn_contouring(vbuf, chunk);
    }

    for ly in 0..Chunk::LEN {
        for lz in 0..Chunk::LEN {
            for lx in 0..Chunk::LEN {
                let lp = IVec3::new(lx, ly, lz);

                let vox = chunk.at_voxel(lp);

                if !vox.is_nil() && vox.is_cube() {
                    put_cube(vbuf, lp, chunk, vox.tex_id);
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

mod sn {
    use bevy::math::{ivec3, vec2, vec3, IVec3, Vec3};
    use bevy_egui::egui::emath::inverse_lerp;

    use crate::util::vtx::VertexBuffer;

    use super::{Chunk, Vox};


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
            let edge = EDGE[edge_i];
            let v0 = VERT[edge[0]];
            let v1 = VERT[edge[1]];
            let c0 = chunk.get_voxel_rel_or_default(lp + v0);
            let c1 = chunk.get_voxel_rel_or_default(lp + v1);

            if sn_signchanged(&c0, &c1) {
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
        let val = chunk.get_voxel_rel_or_default(lp).isovalue();
        vec3(
            chunk.get_voxel_rel_or_default(lp + IVec3::X).isovalue() - val,
            chunk.get_voxel_rel_or_default(lp + IVec3::Y).isovalue() - val,
            chunk.get_voxel_rel_or_default(lp + IVec3::Z).isovalue() - val,
            // chunk.get_cell_rel(lp + IVec3::X).value - chunk.get_cell_rel(lp - IVec3::X).value,
            // chunk.get_cell_rel(lp + IVec3::Y).value - chunk.get_cell_rel(lp - IVec3::Y).value,
            // chunk.get_cell_rel(lp + IVec3::Z).value - chunk.get_cell_rel(lp - IVec3::Z).value,
        )
        // Normalize may fail since Isovalue may non-differenced e.g. all water isoval == 0.1
        .try_normalize()
        .unwrap_or(Vec3::NEG_Y) // NEG_Y will be Y after grad-to-normal flip.
    }

    pub fn sn_contouring(vbuf: &mut VertexBuffer, chunk: &Chunk) {
        for ly in 0..Chunk::LEN {
            for lz in 0..Chunk::LEN {
                for lx in 0..Chunk::LEN {
                    let lp = IVec3::new(lx, ly, lz);
                    let c0 = chunk.at_voxel(lp);

                    // for 3 axes edges, if sign-changed, connect adjacent 4 cells' vertices
                    for axis_i in 0..3 {
                        let c1 = match chunk.get_voxel_rel(lp + AXES[axis_i]) {
                            None => continue, // do not generate face if it's a Nil Cell (non-loaded)
                            Some(c1) => c1,
                        };
                        if !sn_signchanged(c0, &c1) {
                            continue;
                        }

                        let winding_flip = c0.is_isoval_empty();

                        for quadvert_i in 0..6 {
                            let winded_vi = if winding_flip { 5 - quadvert_i } else { quadvert_i };

                            let p = lp + ADJACENT[axis_i][winded_vi];
                            let c = chunk.get_voxel_rel_or_default(p);

                            let fp = sn_featurepoint(p, chunk);
                            let norm = -sn_grad(p, chunk);

                            let mut nearest_val = f32::INFINITY;
                            let mut nearest_tex = c.tex_id;
                            let mut nearest_lit = 0;
                            for vert in VERT {
                                let c = chunk.get_voxel_rel_or_default(p + vert);
                                if !c.is_isoval_empty() && c.isovalue() < nearest_val {
                                    nearest_val = c.isovalue();
                                    nearest_tex = c.tex_id;
                                    nearest_lit = c.light.red();
                                    // assert(!c.is_tex_empty());  the nearest_tex shouldn't be Nil
                                }
                            }

                            vbuf.push_vertex(p.as_vec3() + fp + 0.5, vec2(nearest_tex as f32, nearest_lit as f32), norm);
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
        let face_dir = Vec3::from_slice(&CUBE_NORM[face_i * 18..]).as_ivec3(); // 18: 3 scalar * 3 vertex * 2 triangle

        // skip the face if there's Obaque Cube
        let neib = chunk.get_voxel_rel_or_default(lp + face_dir);
        if  neib.is_obaque_cube() {
            continue;
        }

        for vert_i in 0..6 {
            // let uv = Vec2::from_slice(&CUBE_UV[face_i * 12 + vert_i * 2..]);

            vbuf.push_vertex(
                Vec3::from_slice(&CUBE_POS[face_i * 18 + vert_i * 3..]) + lp.as_vec3(),
                Vec2::new(tex_id as f32, neib.light.red() as f32),
                Vec3::from_slice(&CUBE_NORM[face_i * 18 + vert_i * 3..]),
            );
        }
    }
}

// put a -X face in middle of pos. for foliages.
pub fn put_face(vbuf: &mut VertexBuffer, tex_id: u16, pos: Vec3, rot: Quat, scale: Vec2) {
    // -X Face
    for i in 0..6 {
        // 6 verts
        let p = Vec3::from_slice(&CUBE_POS[i * 3..]) - vec3(0.0, 0.5, 0.5); // -0.5: centerized for proper rotation
        let p = (rot * (p * vec3(1.0, scale.y, scale.x))) + pos;

        let n = Vec3::from_slice(&CUBE_NORM[i * 3..]);
        let n = rot * n;

        let uv = Vec2::from_slice(&CUBE_UV[i * 2..]);
        let uv = VoxTex::map_uv(uv, tex_id);
        // uv.x += tex_id;
        // uv.y += light;

        vbuf.push_vertex(p, uv, n);
    }
}

pub fn put_leaves(vbuf: &mut VertexBuffer, pos: Vec3, tex_id: u16) {
    let deg45 = PI / 4.0;
    let siz = 1.4;

    put_face(vbuf, tex_id, pos + 0.5, Quat::from_axis_angle(Vec3::Y, deg45), vec2(1.4, 1.0) * siz);
    put_face(vbuf, tex_id, pos + 0.5, Quat::from_axis_angle(Vec3::Y, deg45*3.0), vec2(1.4, 1.0) * siz);
    put_face(vbuf, tex_id, pos + 0.5, Quat::from_axis_angle(Vec3::Z, -deg45), vec2(1.0, 1.4) * siz);
    put_face(vbuf, tex_id, pos + 0.5, Quat::from_axis_angle(Vec3::Z, -deg45*3.0), vec2(1.0, 1.4) * siz);
}

pub fn put_grass(vbuf: &mut VertexBuffer, pos: Vec3, tex_id: u16) {
    let ang = PI / 3.0;
    let siz = 1.4;

    put_face(vbuf, tex_id, pos + 0.5, Quat::from_axis_angle(Vec3::Y, ang), Vec2::ONE * siz);
    put_face(vbuf, tex_id, pos + 0.5, Quat::from_axis_angle(Vec3::Y, ang * 2.), Vec2::ONE * siz);
    put_face(vbuf, tex_id, pos + 0.5, Quat::from_axis_angle(Vec3::Y, ang * 3.), Vec2::ONE * siz);
}

// fn mat_model(pos: Vec3, rot: Mat3, scale: Vec3) {

// }
