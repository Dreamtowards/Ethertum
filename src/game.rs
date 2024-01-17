
use std::f32::consts::{PI, TAU};

use bevy::{
    prelude::*, 
    window::{CursorGrabMode, PrimaryWindow, WindowMode}, 
    pbr::{ScreenSpaceAmbientOcclusionBundle, DirectionalLightShadowMap}, 
    core_pipeline::experimental::taa::{TemporalAntiAliasBundle, TemporalAntiAliasPlugin}, 
};
use bevy_atmosphere::prelude::*;
use bevy_editor_pls::editor::EditorEvent;
use bevy_xpbd_3d::prelude::*;

use crate::character_controller::{CharacterControllerCamera, CharacterController, CharacterControllerBundle, CharacterControllerPlugin};

use crate::voxel::VoxelPlugin;


pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {

        // Atmosphere
        app.insert_resource(AtmosphereModel::default());
        app.add_plugins(AtmospherePlugin);

        // ShadowMap sizes
        app.insert_resource(DirectionalLightShadowMap {
            size: 512,
        });
        
        // Physics
        app.add_plugins(PhysicsPlugins::default());

        // SSAO
        // app.add_plugins(TemporalAntiAliasPlugin);
        // app.insert_resource(AmbientLight {
        //         brightness: 0.05,
        //         ..default()
        //     });

        // CharacterController
        app.add_plugins(CharacterControllerPlugin);


        // WorldInfo
        app.insert_resource(WorldInfo::new());
        app.register_type::<WorldInfo>();
        

        // ChunkSystem
        app.add_plugins(VoxelPlugin);
        
        

        app.add_systems(Startup, startup);
        app.add_systems(Update, tick_world);

        app.add_systems(Update, handle_inputs);
        
    }
}



// Simple environment
fn startup(
    assets: Res<AssetServer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut editor_events: EventWriter<bevy_editor_pls::editor::EditorEvent>,
) {
    // Grab mouse at startup.
    editor_events.send(bevy_editor_pls::editor::EditorEvent::Toggle { now_active: false });

    // Logical Player
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Capsule {
                radius: 0.4,
                depth: 1.0,
                ..default()
            })),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_xyz(0.0, 1.5, 0.0),
            ..default()
        },
        CharacterControllerBundle::new(
            Collider::capsule(1., 0.4), 
            CharacterController {
                is_flying: true,
                ..default()
            }),
        
        Name::new("Player"),
    ));

    // Camera
    commands.spawn((
        Camera3dBundle {
            projection: Projection::Perspective(PerspectiveProjection {
                fov: TAU / 4.6,
                ..default()
            }),
            camera: Camera {
                hdr: true,
                ..default()
            },
            ..default()
        },
        AtmosphereCamera::default(), // Marks camera as having a skybox, by default it doesn't specify the render layers the skybox can be seen on
        CharacterControllerCamera,

        Name::new("Camera"),
    ));
    // .insert(ScreenSpaceAmbientOcclusionBundle::default())
    // .insert(TemporalAntiAliasBundle::default());

    // Sun
    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                // shadows_enabled: true,
                ..default()
            },
            ..default()
        },
        Sun, // Marks the light as Sun
        
        Name::new("Sun"),
    ));


    // commands.spawn((
    //     SceneBundle {
    //         scene: assets.load("spaceship.glb#Scene0"),
    //         transform: Transform::from_xyz(0., 0., -10.),
    //         ..default()
    //     },
    //     AsyncSceneCollider::new(Some(ComputedCollider::TriMesh)),
    //     RigidBody::Static,
    // ));

    // // Floor
    // commands.spawn((
    //     SceneBundle {
    //         scene: assets.load("playground.glb#Scene0"),
    //         transform: Transform::from_xyz(0.5, -5.5, 0.5),
    //         ..default()
    //     },
    //     AsyncSceneCollider::new(Some(ComputedCollider::TriMesh)),
    //     RigidBody::Static,
    // ));
    
    // commands.spawn((
    //     PbrBundle {
    //         mesh: meshes.add(Mesh::from(shape::Box::new(5., 8., 5.))),
    //         transform: Transform::from_xyz(0.5, -5.5, 0.5),
    //         ..default()
    //     },
    //     AsyncCollider(ComputedCollider::TriMesh),
    //     RigidBody::Static,
    // ));

    // // Cube
    // commands.spawn((
    //     RigidBody::Dynamic,
    //     AngularVelocity(Vec3::new(2.5, 3.4, 1.6)),
    //     Collider::cuboid(1.0, 1.0, 1.0),
    //     PbrBundle {
    //         mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
    //         material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
    //         transform: Transform::from_xyz(0.0, 4.0, 0.0),
    //         ..default()
    //     },
    // ));
    
}



