

mod chunk;
mod chunk_system;
mod material;

mod meshgen;
mod worldgen;

use bevy_inspector_egui::quick::AssetInspectorPlugin;
use chunk::*;
use futures_lite::future;
use meshgen::*;
use worldgen::*;
use crate::character_controller::CharacterControllerCamera;

pub use chunk_system::{ChunkSystem, ChunkPtr};

use bevy_xpbd_3d::components::{AsyncCollider, Collider, ComputedCollider, RigidBody};

use bevy::{
    prelude::*, 
    render::{render_resource::{PrimitiveTopology, AsBindGroup}, primitives::Aabb}, 
    utils::{HashMap, FloatOrd}, 
    tasks::{AsyncComputeTaskPool, Task}, 
    reflect::TypeUuid, asset::ReflectAsset, asset::ReflectHandle
};

use std::sync::{Arc, RwLock};
use std::cell::RefCell;
use once_cell::sync::Lazy;
use thread_local::ThreadLocal;



pub struct VoxelPlugin;

impl Plugin for VoxelPlugin {
    fn build(&self, app: &mut App) {
        
        app.insert_resource(ChunkSystem::new(2));
        app.register_type::<ChunkSystem>();

        app.add_plugins(MaterialPlugin::<TerrainMaterial>::default());
        app.register_asset_reflect::<TerrainMaterial>();

        app.add_systems(Startup, startup);

        app.add_systems(Update, 
            (
                
                chunks_remesh,
                chunks_detect_load_and_unload, 
                // chunks_apply_loaded
                // chunks_apply_remeshed
            ).chain()
        );
    }
}


fn startup(
    mut chunk_sys: ResMut<ChunkSystem>,
    
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
) {
    let mtl = terrain_materials.add(TerrainMaterial {
        texture_diffuse: Some(asset_server.load("cache/atlas_diff.png")), 
        texture_normal: Some(asset_server.load("cache/atlas_norm.png")), 
        texture_dram: Some(asset_server.load("cache/atlas_dram.png")), 
        ..default()
    });
    chunk_sys.vox_mtl = mtl;

    // ChunkSystem entity. all chunk entities will be spawn as children.
    chunk_sys.entity = commands.spawn((
        Name::new("ChunkSystem"),
        InheritedVisibility::VISIBLE,
        GlobalTransform::IDENTITY,
        Transform::IDENTITY,
    )).id();

}


#[derive(Component)]
pub struct ChunkComponent {
    pub chunkpos: IVec3,
}

impl ChunkComponent {
    fn new(chunkpos: IVec3) -> Self {
        Self {
            chunkpos,
        }
    }
}

// #[derive(Component)]
// struct ChunkMeshingTask;//(Task<Mesh>);



fn chunks_detect_load_and_unload(
    query_cam: Query<&Transform, With<CharacterControllerCamera>>,
    query_chunks: Query<(Entity, &ChunkComponent)>,

    mut chunk_sys: ResMut<ChunkSystem>,
    
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // let chunk_sys_entity = commands.entity(chunk_sys.entity);

    let vp = Chunk::as_chunkpos(query_cam.single().translation.as_ivec3());  // viewer pos
    let vd = chunk_sys.view_distance;

    // Chunks Detect Load/Gen
    for y in -vd.y..=vd.y {
        for z in -vd.x..=vd.x {
            for x in -vd.x..=vd.x {
                let chunkpos = IVec3::new(x, y, z) * Chunk::SIZE + vp;

                // the chunk already exists, skip.
                if chunk_sys.has_chunk(chunkpos) || chunk_sys.chunks_loading.contains_key(&chunkpos) {
                    continue;
                }


                let task = AsyncComputeTaskPool::get().spawn(async move {

                    let chunkptr = Arc::new(RwLock::new(Chunk::new(chunkpos)));

                    {
                        let mut chunk = chunkptr.write().unwrap();
                        
                        WorldGen::generate_chunk(&mut chunk);
                    }

                    info!("Load Chunk: {:?}", chunkpos);

                    chunkptr
                });

                chunk_sys.chunks_loading.insert(chunkpos, task);
            }
        }
    }

    // Apply Loaded Chunks. todo Refactor.
    let chunksys_entity = chunk_sys.entity;
    let material_handle = chunk_sys.vox_mtl.clone();
    let mut goingtospawn = Vec::new();
    chunk_sys.chunks_loading.retain(|chunkpos, task| {
        if let Some(chunkptr) = future::block_on(future::poll_once(task)) {

            {
                let mut chunk = chunkptr.write().unwrap();

                chunk.mesh_handle = meshes.add(Mesh::new(PrimitiveTopology::TriangleList));
        
                chunk.entity = commands.spawn((
                    ChunkComponent::new(*chunkpos),
                    MaterialMeshBundle {
                        mesh: chunk.mesh_handle.clone(),
                        material: material_handle.clone(),
                        transform: Transform::from_translation(chunkpos.as_vec3()),
                        visibility: Visibility::Hidden,  // Hidden is required since Mesh is empty.
                        ..default()
                    },
                    Aabb::from_min_max(Vec3::ZERO, Vec3::ONE * (Chunk::SIZE as f32)),
                    
                    RigidBody::Static,
                )).set_parent(chunksys_entity).id();
            }

            goingtospawn.push(chunkptr);

            return false;
        }
        true
    });
    for chunkptr in goingtospawn {
        chunk_sys.spawn_chunk(chunkptr);
    }


    // Chunks Detect Unload
    for (entity, chunk_comp) in query_chunks.iter() {
        let chunkpos = chunk_comp.chunkpos;
        
        if (vp.x - chunkpos.x).abs() > vd.x * Chunk::SIZE ||
           (vp.z - chunkpos.z).abs() > vd.x * Chunk::SIZE ||
           (vp.y - chunkpos.y).abs() > vd.y * Chunk::SIZE {

            info!("Unload Chunk: {:?}", chunkpos);
            commands.entity(entity).despawn_recursive();
            chunk_sys.despawn_chunk(chunkpos);
        }
        else if chunk_sys.dbg_remesh_all_chunks 
        {
            chunk_sys.mark_chunk_remesh(chunkpos);
        }

    }
    chunk_sys.dbg_remesh_all_chunks = false;

}





