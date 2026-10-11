use avian3d::prelude::*;
use bevy::{
    asset::RenderAssetUsages,
    color::palettes::css,
    image::{ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    pbr::{ExtendedMaterial, MaterialExtension},
    platform::collections::{HashMap, HashSet},
    prelude::*,
    render::render_resource::PrimitiveTopology,
    tasks::AsyncComputeTaskPool,
};
use leafwing_input_manager::action_state::ActionState;

use super::{
    meshgen,
    render::{self, FoliageMaterial, LiquidMaterial, TerrainMaterial},
    ChannelRx, ChannelTx, Chunk, ChunkSystem, VoxShape,
};
use crate::{client::prelude::*, util::iter};

pub struct ClientVoxelPlugin;

impl Plugin for ClientVoxelPlugin {
    fn build(&self, app: &mut App) {
        render::init(app);

        app.add_systems(First, on_world_init.run_if(condition::load_world));
        app.add_systems(Last, on_world_exit.run_if(condition::unload_world()));

        app.insert_resource(VoxelBrush::default());
        app.register_type::<VoxelBrush>();

        app.insert_resource(HitResult::default());
        app.register_type::<HitResult>();

        // app.add_systems(PreUpdate, raycast.run_if(condition::in_world));

        app.add_systems(
            Update,
            (
                raycast,
                chunks_detect_load_and_unload,
                chunks_remesh_enqueue,
                draw_gizmos,
                draw_crosshair_cube.in_set(PhysicsSet::Writeback),
            )
                .chain()
                .run_if(condition::in_world),
        );

        // Draw Crosshair
        // app.add_systems(PostUpdate, draw_crosshair_cube.after(bevy_xpbd_3d::PhysicsSet::Sync).before(bevy::transform::TransformSystem::TransformPropagate));
    }
}

fn on_world_init(
    mut cmds: Commands,
    asset_server: Res<AssetServer>,
    mut mtls_terrain: ResMut<Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>>,
    mut mtls_foliage: ResMut<Assets<ExtendedMaterial<StandardMaterial, FoliageMaterial>>>,
    mut mtls_liquid: ResMut<Assets<ExtendedMaterial<StandardMaterial, LiquidMaterial>>>,
    // mut meshes: ResMut<Assets<Mesh>>,
) {
    info!("Init ClientChunkSystem");
    let (tx, rx) = crate::channel_impl::unbounded::<ChunkRemeshData>();
    cmds.insert_resource(ChannelTx(tx));
    cmds.insert_resource(ChannelRx(rx));
    let (tx, rx) = crate::channel_impl::unbounded::<ChunkLoadingData>();
    cmds.insert_resource(ChannelTx(tx));
    cmds.insert_resource(ChannelRx(rx));
    let mut chunk_sys = ClientChunkSystem::new();

    chunk_sys.mtl_terrain = mtls_terrain.add(ExtendedMaterial {
        base: StandardMaterial {
            base_color_texture: Some(asset_server.load("baked/atlas_diff.png")),
            normal_map_texture: Some(asset_server.load("baked/atlas_norm.png")),
            alpha_mode: AlphaMode::Opaque,
            ..default()
        },
        extension: TerrainMaterial {
            dram_texture: Some(asset_server.load("baked/atlas_dram.png")),
            ..default()
        },
    });

    chunk_sys.mtl_foliage = mtls_foliage.add(ExtendedMaterial {
        base: StandardMaterial {
            base_color: css::BLACK.into(),
            base_color_texture: Some(asset_server.load("baked/atlas_diff_foli.png")),
            perceptual_roughness: 0.0,
            ..default()
        },
        extension: FoliageMaterial { ..default() },
    });

    chunk_sys.mtl_liquid = mtls_liquid.add(ExtendedMaterial {
        base: StandardMaterial {
            base_color: css::BLACK.into(),
            perceptual_roughness: 0.0,
            ..default()
        },
        extension: LiquidMaterial {
            normals: asset_server.load_with_settings::<Image, ImageLoaderSettings>("water_normals.png", |settings| {
                settings.is_srgb = false;
                settings.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                    address_mode_u: ImageAddressMode::Repeat,
                    address_mode_v: ImageAddressMode::Repeat,
                    mag_filter: ImageFilterMode::Linear,
                    min_filter: ImageFilterMode::Linear,
                    ..default()
                });
            }),
        },
    });

    // ChunkSystem entity. all chunk entities will be spawn as children. (for almost no reason. just for editor hierarchy)
    chunk_sys.entity = cmds
        .spawn((
            Name::new("ChunkSystem"),
            InheritedVisibility::VISIBLE,
            GlobalTransform::IDENTITY,
            Transform::IDENTITY,
            DespawnOnWorldUnload,
        ))
        .id();

    cmds.insert_resource(chunk_sys);
}

