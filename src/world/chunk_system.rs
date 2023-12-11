

// ChunkSystem ECS

use bevy::{
    prelude::*, 
    render::{render_resource::PrimitiveTopology, primitives::Aabb}, 
    utils::HashMap
};

pub struct VoxelPlugin;

impl Plugin for VoxelPlugin {
    fn build(&self, app: &mut App) {
        

        app.insert_resource(ChunkSystem::new());


        app.add_systems(Startup, startup);

        app.add_systems(Update, 
            (
                chunks_detect_load, 
                chunks_detect_remesh
            )
        );

    }
}

use crate::world::chunk::*;

fn startup(
    mut chunk_sys: ResMut<ChunkSystem>,
    
    mut commands: Commands,
) {

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



fn chunks_detect_load(
    mut chunk_sys: ResMut<ChunkSystem>,
    
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // let chunk_sys_entity = commands.entity(chunk_sys.entity);

    let vp = IVec3::ZERO;  //Chunk::as_chunkpos(query_cam.single().translation.as_ivec3());  // viewer pos
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

                ChunkGenerator::generate_chunk(&mut chunk);


                chunk.entity = commands.spawn((
                    ChunkComponent::new(chunkpos),
                    PbrBundle {
                        mesh: mesh,
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

                info!("Chunk: {:?}", chunkpos);
            }
        }
    }
}

fn chunks_detect_remesh(
    mut chunk_sys: ResMut<ChunkSystem>,

    mut meshes: ResMut<Assets<Mesh>>,

    mut query: Query<(&Handle<Mesh>, &mut ChunkRemeshState, &ChunkComponent, &mut Visibility)>,
) {

    for (mesh_id, mut stat, chunkinfo, mut vis) in query.iter_mut() {
        if let ChunkRemeshState::Pending = *stat {
            *vis = Visibility::Visible;


            let mesh = generate_chunk_mesh();
            *meshes.get_mut(mesh_id).unwrap() = mesh;

            *stat = ChunkRemeshState::Completed;

            info!("ReMesh {:?}", chunkinfo.chunkpos);
        }
    }

    // chunk_sys.chunks_meshing.retain(|chunkpos, stat| {
    //     if ChunkMeshingState::Pending = stat {
    //         let chunk = chunk_sys.get_chunk(chunkpos);

    //     }
    //     true
    // });


}


fn generate_chunk_mesh() -> Mesh {
    Mesh::new(PrimitiveTopology::TriangleList)
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION, 
        vec![
            [-0.5, 0.5, -0.5], // vertex with index 0
            [0.5, 0.5, -0.5], // vertex with index 1
            [0.5, 0.5, 0.5], // etc. until 23
            [-0.5, 0.5, 0.5],
        ]
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0, 
        vec![
            // Assigning the UV coords for the top side.
            [0.0, 0.2], [0.0, 0.0], [1.0, 0.0], [1.0, 0.25],
        ]
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vec![
            // Normals for the top side (towards +y)
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ]
    )
    .with_indices(Some(bevy::render::mesh::Indices::U32(vec![
        0,3,1 , 1,3,2,
    ])))
}







// ChunkSystem




// pub enum ChunkMeshingState {
//     Pending,
//     Meshing,//(Task<Mesh>),
//     Completed,
// }

pub type ChunkPtr = Box<Chunk>; //Arc<RwLock<Chunk>>;

#[derive(Resource)]
pub struct ChunkSystem {

    /// all loaded chunks.
    /// ChunkList can be read (by multiple threads) at the same time, but only one can be writing at the same time and no other can be reading at this time.
    // 设计一个高性能区块系统，这两个区块列表 及每个区块 都有RwLock特性，即 可同时可被多处读，但只能被互斥写
    // linear-list of loaded chunks.
    // chunks: Arc<RwLock<HashMap<IVec3, Arc<RwLock<Chunk>>>>>, 
    chunks: HashMap<IVec3, ChunkPtr>,

    // Spare Voxel Octree for Spatial lookup acceleration.
    // chunks_svo: SVO<Arc<RwLock<Chunk>>>,

    // pub chunks_loading: HashSet<IVec3>,
    // pub chunks_meshing: HashMap<IVec3, ChunkMeshingState>,

    pub view_distance: IVec2,

    pub entity: Entity,

}

impl ChunkSystem {

    pub fn new() -> Self {
        Self { 
            chunks: HashMap::new(), //Arc::new(RwLock::new(HashMap::new())), 
            view_distance: IVec2::new(2, 2),
            // chunks_loading: HashSet::new(),
            // chunks_meshing: HashMap::new(),
            entity: Entity::PLACEHOLDER,
        }
    }

    // pub fn get_chunk(&self, chunkpos: IVec3) -> Option<ChunkPtr> {
    //     assert!(Chunk::is_chunkpos(chunkpos));

    //     if let Some(chunk) = self.chunks.get(&chunkpos) {  //.read().unwrap().get(&chunkpos) {
    //         Some(chunk.clone())
    //     } else {
    //         None
    //     }
    // }

    pub fn has_chunk(&self, chunkpos: IVec3) -> bool {
        assert!(Chunk::is_chunkpos(chunkpos));

         self.chunks.contains_key(&chunkpos)  //.read().unwrap().contains_key(&chunkpos)
    }

    pub fn num_chunks(&self) -> usize {

        self.chunks.len() //.read().unwrap().len()
    }

    // pub fn provide_chunk(&self, chunkpos: IVec3) -> ChunkPtr {
    //     assert!(!self.has_chunk(chunkpos));

    //     let mut chunk = Arc::new(RwLock::new(Chunk::new(chunkpos)));

    //     let load = false;  // chunk_loader.load_chunk(chunk);

    //     if !load {

    //         ChunkGenerator::generate_chunk(chunk.write().unwrap().borrow_mut());
    //     }

    //     chunk
    // }


    pub fn spawn_chunk(&mut self, chunk: ChunkPtr) {
        let chunkpos = chunk.chunkpos;


        self.chunks.insert(chunkpos, chunk);  //.write().unwrap()
        // // There is no need to cast shadows for chunks below the surface.
        // if chunkpos.y <= 64 {
        //     entity_commands.insert(NotShadowCaster);
        // }

        // self.set_chunk_meshing(chunkpos, ChunkMeshingState::Pending);

    }

    pub fn despawn_chunk(&mut self, chunkpos: IVec3, cmds: &mut Commands) -> Option<ChunkPtr> {

        if let Some(chunk) = self.chunks.remove(&chunkpos) {  //.write().unwrap()

            cmds.entity(chunk.entity).despawn_recursive();

            Some(chunk)
        } else {
            None
        }
    }

    // pub fn set_chunk_meshing(&mut self, chunkpos: IVec3, stat: ChunkMeshingState) {
    //     self.chunks_meshing.insert(chunkpos, stat);
    // }

}








pub struct ChunkGenerator {

}

impl ChunkGenerator {

    pub fn generate_chunk(chunk: &mut Chunk) {

        // for y in 0..Chunk::SIZE {
        //     for z in 0..Chunk::SIZE {
        //         for x in 0..Chunk::SIZE {
        //             let lp = IVec3::new(x, y, z);

        //         }
        //     }
        // }

        chunk.set_cell(IVec3::new(0,0,0), Cell::new(1., 1))

    }

    fn populate_chunk(chunk: &mut Chunk) {

    }

}