static THREAD_LOCAL_VERTEX_BUFFERS: Lazy<ThreadLocal<RefCell<VertexBuffer>>> =
    Lazy::new(ThreadLocal::default);


fn chunks_remesh(
    mut commands: Commands,

    mut chunk_sys: ResMut<ChunkSystem>,
    mut meshes: ResMut<Assets<Mesh>>,

    // mut query: Query<(Entity, &Handle<Mesh>, &mut ChunkMeshingTask, &ChunkComponent, &mut Visibility)>,
) {
    let mut chunks_remesh = Vec::from_iter(chunk_sys.chunks_remesh.iter().cloned());

    let p = IVec3::ZERO;
    chunks_remesh.sort_unstable_by_key(|v| {
        FloatOrd(v.distance_squared(p) as f32)
    });

    const MAX_CONCURRENT_MESHING: usize = 10;

    for chunkpos in chunks_remesh {
        
        if chunk_sys.chunks_meshing.len() > MAX_CONCURRENT_MESHING {
            break;
        }

        if let Some(chunkptr) = chunk_sys.get_chunk(chunkpos) {
            let chunkptr = chunkptr.clone();

            let task = AsyncComputeTaskPool::get().spawn(async move {
                let mut vbuf = THREAD_LOCAL_VERTEX_BUFFERS
                    .get_or(|| { RefCell::new(VertexBuffer::default()) })
                    .borrow_mut();
    
                let entity;
                let mesh_handle;
                {
                    let chunk = chunkptr.read().unwrap();
                    
                    // Generate Mesh
                    MeshGen::generate_chunk_mesh(&mut vbuf, &chunk); 

                    entity = chunk.entity;
                    mesh_handle = chunk.mesh_handle.clone();
                }

                let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
                vbuf.to_mesh(&mut mesh);  // exoprt mesh
                vbuf.clear();

                // Build Collider of TriMesh
                let collider = Collider::trimesh_from_mesh(&mesh);

                info!("Generated ReMesh");

                (mesh, collider, entity, mesh_handle)
            });
    
            info!("Queued ReMesh");
            chunk_sys.chunks_meshing.insert(chunkpos, task);
        }
        chunk_sys.chunks_remesh.remove(&chunkpos);
    }

    chunk_sys.chunks_meshing.retain(|chunkpos, task| {
        if let Some(r) = future::block_on(future::poll_once(task)) {

            // Update Mesh Asset
            *meshes.get_mut(r.3).unwrap() = r.0;

            // Update Phys Collider TriMesh
            if let Some(collider) = r.1 {
                if let Some(mut cmds) = commands.get_entity(r.2) {
                    cmds.remove::<Collider>()
                        .insert(collider)
                        .insert(Visibility::Visible);
                }
            }

            info!("Applied ReMesh");

            return false;
        }
        true
    });

}









#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[reflect(Asset)]
// #[uuid = "8014bf20-d959-11ed-afa1-0242ac120001"]
pub struct TerrainMaterial {

    #[sampler(0)]
    #[texture(1)]
    pub texture_diffuse: Option<Handle<Image>>,
    #[texture(2)]
    pub texture_normal: Option<Handle<Image>>,
    #[texture(3)]
    pub texture_dram: Option<Handle<Image>>,

    #[uniform(4)]
    pub triplanar_blend_sharpness: f32,
    #[uniform(5)]
    pub normal_intensity: f32,
    #[uniform(6)]
    pub triplanar_blend_pow: f32,
    #[uniform(7)]
    pub heightmap_blend_pow: f32,  // littler=mix, greater=distinct, opt 0.3 - 0.6, 0.48 = nature
}

impl Default for TerrainMaterial {
    fn default() -> Self {
        Self {
            texture_diffuse: None,
            texture_normal: None,
            texture_dram: None,
            triplanar_blend_sharpness: 0.35,
            normal_intensity: 1.0,
            triplanar_blend_pow: 4.5,
            heightmap_blend_pow: 0.48,
        }
    }
}

impl Material for TerrainMaterial {
    fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
        "shaders/terrain.wgsl".into()
    }
    fn vertex_shader() -> bevy::render::render_resource::ShaderRef {
        "shaders/terrain.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }
}