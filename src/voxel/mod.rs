

mod chunk;
mod chunk_system;

use chunk::*;
use chunk_system::*;



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




