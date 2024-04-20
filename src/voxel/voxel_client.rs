use bevy::{
    asset::ReflectAsset,
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{AsBindGroup, PrimitiveTopology},
    },
    tasks::AsyncComputeTaskPool,
    utils::{HashMap, HashSet},
};
use bevy_xpbd_3d::plugins::{
    collision::Collider,
    spatial_query::{SpatialQuery, SpatialQueryFilter},
};
use leafwing_input_manager::action_state::ActionState;

use super::{meshgen::MeshGen, ChannelRx, ChannelTx, Chunk, ChunkPtr, ChunkSystem, VoxShape};
use crate::{
    client::{
        character_controller::{CharacterController, CharacterControllerCamera},
        game_client::{condition, ClientInfo, DespawnOnWorldUnload},
        prelude::{ClientSettings, InputAction},
        ui::CurrentUI,
    },
    util::{iter, AsRefMut},
};

pub struct ClientVoxelPlugin;

impl Plugin for ClientVoxelPlugin {
    fn build(&self, app: &mut App) {
        // Render Shader.
        app.add_plugins(MaterialPlugin::<TerrainMaterial>::default());
        app.register_asset_reflect::<TerrainMaterial>(); // debug

        {
            let (tx, rx) = crate::channel_impl::unbounded::<ChunkRemeshData>();
            app.insert_resource(ChannelTx(tx));
            app.insert_resource(ChannelRx(rx));

            let (tx, rx) = crate::channel_impl::unbounded::<Chunk>();
            app.insert_resource(ChannelTx(tx));
            app.insert_resource(ChannelRx(rx));
        }

        app.add_systems(First, on_world_init.run_if(condition::load_world));
        app.add_systems(Last, on_world_exit.run_if(condition::unload_world()));

        app.insert_resource(VoxelBrush::default());
        app.register_type::<VoxelBrush>();

        app.insert_resource(HitResult::default());
        app.register_type::<HitResult>();

        app.add_systems(
            Update,
            (chunks_detect_load_and_unload, chunks_remesh_enqueue, raycast, draw_gizmos, draw_crosshair_cube)
                .chain()
                .run_if(condition::in_world),
        );
    }
}

fn on_world_init(
    mut cmds: Commands,
    asset_server: Res<AssetServer>,
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
    mut std_mtls: ResMut<Assets<StandardMaterial>>,
) {
    info!("Init ClientChunkSystem");
    let mut chunk_sys = ClientChunkSystem::new();

    chunk_sys.mtl_terrain = terrain_materials.add(TerrainMaterial {
        texture_diffuse: Some(asset_server.load("baked/atlas_diff.png")),
        texture_normal: Some(asset_server.load("baked/atlas_norm.png")),
        texture_dram: Some(asset_server.load("baked/atlas_dram.png")),
        ..default()
    });

    chunk_sys.mtl_foliage = std_mtls.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("baked/atlas_diff_foli.png")),
        // normal_map_texture: if has_norm {Some(asset_server.load(format!("models/{name}/norm.png")))} else {None},
        double_sided: true,
        alpha_mode: AlphaMode::Mask(0.5),
        cull_mode: None,
        ..default()
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

fn on_world_exit(mut cmds: Commands) {
    info!("Clear ClientChunkSystem");
    cmds.remove_resource::<ClientChunkSystem>();
}

type ChunkLoadingData = Chunk;

