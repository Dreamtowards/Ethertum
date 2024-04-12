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
use bevy_renet::renet::RenetClient;
use bevy_xpbd_3d::plugins::{
    collision::Collider,
    spatial_query::{SpatialQuery, SpatialQueryFilter},
};

use super::{meshgen::MeshGen, ChannelRx, ChannelTx, Chunk, ChunkPtr, ChunkSystem};
use crate::{
    character_controller::{CharacterController, CharacterControllerCamera},
    game_client::{condition, ClientInfo, DespawnOnWorldUnload},
    net::{CPacket, CellData, RenetClientHelper},
    ui::CurrentUI,
    util::iter,
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
        }

        app.add_systems(First, on_world_init.run_if(condition::load_world));
        app.add_systems(Last, on_world_exit.run_if(condition::unload_world()));

        app.insert_resource(HitResult::default());
        app.add_systems(Update, (raycast, chunks_remesh_enqueue, draw_gizmos).chain().run_if(condition::in_world));
    }
}

fn on_world_init(mut cmds: Commands, asset_server: Res<AssetServer>, mut terrain_materials: ResMut<Assets<TerrainMaterial>>) {
    info!("Init ChunkSystem");

    let mut chunk_sys = ClientChunkSystem::new();

    chunk_sys.shader_terrain = terrain_materials.add(TerrainMaterial {
        texture_diffuse: Some(asset_server.load("baked/atlas_diff.png")),
        texture_normal: Some(asset_server.load("baked/atlas_norm.png")),
        texture_dram: Some(asset_server.load("baked/atlas_dram.png")),
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
    // mut query: Query<(Entity, &Handle<Mesh>, &mut ChunkMeshingTask, &ChunkComponent, &mut Visibility)>,
    mut cli: ResMut<ClientInfo>,
    tx_chunks_meshing: Res<ChannelTx<ChunkRemeshData>>,
    rx_chunks_meshing: Res<ChannelRx<ChunkRemeshData>>,
) {
    let mut chunks_remesh = Vec::from_iter(chunk_sys.chunks_remesh.iter().cloned());

    // Sort by Distance from the Camera.
    let cam_cp = Chunk::as_chunkpos(query_cam.single().translation.as_ivec3());
    chunks_remesh.sort_unstable_by_key(|cp: &IVec3| bevy::utils::FloatOrd(cp.distance_squared(cam_cp) as f32));

    for chunkpos in chunks_remesh {
        if cli.chunks_meshing.len() >= cli.max_concurrent_meshing {
            break;
        }
        if cli.chunks_meshing.contains(&chunkpos) {
            continue;
        }

        if let Some(chunkptr) = chunk_sys.get_chunk(chunkpos) {
            cli.chunks_meshing.insert(chunkpos);

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
                    let chunk = chunkptr.read().unwrap();

                    // Generate Mesh
                    MeshGen::generate_chunk_mesh(&mut _vbuf.0, &chunk);

                    MeshGen::generate_chunk_mesh_foliage(&mut _vbuf.1, &chunk);

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

        cli.chunks_meshing.remove(&chunkpos);
        // info!("[ReMesh Completed] Pos: {}; ReMesh: {}, Meshing: {}: tx: {}, rx: {}", chunkpos, chunk_sys.chunks_remesh.len(), cli.chunks_meshing.len(), tx_chunks_meshing.len(), rx_chunks_meshing.len());
    }
}

// separated from .. due to Parallel Excution
// fn chunks_remesh_dequeue() {

// }

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
    query_cam: Query<&GlobalTransform, With<CharacterControllerCamera>>, // ray
    query_player: Query<Entity, With<CharacterController>>,              // exclude collider

    mut hit_result: ResMut<HitResult>,
    mouse_btn: Res<ButtonInput<MouseButton>>,
    chunk_sys: ResMut<ClientChunkSystem>,
    mut net_client: ResMut<RenetClient>,
    cli: Res<ClientInfo>,
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
    } else {
        hit_result.is_hit = false;
    }

    // ############ Break & Place ############

    if cli.curr_ui != CurrentUI::None {
        // todo: cli.is_manipulating()
        return;
    }

    let do_break = mouse_btn.just_pressed(MouseButton::Left);
    let do_place = mouse_btn.just_pressed(MouseButton::Right);
    if hit_result.is_hit && (do_break || do_place) {
        let n = cli.brush_size as i32;

        // These code is Horrible

        let mut map = HashMap::new();
        iter::iter_aabb(n, n, |lp| {
            // +0.01*norm: for placing cube like MC.

            let p = (hit_result.position + if do_place { 1. } else { -1. } * 0.01 * hit_result.normal)
                .floor()
                .as_ivec3()
                + lp;
            let chunkpos = Chunk::as_chunkpos(p);

            // chunk_sys.mark_chunk_remesh(Chunk::as_chunkpos(p));

            let pack = map.entry(chunkpos).or_insert_with(Vec::new);

            let chunk = chunk_sys.get_chunk(chunkpos).unwrap().read().unwrap();

            let mut c = *chunk.get_cell(Chunk::as_localpos(p));

            let f = (n as f32 - lp.as_vec3().length()).max(0.) * cli.brush_strength;

            c.set_isovalue(c.isovalue() + if do_break { -f } else { f });

            if f > 0.0 || (n == 0 && f == 0.0) {
                // placing single
                if do_place {
                    // && c.tex_id == 0 {
                    c.tex_id = cli.brush_tex;
                    c.shape_id = cli.brush_shape;

                    // placing Block
                    if cli.brush_shape != 0 {
                        c.set_isovalue(0.0);
                    }
                } else if c.is_isoval_empty() {
                    c.tex_id = 0;
                }
            }

            pack.push(CellData::from_cell(Chunk::local_idx(Chunk::as_localpos(p)) as u16, &c));
        });

        info!("Modify terrain sent {}", map.len());
        for e in map {
            net_client.send_packet(&CPacket::ChunkModify { chunkpos: e.0, voxel: e.1 });
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
        for cp in cli.chunks_meshing.iter() {
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

    pub shader_terrain: Handle<TerrainMaterial>,
    pub entity: Entity,
}

impl ChunkSystem for ClientChunkSystem {
    fn get_chunks(&self) -> &HashMap<IVec3, ChunkPtr> {
        &self.chunks
    }
}

impl ClientChunkSystem {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::default(),
            chunks_remesh: HashSet::default(),

            shader_terrain: Handle::default(),
            entity: Entity::PLACEHOLDER,
        }
    }

    pub fn mark_chunk_remesh(&mut self, chunkpos: IVec3) {
        self.chunks_remesh.insert(chunkpos);
    }

    pub fn spawn_chunk(&mut self, chunkptr: ChunkPtr) {
        let chunkpos;
        {
            let mut chunk = chunkptr.write().unwrap();
            chunkpos = chunk.chunkpos;

            let mut neighbors_nearby_completed = Vec::new();

            for neib_idx in 0..Chunk::NEIGHBOR_DIR.len() {
                let neib_dir = Chunk::NEIGHBOR_DIR[neib_idx];
                let neib_chunkpos = chunkpos + neib_dir * Chunk::SIZE;

                // todo: delay remesh or only remesh full-neighbor complete chunks

                // set neighbor_chunks cache
                chunk.neighbor_chunks[neib_idx] = {
                    if let Some(neib_chunkptr) = self.get_chunk(neib_chunkpos) {
                        let mut neib_chunk = neib_chunkptr.write().unwrap();

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
