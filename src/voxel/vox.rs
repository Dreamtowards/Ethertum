

use crate::prelude::*;

// naming VoxelUnit?: BlockState, Cell, Voxel or Vox?
#[derive(Clone, Copy)]
pub struct Vox {
    pub tex_id: u16,

    pub shape_id: VoxShape,

    pub light: VoxLight,

    /// SDF value, used for Isosurface Extraction.
    /// 0 -> surface, +0 positive -> void, -0 negative -> solid.
    pub isoval: u8,
    // Cached FeaturePoint
    // pub cached_fp: Vec3,
    // pub cached_norm: Vec3,
}

impl Default for Vox {
    fn default() -> Self {
        Vox {
            shape_id: VoxShape::Isosurface,
            tex_id: VoxTex::Nil,
            light: VoxLight::default(),
            isoval: isoval_u8(0.0),
            // cached_fp: Vec3::INFINITY,
            // cached_norm: Vec3::INFINITY,
        }
    }
}

// todo: bugfix 自从压缩成u8后，地下有一些小碎片三角形 可能是由于精度误差 -0.01 变成 0.01 之类的了。要
// 修复这种bug 应该是把接近0的数值扩大 再转成u8.
fn isoval_u8(f: f32) -> u8 {
    ((f.clamp(-1.0, 1.0) + 1.0) / 2.0 * 255.0) as u8
}

fn isoval_ndc(u: u8) -> f32 {
    u as f32 / 255.0 * 2.0 - 1.0
}

impl Vox {
    pub fn new(tex_id: u16, shape_id: VoxShape, isovalue: f32) -> Self {
        Self {
            tex_id,
            shape_id,
            isoval: isoval_u8(isovalue),
            ..default()
        }
    }

    pub fn is_nil(&self) -> bool {
        self.tex_id == 0
    }

    pub fn is_cube(&self) -> bool {
        self.shape_id == VoxShape::Cube || unsafe{super::meshgen::DBG_FORCE_BLOCKY}
    }

    pub fn isovalue(&self) -> f32 {
        if self.shape_id != VoxShape::Isosurface {
            return 0.0;
        }
        isoval_ndc(self.isoval)
    }
    pub fn set_isovalue(&mut self, val: f32) {
        self.isoval = isoval_u8(val);
    }
    pub fn is_isoval_empty(&self) -> bool {
        self.isovalue() <= 0.0
    }

    pub fn is_obaque_cube(&self) -> bool {
        self.is_cube() && !self.is_nil()
    }
}


// VoxShape

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize, Default, Reflect)]
pub enum VoxShape {
    #[default]
    Isosurface,

    Cube,
    Leaves,
    Grass,

    SlabYMin,
    SlabYMax,
    SlabXMin,
    SlabXMax,
    SlabZMin,
    SlabZMax,

    Fence,
    // CustomMesh {
    //     mesh_id: u16,
    // }
}


pub struct VoxTex {
    id: u16,
    light_emission: u8,
}

#[allow(non_upper_case_globals)]
impl VoxTex {

    pub const Nil: u16 = 0;
    pub const Stone: u16 = 22;
    pub const Dirt: u16 = 1;
    pub const Grass: u16 = 12; // 7 11
    pub const Water: u16 = 24;
    pub const Sand: u16 = 19;
    pub const Log: u16 = 13;

    pub const ShortGrass: u16 = 13;
    pub const Bush: u16 = 14;
    pub const Rose: u16 = 15;
    pub const Fern: u16 = 16;
    pub const Leaves: u16 = 23;

    // [0,1] -> [0,1]
    pub fn map_uv(uv: Vec2, tex_id: u16) -> Vec2 {
        const TEX_CAP: f32 = 24.;
        let tex = tex_id - 1; // -1: offset the 0 Nil
        Vec2::new(uv.x / TEX_CAP + tex as f32 / TEX_CAP, uv.y)
    }
}

#[derive(Default, Clone, Copy)]
pub struct VoxLight {
    // 4*u4 channel: Sky, R, G, B
    light: u16,
}

impl VoxLight {
    pub fn new(sky: u16, r: u16, g: u16, b: u16) -> Self {
        let mut l = VoxLight::default();
        l.set_sky(sky);
        l.set_red(r);
        l.set_green(g);
        l.set_blue(b);
        l
    }

    pub fn sky(&self) -> u16 {
        self.light >> 12
    }
    pub fn red(&self) -> u16 {
        (self.light >> 8) & 0xF
    }
    pub fn green(&self) -> u16 {
        (self.light >> 4) & 0xF
    }
    pub fn blue(&self) -> u16 {
        self.light & 0xF
    }
    pub fn get(&self, chan: u8) -> u16 {
        if chan == Self::SKY { return self.sky(); }
        if chan == Self::RED { return self.red(); }
        if chan == Self::GREEN { return self.green(); }
        if chan == Self::BLUE { return self.blue(); }
        panic!("illegal channel {chan}");
    }

    pub fn set_sky(&mut self, v: u16) {
        self.light = (self.light & !(0xF << 12)) | (v << 12);
    }
    pub fn set_red(&mut self, v: u16) {
        self.light = (self.light & !(0xF << 8)) | ((v & 0xF) << 8);
    }
    pub fn set_green(&mut self, v: u16) {
        self.light = (self.light & !(0xF << 4)) | ((v & 0xF) << 4);
    }
    pub fn set_blue(&mut self, v: u16) {
        self.light = (self.light & !0xF) | (v & 0xF);
    }
    pub fn set(&mut self, chan: u8, val: u16) {
        if chan == Self::SKY { self.set_sky(val); }
        if chan == Self::RED { self.set_red(val); }
        if chan == Self::GREEN { self.set_green(val); }
        if chan == Self::BLUE { self.set_blue(val); }
        panic!("illegal channel {chan}");
    }

    // Channels
    pub const SKY: u8 = 0;
    pub const RED: u8 = 1;
    pub const GREEN: u8 = 2;
    pub const BLUE: u8 = 3;
}

impl std::fmt::Display for VoxLight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Sky {}, R {}, G {}, B {}", self.sky(), self.red(), self.green(), self.blue()))
    }
}