fn tick_world(
    mut atmosphere: AtmosphereMut<Nishita>,
    mut query: Query<(&mut Transform, &mut DirectionalLight), With<Sun>>,
    mut worldinfo: ResMut<WorldInfo>,
    time: Res<Time>,
) {
    // worldinfo.tick_timer.tick(time.delta());
    // if !worldinfo.tick_timer.just_finished() {
    //     return;
    // }
    // let dt_sec = worldinfo.tick_timer.duration().as_secs_f32();  // constant time step?

    // // Pause & Steps
    // if worldinfo.is_paused {
    //     if  worldinfo.paused_steps > 0 {
    //         worldinfo.paused_steps -= 1;
    //     } else {
    //         return;
    //     }
    // }
    let dt_sec = time.delta_seconds();
    

    worldinfo.time_inhabited += dt_sec;
    
    // DayTime
    if  worldinfo.daytime_length != 0. {
        worldinfo.daytime += dt_sec / worldinfo.daytime_length;
        worldinfo.daytime -= worldinfo.daytime.trunc();  // trunc to [0-1]
    }



    // Atmosphere SunPos
    let sun_ang = worldinfo.daytime * PI*2.;
    atmosphere.sun_position = Vec3::new(sun_ang.cos(), sun_ang.sin(), 0.);

    if let Some((mut light_trans, mut directional)) = query.single_mut().into() {
        directional.illuminance = sun_ang.sin().max(0.0).powf(2.0) * 100000.0;
        
        // or from000.looking_at()
        light_trans.rotation = Quat::from_rotation_z(sun_ang) * Quat::from_rotation_y(PI / 2.3);
    }
}



// fn update_chunks_loadance(
//     query_cam: Query<&Transform, With<CharacterControllerCamera>>,
//     mut chunk_sys: ResMut<ChunkSystem>,
//     mut chunks_loading: Local<HashMap<IVec3, Task<ChunkPtr>>>,
//     mut chunks_meshing: Local<HashMap<IVec3, ChunkMeshingState>>,
    
//     mut commands: Commands,
//     mut meshes: ResMut<Assets<Mesh>>,
//     mut materials: ResMut<Assets<StandardMaterial>>,
// ) {
//     // let thread_pool = AsyncComputeTaskPool::get();

//     let vp = Chunk::as_chunkpos(query_cam.single().translation.as_ivec3()) * 0;  // view pos
//     let vd = chunk_sys.view_distance;

//     // Chunks Detect Load/Gen
//     for y in -vd.y..=vd.y {
//         for z in -vd.x..=vd.x {
//             for x in -vd.x..=vd.x {
//                 let chunkpos = IVec3::new(x, y, z) * Chunk::SIZE + vp;

//                 if chunk_sys.has_chunk(chunkpos) || chunks_loading.contains_key(&chunkpos) {
//                     continue;
//                 }

//                 let mut chunk = Chunk::new(chunkpos);
                
//                 ChunkGenerator::generate_chunk(&mut chunk);

//                 let mesh = meshes.add(Mesh::new(PrimitiveTopology::TriangleList));

//                 chunk_sys.spawn_chunk(chunk, &mut commands, mesh);

//                 // let chunk_sys = chunk_sys.clone();
//                 // let task = thread_pool.spawn(async move {

//                 //     info!("Providing {:?}", chunkpos);

//                 //     // provide chunk (load or gen)
//                 //     chunk_sys.provide_chunk(chunkpos)

//                 // });
//                 // chunks_loading.insert(chunkpos, task);
//             }
//         }
//     }

//     // use float_ord::FloatOrd;
//     // use core::slice;
//     // chunks_loading.sort_unstable_by_key(|key| {
//         // FloatOrd(key.as_vec3().distance(player_pos.chunk_min.as_vec3()))
//     // });
//     // DoesNeeds? Chunks Loaded IntoWorld Batch (for reduce LockWrite)
    
//     // chunks_loading.retain(|chunkpos, task| {
//     //     if task.is_finished() {
//     //         if let Some(chunk) = future::block_on(future::poll_once(task)) {
                

//     //             chunk_sys.spawn_chunk(chunk, &mut commands);

//     //             chunks_meshing.insert(*chunkpos, ChunkMeshingState::Pending);

//     //             info!("spawn_chunk {:?}", chunkpos);
//     //             return false;
//     //         } else {
//     //             info!("NotAvailable {:?}", chunkpos);
//     //         }
//     //     }
//     //     true
//     // });

