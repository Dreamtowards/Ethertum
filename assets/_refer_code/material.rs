// enum VoxelShapeId {
//     Isosurface = 0,
//     Silhouette = 1,
//     Mesh,
// }
// struct Material {
//     hardness: f32,

//     /// Foliage/Vegetable Materials will generate to another mesh., with Double-Sided (NoCulling), NoCollision, WavingVertex Rendering
//     is_foliage: bool,
//     // custom_mesh
//     shape_id: VoxelShapeId,
//     tex_id: u32,
//     // item: Rc<Item>
// }

// impl Default for Material {
//     fn default() -> Self {
//         Self {
//             hardness: 1.,
//             is_foliage: false,
//             shape_id: VoxelShapeId::Isosurface,
//             tex_id: 0,
//         }
//     }
// }


pub mod mtl {
}

// use crate::util::registry::*;
// static REGISTRY: Registry<Material> = Registry::default();

pub mod mtl_tex {
}
