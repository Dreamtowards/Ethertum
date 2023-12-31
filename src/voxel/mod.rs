

mod chunk;
mod chunk_system;
mod material;

mod meshgen;
mod worldgen;

use chunk::*;
use chunk_system::*;
use meshgen::*;
use worldgen::*;
use crate::character_controller::CharacterControllerCamera;

use bevy_xpbd_3d::components::{AsyncCollider, Collider, ComputedCollider, RigidBody};

use bevy::{
    prelude::*, 
    render::{render_resource::{PrimitiveTopology, AsBindGroup}, primitives::Aabb}, 
    utils::HashMap, 
    tasks::{AsyncComputeTaskPool, Task}, reflect::TypeUuid
};

use std::cell::RefCell;
use once_cell::sync::Lazy;
use thread_local::ThreadLocal;



pub struct VoxelPlugin;

impl Plugin for VoxelPlugin {
    fn build(&self, app: &mut App) {
        
        app.insert_resource(ChunkSystem::new(2));
        app.register_type::<ChunkSystem>();

        app.add_plugins(MaterialPlugin::<TerrainMaterial>::default());

        app.add_systems(Startup, startup);

        app.add_systems(Update, 
            (
                chunks_detect_load_and_unload, 
                // chunks_apply_loaded
                chunks_detect_remesh_dispatch 
                // chunks_apply_remeshed
            )
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
        val: 10.,
        texture_diffuse: Some(asset_server.load("cache/atlas_diff.png")), 
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

#[derive(Component)]
struct ChunkMeshingTask;//(Task<Mesh>);



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
                if chunk_sys.has_chunk(chunkpos) {
                    continue;
                }

                let mut chunk = Box::new(Chunk::new(chunkpos));
                
                WorldGen::generate_chunk(&mut chunk);

                chunk.entity = commands.spawn((
                    ChunkComponent::new(chunkpos),
                    MaterialMeshBundle {
                        mesh: meshes.add(Mesh::new(PrimitiveTopology::TriangleList)),
                        material: chunk_sys.vox_mtl.clone(),
                        transform: Transform::from_translation(chunkpos.as_vec3()),
                        visibility: Visibility::Hidden,  // Hidden is required since Mesh is empty.
                        ..default()
                    },
                    Aabb::from_min_max(Vec3::ZERO, Vec3::ONE * (Chunk::SIZE as f32)),
                    
                    ChunkMeshingTask,
                    RigidBody::Static,
                )).set_parent(chunk_sys.entity).id();

                // NotShadowCaster


    
                chunk_sys.spawn_chunk(chunk);

                // chunk_sys.chunks_meshing.insert(chunkpos, ChunkMeshingState::Pending);

                info!("Load Chunk: {:?}", chunkpos);
            }
        }
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

    }
}





static SHARED_POOL_MESH_BUFFERS: Lazy<ThreadLocal<RefCell<VertexBuffer>>> =
    Lazy::new(ThreadLocal::default);


fn chunks_detect_remesh_dispatch(
    chunk_sys: ResMut<ChunkSystem>,
    mut commands: Commands,

    mut meshes: ResMut<Assets<Mesh>>,

    mut query: Query<(Entity, &Handle<Mesh>, &mut ChunkMeshingTask, &ChunkComponent, &mut Visibility)>,
) {

    for (entity, mesh_handle, mut meshing_task, chunk_info, mut visibility) in query.iter_mut() {
        
        let chunkpos = chunk_info.chunkpos;

        // !!Problematic Unwarp
        let chunk = chunk_sys.get_chunk(chunkpos).unwrap();

        let mut vbuf = VertexBuffer::default();

        MeshGen::generate_chunk_mesh(&mut vbuf, chunk);

        *meshes.get_mut(mesh_handle).unwrap() = vbuf.into_mesh();


        if let Some(collider) = Collider::trimesh_from_mesh(meshes.get(mesh_handle).unwrap()) {

            commands.entity(entity).remove::<Collider>().insert(collider);
            
            info!("TriMesh {:?}", chunkpos);
        }

        *visibility = Visibility::Visible;

        commands.entity(entity).remove::<ChunkMeshingTask>();

        info!("ReMesh {:?}", chunkpos);
    }
    
    // let task_pool = AsyncComputeTaskPool::get();

    // task_pool.spawn(async move {
    //     let mut mesh_buffer = SHARED_POOL_MESH_BUFFERS
    //         .get_or(|| {
    //             RefCell::new(VertexBuffer::with_capacity(1024))
    //         })
    //         .borrow_mut();

    //     MeshGen::generate_chunk_mesh(&mut mesh_buffer, chunk);
    // });

    // chunk_sys.chunks_meshing.retain(|chunkpos, stat| {
    //     if ChunkMeshingState::Pending = stat {
    //         let chunk = chunk_sys.get_chunk(chunkpos);

    //     }
    //     true
    // });


}









#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, TypeUuid)]
#[uuid = "8014bf20-d959-11ed-afa1-0242ac120001"]
pub struct TerrainMaterial {
    
	#[uniform(0)]
    val: f32,

    #[texture(1)]
    #[sampler(2)]
    pub texture_diffuse: Option<Handle<Image>>,
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