fn chunks_detect_load_and_unload(
    query_cam: Query<&Transform, With<CharacterControllerCamera>>,
    mut chunk_sys: ResMut<ClientChunkSystem>,
    mut chunks_loading: Local<HashSet<IVec3>>, // for detect/skip if is loading
    cfg: Res<ClientSettings>,

    mut cmds: Commands,
    mut meshes: ResMut<Assets<Mesh>>,

    tx_chunk_load: Res<ChannelTx<ChunkLoadingData>>,
    rx_chunk_load: Res<ChannelRx<ChunkLoadingData>>,
) {
    let vp = Chunk::as_chunkpos(query_cam.single().translation.as_ivec3()); // viewer pos
    let vd = cfg.chunks_load_distance;

    // Chunks Detect Load/Gen

    iter::iter_center_spread(vd.x, vd.y, |rp| {
        if chunks_loading.len() > 8 {
            //chunk_sys.max_concurrent_loading {
            return;
        }
        let chunkpos = rp * Chunk::SIZE + vp;

        // the chunk already exists, skip.
        if chunk_sys.has_chunk(chunkpos) || chunks_loading.contains(&chunkpos) {
            return;
        }

        let tx = tx_chunk_load.clone();
        let task = AsyncComputeTaskPool::get().spawn(async move {
            // info!("Load Chunk: {:?}", chunkpos);
            let mut chunk = Chunk::new(chunkpos);

            super::WorldGen::generate_chunk(&mut chunk);

            tx.send(chunk).unwrap();
        });
        task.detach();
        chunks_loading.insert(chunkpos);
    });

    while let Ok(chunk) = rx_chunk_load.try_recv() {
        chunks_loading.remove(&chunk.chunkpos);

        chunk_sys.spawn_chunk(chunk, &mut cmds, &mut meshes);
    }
}

type ChunkRemeshData = (IVec3, Entity, Mesh, Handle<Mesh>, Option<Collider>, Mesh, Handle<Mesh>);

use crate::voxel::meshgen::VertexBuffer;
use once_cell::sync::Lazy;
use std::{cell::RefCell, sync::Arc};
use thread_local::ThreadLocal;

static THREAD_LOCAL_VERTEX_BUFFERS: Lazy<ThreadLocal<RefCell<(VertexBuffer, VertexBuffer)>>> = Lazy::new(ThreadLocal::default);

