

use bevy::{asset::ReflectAsset, prelude::*, render::render_resource::{AsBindGroup, PrimitiveTopology}, tasks::AsyncComputeTaskPool, utils::{HashMap, HashSet}};
use bevy_renet::renet::RenetClient;
use bevy_xpbd_3d::{components::Collider, plugins::spatial_query::{SpatialQuery, SpatialQueryFilter}};

use crate::{
    character_controller::{CharacterController, CharacterControllerCamera}, game::{condition, ClientInfo, DespawnOnWorldUnload}, net::{CPacket, CellData, RenetClientHelper, SPacket}, ui::CurrentUI, util::iter
};
use super::{material::mtl, meshgen::MeshGen, Chunk, ChunkPtr, ChunkSystem, MpscRx, MpscTx};




pub struct ClientVoxelPlugin;

impl Plugin for ClientVoxelPlugin {
    fn build(&self, app: &mut App) {

        // ClientChunkSystem.
        app.insert_resource(ClientChunkSystem::new());

        // Render Shader.
        app.add_plugins(MaterialPlugin::<TerrainMaterial>::default());
        app.register_asset_reflect::<TerrainMaterial>();  // debug

        {
            let (tx, rx) = crate::channel_impl::unbounded::<ChunkRemeshData>();
            app.insert_resource(MpscTx(tx));
            app.insert_resource(MpscRx(rx));
        }

        // Startup Init
        app.add_systems(First, startup.run_if(condition::load_world()));

        app.insert_resource(HitResult::default());
        app.add_systems(Update,
            (
                raycast,
                chunks_remesh_enqueue,
                draw_gizmos,
            )
            .chain()
            .run_if(condition::in_world()),
        );
    }
}




fn startup(
    mut chunk_sys: ResMut<ClientChunkSystem>,

    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
) {
    info!("Init ChunkSystem");

    chunk_sys.shader_terrain = terrain_materials.add(TerrainMaterial {
        texture_diffuse: Some(asset_server.load("cache/atlas_diff.png")),
        texture_normal: Some(asset_server.load("cache/atlas_norm.png")),
        texture_dram: Some(asset_server.load("cache/atlas_dram.png")),
        ..default()
    });

    // ChunkSystem entity. all chunk entities will be spawn as children. (for almost no reason. just for editor hierarchy)
    chunk_sys.entity = commands
        .spawn((
            Name::new("ChunkSystem"),
            InheritedVisibility::VISIBLE,
            GlobalTransform::IDENTITY,
            Transform::IDENTITY,
            DespawnOnWorldUnload,
        ))
        .id();
}




type ChunkRemeshData = (IVec3, Mesh, Option<Collider>, Entity, Handle<Mesh>);

use once_cell::sync::Lazy;
use thread_local::ThreadLocal;
use std::{cell::RefCell, sync::Arc};
use crate::voxel::meshgen::VertexBuffer;

static THREAD_LOCAL_VERTEX_BUFFERS: Lazy<ThreadLocal<RefCell<VertexBuffer>>> = Lazy::new(ThreadLocal::default);

