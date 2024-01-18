




struct Material {

    hardness: f32,

    /// Foliage/Vegetable Materials will generate to another mesh., with Double-Sided (NoCulling), NoCollision, WavingVertex Rendering
    is_foliage: bool,

    // custom_mesh
    // tex_id

    // item: Rc<Item>

}

impl Default for Material {
    
    fn default() -> Self {
        Self {
            hardness: 1.,
            is_foliage: false,
        }
    }

}

pub mod mtl {
    pub const STONE: u16 = 2;
    pub const DIRT: u16 = 0;
    pub const GRASS: u16 = 7;
    pub const WATER: u16 = 23;
    pub const SAND: u16 = 18;


}


use crate::util::registry::*;


// static REGISTRY: Registry<Material> = Registry::default();