fn chunks_remesh_enqueue(
    mut commands: Commands,

    query_cam: Query<&Transform, With<CharacterControllerCamera>>,
    mut chunk_sys: ResMut<ClientChunkSystem>,
    mut meshes: ResMut<Assets<Mesh>>,

    tx_chunks_meshing: Res<ChannelTx<ChunkRemeshData>>,
    rx_chunks_meshing: Res<ChannelRx<ChunkRemeshData>>,
) {
    let mut chunks_remesh = Vec::from_iter(chunk_sys.chunks_remesh.iter().cloned());

    // Sort by Distance from the Camera.
    let cam_cp = Chunk::as_chunkpos(query_cam.single().translation.as_ivec3());
    chunks_remesh.sort_unstable_by_key(|cp: &IVec3| bevy::utils::FloatOrd(cp.distance_squared(cam_cp) as f32));

    for chunkpos in chunks_remesh {
        if chunk_sys.chunks_meshing.len() >= chunk_sys.max_concurrent_meshing {
            break;
        }
        if chunk_sys.chunks_meshing.contains(&chunkpos) {
            continue;
        }

        let mut has = false;
        if let Some(chunkptr) = chunk_sys.get_chunk(chunkpos) {
            has = true;

            let chunkptr = chunkptr.clone();
            let tx = tx_chunks_meshing.clone();

            let task = AsyncComputeTaskPool::get().spawn(async move {
                let mut _vbuf = THREAD_LOCAL_VERTEX_BUFFERS
                    .get_or(|| RefCell::new((VertexBuffer::default(), VertexBuffer::default())))
                    .borrow_mut();
                // 0: vbuf_terrain, 1: vbuf_foliage

                // let dbg_time = Instant::now();
                let entity;
                let mesh_handle;
                let mesh_handle_foliage;
                {
                    let chunk = chunkptr.as_ref();

                    // Generate Mesh
                    MeshGen::generate_chunk_mesh(&mut _vbuf.0, chunk);

                    MeshGen::generate_chunk_mesh_foliage(&mut _vbuf.1, chunk);

                    entity = chunk.entity;
                    mesh_handle = chunk.mesh_handle.clone();
                    mesh_handle_foliage = chunk.mesh_handle_foliage.clone();
                }
                // let dbg_time = Instant::now() - dbg_time;

                // vbuf.compute_flat_normals();
                _vbuf.0.compute_smooth_normals();

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

                let mut mesh = Mesh::new(
                    PrimitiveTopology::TriangleList,
                    RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
                );
                _vbuf.0.to_mesh(&mut mesh);
                _vbuf.0.clear();

                // Foliage
                _vbuf.1.compute_indexed_naive();

                let mut mesh_foliage = Mesh::new(
                    PrimitiveTopology::TriangleList,
                    RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
                );
                _vbuf.1.to_mesh(&mut mesh_foliage);
                _vbuf.1.clear();

                // Build Collider of TriMesh
                let collider = Collider::trimesh_from_mesh(&mesh);

                tx.send((chunkpos, entity, mesh, mesh_handle, collider, mesh_foliage, mesh_handle_foliage))
                    .unwrap();
            });
            task.detach();

            // info!("[ReMesh Enqueued] Pos: {}; ReMesh: {}, Meshing: {}: tx: {}, rx: {}", chunkpos, chunk_sys.chunks_remesh.len(), cli.chunks_meshing.len(), tx_chunks_meshing.len(), rx_chunks_meshing.len());
        }
        if has {
            chunk_sys.chunks_meshing.insert(chunkpos);
        }
        chunk_sys.chunks_remesh.remove(&chunkpos);
    }

    while let Ok((chunkpos, entity, mesh, mesh_handle, collider, mesh_foliage, mesh_handle_foliage)) = rx_chunks_meshing.try_recv() {
        // Update Mesh Asset
        *meshes.get_mut(mesh_handle).unwrap() = mesh;

        *meshes.get_mut(mesh_handle_foliage).unwrap() = mesh_foliage;

        // Update Phys Collider TriMesh
        if let Some(collider) = collider {
            if let Some(mut cmds) = commands.get_entity(entity) {
                // note: use try_insert cuz the entity may already been unloaded when executing the cmds (?)
                cmds.remove::<Collider>().try_insert(collider).try_insert(Visibility::Visible);
            }
        }

        chunk_sys.chunks_meshing.remove(&chunkpos);
        // info!("[ReMesh Completed] Pos: {}; ReMesh: {}, Meshing: {}: tx: {}, rx: {}", chunkpos, chunk_sys.chunks_remesh.len(), cli.chunks_meshing.len(), tx_chunks_meshing.len(), rx_chunks_meshing.len());
    }
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct VoxelBrush {
    pub size: f32,
    pub strength: f32,
    pub shape: VoxShape,
    pub tex: u16,
}
impl Default for VoxelBrush {
    fn default() -> Self {
        Self {
            size: 4.,
            strength: 0.8,
            shape: VoxShape::Isosurface,
            tex: 21,
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
    let cam_trans = query_cam.single();
    let ray_pos = cam_trans.translation();
    let ray_dir = cam_trans.forward();

    let player_entity = query_player.get_single().unwrap_or(Entity::PLACEHOLDER);

    if let Some(hit) = spatial_query.cast_ray(
        ray_pos,
        Direction3d::new_unchecked(ray_dir),
        100.,
        true,
        SpatialQueryFilter::default().with_excluded_entities(vec![player_entity]),
    ) {
        hit_result.is_hit = true;
        hit_result.normal = hit.normal;
        // hit_result.entity = hit.entity;
        let dist = hit.time_of_impact;
        hit_result.distance = dist;
        hit_result.position = ray_pos + ray_dir * dist;

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

    let action_state = query_input.single();
    let do_break = action_state.just_pressed(&InputAction::Attack);
    let do_place = action_state.just_pressed(&InputAction::UseItem);

    if hit_result.is_hit && (do_break || do_place) {
        let brush = &*vox_brush;
        let n = brush.size as i32;

        // These code is Horrible

        iter::iter_aabb(n, n, |lp| {
            // +0.01*norm: for placing cube like MC.
            let p = hit_result.voxel_pos + lp + 
                if do_place {1} else {0} * hit_result.normal.normalize_or_zero().as_ivec3();

            if let Some(v) = chunk_sys.get_voxel(p) {
                let v = v.as_ref_mut();
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

                chunk_sys.mark_chunk_remesh(Chunk::as_chunkpos(p)); // CLIS
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

fn draw_crosshair_cube(mut gizmos: Gizmos, hit_result: Res<HitResult>, vbrush: Res<VoxelBrush>,) {

    if hit_result.is_hit {
        if vbrush.shape == VoxShape::Isosurface {
            gizmos.sphere(hit_result.position, Quat::IDENTITY, vbrush.size, Color::BLACK);
        } else {
            let trans = Transform::from_translation(hit_result.voxel_pos.as_vec3() + 0.5)
                .with_scale(Vec3::ONE * vbrush.size.floor());

            gizmos.cuboid(trans, Color::BLACK);
        }
    }
}

fn draw_gizmos(mut gizmos: Gizmos, chunk_sys: Res<ClientChunkSystem>, cli: Res<ClientInfo>, query_cam: Query<&Transform, With<CharacterController>>) {
    // // chunks loading
    // for cp in chunk_sys.chunks_loading.keys() {
    //     gizmos.cuboid(
    //         Transform::from_translation(cp.as_vec3()).with_scale(Vec3::splat(Chunk::SIZE as f32)),
    //         Color::GREEN,
    //     );
    // }

    // all loaded chunks
    if cli.dbg_gizmo_all_loaded_chunks {
        for cp in chunk_sys.get_chunks().keys() {
            gizmos.cuboid(
                Transform::from_translation(cp.as_vec3() + 0.5 * Chunk::SIZE as f32).with_scale(Vec3::splat(Chunk::SIZE as f32)),
                Color::DARK_GRAY,
            );
        }
    }

    if cli.dbg_gizmo_curr_chunk {
        if let Ok(trans) = query_cam.get_single() {
            let cp = Chunk::as_chunkpos(trans.translation.as_ivec3());
            gizmos.cuboid(
                Transform::from_translation(cp.as_vec3() + 0.5 * Chunk::SIZE as f32).with_scale(Vec3::splat(Chunk::SIZE as f32)),
                Color::GRAY,
            );
        }
    }

    if cli.dbg_gizmo_remesh_chunks {
        // chunks remesh
        for cp in chunk_sys.chunks_remesh.iter() {
            gizmos.cuboid(
                Transform::from_translation(cp.as_vec3() + 0.5 * Chunk::SIZE as f32).with_scale(Vec3::splat(Chunk::SIZE as f32)),
                Color::ORANGE,
            );
        }

        // chunks meshing
        for cp in chunk_sys.chunks_meshing.iter() {
            gizmos.cuboid(
                Transform::from_translation(cp.as_vec3() + 0.5 * Chunk::SIZE as f32).with_scale(Vec3::splat(Chunk::SIZE as f32)),
                Color::RED,
            );
        }
    }
}

///////////////////////////////////////////////////
//////////////// ClientChunkSystem ////////////////
///////////////////////////////////////////////////

#[derive(Resource)]
pub struct ClientChunkSystem {
    pub chunks: HashMap<IVec3, ChunkPtr>,

    // mark to ReMesh
    pub chunks_remesh: HashSet<IVec3>,

    pub mtl_terrain: Handle<TerrainMaterial>,
    pub mtl_foliage: Handle<StandardMaterial>,
    pub entity: Entity,

    pub max_concurrent_meshing: usize,
    pub chunks_meshing: HashSet<IVec3>,
    // pub chunks_load_distance: IVec2, // not real, but send to server,
}

impl ChunkSystem for ClientChunkSystem {
    fn get_chunks(&self) -> &HashMap<IVec3, ChunkPtr> {
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
            entity: Entity::PLACEHOLDER,

            max_concurrent_meshing: 8,
            chunks_meshing: HashSet::default(),
        }
    }

    pub fn mark_chunk_remesh(&mut self, chunkpos: IVec3) {
        self.chunks_remesh.insert(chunkpos);
    }

    pub fn spawn_chunk(&mut self, mut chunk: Chunk, cmds: &mut Commands, meshes: &mut Assets<Mesh>) {
        let chunkpos = chunk.chunkpos;

        let aabb = bevy::render::primitives::Aabb::from_min_max(Vec3::ZERO, Vec3::ONE * (Chunk::SIZE as f32));

        chunk.mesh_handle = meshes.add(Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::MAIN_WORLD));
        chunk.mesh_handle_foliage = meshes.add(Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::MAIN_WORLD));

        chunk.entity = cmds
            .spawn((
                // ChunkComponent::new(*chunkpos),
                MaterialMeshBundle {
                    mesh: chunk.mesh_handle.clone(),
                    material: self.mtl_terrain.clone(), //materials.add(Color::rgb(0.8, 0.7, 0.6)),
                    transform: Transform::from_translation(chunkpos.as_vec3()),
                    visibility: Visibility::Hidden, // Hidden is required since Mesh is empty. or WGPU will crash. even if use default Inherite
                    ..default()
                },
                aabb,
                bevy_xpbd_3d::components::RigidBody::Static,
            ))
            .with_children(|parent| {
                parent.spawn((
                    MaterialMeshBundle {
                        mesh: chunk.mesh_handle_foliage.clone(),
                        material: self.mtl_foliage.clone(),
                        // visibility: Visibility::Visible, // Hidden is required since Mesh is empty. or WGPU will crash
                        ..default()
                    },
                    aabb,
                ));
            })
            .set_parent(self.entity)
            .id();

        let chunkptr = Arc::new(chunk);

        let chunkpos;
        {
            let chunk = chunkptr.as_ref_mut();
            chunkpos = chunk.chunkpos;

            let mut neighbors_nearby_completed = Vec::new();

            for neib_idx in 0..Chunk::NEIGHBOR_DIR.len() {
                let neib_dir = Chunk::NEIGHBOR_DIR[neib_idx];
                let neib_chunkpos = chunkpos + neib_dir * Chunk::SIZE;

                // todo: delay remesh or only remesh full-neighbor complete chunks

                // set neighbor_chunks cache
                chunk.neighbor_chunks[neib_idx] = {
                    if let Some(neib_chunkptr) = self.get_chunk(neib_chunkpos) {
                        let neib_chunk = neib_chunkptr.as_ref_mut();

                        // update neighbor's `neighbor_chunk`
                        neib_chunk.neighbor_chunks[Chunk::neighbor_idx_opposite(neib_idx)] = Some(Arc::downgrade(&chunkptr));

                        if neib_chunk.is_neighbors_complete() {
                            neighbors_nearby_completed.push(neib_chunk.chunkpos);
                        }

                        Some(Arc::downgrade(neib_chunkptr))
                    } else {
                        None
                    }
                }
            }

            // if chunk.is_neighbors_complete() {
            self.mark_chunk_remesh(chunkpos);
            // }
            for cp in neighbors_nearby_completed {
                self.mark_chunk_remesh(cp);
            }
        }

        self.chunks.insert(chunkpos, chunkptr);

        // // There is no need to cast shadows for chunks below the surface.
        // if chunkpos.y <= 64 {
        //     entity_commands.insert(NotShadowCaster);
        // }

        // self.set_chunk_meshing(chunkpos, ChunkMeshingState::Pending);
    }

    pub fn despawn_chunk(&mut self, chunkpos: IVec3) -> Option<ChunkPtr> {
        let chunk = self.chunks.remove(&chunkpos)?;

        //cmds.entity(chunk.entity).despawn_recursive();

        Some(chunk)
    }
}

////////////////////////////////////////
//////////////// Render ////////////////
////////////////////////////////////////

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

    // Web requires 16x bytes data. (As the device does not support `DownlevelFlags::BUFFER_BINDINGS_NOT_16_BYTE_ALIGNED`)
    #[uniform(4)]
    pub wasm0: Vec4,
    // pub sample_scale: f32,
    // #[uniform(5)]
    // pub normal_intensity: f32,
    // #[uniform(6)]
    // pub triplanar_blend_pow: f32,
    // #[uniform(7)]
    // pub heightmap_blend_pow: f32, // littler=mix, greater=distinct, opt 0.3 - 0.6, 0.48 = nature
}

impl Default for TerrainMaterial {
    fn default() -> Self {
        Self {
            texture_diffuse: None,
            texture_normal: None,
            texture_dram: None,

            wasm0: Vec4::new(1.0, 1.0, 4.5, 0.48),
            // sample_scale: 1.0,
            // normal_intensity: 1.0,s
            // triplanar_blend_pow: 4.5,
            // heightmap_blend_pow: 0.48,
        }
    }
}

impl Material for TerrainMaterial {
    fn vertex_shader() -> bevy::render::render_resource::ShaderRef {
        "shaders/terrain.wgsl".into()
    }
    fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
        "shaders/terrain.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }
}
