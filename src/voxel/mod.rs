
mod chunk;
pub use chunk::{Chunk, Cell};

// mod chunk_system;
// pub use chunk_system::{ChunkPtr};

mod voxel_server;
mod voxel_client;
pub use voxel_server::{ServerVoxelPlugin, ServerChunkSystem};
pub use voxel_client::{ClientVoxelPlugin, ClientChunkSystem, HitResult};

mod material;
mod meshgen;
mod worldgen;
pub use worldgen::WorldGen;

use std::sync::{Arc, RwLock};
pub type ChunkPtr = Arc<RwLock<Chunk>>;  // Box<Chunk>;         not supported for SharedPtr


use bevy::{prelude::*, utils::HashMap};

#[derive(Resource, Deref, Clone)]
struct MpscTx<T>(crate::channel_impl::Sender<T>);

#[derive(Resource, Deref, Clone)]
struct MpscRx<T>(crate::channel_impl::Receiver<T>);




#[derive(Component)]
pub struct ChunkComponent {
    pub chunkpos: IVec3,
}

impl ChunkComponent {
    pub fn new(chunkpos: IVec3) -> Self {
        Self { chunkpos }
    }
}



pub trait ChunkSystem {
    
    fn get_chunks(&self) -> &HashMap<IVec3, ChunkPtr>;

    fn get_chunk(&self, chunkpos: IVec3) -> Option<&ChunkPtr> {
        assert!(Chunk::is_chunkpos(chunkpos));
        self.get_chunks().get(&chunkpos)
    }

    fn has_chunk(&self, chunkpos: IVec3) -> bool {
        assert!(Chunk::is_chunkpos(chunkpos));
        self.get_chunks().contains_key(&chunkpos)
    }

    fn num_chunks(&self) -> usize {
        self.get_chunks().len() 
    }

}



// #[derive(Component)]
// struct ChunkMeshingTask;//(Task<Mesh>);

// fn chunks_detect_load_and_unload(
//     query_cam: Query<&Transform, With<CharacterControllerCamera>>,
//     query_chunks: Query<(Entity, &ChunkComponent)>,

//     mut chunk_sys: ResMut<ChunkSystem>,

//     mut commands: Commands,
//     mut meshes: ResMut<Assets<Mesh>>,

//     chunk_data_tx: Res<ChunkDataTx<ChunkLoadingData>>,
//     chunk_data_rx: Res<ChunkDataRx<ChunkLoadingData>>,
// ) {
//     // let chunk_sys_entity = commands.entity(chunk_sys.entity);

//     let vp = Chunk::as_chunkpos(query_cam.single().translation.as_ivec3()); // viewer pos
//     let vd = chunk_sys.view_distance;

//     // Chunks Detect Load/Gen

//     let max_n = vd.max_element();
//     'outer: for n in 0..max_n {
//         for y in -n..=n {
//             for z in -n..=n {
//                 for x in -n..=n {
//                     if chunk_sys.chunks_loading.len() > chunk_sys.max_concurrent_loading {
//                         break 'outer;
//                     }
//                     if x.abs() < n && y.abs() < n && z.abs() < n {
//                         continue;
//                     }
//                     if x.abs() > vd.x || y.abs() > vd.y || z.abs() > vd.x {
//                         continue;
//                     }

//                     let chunkpos = IVec3::new(x, y, z) * Chunk::SIZE + vp;

//                     // the chunk already exists, skip.
//                     if chunk_sys.has_chunk(chunkpos) || chunk_sys.chunks_loading.contains_key(&chunkpos) {
//                         continue;
//                     }

//                     #[cfg(feature = "experimental_channel")]
//                     let tx = chunk_data_tx.clone();

//                     let task = AsyncComputeTaskPool::get().spawn(async move {
//                         // info!("Load Chunk: {:?}", chunkpos);
//                         let mut chunk = Chunk::new(chunkpos);
//                         WorldGen::generate_chunk(&mut chunk);
//                         let chunkptr = Arc::new(RwLock::new(chunk));

//                         #[cfg(feature = "target_native_os")]
//                         {
//                             chunkptr
//                         }

//                         #[cfg(feature = "experimental_channel")]
//                         tx.send((chunkpos, chunkptr)).unwrap();
//                     });

//                     #[cfg(feature = "target_native_os")]
//                     chunk_sys.chunks_loading.insert(chunkpos, task);

//                     #[cfg(feature = "experimental_channel")]
//                     {
//                         task.detach();
//                         chunk_sys.chunks_loading.insert(chunkpos, ());
//                     }

//                     // gizmo.cuboid(Transform::from_translation(p.as_vec3()).with_scale(Vec3::splat(16.)), Color::RED);
//                 }
//             }
//         }
//     }

//     // Apply Loaded Chunks. todo Refactor.
//     let chunksys_entity = chunk_sys.entity;
//     let material_handle = chunk_sys.vox_mtl.clone();
//     let mut goingtospawn = Vec::new();

//     #[cfg(feature = "target_native_os")]
//     chunk_sys.chunks_loading.retain(|chunkpos, task| {
//         if task.is_finished() {
//             let chunkptr = future::block_on(future::poll_once(task)).unwrap();
//             {
//                 let mut chunk = chunkptr.write().unwrap();

//                 chunk.mesh_handle = meshes.add(Mesh::new(PrimitiveTopology::TriangleList));

//                 chunk.entity = commands
//                     .spawn((
//                         ChunkComponent::new(*chunkpos),
//                         MaterialMeshBundle {
//                             mesh: chunk.mesh_handle.clone(),
//                             material: material_handle.clone(),
//                             transform: Transform::from_translation(chunkpos.as_vec3()),
//                             visibility: Visibility::Hidden, // Hidden is required since Mesh is empty.
//                             ..default()
//                         },
//                         Aabb::from_min_max(Vec3::ZERO, Vec3::ONE * (Chunk::SIZE as f32)),
//                         RigidBody::Static,
//                     ))
//                     .set_parent(chunksys_entity)
//                     .id();
//             }

//             goingtospawn.push(chunkptr);

//             return false; // remove task.
//         }
//         true
//     });

//     #[cfg(feature = "experimental_channel")]
//     while let Ok((chunkpos, chunkptr)) = chunk_data_rx.try_recv() {
//         chunk_sys.chunks_loading.remove(&chunkpos);

//         {
//             let mut chunk = chunkptr.write().unwrap();

//             chunk.mesh_handle = meshes.add(Mesh::new(PrimitiveTopology::TriangleList));

//             chunk.entity = commands
//                 .spawn((
//                     ChunkComponent::new(chunkpos),
//                     MaterialMeshBundle {
//                         mesh: chunk.mesh_handle.clone(),
//                         material: material_handle.clone(),
//                         transform: Transform::from_translation(chunkpos.as_vec3()),
//                         visibility: Visibility::Hidden, // Hidden is required since Mesh is empty.
//                         ..default()
//                     },
//                     Aabb::from_min_max(Vec3::ZERO, Vec3::ONE * (Chunk::SIZE as f32)),
//                     RigidBody::Static,
//                 ))
//                 .set_parent(chunksys_entity)
//                 .id();
//         }

//         goingtospawn.push(chunkptr);
//     }

//     for chunkptr in goingtospawn {
//         chunk_sys.spawn_chunk(chunkptr);
//     }
