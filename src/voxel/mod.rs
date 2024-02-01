mod chunk;
mod chunk_system;
mod material;
mod meshgen;
mod worldgen;

use self::material::mtl;
use crate::character_controller::{CharacterController, CharacterControllerCamera};
use crate::game::{condition, GameInput, InWorldState, WorldInfo};
use crate::util::iter;
use chunk::*;
pub use chunk_system::*;
use meshgen::*;
use worldgen::*;

use bevy::{
    asset::ReflectAsset,
    math::{ivec2, ivec3},
    prelude::*,
    render::{
        primitives::Aabb,
        render_resource::{AsBindGroup, PrimitiveTopology},
    },
    tasks::{AsyncComputeTaskPool, Task},
    utils::{FloatOrd, HashMap},
};
use bevy_xpbd_3d::{
    components::{AsyncCollider, Collider, ComputedCollider, RigidBody},
    plugins::spatial_query::{SpatialQuery, SpatialQueryFilter},
};
use futures_lite::future;
use once_cell::sync::Lazy;
use std::sync::{Arc, RwLock};
use std::{cell::RefCell, time::Instant};
use thread_local::ThreadLocal;

pub struct VoxelPlugin;

impl Plugin for VoxelPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ChunkSystem::new(ivec2(10, 3)));
        app.register_type::<ChunkSystem>();

        app.add_plugins(MaterialPlugin::<TerrainMaterial>::default());
        app.register_asset_reflect::<TerrainMaterial>();

        app.insert_resource(HitResult::default());

        app.add_systems(OnEnter(InWorldState::InWorld), startup);

        app.add_systems(
            Update,
            (
                raycast,
                chunks_remesh,
                chunks_detect_load_and_unload,
                gizmos,
                // chunks_apply_loaded
                // chunks_apply_remeshed
            )
                .chain()
                .run_if(condition::in_world()),
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
    chunk_sys.entity = commands
        .spawn((
            Name::new("ChunkSystem"),
            InheritedVisibility::VISIBLE,
            GlobalTransform::IDENTITY,
            Transform::IDENTITY,
        ))
        .id();
}

#[derive(Component)]
pub struct ChunkComponent {
    pub chunkpos: IVec3,
}

impl ChunkComponent {
    fn new(chunkpos: IVec3) -> Self {
        Self { chunkpos }
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

    let vp = Chunk::as_chunkpos(query_cam.single().translation.as_ivec3()); // viewer pos
    let vd = chunk_sys.view_distance;

    // Chunks Detect Load/Gen

    const MAX_CONCURRENT_LOADING: usize = 16;

    let max_n = vd.max_element();
    'outer: for n in 0..max_n {
        for y in -n..=n {
            for z in -n..=n {
                for x in -n..=n {
                    if chunk_sys.chunks_loading.len() > MAX_CONCURRENT_LOADING {
                        break 'outer;
                    }
                    if x.abs() < n && y.abs() < n && z.abs() < n {
                        continue;
                    }
                    if x.abs() > vd.x || y.abs() > vd.y || z.abs() > vd.x {
                        continue;
                    }

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

                        // info!("Load Chunk: {:?}", chunkpos);

                        chunkptr
                    });

                    chunk_sys.chunks_loading.insert(chunkpos, task);

                    // gizmo.cuboid(Transform::from_translation(p.as_vec3()).with_scale(Vec3::splat(16.)), Color::RED);
                }
            }
        }
    }

    // Apply Loaded Chunks. todo Refactor.
    let chunksys_entity = chunk_sys.entity;
    let material_handle = chunk_sys.vox_mtl.clone();
    let mut goingtospawn = Vec::new();
    chunk_sys.chunks_loading.retain(|chunkpos, task| {
        if task.is_finished() {
            let chunkptr = future::block_on(future::poll_once(task)).unwrap();
            {
                let mut chunk = chunkptr.write().unwrap();

                chunk.mesh_handle = meshes.add(Mesh::new(PrimitiveTopology::TriangleList));

                chunk.entity = commands
                    .spawn((
                        ChunkComponent::new(*chunkpos),
                        MaterialMeshBundle {
                            mesh: chunk.mesh_handle.clone(),
                            material: material_handle.clone(),
                            transform: Transform::from_translation(chunkpos.as_vec3()),
                            visibility: Visibility::Hidden, // Hidden is required since Mesh is empty.
                            ..default()
                        },
                        Aabb::from_min_max(Vec3::ZERO, Vec3::ONE * (Chunk::SIZE as f32)),
                        RigidBody::Static,
                    ))
                    .set_parent(chunksys_entity)
                    .id();
            }

            goingtospawn.push(chunkptr);

            return false; // remove task.
        }
        true
    });
    for chunkptr in goingtospawn {
        chunk_sys.spawn_chunk(chunkptr);
    }

    // Chunks Detect Unload
    for (entity, chunk_comp) in query_chunks.iter() {
        let chunkpos = chunk_comp.chunkpos;

        if (vp.x - chunkpos.x).abs() > vd.x * Chunk::SIZE
            || (vp.z - chunkpos.z).abs() > vd.x * Chunk::SIZE
            || (vp.y - chunkpos.y).abs() > vd.y * Chunk::SIZE
        {
            // info!("Unload Chunk: {:?}", chunkpos);
            commands.entity(entity).despawn_recursive();
            chunk_sys.despawn_chunk(chunkpos);
        } else if chunk_sys.dbg_remesh_all_chunks {
            chunk_sys.mark_chunk_remesh(chunkpos);
        }
    }
    chunk_sys.dbg_remesh_all_chunks = false;
}

