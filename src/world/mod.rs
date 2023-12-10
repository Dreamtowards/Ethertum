
use std::{f32::consts::{PI, TAU}, sync::Arc};

use bevy::{
    prelude::*, utils::HashMap, 
    window::{CursorGrabMode, PrimaryWindow}, 
    pbr::{ScreenSpaceAmbientOcclusionBundle, ScreenSpaceAmbientOcclusionSettings}, 
    core_pipeline::experimental::taa::{TemporalAntiAliasBundle, TemporalAntiAliasPlugin}, tasks::{AsyncComputeTaskPool, Task}, render::{mesh::Indices, render_resource::PrimitiveTopology}
};

use bevy_atmosphere::prelude::*;

use bevy_editor_pls::editor::EditorEvent;
use bevy_xpbd_3d::prelude::*;
use futures_lite::future;

use crate::controller::{CharacterControllerCamera, CharacterController, CharacterControllerBundle, CharacterControllerPlugin};

pub struct WorldPlugin;


mod chunk;
use chunk::Chunk;
use chunk::ChunkSystem;
use chunk::ChunkPtr;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {


        // Atmosphere
        app.insert_resource(AtmosphereModel::default());
        app.add_plugins(AtmospherePlugin);
        
        // Physics
        app.add_plugins(PhysicsPlugins::default());

        // SSAO
        app.add_plugins(TemporalAntiAliasPlugin);
        app.insert_resource(AmbientLight {
                brightness: 0.05,
                ..default()
            });

        // CharacterController
        app.add_plugins(CharacterControllerPlugin);


        app.insert_resource(WorldInfo::new());
        // app.register_type::<WorldInfo>();
        
        // ChunkSystem
        // app.insert_resource(ChunkSystem::new());
        app.add_systems(Update, update_chunks_loadance);


        app.add_systems(Startup, startup);
        app.add_systems(Update, tick_world);

        app.add_systems(Update, editor_pause);
        app.add_systems(Update, client_inputs);
        
    }
}

fn update_chunks_loadance(
    query_cam: Query<&Transform, With<CharacterControllerCamera>>,
    mut worldinfo: ResMut<WorldInfo>,
    mut chunks_loading: Local<HashMap<IVec3, Task<ChunkPtr>>>,
    
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let thread_pool = AsyncComputeTaskPool::get();
    let chunk_sys = worldinfo.chunk_system.clone();

    let vp = Chunk::as_chunkpos(query_cam.single().translation.as_ivec3()) * 0;  // view pos
    let vd = chunk_sys.view_distance;

    // Chunks Detect Load/Gen
    for y in -vd.y..=vd.y {
        for z in -vd.x..=vd.x {
            for x in -vd.x..=vd.x {
                let chunkpos = IVec3::new(x, y, z) * Chunk::SIZE + vp;

                if chunk_sys.has_chunk(chunkpos) || chunks_loading.contains_key(&chunkpos) {
                    continue;
                }

                let chunk_sys = chunk_sys.clone();
                let task = thread_pool.spawn(async move {

                    info!("Providing {:?}", chunkpos);
                    // provide chunk (load or gen)
                    chunk_sys.provide_chunk(chunkpos)

                });
                chunks_loading.insert(chunkpos, task);
            }
        }
    }
    // DoesNeeds? Chunks Loaded IntoWorld Batch (for reduce LockWrite)
    

    // for (chunkpos, task) in chunks_loading.iter_mut() {
    //     if task.is_finished() {
    //         // load to world
    //         if let Some(chunk) = future::block_on(future::poll_once(task)) {
                
    //             chunk_sys.spawn_chunk(chunk);
    //         } else {
    //             info!("NotAvailable");
    //         }

    //         info!("ChunkProvided: {:?}", chunkpos);
    //     }
    // }
    
    chunks_loading.retain(|chunkpos, task| {
        if task.is_finished() {
            if let Some(chunk) = future::block_on(future::poll_once(task)) {
                
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(generate_chunk_mesh()),
            transform: Transform::from_translation(chunkpos.as_vec3()),
            ..default()
        },
        
        AsyncCollider(ComputedCollider::TriMesh),
        RigidBody::Static,
    ));
                chunk_sys.spawn_chunk(chunk);
                info!("spawn_chunk {:?}", chunkpos);
                return false;
            } else {
                info!("NotAvailable {:?}", chunkpos);
            }
        }
        true
    });

    // Chunks Unload
    // for chunkpos in chunk_sys.chunks.keys() {

    //     // if should_unload()
    //     if (*chunkpos - vp).abs().max_element() {

    //     }
    // }


    // Chunks Detect Meshing

    // Chunks Upload Mesh.



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
    .with_indices(Some(Indices::U32(vec![
        0,3,1 , 1,3,2,
    ])))
}




fn editor_pause(
    mut editor_events: EventReader<bevy_editor_pls::editor::EditorEvent>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    mut controller_query: Query<&mut CharacterController>,
) {
    let mut window = window_query.single_mut();

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
}

fn client_inputs(
    key_input: Res<Input<KeyCode>>,
    mouse_input: Res<Input<MouseButton>>,
) {


    // if player.is_using_item() {
    //     if !keyUseItem {
    //         stopUseItem;
    //     }
    // } else {

    //     if mouse_input.just_pressed(MouseButton::Left) {
    //         // Attack/Destroy
    //         // if ENTITY
    //         // attackEntity(hit.entity)
    //         // BLOCK
    //     } else if mouse_input.just_pressed(MouseButton::Right) {
    //         // Use/Place
    //         // if ENTITY
    //         // interact
    //         // if BLOCK
    //         // if PlayerRightClick
    //         //      swing_item


    //     } else if mouse_input.just_pressed(MouseButton::Middle) {
    //         // Pick

    //     }
    // }
}








