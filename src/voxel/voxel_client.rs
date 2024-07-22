use bevy::{
    asset::ReflectAsset, color::palettes::css, pbr::{ExtendedMaterial, MaterialExtension}, prelude::*, render::{
        render_asset::RenderAssetUsages,
        render_resource::{AsBindGroup, PrimitiveTopology, ShaderRef}, texture::{ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    }, tasks::AsyncComputeTaskPool, utils::{HashMap, HashSet}
};
use avian3d::prelude::*;
use leafwing_input_manager::action_state::ActionState;

use super::{meshgen, ChannelRx, ChannelTx, Chunk, ChunkPtr, ChunkSystem, VoxShape};
use crate::{
    client::prelude::*,
    util::{as_mut, iter, AsMutRef},
};

pub struct ClientVoxelPlugin;

impl Plugin for ClientVoxelPlugin {
    fn build(&self, app: &mut App) {
        // Render Shader.
        app.add_plugins(MaterialPlugin::<TerrainMaterial>::default());
        app.register_asset_reflect::<TerrainMaterial>(); // debug
        app.add_plugins(MaterialPlugin::<ExtendedMaterial<StandardMaterial, FoliageMaterial>>::default());
        app.register_asset_reflect::<ExtendedMaterial<StandardMaterial, FoliageMaterial>>(); // debug
        app.add_plugins(MaterialPlugin::<ExtendedMaterial<StandardMaterial, Water>>::default());

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

        // app.add_systems(PreUpdate, raycast.run_if(condition::in_world));

        app.add_systems(
            Update,
            (
                raycast,
                chunks_detect_load_and_unload,
                chunks_remesh_enqueue,
                draw_gizmos,
                draw_crosshair_cube.in_set(PhysicsSet::Sync),
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
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
    mut foliage_mtls: ResMut<Assets<ExtendedMaterial<StandardMaterial, FoliageMaterial>>>,
    mut mtls_liquid: ResMut<Assets<ExtendedMaterial<StandardMaterial, Water>>>,
    mut mtls_standard: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    info!("Init ClientChunkSystem");
    let mut chunk_sys = ClientChunkSystem::new();

    chunk_sys.mtl_terrain = terrain_materials.add(TerrainMaterial {
        texture_diffuse: Some(asset_server.load("baked/atlas_diff.png")),
        texture_normal: Some(asset_server.load("baked/atlas_norm.png")),
        texture_dram: Some(asset_server.load("baked/atlas_dram.png")),
        ..default()
    });

    // chunk_sys.mtl_foliage = std_mtls.add(StandardMaterial {
    //     base_color_texture: Some(asset_server.load("baked/atlas_diff_foli.png")),
    //     // normal_map_texture: if has_norm {Some(asset_server.load(format!("models/{name}/norm.png")))} else {None},
    //     double_sided: true,
    //     alpha_mode: AlphaMode::Mask(0.5),
    //     cull_mode: None,
    //     unlit: true,
    //     ..default()
    // });
    chunk_sys.mtl_std = mtls_standard.add(StandardMaterial {
        ..default()
    });

    chunk_sys.mtl_foliage = foliage_mtls.add(ExtendedMaterial {
        base: StandardMaterial {
            base_color: css::BLACK.into(),
            base_color_texture: Some(asset_server.load("baked/atlas_diff_foli.png")),
            perceptual_roughness: 0.0,
            ..default()
        },
        extension: FoliageMaterial {
            ..default()
        }
    });

    chunk_sys.mtl_liquid = mtls_liquid.add(ExtendedMaterial {
        base: StandardMaterial {
            base_color: css::BLACK.into(),
            perceptual_roughness: 0.0,
            ..default()
        },
        extension: Water {
            normals: asset_server.load_with_settings::<Image, ImageLoaderSettings>(
                "water_normals.png",
                |settings| {
                    settings.is_srgb = false;
                    settings.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                        address_mode_u: ImageAddressMode::Repeat,
                        address_mode_v: ImageAddressMode::Repeat,
                        mag_filter: ImageFilterMode::Linear,
                        min_filter: ImageFilterMode::Linear,
                        ..default()
                    });
                },
            ),
        },
    });

    
    // cmds.spawn(MaterialMeshBundle {
    //     mesh: meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(1.0))),
    //     material: mtls_liquid.add(ExtendedMaterial {
    //         base: StandardMaterial {
    //             base_color: css::BLACK.into(),
    //             perceptual_roughness: 0.0,
    //             ..default()
    //         },
    //         extension: Water {
    //             normals: asset_server.load_with_settings::<Image, ImageLoaderSettings>(
    //                 "water_normals.png",
    //                 |settings| {
    //                     settings.is_srgb = false;
    //                     settings.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
    //                         address_mode_u: ImageAddressMode::Repeat,
    //                         address_mode_v: ImageAddressMode::Repeat,
    //                         mag_filter: ImageFilterMode::Linear,
    //                         min_filter: ImageFilterMode::Linear,
    //                         ..default()
    //                     });
    //                 },
    //             ),
    //         },
    //     }),
    //     transform: Transform::from_scale(Vec3::splat(100.0)).with_translation(Vec3::Y * 10.),
    //     ..default()
    // });

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
        let chunkpos = rp * Chunk::LEN + vp;

        // the chunk already exists, skip.
        if chunk_sys.has_chunk(chunkpos) || chunks_loading.contains(&chunkpos) {
            return;
        }

        let tx = tx_chunk_load.clone();
        let task = AsyncComputeTaskPool::get().spawn(async move {
            // info!("Load Chunk: {:?}", chunkpos);
            let mut chunk = Chunk::new(chunkpos);

            super::worldgen::generate_chunk(&mut chunk);

            tx.send(chunk).unwrap();
        });
        task.detach();
        chunks_loading.insert(chunkpos);
    });

    while let Ok(chunk) = rx_chunk_load.try_recv() {
        chunks_loading.remove(&chunk.chunkpos);

        chunk_sys.spawn_chunk(chunk, &mut cmds, &mut meshes);
    }

    // Chunks Unload

    let chunkpos_all = Vec::from_iter(chunk_sys.get_chunks().keys().cloned());
    for chunkpos in chunkpos_all {
        if !crate::voxel::is_chunk_in_load_distance(vp, chunkpos, vd) {
            chunk_sys.despawn_chunk(chunkpos, &mut cmds);
        }
    }
}