//     // Chunks Unload
//     // for chunkpos in chunk_sys.chunks.keys() {

//     //     // if should_unload()
//     //     if (*chunkpos - vp).abs().max_element() {

//     //     }
//     // }


//     // Chunks Detect Meshing
//     for (chunkpos, stat) in chunks_meshing.iter_mut() {
//         if let ChunkMeshingState::Pending = stat {


//             // let task = thread_pool.spawn(async move {

//             //     let mesh = generate_chunk_mesh();
//             //     // MeshGen::generate_chunk_mesh();
//             //     // mesh_solid
//             //     // mesh_foliage
//             //     mesh
//             // });

//             let mesh = generate_chunk_mesh();

//             meshes.get_mut(id)

//             *stat = ChunkMeshingState::Meshing;//(task);
//         }
//     }

//     // Chunks Upload Mesh.

//     // chunks_meshing.retain(|chunkpos, stat| {
//     //     if let ChunkMeshingState::Meshing(task) = stat {
//     //         if task.is_finished() {
//     //             if let Some(chunk_mesh) = future::block_on(future::poll_once(task)) {
                
                    
//     //                 return false;
//     //             }
//     //         }
//     //     }
//     //     true
//     // });



// }

// fn generate_chunk_mesh() -> Mesh {
//     Mesh::new(PrimitiveTopology::TriangleList)
//     .with_inserted_attribute(
//         Mesh::ATTRIBUTE_POSITION, 
//         vec![
//             [-0.5, 0.5, -0.5], // vertex with index 0
//             [0.5, 0.5, -0.5], // vertex with index 1
//             [0.5, 0.5, 0.5], // etc. until 23
//             [-0.5, 0.5, 0.5],
//         ]
//     )
//     .with_inserted_attribute(
//         Mesh::ATTRIBUTE_UV_0, 
//         vec![
//             // Assigning the UV coords for the top side.
//             [0.0, 0.2], [0.0, 0.0], [1.0, 0.0], [1.0, 0.25],
//         ]
//     )
//     .with_inserted_attribute(
//         Mesh::ATTRIBUTE_NORMAL,
//         vec![
//             // Normals for the top side (towards +y)
//             [0.0, 1.0, 0.0],
//             [0.0, 1.0, 0.0],
//             [0.0, 1.0, 0.0],
//             [0.0, 1.0, 0.0],
//         ]
//     )
//     .with_indices(Some(Indices::U32(vec![
//         0,3,1 , 1,3,2,
//     ])))
// }




fn handle_inputs(
    mut editor_events: EventReader<bevy_editor_pls::editor::EditorEvent>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    mut controller_query: Query<&mut CharacterController>,
    key: Res<Input<KeyCode>>,
    // mouse_input: Res<Input<MouseButton>>,
) {
    let mut window = window_query.single_mut();

    // Toggle MouseGrab
    for event in editor_events.read() {
        match *event {
            EditorEvent::Toggle { now_active } => {
                let playing = !now_active;
                window.cursor.grab_mode = if playing {CursorGrabMode::Locked} else {CursorGrabMode::None};
                window.cursor.visible = !playing;
                for mut controller in &mut controller_query {
                    controller.enable_input = playing;
                }
            },
            _ => ()
        }
    }
    
    // Toggle Fullscreen
    if  key.just_pressed(KeyCode::F11) ||
        (key.pressed(KeyCode::AltLeft) && key.just_pressed(KeyCode::Return)) 
    {
        window.mode = if window.mode != WindowMode::Fullscreen {
            WindowMode::Fullscreen
        } else {
            WindowMode::Windowed
        };
    }
}






#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct WorldInfo {
    
    pub seed: u64,

    pub name: String,

    pub daytime: f32,

    // seconds a day time long
    pub daytime_length: f32,  

    // seconds
    pub time_inhabited: f32,

    time_created: u64,
    time_modified: u64,
    
    tick_timer: Timer,

    is_paused: bool,
    paused_steps: i32,

}

impl WorldInfo {
    fn new() -> Self {
        WorldInfo {
            seed: 0,
            name: "None Name".into(),
            daytime: 0.15,
            daytime_length: 60. * 24.,

            time_inhabited: 0.,
            time_created: 0,
            time_modified: 0,
            
            tick_timer: Timer::new(
                bevy::utils::Duration::from_secs_f32(1. / 20.),
                TimerMode::Repeating,
            ),

            is_paused: false,
            paused_steps: 0,
        }
    }
}


#[derive(Component)]
struct Sun;  // marker