#[derive(Resource)]
struct WorldInfo {
    
    seed: u64,

    name: String,

    daytime: f32,

    // seconds a day time long
    daytime_length: f32,  

    // seconds
    time_inhabited: f32,

    time_created: u64,
    time_modified: u64,
    
    tick_timer: Timer,

    is_paused: bool,
    paused_steps: i32,

    chunk_system: Arc<ChunkSystem>,
}

impl WorldInfo {
    fn new() -> Self {
        WorldInfo {
            seed: 0,
            name: "None Name".into(),
            daytime: 0.,
            daytime_length: 60. * 2.,

            time_inhabited: 0.,
            time_created: 0,
            time_modified: 0,
            
            tick_timer: Timer::new(
                bevy::utils::Duration::from_secs_f32(1. / 20.), // Update our atmosphere every 50ms (in a real game, this would be much slower, but for the sake of an example we use a faster update)
                TimerMode::Repeating,
            ),

            is_paused: false,
            paused_steps: 0,

            chunk_system: Arc::new(ChunkSystem::new()),
        }
    }
}


#[derive(Component)]
struct Sun;



// Simple environment
fn startup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {

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
        CharacterControllerBundle::new(Collider::capsule(1., 0.4)),

        
        // PbrBundle {
        //     mesh: meshes.add(Mesh::from(shape::Capsule {
        //         radius: 0.4,
        //         ..default()
        //     })),
        //     material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        //     transform: Transform::from_xyz(0.0, 1.5, 0.0),
        //     ..default()
        // },
        // plugin::CharacterControllerBundle::new(Collider::capsule(1.0, 0.4), Vec3::NEG_Y * 9.81 * 2.0)
        //     .with_movement(30.0, 0.92, 7.0, (30.0f32).to_radians()),
    )).with_children(|p| {
        p.spawn(SpotLightBundle {
            spot_light: SpotLight {
                intensity: 1500.0,
                shadows_enabled: true,
                ..default()
            },
            transform: Transform::from_xyz(0., 0., -0.5),
            ..default()
        });
    });

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

    ))
    .insert(ScreenSpaceAmbientOcclusionBundle::default())
    .insert(TemporalAntiAliasBundle::default());

    // Sun
    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                shadows_enabled: true,
                ..default()
            },
            ..default()
        },
        Sun, // Marks the light as Sun
    ));


    commands.spawn((
        SceneBundle {
            scene: assets.load("spaceship.glb#Scene0"),
            transform: Transform::from_xyz(0., 0., -10.),
            ..default()
        },
        AsyncSceneCollider::new(Some(ComputedCollider::TriMesh)),
        RigidBody::Static,
    ));

    // Floor
    commands.spawn((
        SceneBundle {
            scene: assets.load("playground.glb#Scene0"),
            transform: Transform::from_xyz(0., 0., -10.),
            ..default()
        },
        AsyncSceneCollider::new(Some(ComputedCollider::TriMesh)),
        RigidBody::Static,
    ));

    // Cube
    commands.spawn((
        RigidBody::Dynamic,
        AngularVelocity(Vec3::new(2.5, 3.4, 1.6)),
        Collider::cuboid(1.0, 1.0, 1.0),
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_xyz(0.0, 4.0, 0.0),
            ..default()
        },
    ));
    // // Light
    // commands.spawn(PointLightBundle {
    //     point_light: PointLight {
    //         intensity: 1500.0,
    //         shadows_enabled: true,
    //         ..default()
    //     },
    //     transform: Transform::from_xyz(0., 0.0, 0.0),
    //     ..default()
    // });
}



fn tick_world(
    mut atmosphere: AtmosphereMut<Nishita>,
    mut query: Query<(&mut Transform, &mut DirectionalLight), With<Sun>>,
    mut worldinfo: ResMut<WorldInfo>,
    time: Res<Time>,
) {
    worldinfo.tick_timer.tick(time.delta());
    if !worldinfo.tick_timer.just_finished() {
        return;
    }

    // Pause & Steps
    if worldinfo.is_paused {
        if  worldinfo.paused_steps > 0 {
            worldinfo.paused_steps -= 1;
        } else {
            return;
        }
    }
    

    let dt_sec = worldinfo.tick_timer.duration().as_secs_f32();  // constant time step?
    worldinfo.time_inhabited += dt_sec;
    
    // DayTime
    worldinfo.daytime += dt_sec / worldinfo.daytime_length;
    worldinfo.daytime -= worldinfo.daytime.trunc();  // trunc to [0-1]

    // SunPos
    let sun_ang = worldinfo.daytime * PI*2.;

    // Atmosphere SunPos
    atmosphere.sun_position = Vec3::new(sun_ang.cos(), sun_ang.sin(), 0.);

    if let Some((mut light_trans, mut directional)) = query.single_mut().into() {
        directional.illuminance = sun_ang.sin().max(0.0).powf(2.0) * 100000.0;
        
        // or from000.looking_at()
        light_trans.rotation = Quat::from_rotation_z(sun_ang) * Quat::from_rotation_y(PI / 2.3);
    }
}

