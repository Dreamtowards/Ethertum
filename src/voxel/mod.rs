

mod chunk;
mod chunk_system;
mod material;

mod meshgen;
mod worldgen;

use bevy::transform::commands;
use bevy_xpbd_3d::components::AsyncCollider;
use bevy_xpbd_3d::components::Collider;
use bevy_xpbd_3d::components::ComputedCollider;
use chunk::*;
use chunk_system::*;
use meshgen::MeshGen;
use meshgen::VertexBuffer;

use crate::{voxel::worldgen::WorldGen, character_controller::CharacterControllerCamera};

use bevy::{
    prelude::*, 
    render::{render_resource::PrimitiveTopology, primitives::Aabb}, 
    utils::HashMap, tasks::AsyncComputeTaskPool
};

use std::cell::RefCell;
use once_cell::sync::Lazy;
use thread_local::ThreadLocal;

pub struct VoxelPlugin;

impl Plugin for VoxelPlugin {
    fn build(&self, app: &mut App) {
        

        app.insert_resource(ChunkSystem::new(0));
        app.register_type::<ChunkSystem>();


        app.add_systems(Startup, startup);

        app.add_systems(Update, 
            (
                chunks_detect_load_dispatch, 
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
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mtl = materials.add(Color::rgb(0.8, 0.7, 0.6).into());
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
enum ChunkRemeshState {
    Pending,
    Meshing,
    Completed,

}



fn chunks_detect_load_dispatch(
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

                if chunk_sys.has_chunk(chunkpos) {
                    continue;
                }

                let mut chunk = Box::new(Chunk::new(chunkpos));
                
                let mesh = meshes.add(Mesh::new(PrimitiveTopology::TriangleList));

                WorldGen::generate_chunk(&mut chunk);


                chunk.entity = commands.spawn((
                    ChunkComponent::new(chunkpos),
                    PbrBundle {
                        mesh: mesh,
                        material: chunk_sys.vox_mtl.clone(),
                        transform: Transform::from_translation(chunkpos.as_vec3()),
                        visibility: Visibility::Hidden,  // Hidden is required since Mesh is empty.
                        ..default()
                    },
                    Aabb::from_min_max(Vec3::ZERO, Vec3::ONE * (Chunk::SIZE as f32)),

                    ChunkRemeshState::Pending,
                    
                    // AsyncCollider(ComputedCollider::TriMesh),
                    // RigidBody::Static,
                )).set_parent(chunk_sys.entity).id();



    
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
    mut chunk_sys: ResMut<ChunkSystem>,
    mut commands: Commands,

    mut meshes: ResMut<Assets<Mesh>>,

    mut query: Query<(Entity, &Handle<Mesh>, &mut ChunkRemeshState, &ChunkComponent, &mut Visibility)>,
) {

    for (entity, mesh_handle, mut stat, chunkinfo, mut vis) in query.iter_mut() {
        if let ChunkRemeshState::Pending = *stat {
            *vis = Visibility::Visible;

            // !!Problematic Unwarp
            let chunk = chunk_sys.get_chunk(chunkinfo.chunkpos).unwrap();

            let mut vbuf = VertexBuffer::default();

            MeshGen::generate_chunk_mesh(&mut vbuf, chunk);

            *meshes.get_mut(mesh_handle).unwrap() = vbuf.into_mesh();

            *stat = ChunkRemeshState::Completed;


            if let Some(collider) = Collider::trimesh_from_mesh(meshes.get(mesh_handle).unwrap()) {

                //commands.entity(entity).remove::<Collider>().insert(collider);
                
                info!("TriMesh {:?}", chunkinfo.chunkpos);
            }



            info!("ReMesh {:?}", chunkinfo.chunkpos);
        }
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

