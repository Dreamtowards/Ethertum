use bevy::math::IVec2;


enum MtlShape {
    Isosurface,
    Silhouette,
    Mesh,
}
struct Material {

    hardness: f32,

    /// Foliage/Vegetable Materials will generate to another mesh., with Double-Sided (NoCulling), NoCollision, WavingVertex Rendering
    is_foliage: bool,
    // custom_mesh

    shape_id: MtlShape,
    tex_id: u32,

    // item: Rc<Item>
}

impl Default for Material {
    fn default() -> Self {
        Self {
            hardness: 1.,
            is_foliage: false,
            shape_id: MtlShape::Isosurface,
            tex_id: 0,
        }
    }
}

pub mod mtl {
    pub const NIL: u16 = 0;
    pub const STONE: u16 = 22;
    pub const DIRT: u16 = 1;
    pub const GRASS: u16 = 12; // 7 11
    pub const WATER: u16 = 24;
    pub const SAND: u16 = 19;
    pub const LOG: u16 = 13;

    pub const SHORTGRASS: u16 = 13;
    pub const BUSH: u16 = 14;
    pub const ROSE: u16 = 15;
    pub const FERN: u16 = 16;
    pub const LEAVES: u16 = 23;
}

// use crate::util::registry::*;
// static REGISTRY: Registry<Material> = Registry::default();

pub mod mtl_tex {
    use bevy::math::Vec2;


    pub fn map_uv(uv: Vec2, tex_id: u16) -> Vec2 {
        const TEX_CAP: f32 = 24.;
        let tex = tex_id - 1;  // -1: offset the 0 Nil
        Vec2::new(uv.x / TEX_CAP + tex as f32 / TEX_CAP, uv.y)
    }
}