type ChunkRemeshData = (IVec3, Entity, Mesh, Handle<Mesh>, Option<Collider>, Mesh, Handle<Mesh>, Mesh, Handle<Mesh>);

use once_cell::sync::Lazy;
use std::{cell::RefCell, sync::Arc};
use thread_local::ThreadLocal;
use crate::util::vtx::VertexBuffer;

static THREAD_LOCAL_VERTEX_BUFFERS: Lazy<ThreadLocal<RefCell<(VertexBuffer, VertexBuffer, VertexBuffer)>>> = Lazy::new(ThreadLocal::default);

fn chunks_remesh_enqueue(
    mut commands: Commands,

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
    let cam_cp = Chunk::as_chunkpos(query_cam.single().translation.as_ivec3());
    chunks_remesh.sort_unstable_by_key(|cp: &IVec3| bevy::math::FloatOrd(cp.distance_squared(cam_cp) as f32));

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
                    .get_or(|| RefCell::new((VertexBuffer::default(), VertexBuffer::default(), VertexBuffer::default())))
                    .borrow_mut();
                // 0: vbuf_terrain, 1: vbuf_foliage, 2: vbuf_liquid

                // let dbg_time = Instant::now();
                let entity;
                let mesh_handle_terrain;
                let mesh_handle_foliage;
                let mesh_handle_liquid;
                {
                    let chunk = chunkptr.as_ref();

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

                tx.send((chunkpos, entity, mesh_terrain, mesh_handle_terrain, collider, mesh_foliage, mesh_handle_foliage, mesh_liquid, mesh_handle_liquid))
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

    while let Ok((chunkpos, entity, mesh_terrain, mesh_handle_terrain, collider, mesh_foliage, mesh_handle_foliage, mesh_liquid, mesh_handle_liquid)) = rx_chunks_meshing.try_recv() {
        // Update Mesh Asset
        *meshes.get_mut(mesh_handle_terrain.id()).unwrap() = mesh_terrain;

        *meshes.get_mut(mesh_handle_foliage.id()).unwrap() = mesh_foliage;

        *meshes.get_mut(mesh_handle_liquid.id()).unwrap() = mesh_liquid;

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
        ray_dir,
        100.,
        true,
        SpatialQueryFilter::default().with_excluded_entities(vec![player_entity]),
    ) {
        hit_result.is_hit = true;
        hit_result.normal = hit.normal;
        // hit_result.entity = hit.entity;
        let dist = hit.time_of_impact;
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

    let action_state = query_input.single();
    let do_break = action_state.just_pressed(&InputAction::Attack);
    let do_place = action_state.just_pressed(&InputAction::UseItem);

    if hit_result.is_hit && (do_break || do_place) {
        let brush = &*vox_brush;
        let n = brush.size as i32;

        // These code is Horrible

        iter::iter_aabb(n, n, |lp| {
            // +0.01*norm: for placing cube like MC.
            let p = hit_result.voxel_pos + lp + if do_place { 1 } else { 0 } * hit_result.normal.normalize_or_zero().as_ivec3();

            if let Some(v) = chunk_sys.get_voxel(p) {
                let v = v.as_mut();
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

fn draw_crosshair_cube(mut gizmos: Gizmos, hit_result: Res<HitResult>, vbrush: Res<VoxelBrush>) {
    if hit_result.is_hit {
        if vbrush.shape == VoxShape::Isosurface {
            gizmos.sphere(hit_result.position, Quat::IDENTITY, vbrush.size, Color::BLACK);
        } else {
            let trans = Transform::from_translation(hit_result.voxel_pos.as_vec3() + 0.5).with_scale(Vec3::ONE * vbrush.size.floor());

            gizmos.cuboid(trans, Color::BLACK);
        }
    }
}

fn draw_gizmos(mut gizmos: Gizmos, chunk_sys: Res<ClientChunkSystem>, cli: Res<ClientInfo>, query_cam: Query<&Transform, With<CharacterController>>) {
    // // chunks loading
    // for cp in chunk_sys.chunks_loading.keys() {
    //     gizmos.cuboid(
    //         Transform::from_translation(cp.as_vec3()).with_scale(Vec3::splat(Chunk::LEN as f32)),
    //         Color::GREEN,
    //     );
    // }

    // all loaded chunks
    if cli.dbg_gizmo_all_loaded_chunks {
        for cp in chunk_sys.get_chunks().keys() {
            gizmos.cuboid(
                Transform::from_translation(cp.as_vec3() + 0.5 * Chunk::LEN as f32).with_scale(Vec3::splat(Chunk::LEN as f32)),
                Srgba::gray(0.25),
            );
        }
    }

    if cli.dbg_gizmo_curr_chunk {
        if let Ok(trans) = query_cam.get_single() {
            let cp = Chunk::as_chunkpos(trans.translation.as_ivec3());
            gizmos.cuboid(
                Transform::from_translation(cp.as_vec3() + 0.5 * Chunk::LEN as f32).with_scale(Vec3::splat(Chunk::LEN as f32)),
                Srgba::gray(0.7),
            );
        }
    }

    if cli.dbg_gizmo_remesh_chunks {
        // chunks remesh
        for cp in chunk_sys.chunks_remesh.iter() {
            gizmos.cuboid(
                Transform::from_translation(cp.as_vec3() + 0.5 * Chunk::LEN as f32).with_scale(Vec3::splat(Chunk::LEN as f32)),
                css::ORANGE,
            );
        }

        // chunks meshing
        for cp in chunk_sys.chunks_meshing.iter() {
            gizmos.cuboid(
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
    pub chunks: HashMap<IVec3, ChunkPtr>,

    // mark to ReMesh
    pub chunks_remesh: HashSet<IVec3>,

    pub mtl_terrain: Handle<TerrainMaterial>,
    pub mtl_foliage: Handle<ExtendedMaterial<StandardMaterial, FoliageMaterial>>,
    pub mtl_liquid: Handle<ExtendedMaterial<StandardMaterial, Water>>,
    pub mtl_std: Handle<StandardMaterial>,
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
            mtl_liquid: Handle::default(),
            mtl_std: Handle::default(),
            entity: Entity::PLACEHOLDER,

            max_concurrent_meshing: 8,
            chunks_meshing: HashSet::default(),
        }
    }

    pub fn mark_chunk_remesh(&self, chunkpos: IVec3) {
        as_mut(self).chunks_remesh.insert(chunkpos);
    }

    pub fn spawn_chunk(&mut self, mut chunk: Chunk, cmds: &mut Commands, meshes: &mut Assets<Mesh>) {
        let chunkpos = chunk.chunkpos;

        let aabb = bevy::render::primitives::Aabb::from_min_max(Vec3::ZERO, Vec3::ONE * (Chunk::LEN as f32));

        chunk.mesh_handle_terrain = meshes.add(Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::MAIN_WORLD));
        chunk.mesh_handle_foliage = meshes.add(Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::MAIN_WORLD));
        chunk.mesh_handle_liquid  = meshes.add(Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::MAIN_WORLD));

        chunk.entity = cmds
            .spawn((
                // ChunkComponent::new(*chunkpos),
                MaterialMeshBundle {
                    mesh: chunk.mesh_handle_terrain.clone(),
                    material: self.mtl_terrain.clone(), //materials.add(Color::rgb(0.8, 0.7, 0.6)),
                    transform: Transform::from_translation(chunkpos.as_vec3()),
                    visibility: Visibility::Hidden, // Hidden is required since Mesh is empty. or WGPU will crash. even if use default Inherite
                    ..default()
                },
                aabb,
                avian3d::prelude::RigidBody::Static,
            ))
            .with_children(|parent| {
                parent.spawn((
                    MaterialMeshBundle {
                        mesh: chunk.mesh_handle_foliage.clone(),
                        material: self.mtl_foliage.clone(),
                        visibility: Visibility::Visible, // Hidden is required since Mesh is empty. or WGPU will crash
                        ..default()
                    },
                    aabb,
                ));
                parent.spawn((
                    MaterialMeshBundle {
                        mesh: chunk.mesh_handle_liquid.clone(),
                        material: self.mtl_liquid.clone(),
                        visibility: Visibility::Visible, 
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
            let chunk = chunkptr.as_mut();
            chunkpos = chunk.chunkpos;
            chunk.chunkptr_weak = Arc::downgrade(&chunkptr);

            // let mut neighbors_completed = Vec::new();

            for neib_idx in 0..Chunk::NEIGHBOR_DIR.len() {
                let neib_dir = Chunk::NEIGHBOR_DIR[neib_idx];
                let neib_chunkpos = chunkpos + neib_dir * Chunk::LEN;

                // set neighbor_chunks cache
                chunk.neighbor_chunks[neib_idx] = {
                    if let Some(neib_chunkptr) = self.get_chunk(neib_chunkpos) {
                        let neib_chunk = neib_chunkptr.as_mut();

                        // update neighbor's `neighbor_chunk`
                        neib_chunk.neighbor_chunks[Chunk::neighbor_idx_opposite(neib_idx)] = Some(Arc::downgrade(&chunkptr));

                        if neib_chunk.is_neighbors_all_loaded() && !neib_chunk.is_populated {
                            // neighbors_completed.push(neib_chunk.chunkpos);
                            neib_chunk.is_populated = true;
                            super::worldgen::populate_chunk(neib_chunk); // todo: ChunkGen Thread

                            self.mark_chunk_remesh(neib_chunk.chunkpos);

                            // fixed: chunk border mesh outdated issue due to population update.
                            for (idx, nneib) in neib_chunk.neighbor_chunks.iter().enumerate() {
                                if nneib.is_some() {
                                    self.mark_chunk_remesh(neib_chunk.chunkpos + Chunk::NEIGHBOR_DIR[idx] * Chunk::LEN);
                                }
                            }
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
            // for cp in neighbors_completed {
            //     self.mark_chunk_remesh(cp);
            // }
        }

        self.chunks.insert(chunkpos, chunkptr);

        // // There is no need to cast shadows for chunks below the surface.
        // if chunkpos.y <= 64 {
        //     entity_commands.insert(NotShadowCaster);
        // }
    }

    pub fn despawn_chunk(&mut self, chunkpos: IVec3, cmds: &mut Commands) -> Option<ChunkPtr> {
        let chunk = self.chunks.remove(&chunkpos)?;

        // update neighbors' `neighbors_chunk`
        for neib_idx in 0..Chunk::NEIGHBOR_DIR.len() {
            if let Some(neib_chunkptr) = chunk.get_chunk_neib(neib_idx) {
                let neib_chunk = neib_chunkptr.as_mut(); // problematic: may cause data tiring

                neib_chunk.neighbor_chunks[Chunk::neighbor_idx_opposite(neib_idx)] = None;
            }
        }

        cmds.entity(chunk.entity).despawn_recursive();

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

            wasm0: Vec4::new(1.5, 1.0, 4.5, 0.48),
            // sample_scale: 1.0,
            // normal_intensity: 1.0,s
            // triplanar_blend_pow: 4.5,
            // heightmap_blend_pow: 0.48,
        }
    }
}

impl Material for TerrainMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/terrain.wgsl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "shaders/terrain.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }
}

// Foliage

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[reflect(Asset)]
pub struct FoliageMaterial {
    
}

impl Default for FoliageMaterial {
    fn default() -> Self {
        Self { 
        }
    }
}

impl MaterialExtension for FoliageMaterial {
    // fn vertex_shader() -> ShaderRef {
    //     "shaders/foliage.wgsl".into()
    // }
    // fn fragment_shader() -> ShaderRef {
    //     "shaders/foliage.wgsl".into()
    // }
    fn deferred_vertex_shader() -> ShaderRef {
        "shaders/foliage.wgsl".into()
    }
    fn deferred_fragment_shader() -> ShaderRef {
        "shaders/foliage.wgsl".into()
    }

    fn specialize(
            _pipeline: &bevy::pbr::MaterialExtensionPipeline,
            descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
            _layout: &bevy::render::mesh::MeshVertexBufferLayoutRef,
            _key: bevy::pbr::MaterialExtensionKey<Self>,
        ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}


#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct Water {
    /// The normal map image.
    ///
    /// Note that, like all normal maps, this must not be loaded as sRGB.
    #[texture(100)]
    #[sampler(101)]
    normals: Handle<Image>,

    
}

impl MaterialExtension for Water {
    fn deferred_fragment_shader() -> ShaderRef {
        "shaders/liquid.wgsl".into()
    }
}