fn on_world_exit(mut cmds: Commands, chunk_sys: Option<Res<ClientChunkSystem>>) {
    info!("Clear ClientChunkSystem");
    if let Some(chunk_sys) = chunk_sys {
        chunk_sys.cancelled.store(true, Ordering::Relaxed);
    }
    cmds.remove_resource::<ClientChunkSystem>();
    cmds.remove_resource::<ChannelTx<ChunkRemeshData>>();
    cmds.remove_resource::<ChannelRx<ChunkRemeshData>>();
    cmds.remove_resource::<ChannelTx<ChunkLoadingData>>();
    cmds.remove_resource::<ChannelRx<ChunkLoadingData>>();
}

struct ChunkLoadingData {
    epoch: u64,
    request: u64,
    chunk: Chunk,
}

fn chunks_detect_load_and_unload(
    query_cam: Query<&Transform, With<CharacterControllerCamera>>,
    mut chunk_sys: ResMut<ClientChunkSystem>,
    cfg: Res<ClientSettings>,

    mut cmds: Commands,
    mut meshes: ResMut<Assets<Mesh>>,

    tx_chunk_load: Res<ChannelTx<ChunkLoadingData>>,
    rx_chunk_load: Res<ChannelRx<ChunkLoadingData>>,
) {
    let vp = Chunk::as_chunkpos(query_cam.single().unwrap().translation.as_ivec3()); // viewer pos
    let vd = cfg.chunks_load_distance;

    // Chunks Detect Load/Gen

    iter::iter_center_spread(vd.x, vd.y, |rp| {
        if chunk_sys.chunks_loading.len() >= 8 {
            //chunk_sys.max_concurrent_loading {
            return;
        }
        let chunkpos = rp * Chunk::LEN + vp;

        // the chunk already exists, skip.
        if chunk_sys.has_chunk(chunkpos) || chunk_sys.chunks_loading.contains_key(&chunkpos) {
            return;
        }

        let tx = tx_chunk_load.clone();
        let epoch = chunk_sys.epoch;
        let request = chunk_sys.next_revision();
        let cancelled = chunk_sys.cancelled.clone();
        let task = AsyncComputeTaskPool::get().spawn(async move {
            if cancelled.load(Ordering::Relaxed) {
                return;
            }
            let mut chunk = Chunk::new(chunkpos);

            super::worldgen::generate_chunk(&mut chunk);

            if !cancelled.load(Ordering::Relaxed) {
                let _ = tx.send(ChunkLoadingData { epoch, request, chunk });
            }
        });
        task.detach();
        chunk_sys.chunks_loading.insert(chunkpos, request);
    });

    while let Ok(result) = rx_chunk_load.try_recv() {
        let cp = result.chunk.chunkpos;
        if !chunk_sys.accept_load(cp, result.epoch, result.request) {
            continue;
        }
        if super::is_chunk_in_load_distance(vp, cp, vd) {
            chunk_sys.spawn_chunk(result.chunk, &mut cmds, &mut meshes);
        }
    }

    // Chunks Unload

    let chunkpos_all = Vec::from_iter(chunk_sys.get_chunks().keys().cloned());
    for chunkpos in chunkpos_all {
        if !crate::voxel::is_chunk_in_load_distance(vp, chunkpos, vd) {
            chunk_sys.despawn_chunk(chunkpos, &mut cmds);
        }
    }
}