fn chunks_remesh_enqueue(
    mut commands: Commands,

    query_cam: Query<&Transform, With<CharacterControllerCamera>>,
    mut chunk_sys: ResMut<ClientChunkSystem>,
    mut meshes: ResMut<Assets<Mesh>>,
    // mut query: Query<(Entity, &Handle<Mesh>, &mut ChunkMeshingTask, &ChunkComponent, &mut Visibility)>,

    tx_chunks_meshing: Res<MpscTx<ChunkRemeshData>>,
    rx_chunks_meshing: Res<MpscRx<ChunkRemeshData>>,
) {
    let mut chunks_remesh = Vec::from_iter(chunk_sys.chunks_remesh.iter().cloned());

    // Sort by Distance from the Camera.
    let cam_cp = Chunk::as_chunkpos(query_cam.single().translation.as_ivec3());
    chunks_remesh.sort_unstable_by_key(|cp: &IVec3| bevy::utils::FloatOrd(cp.distance_squared(cam_cp) as f32));

    for chunkpos in chunks_remesh {
        if tx_chunks_meshing.len() >= chunk_sys.max_concurrent_meshing {
            break;
        }

        if let Some(chunkptr) = chunk_sys.get_chunk(chunkpos) {
            let chunkptr = chunkptr.clone();

            let tx = tx_chunks_meshing.clone();

            let task = AsyncComputeTaskPool::get().spawn(async move {
                let mut vbuf = THREAD_LOCAL_VERTEX_BUFFERS.get_or(|| RefCell::new(VertexBuffer::default())).borrow_mut();

                // let dbg_time = Instant::now();
                let entity;
                let mesh_handle;
                {
                    let chunk = chunkptr.read().unwrap();

                    // Generate Mesh
                    MeshGen::generate_chunk_mesh(&mut vbuf, &chunk);

                    entity = chunk.entity;
                    mesh_handle = chunk.mesh_handle.clone();
                }
                // let dbg_time = Instant::now() - dbg_time;

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

                tx.send((chunkpos, mesh, collider, entity, mesh_handle)).unwrap();
            });

            task.detach();
            // chunk_sys.chunks_meshing.insert(chunkpos, ());
        }
        chunk_sys.chunks_remesh.remove(&chunkpos);
    }

    while let Ok((chunkpos, mesh, collider, entity, mesh_handle)) = rx_chunks_meshing.try_recv() {
        // chunk_sys.chunks_meshing.remove(&chunkpos);
        // todo: .remove

        // Update Mesh Asset
        *meshes.get_mut(mesh_handle).unwrap() = mesh;

        // Update Phys Collider TriMesh
        if let Some(collider) = collider {
            if let Some(mut cmds) = commands.get_entity(entity) {
                // the entity may be already unloaded ?
                cmds.remove::<Collider>().insert(collider).insert(Visibility::Visible);
            }
        }
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

    query_cam: Query<&GlobalTransform, With<CharacterControllerCamera>>,  // ray
    query_player: Query<Entity, With<CharacterController>>,  // exclude collider

    mut hit_result: ResMut<HitResult>,

    mouse_btn: Res<Input<MouseButton>>,

    mut chunk_sys: ResMut<ClientChunkSystem>,

    curr_ui: Res<State<CurrentUI>>,

    mut net_client: ResMut<RenetClient>,
) {
    let cam_trans = query_cam.single();
    let ray_pos = cam_trans.translation();
    let ray_dir = cam_trans.forward();

    let player_entity = query_player.single();

    if let Some(hit) = spatial_query.cast_ray(ray_pos, ray_dir, 100.,
        true, SpatialQueryFilter::default().without_entities(vec![player_entity]),
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

    if *curr_ui != CurrentUI::None {  // todo: cli.is_manipulating()
        return;
    }

    let do_break = mouse_btn.just_pressed(MouseButton::Left);
    let do_place = mouse_btn.just_pressed(MouseButton::Right);
    if hit_result.is_hit && (do_break || do_place) {
        let n = 5;

        // These code is Horrible

        let mut map = HashMap::new();
        iter::iter_aabb(n, n, |lp| {
            let p = hit_result.position.as_ivec3() + *lp;
            let chunkpos = Chunk::as_chunkpos(p);

            // chunk_sys.mark_chunk_remesh(Chunk::as_chunkpos(p));

            let pack = map.entry(chunkpos).or_insert_with(|| Vec::new() );

            let chunk = chunk_sys.get_chunk(chunkpos).unwrap().read().unwrap();

            let mut c = *chunk.get_cell(Chunk::as_localpos(p));

            let f = (n as f32 - lp.as_vec3().length()).max(0.);

            c.value += if do_break { -f } else { f };

            if do_place && c.mtl == 0 {
                c.mtl = mtl::STONE;
            }
            
            pack.push(CellData {
                local_idx: Chunk::local_idx(Chunk::as_localpos(p)) as u16,
                density: c.value,
                mtl_id: c.mtl
            });
        });

        info!("Modify terrain sent {}", map.len());
        for e in map {
            net_client.send_packet(&CPacket::ChunkModify { chunkpos: e.0, voxel: e.1 });
        }
    }
}
















fn draw_gizmos(mut gizmos: Gizmos, chunk_sys: Res<ClientChunkSystem>, clientinfo: Res<ClientInfo>) {
    // // chunks loading
    // for cp in chunk_sys.chunks_loading.keys() {
    //     gizmos.cuboid(
    //         Transform::from_translation(cp.as_vec3()).with_scale(Vec3::splat(Chunk::SIZE as f32)),
    //         Color::GREEN,
    //     );
    // }

    // all loaded chunks
    if clientinfo.dbg_gizmo_all_loaded_chunks {
        for cp in chunk_sys.get_chunks().keys() {
            gizmos.cuboid(
                Transform::from_translation(cp.as_vec3()).with_scale(Vec3::splat(Chunk::SIZE as f32)),
                Color::DARK_GRAY,
            );
        }
    }

    // chunks remesh
    for cp in chunk_sys.chunks_remesh.iter() {
        gizmos.cuboid(
            Transform::from_translation(cp.as_vec3()).with_scale(Vec3::splat(Chunk::SIZE as f32)),
            Color::ORANGE,
        );
    }

    // chunks meshing
    // for cp in chunk_sys.chunks_meshing.keys() {
    //     gizmos.cuboid(
    //         Transform::from_translation(cp.as_vec3()).with_scale(Vec3::splat(Chunk::SIZE as f32)),
    //         Color::RED,
    //     );
    // }
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
    pub max_concurrent_meshing: usize,
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
            max_concurrent_meshing: 16,
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

            let mut load = Vec::new();

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
                            load.push(neib_chunk.chunkpos);
                        }
                        

                        Some(Arc::downgrade(neib_chunkptr))
                    } else {
                        None
                    }
                }
            }

            if chunk.is_neighbors_complete() {
                self.mark_chunk_remesh(chunkpos);
            }
            for cp in load {
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
            // normal_intensity: 1.0,
            // triplanar_blend_pow: 4.5,
            // heightmap_blend_pow: 0.48,
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
