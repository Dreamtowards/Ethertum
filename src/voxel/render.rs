use bevy::{asset::ReflectAsset, pbr::{ExtendedMaterial, MaterialExtension}, render::render_resource::{AsBindGroup, ShaderRef}};

use crate::prelude::*;


pub fn init(app: &mut App)
{
    // Render Shader.
    app.add_plugins(MaterialPlugin::<ExtendedMaterial<StandardMaterial, TerrainMaterial>>::default());
    app.register_asset_reflect::<ExtendedMaterial<StandardMaterial, TerrainMaterial>>(); 
    app.add_plugins(MaterialPlugin::<ExtendedMaterial<StandardMaterial, FoliageMaterial>>::default());
    app.register_asset_reflect::<ExtendedMaterial<StandardMaterial, FoliageMaterial>>(); 
    app.add_plugins(MaterialPlugin::<ExtendedMaterial<StandardMaterial, LiquidMaterial>>::default());
    app.register_asset_reflect::<ExtendedMaterial<StandardMaterial, LiquidMaterial>>(); 

}


////////////////////////////////////////
//////////////// Render ////////////////
////////////////////////////////////////

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[reflect(Asset)]
// #[uuid = "8014bf20-d959-11ed-afa1-0242ac120001"]
pub struct TerrainMaterial {
    #[texture(100)]
    pub dram_texture: Option<Handle<Image>>,
    
    #[uniform(101)]
    pub sample_scale: f32,
    // #[uniform(2)]
    // pub triplanar_blend_pow: f32,
    // #[uniform(3)]
    // pub heightmap_blend_pow: f32, // littler=mix, greater=distinct, opt 0.3 - 0.6, 0.48 = nature

    // #[sampler(0)]
    // #[texture(1)]
    // pub texture_diffuse: Option<Handle<Image>>,
    // #[texture(2)]
    // pub texture_normal: Option<Handle<Image>>,
    // Web requires 16x bytes data. (As the device does not support `DownlevelFlags::BUFFER_BINDINGS_NOT_16_BYTE_ALIGNED`)
    // #[uniform(4)]
    // pub wasm0: Vec4,
    // pub sample_scale: f32,
    // #[uniform(5)]
    // pub normal_intensity: f32,
    // #[uniform(6)]
    // pub triplanar_blend_pow: f32,
    // #[uniform(7)]
    // pub heightmap_blend_pow: f32, // littler=mix, greater=distinct, opt 0.3 - 0.6, 0.48 = nature
}

impl Default for TerrainMaterial {
    fn default() -> Self {
        Self {
            dram_texture: None,

            sample_scale: 1.5,
            // triplanar_blend_pow: 4.5,
            // heightmap_blend_pow: 0.48,
            // texture_diffuse: None,
            // texture_normal: None,
            // wasm0: Vec4::new(1.5, 1.0, 4.5, 0.48),
            // normal_intensity: 1.0,
        }
    }
}

impl MaterialExtension for TerrainMaterial {
    fn deferred_vertex_shader() -> ShaderRef {
        "shaders/terrain.wgsl".into()
    }
    fn deferred_fragment_shader() -> ShaderRef {
        "shaders/terrain.wgsl".into()
    }
}

// Foliage

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[reflect(Asset)]
pub struct FoliageMaterial {
    
}

impl Default for FoliageMaterial {
    fn default() -> Self {
        Self { 
        }
    }
}

impl MaterialExtension for FoliageMaterial {
    // fn vertex_shader() -> ShaderRef {
    //     "shaders/foliage.wgsl".into()
    // }
    // fn fragment_shader() -> ShaderRef {
    //     "shaders/foliage.wgsl".into()
    // }
    fn deferred_vertex_shader() -> ShaderRef {
        "shaders/foliage.wgsl".into()
    }
    fn deferred_fragment_shader() -> ShaderRef {
        "shaders/foliage.wgsl".into()
    }

    fn specialize(
            _pipeline: &bevy::pbr::MaterialExtensionPipeline,
            descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
            _layout: &bevy::render::mesh::MeshVertexBufferLayoutRef,
            _key: bevy::pbr::MaterialExtensionKey<Self>,
        ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}


#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[reflect(Asset)]
pub struct LiquidMaterial {
    /// The normal map image.
    ///
    /// Note that, like all normal maps, this must not be loaded as sRGB.
    #[texture(100)]
    #[sampler(101)]
    pub normals: Handle<Image>,

    
}

impl MaterialExtension for LiquidMaterial {
    fn deferred_fragment_shader() -> ShaderRef {
        "shaders/liquid.wgsl".into()
    }
}