type ChunkRemeshData = (
    IVec3,
    MeshTicket,
    Entity,
    Mesh,
    Handle<Mesh>,
    Option<Collider>,
    Mesh,
    Handle<Mesh>,
    Mesh,
    Handle<Mesh>,
);

use crate::util::vtx::VertexBuffer;
use once_cell::sync::Lazy;
use std::{
    cell::RefCell,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};
use thread_local::ThreadLocal;

static THREAD_LOCAL_VERTEX_BUFFERS: Lazy<ThreadLocal<RefCell<(VertexBuffer, VertexBuffer, VertexBuffer)>>> = Lazy::new(ThreadLocal::default);

fn chunks_remesh_enqueue(
    mut commands: Commands,
    query_children: Query<&Children>,

    query_cam: Query<&Transform, With<CharacterControllerCamera>>,
    mut chunk_sys: ResMut<ClientChunkSystem>,
    mut meshes: ResMut<Assets<Mesh>>,

    tx_chunks_meshing: Res<ChannelTx<ChunkRemeshData>>,
    rx_chunks_meshing: Res<ChannelRx<ChunkRemeshData>>,
    // mut foliage_mtls: ResMut<Assets<FoliageMaterial>>,
    // time: Res<Time>,
) {
    // foliage_mtls.get_mut(chunk_sys.mtl_foliage.id()).unwrap().time = time.elapsed_seconds();

    let mut chunks_remesh = Vec::from_iter(chunk_sys.chunks_remesh.iter().cloned());

    // Sort by Distance from the Camera.
    let cam_cp = Chunk::as_chunkpos(query_cam.single().unwrap().translation.as_ivec3());
    chunks_remesh.sort_unstable_by_key(|cp: &IVec3| bevy::math::FloatOrd(cp.distance_squared(cam_cp) as f32));

    for chunkpos in chunks_remesh {
        if chunk_sys.chunks_meshing.len() >= chunk_sys.max_concurrent_meshing {
            break;
        }
        if chunk_sys.chunks_meshing.contains_key(&chunkpos) {
            continue;
        }

        if let Some(chunk) = chunk_sys.snapshot_chunk(chunkpos) {
            let ticket = chunk_sys.mesh_ticket(chunkpos);
            chunk_sys.chunks_meshing.insert(chunkpos, ticket);
            let cancelled = chunk_sys.cancelled.clone();
            let tx = tx_chunks_meshing.clone();

            let task = AsyncComputeTaskPool::get().spawn(async move {
                if cancelled.load(Ordering::Relaxed) {
                    return;
                }
                let mut _vbuf = THREAD_LOCAL_VERTEX_BUFFERS
                    .get_or(|| RefCell::new((VertexBuffer::default(), VertexBuffer::default(), VertexBuffer::default())))
                    .borrow_mut();
                // 0: vbuf_terrain, 1: vbuf_foliage, 2: vbuf_liquid

                // let dbg_time = Instant::now();
                let entity;
                let mesh_handle_terrain;
                let mesh_handle_foliage;
                let mesh_handle_liquid;
                {
                    let chunk = &chunk;

                    // Generate Mesh
                    meshgen::generate_chunk_mesh(&mut _vbuf.0, chunk);

                    meshgen::generate_chunk_mesh_foliage(&mut _vbuf.1, chunk);

                    meshgen::generate_chunk_mesh_liquid(&mut _vbuf.2, chunk);

                    entity = chunk.entity;
                    mesh_handle_terrain = chunk.mesh_handle_terrain.clone();
                    mesh_handle_foliage = chunk.mesh_handle_foliage.clone();
                    mesh_handle_liquid = chunk.mesh_handle_liquid.clone();
                }
                // let dbg_time = Instant::now() - dbg_time;

                // vbuf.compute_flat_normals();
                // _vbuf.0.compute_smooth_normals();

                // let nv = vbuf.vertices.len();
                // vbuf.compute_indexed();  // save 70%+ vertex data space!
                // todo: Cannot use Real IndexedBuffer, it caused WGSL @builtin(vertex_index) produce invalid Barycentric Coordinate, fails material interpolation.
                // vulkan also have this issue, but extension
                //   #extension GL_EXT_fragment_shader_barycentric : enable
                //   layout(location = 2) pervertexEXT in int in_MtlIds[];  gl_BaryCoordEXT
                // would fix the problem in vulkan.
                _vbuf.0.compute_indexed_naive();

                // if nv != 0 {
                //     info!("Generated ReMesh verts: {} before: {} after {}, saved: {}%",
                //     vbuf.vertex_count(), nv, vbuf.vertices.len(), (1.0 - vbuf.vertices.len() as f32/nv as f32) * 100.0);
                // }

                let mut mesh_terrain = Mesh::new(
                    PrimitiveTopology::TriangleList,
                    RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
                );
                _vbuf.0.to_mesh(&mut mesh_terrain);
                _vbuf.0.clear();

                // Build Collider of TriMesh
                let collider = Collider::trimesh_from_mesh(&mesh_terrain);

                // Foliage
                _vbuf.1.compute_indexed_naive();

                let mut mesh_foliage = Mesh::new(
                    PrimitiveTopology::TriangleList,
                    RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
                );
                _vbuf.1.to_mesh(&mut mesh_foliage);
                _vbuf.1.clear();

                // Liquid
                _vbuf.2.compute_indexed_naive();

                let mut mesh_liquid = Mesh::new(
                    PrimitiveTopology::TriangleList,
                    RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
                );
                _vbuf.2.to_mesh(&mut mesh_liquid);
                _vbuf.2.clear();

                if !cancelled.load(Ordering::Relaxed) {
                    let _ = tx.send((
                        chunkpos,
                        ticket,
                        entity,
                        mesh_terrain,
                        mesh_handle_terrain,
                        collider,
                        mesh_foliage,
                        mesh_handle_foliage,
                        mesh_liquid,
                        mesh_handle_liquid,
                    ));
                }
            });
            task.detach();

            // info!("[ReMesh Enqueued] Pos: {}; ReMesh: {}, Meshing: {}: tx: {}, rx: {}", chunkpos, chunk_sys.chunks_remesh.len(), cli.chunks_meshing.len(), tx_chunks_meshing.len(), rx_chunks_meshing.len());
        }

        chunk_sys.chunks_remesh.remove(&chunkpos);
    }

    while let Ok((
        chunkpos,
        ticket,
        entity,
        mut mesh_terrain,
        mesh_handle_terrain,
        collider,
        mut mesh_foliage,
        mesh_handle_foliage,
        mut mesh_liquid,
        mesh_handle_liquid,
    )) = rx_chunks_meshing.try_recv()
    {
        if !chunk_sys.accept_mesh(chunkpos, ticket) {
            continue;
        }
        let terrain_visibility = super::chunk_mesh::prepare_chunk_mesh(&mut mesh_terrain);
        let foliage_visibility = super::chunk_mesh::prepare_chunk_mesh(&mut mesh_foliage);
        let liquid_visibility = super::chunk_mesh::prepare_chunk_mesh(&mut mesh_liquid);

        // Update Mesh Asset
        *meshes.get_mut(mesh_handle_terrain.id()).unwrap() = mesh_terrain;

        *meshes.get_mut(mesh_handle_foliage.id()).unwrap() = mesh_foliage;

        *meshes.get_mut(mesh_handle_liquid.id()).unwrap() = mesh_liquid;

        // A populated GPU mesh can remain cached after a CPU-only empty update.
        // Hide each empty surface explicitly, including foliage and liquid children.
        if let Ok(mut cmds) = commands.get_entity(entity) {
            cmds.try_remove::<Collider>().try_insert(terrain_visibility);
            if let Some(collider) = collider {
                cmds.try_insert(collider);
            }
        }
        if let Ok(children) = query_children.get(entity) {
            for (child, visibility) in children.iter().zip([foliage_visibility, liquid_visibility]) {
                commands.entity(child).try_insert(visibility);
            }
        }

        // info!("[ReMesh Completed] Pos: {}; ReMesh: {}, Meshing: {}: tx: {}, rx: {}", chunkpos, chunk_sys.chunks_remesh.len(), cli.chunks_meshing.len(), tx_chunks_meshing.len(), rx_chunks_meshing.len());
    }
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct VoxelBrush {
    pub shape: VoxShape,
    pub tex: u16,
    pub size: f32,
    pub strength: f32,
}
impl Default for VoxelBrush {
    fn default() -> Self {
        Self {
            size: 4.,
            strength: 0.8,
            shape: VoxShape::Isosurface,
            tex: 10,
        }
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
    pub voxel_pos: IVec3,
}

fn raycast(
    spatial_query: SpatialQuery,
    query_cam: Query<&GlobalTransform, With<CharacterControllerCamera>>, // ray
    query_player: Query<Entity, With<CharacterController>>,              // exclude collider
    mut hit_result: ResMut<HitResult>,

    query_input: Query<&ActionState<InputAction>>,
    mut chunk_sys: ResMut<ClientChunkSystem>,
    cli: Res<ClientInfo>,
    vox_brush: Res<VoxelBrush>,
) {
    let cam_trans = query_cam.single().unwrap();
    let ray_pos = cam_trans.translation();
    let ray_dir = cam_trans.forward();

    let player_entity = query_player.single().unwrap_or(Entity::PLACEHOLDER);

    if let Some(hit) = spatial_query.cast_ray(
        ray_pos,
        ray_dir,
        100.,
        true,
        &SpatialQueryFilter::default().with_excluded_entities(vec![player_entity]),
    ) {
        hit_result.is_hit = true;
        hit_result.normal = hit.normal;
        // hit_result.entity = hit.entity;
        let dist = hit.distance;
        hit_result.distance = dist;
        hit_result.position = ray_pos + ray_dir.as_vec3() * dist;

        // commands.entity(hit.entity)

        hit_result.voxel_pos = (hit_result.position + -0.01 * hit_result.normal).floor().as_ivec3();
    } else {
        hit_result.is_hit = false;
    }

    // ############ Break & Place ############

    if cli.curr_ui != CurrentUI::None {
        // todo: cli.is_manipulating()
        return;
    }

    let action_state = query_input.single().unwrap();
    let do_break = action_state.just_pressed(&InputAction::Attack);
    let do_place = action_state.just_pressed(&InputAction::UseItem);

    if hit_result.is_hit && (do_break || do_place) {
        let brush = &*vox_brush;
        let n = brush.size as i32;

        // These code is Horrible

        iter::iter_aabb(n, n, |lp| {
            // +0.01*norm: for placing cube like MC.
            let p = hit_result.voxel_pos + lp + if do_place { 1 } else { 0 } * hit_result.normal.normalize_or_zero().as_ivec3();

            if let Some(v) = chunk_sys.get_voxel_mut(p) {
                let f = (n as f32 - lp.as_vec3().length()).max(0.) * brush.strength;

                v.set_isovalue(v.isovalue() + if do_break { -f } else { f });

                if f > 0.0 || (n == 0 && f == 0.0) {
                    // placing single
                    if do_place {
                        // && c.tex_id == 0 {
                        v.tex_id = brush.tex;
                        v.shape_id = brush.shape;

                        // placing Block
                        if brush.shape != VoxShape::Isosurface {
                            v.set_isovalue(0.0);
                        }
                    } else if v.is_isoval_empty() {
                        v.tex_id = 0;
                    }
                }
            }
        });
        // let mut map = HashMap::new();
        // let pack = map.entry(chunkpos).or_insert_with(Vec::new);
        // pack.push(CellData::from_cell(Chunk::local_idx(Chunk::as_localpos(p)) as u16, &c));
        // info!("Modify terrain sent {}", map.len());
        // for e in map {
        //     net_client.send_packet(&CPacket::ChunkModify { chunkpos: e.0, voxel: e.1 });
        // }
    }
}

fn draw_crosshair_cube(mut gizmos: Gizmos, hit_result: Res<HitResult>, vbrush: Res<VoxelBrush>) {
    if hit_result.is_hit {
        if vbrush.shape == VoxShape::Isosurface {
            //gizmos.sphere(hit_result.position, Quat::IDENTITY, vbrush.size, Color::BLACK);
            gizmos.sphere(Isometry3d::IDENTITY, vbrush.size, Color::BLACK);
        } else {
            let trans = Transform::from_translation(hit_result.voxel_pos.as_vec3() + 0.5).with_scale(Vec3::ONE * vbrush.size.floor());

            gizmos.cube(trans, Color::BLACK);
        }
    }
}

fn draw_gizmos(mut gizmos: Gizmos, chunk_sys: Res<ClientChunkSystem>, cli: Res<ClientInfo>, query_cam: Query<&Transform, With<CharacterController>>) {
    // // chunks loading
    // for cp in chunk_sys.chunks_loading.keys() {
    //     gizmos.cube(
    //         Transform::from_translation(cp.as_vec3()).with_scale(Vec3::splat(Chunk::LEN as f32)),
    //         Color::GREEN,
    //     );
    // }

    // all loaded chunks
    if cli.dbg_gizmo_all_loaded_chunks {
        for cp in chunk_sys.get_chunks().keys() {
            gizmos.cube(
                Transform::from_translation(cp.as_vec3() + 0.5 * Chunk::LEN as f32).with_scale(Vec3::splat(Chunk::LEN as f32)),
                Srgba::gray(0.25),
            );
        }
    }

    if cli.dbg_gizmo_curr_chunk {
        if let Ok(trans) = query_cam.single() {
            let cp = Chunk::as_chunkpos(trans.translation.as_ivec3());
            gizmos.cube(
                Transform::from_translation(cp.as_vec3() + 0.5 * Chunk::LEN as f32).with_scale(Vec3::splat(Chunk::LEN as f32)),
                Srgba::gray(0.7),
            );
        }
    }

    if cli.dbg_gizmo_remesh_chunks {
        // chunks remesh
        for cp in chunk_sys.chunks_remesh.iter() {
            gizmos.cube(
                Transform::from_translation(cp.as_vec3() + 0.5 * Chunk::LEN as f32).with_scale(Vec3::splat(Chunk::LEN as f32)),
                css::ORANGE,
            );
        }

        // chunks meshing
        for cp in chunk_sys.chunks_meshing.keys() {
            gizmos.cube(
                Transform::from_translation(cp.as_vec3() + 0.5 * Chunk::LEN as f32).with_scale(Vec3::splat(Chunk::LEN as f32)),
                css::RED,
            );
        }
    }
}

///////////////////////////////////////////////////
//////////////// ClientChunkSystem ////////////////
///////////////////////////////////////////////////

#[derive(Resource)]
pub struct ClientChunkSystem {
    chunks: HashMap<IVec3, Chunk>,

    // mark to ReMesh
    chunks_remesh: HashSet<IVec3>,

    pub mtl_terrain: Handle<ExtendedMaterial<StandardMaterial, TerrainMaterial>>,
    pub mtl_foliage: Handle<ExtendedMaterial<StandardMaterial, FoliageMaterial>>,
    pub mtl_liquid: Handle<ExtendedMaterial<StandardMaterial, LiquidMaterial>>,
    // pub mtl_std: Handle<StandardMaterial>,
    pub entity: Entity,

    pub max_concurrent_meshing: usize,
    chunks_meshing: HashMap<IVec3, MeshTicket>,
    chunks_loading: HashMap<IVec3, u64>,
    revisions: HashMap<IVec3, u64>,
    revision: u64,
    epoch: u64,
    cancelled: Arc<AtomicBool>,
    force_blocky: bool,
    // pub chunks_load_distance: IVec2, // not real, but send to server,
}

impl ChunkSystem for ClientChunkSystem {
    fn get_chunk_mut(&mut self, cp: IVec3) -> Option<&mut Chunk> {
        self.invalidate_neighborhood(cp);
        self.chunks.get_mut(&cp)
    }

    fn get_voxel_mut(&mut self, p: IVec3) -> Option<&mut super::Vox> {
        // Surface nets/cubes sample a one-voxel halo, including edges and corners.
        for cp in Chunk::mesh_dependents(p) {
            self.mark_chunk_remesh(cp);
        }
        self.chunks
            .get_mut(&Chunk::as_chunkpos(p))
            .map(|chunk| chunk.at_voxel_mut(Chunk::as_localpos(p)))
    }

    fn get_chunks(&self) -> &HashMap<IVec3, Chunk> {
        &self.chunks
    }
}

impl Default for ClientChunkSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientChunkSystem {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::default(),
            chunks_remesh: HashSet::default(),

            mtl_terrain: Handle::default(),
            mtl_foliage: Handle::default(),
            mtl_liquid: Handle::default(),
            // mtl_std: Handle::default(),
            entity: Entity::PLACEHOLDER,

            max_concurrent_meshing: 8,
            chunks_meshing: HashMap::default(),
            chunks_loading: HashMap::default(),
            revisions: HashMap::default(),
            revision: 0,
            epoch: NEXT_WORLD_EPOCH.fetch_add(1, Ordering::Relaxed),
            cancelled: Arc::new(AtomicBool::new(false)),
            force_blocky: false,
        }
    }

    fn next_revision(&mut self) -> u64 {
        self.revision = self.revision.checked_add(1).expect("chunk revision overflow");
        self.revision
    }

    pub fn mark_chunk_remesh(&mut self, chunkpos: IVec3) {
        if !self.chunks.contains_key(&chunkpos) {
            return;
        }
        let revision = self.next_revision();
        self.revisions.insert(chunkpos, revision);
        self.chunks_remesh.insert(chunkpos);
    }

    fn invalidate_neighborhood(&mut self, cp: IVec3) {
        self.mark_chunk_remesh(cp);
        for dir in Chunk::NEIGHBOR_DIR {
            self.mark_chunk_remesh(cp + dir * Chunk::LEN);
        }
    }

    fn mesh_ticket(&self, cp: IVec3) -> MeshTicket {
        MeshTicket {
            epoch: self.epoch,
            revision: self.revisions[&cp],
        }
    }

    fn accept_load(&mut self, cp: IVec3, epoch: u64, request: u64) -> bool {
        if epoch != self.epoch || self.chunks_loading.get(&cp) != Some(&request) {
            return false;
        }
        self.chunks_loading.remove(&cp);
        !self.has_chunk(cp)
    }

    fn accept_mesh(&mut self, cp: IVec3, ticket: MeshTicket) -> bool {
        // An old result must never clear a newer task's tracking entry.
        if ticket.epoch != self.epoch || self.chunks_meshing.get(&cp) != Some(&ticket) {
            return false;
        }
        self.chunks_meshing.remove(&cp);
        self.chunks.contains_key(&cp) && self.revisions.get(&cp) == Some(&ticket.revision)
    }

    pub fn force_blocky(&self) -> bool {
        self.force_blocky
    }

    pub fn num_chunks_meshing(&self) -> usize {
        self.chunks_meshing.len()
    }

    pub fn num_chunks_remesh(&self) -> usize {
        self.chunks_remesh.len()
    }

    pub fn set_force_blocky(&mut self, enabled: bool) {
        if self.force_blocky == enabled {
            return;
        }
        self.force_blocky = enabled;
        let positions: Vec<_> = self.chunks.keys().copied().collect();
        for cp in positions {
            self.chunks.get_mut(&cp).unwrap().force_blocky = enabled;
            self.mark_chunk_remesh(cp);
        }
    }

    /// Population/tree generation edits a detached neighborhood synchronously.
    /// Only changed voxel arrays are installed; render handles and entities stay owned here.
    pub fn edit_neighborhood(&mut self, cp: IVec3, edit: impl FnOnce(&mut Chunk)) {
        let Some(snapshot) = self.snapshot_chunk(cp) else {
            return;
        };
        let mut snapshot = snapshot.into_chunk();
        edit(&mut snapshot);
        let neighbors = std::mem::take(&mut snapshot.neighbor_chunks);
        for updated in std::iter::once(snapshot).chain(neighbors.into_iter().flatten().map(|c| *c)) {
            let cp = updated.chunkpos;
            let Some(current) = self.chunks.get_mut(&cp) else {
                continue;
            };
            current.is_populated = updated.is_populated;
            if !Arc::ptr_eq(&current.voxel, &updated.voxel) {
                current.voxel = updated.voxel;
                self.invalidate_neighborhood(cp);
            }
        }
    }

    pub fn spawn_chunk(&mut self, mut chunk: Chunk, cmds: &mut Commands, meshes: &mut Assets<Mesh>) {
        let chunkpos = chunk.chunkpos;

        let aabb = bevy::camera::primitives::Aabb::from_min_max(Vec3::ZERO, Vec3::ONE * (Chunk::LEN as f32));

        chunk.mesh_handle_terrain = meshes.add(Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::MAIN_WORLD));
        chunk.mesh_handle_foliage = meshes.add(Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::MAIN_WORLD));
        chunk.mesh_handle_liquid = meshes.add(Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::MAIN_WORLD));

        chunk.entity = cmds
            .spawn((
                // ChunkComponent::new(*chunkpos),
                (
                    Mesh3d(chunk.mesh_handle_terrain.clone()),
                    MeshMaterial3d(self.mtl_terrain.clone()), //materials.add(Color::rgb(0.8, 0.7, 0.6)),
                    Transform::from_translation(chunkpos.as_vec3()),
                    Visibility::Hidden, // Hidden is required since Mesh is empty. or WGPU will crash. even if use default Inherite
                ),
                aabb,
                avian3d::prelude::RigidBody::Static,
            ))
            .with_children(|parent| {
                parent.spawn((
                    (
                        Mesh3d(chunk.mesh_handle_foliage.clone()),
                        MeshMaterial3d(self.mtl_foliage.clone()),
                        Visibility::Visible, // Hidden is required since Mesh is empty. or WGPU will crash
                    ),
                    aabb,
                ));
                parent.spawn((
                    (
                        Mesh3d(chunk.mesh_handle_liquid.clone()),
                        MeshMaterial3d(self.mtl_liquid.clone()),
                        Visibility::Visible,
                    ),
                    aabb,
                ));
            })
            .set_parent_in_place(self.entity)
            .id();

        self.chunks_loading.remove(&chunkpos);
        if let Some(previous) = self.chunks.remove(&chunkpos) {
            cmds.entity(previous.entity).try_despawn();
        }
        chunk.force_blocky = self.force_blocky;
        self.chunks.insert(chunkpos, chunk);
        self.invalidate_neighborhood(chunkpos);

        // Newly loaded neighbors can complete the central chunk too.
        for cp in std::iter::once(chunkpos).chain(Chunk::NEIGHBOR_DIR.map(|d| chunkpos + d * Chunk::LEN)) {
            if self.get_chunk(cp).is_some_and(|c| !c.is_populated) && Chunk::NEIGHBOR_DIR.iter().all(|d| self.has_chunk(cp + *d * Chunk::LEN)) {
                self.edit_neighborhood(cp, |chunk| {
                    chunk.is_populated = true;
                    super::worldgen::populate_chunk(chunk);
                });
            }
        }
    }

    pub fn despawn_chunk(&mut self, chunkpos: IVec3, cmds: &mut Commands) -> Option<Chunk> {
        let chunk = self.chunks.remove(&chunkpos)?;
        self.chunks_remesh.remove(&chunkpos);
        self.chunks_meshing.remove(&chunkpos);
        self.chunks_loading.remove(&chunkpos);
        self.revisions.remove(&chunkpos);
        self.invalidate_neighborhood(chunkpos);
        cmds.entity(chunk.entity).try_despawn();
        Some(chunk)
    }
}

static NEXT_WORLD_EPOCH: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MeshTicket {
    epoch: u64,
    revision: u64,
}

impl Drop for ClientChunkSystem {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
#[path = "chunk_tests.rs"]
mod tests;