static THREAD_LOCAL_VERTEX_BUFFERS: Lazy<ThreadLocal<RefCell<VertexBuffer>>> = Lazy::new(ThreadLocal::default);

fn chunks_remesh(
    mut commands: Commands,

    query_cam: Query<&Transform, With<CharacterControllerCamera>>,
    mut chunk_sys: ResMut<ChunkSystem>,
    mut meshes: ResMut<Assets<Mesh>>,
    // mut query: Query<(Entity, &Handle<Mesh>, &mut ChunkMeshingTask, &ChunkComponent, &mut Visibility)>,
) {
    let mut chunks_remesh = Vec::from_iter(chunk_sys.chunks_remesh.iter().cloned());

    let p = Chunk::as_chunkpos(query_cam.single().translation.as_ivec3());
    chunks_remesh.sort_unstable_by_key(|v| FloatOrd(v.distance_squared(p) as f32));

    const MAX_CONCURRENT_MESHING: usize = 16;

    for chunkpos in chunks_remesh {
        if chunk_sys.chunks_meshing.len() > MAX_CONCURRENT_MESHING {
            break;
        }

        if let Some(chunkptr) = chunk_sys.get_chunk(chunkpos) {
            let chunkptr = chunkptr.clone();

            let task = AsyncComputeTaskPool::get().spawn(async move {
                let mut vbuf = THREAD_LOCAL_VERTEX_BUFFERS.get_or(|| RefCell::new(VertexBuffer::default())).borrow_mut();

                let dbg_time = Instant::now();
                let entity;
                let mesh_handle;
                {
                    let chunk = chunkptr.read().unwrap();

                    // Generate Mesh
                    MeshGen::generate_chunk_mesh(&mut vbuf, &chunk);

                    entity = chunk.entity;
                    mesh_handle = chunk.mesh_handle.clone();
                }
                let dbg_time = Instant::now() - dbg_time;

                // vbuf.compute_flat_normals();
                vbuf.compute_smooth_normals();

                // let nv = vbuf.vertices.len();
                // vbuf.compute_indexed();  // save 70%+ vertex data space!
                // todo: Cannot use Real IndexedBuffer, it caused WGSL @builtin(vertex_index) produce invalid Barycentric Coordinate, fails material interpolation.
                // vulkan also have this issue, but extension
                //   #extension GL_EXT_fragment_shader_barycentric : enable
                //   layout(location = 2) pervertexEXT in int in_MtlIds[];  gl_BaryCoordEXT
                // would fix the problem in vulkan.
                vbuf.compute_indexed_naive();

                // if nv != 0 {
                //     info!("Generated ReMesh verts: {} before: {} after {}, saved: {}%",
                //     vbuf.vertex_count(), nv, vbuf.vertices.len(), (1.0 - vbuf.vertices.len() as f32/nv as f32) * 100.0);
                // }

                let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
                vbuf.to_mesh(&mut mesh);
                vbuf.clear();

                // Build Collider of TriMesh
                let collider = Collider::trimesh_from_mesh(&mesh);

                (mesh, collider, entity, mesh_handle)
            });

            // info!("Queued ReMesh");
            chunk_sys.chunks_meshing.insert(chunkpos, task);
        }
        chunk_sys.chunks_remesh.remove(&chunkpos);
    }

    chunk_sys.chunks_meshing.retain(|_chunkpos, task| {
        if task.is_finished() {
            let (mesh, collider, entity, mesh_handle) = future::block_on(future::poll_once(task)).unwrap();

            // Update Mesh Asset
            *meshes.get_mut(mesh_handle).unwrap() = mesh;

            // Update Phys Collider TriMesh
            if let Some(collider) = collider {
                if let Some(mut cmds) = commands.get_entity(entity) {
                    // the entity may be already unloaded ?
                    cmds.remove::<Collider>().insert(collider).insert(Visibility::Visible);
                }
            }

            return false; // remove task.
        }
        true
    });
}

fn gizmos(mut gizmos: Gizmos, chunk_sys: Res<ChunkSystem>) {
    // chunks loading
    for cp in chunk_sys.chunks_loading.keys() {
        gizmos.cuboid(
            Transform::from_translation(cp.as_vec3()).with_scale(Vec3::splat(Chunk::SIZE as f32)),
            Color::GREEN,
        );
    }

    // chunks remesh
    for cp in chunk_sys.chunks_remesh.iter() {
        gizmos.cuboid(
            Transform::from_translation(cp.as_vec3()).with_scale(Vec3::splat(Chunk::SIZE as f32)),
            Color::ORANGE,
        );
    }

    // chunks meshing
    for cp in chunk_sys.chunks_meshing.keys() {
        gizmos.cuboid(
            Transform::from_translation(cp.as_vec3()).with_scale(Vec3::splat(Chunk::SIZE as f32)),
            Color::RED,
        );
    }
}

#[derive(Resource, Reflect, Default, Debug)]
#[reflect(Resource)]
pub struct HitResult {
    pub is_hit: bool,
    pub position: Vec3,
    pub normal: Vec3,
    pub distance: f32,
    // entity: Entity,
    pub is_voxel: bool,
}

fn raycast(
    spatial_query: SpatialQuery,

    query_cam: Query<&GlobalTransform, With<CharacterControllerCamera>>,
    query_player: Query<Entity, With<CharacterController>>,

    mut hit_result: ResMut<HitResult>,

    mouse_btn: Res<Input<MouseButton>>,

    mut chunk_sys: ResMut<ChunkSystem>,

    state_ingame: Res<State<GameInput>>,
) {
    let cam_trans = query_cam.single();
    let ray_pos = cam_trans.translation();
    let ray_dir = cam_trans.forward();

    let player_entity = query_player.single();

    if let Some(hit) = spatial_query.cast_ray(
        ray_pos,
        ray_dir,
        100.,
        true,
        SpatialQueryFilter::default().without_entities(vec![player_entity]),
    ) {
        hit_result.is_hit = true;
        hit_result.normal = hit.normal;
        // hit_result.entity = hit.entity;
        let dist = hit.time_of_impact;
        hit_result.distance = dist;
        hit_result.position = ray_pos + ray_dir * dist;

        // commands.entity(hit.entity)
    } else {
        hit_result.is_hit = false;
    }

    if *state_ingame == GameInput::Paused {
        return;
    }
    let do_break = mouse_btn.just_pressed(MouseButton::Left);
    let do_place = mouse_btn.just_pressed(MouseButton::Right);
    if hit_result.is_hit && (do_break || do_place) {
        let n = 5;

        iter::iter_aabb(n, n, |lp| {
            let p = hit_result.position.as_ivec3() + *lp;

            chunk_sys.mark_chunk_remesh(Chunk::as_chunkpos(p));

            let mut chunk = chunk_sys.get_chunk(Chunk::as_chunkpos(p)).unwrap().write().unwrap();

            let c = chunk.get_cell_mut(Chunk::as_localpos(p));

            let dif = (n as f32 - lp.as_vec3().length()).max(0.);

            c.value += if do_break { -dif } else { dif };

            if do_place && c.mtl == 0 {
                c.mtl = mtl::STONE;
            }
        });
    }
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
    pub sample_scale: f32,
    #[uniform(5)]
    pub normal_intensity: f32,
    #[uniform(6)]
    pub triplanar_blend_pow: f32,
    #[uniform(7)]
    pub heightmap_blend_pow: f32, // littler=mix, greater=distinct, opt 0.3 - 0.6, 0.48 = nature
}

impl Default for TerrainMaterial {
    fn default() -> Self {
        Self {
            texture_diffuse: None,
            texture_normal: None,
            texture_dram: None,
            sample_scale: 1.0,
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
