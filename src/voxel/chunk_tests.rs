use super::*;
use crate::voxel::{Vox, VoxTex};

#[test]
fn floating_world_positions_use_floor_on_every_axis_at_chunk_boundaries() {
    let cases = [
        (-32.1, -48),
        (-16.1, -32),
        (-16.0, -16),
        (-15.9, -16),
        (-0.1, -16),
        (-0.0, 0),
        (0.0, 0),
        (15.9, 0),
        (16.0, 16),
        (32.0, 32),
    ];
    for (x, cx) in cases {
        for (y, cy) in cases {
            for (z, cz) in cases {
                let p = vec3(x, y, z);
                let cp = Chunk::world_to_chunkpos(p);
                assert_eq!(cp, ivec3(cx, cy, cz), "position {p}");
                assert!(Chunk::is_chunkpos(cp));
                assert!(p.cmpge(cp.as_vec3()).all());
                assert!(p.cmplt((cp + IVec3::splat(16)).as_vec3()).all());
            }
        }
    }
}

#[test]
fn negative_fractional_position_agrees_with_local_index_and_load_distance() {
    let p = vec3(-0.1, -16.1, 15.9);
    let cp = Chunk::world_to_chunkpos(p);
    let voxel_pos = p.floor().as_ivec3();
    let local = Chunk::as_localpos(voxel_pos);
    assert_eq!(cp, ivec3(-16, -32, 0));
    assert_eq!(local, IVec3::splat(15));
    assert_eq!(cp + local, voxel_pos);
    assert_eq!(Chunk::local_idx(local), 4095);
    assert_eq!(Chunk::local_idx_pos(4095), local);
    assert!(crate::voxel::is_chunk_in_load_distance(cp, cp, IVec2::ZERO));
    assert!(!crate::voxel::is_chunk_in_load_distance(cp, ivec3(0, -16, 0), IVec2::ZERO));
}

fn insert(world: &mut World, sys: &mut ClientChunkSystem, cp: IVec3) {
    let mut meshes = world.remove_resource::<Assets<Mesh>>().unwrap_or_default();
    sys.spawn_chunk(Chunk::new(cp), &mut world.commands(), &mut meshes);
    world.flush();
    world.insert_resource(meshes);
}

fn fixture() -> (World, ClientChunkSystem) {
    let mut world = World::new();
    let mut sys = ClientChunkSystem::new();
    sys.entity = world.spawn_empty().id();
    (world, sys)
}

fn dispatch(sys: &mut ClientChunkSystem, cp: IVec3) -> MeshTicket {
    let ticket = sys.mesh_ticket(cp);
    sys.chunks_meshing.insert(cp, ticket);
    sys.chunks_remesh.remove(&cp);
    ticket
}

#[test]
fn background_mesh_keeps_center_and_neighbor_data_during_edit_and_unload() {
    let (mut ecs, mut sys) = fixture();
    let cp = ivec3(-16, 0, 0);
    insert(&mut ecs, &mut sys, cp);
    insert(&mut ecs, &mut sys, IVec3::ZERO);
    let p = ivec3(-1, 8, 8);
    *sys.get_voxel_mut(p).unwrap() = Vox::new(1, VoxShape::Cube, 0.0);
    *sys.get_voxel_mut(p + IVec3::X).unwrap() = Vox::new(1, VoxShape::Cube, 0.0);
    let snapshot = sys.snapshot_chunk(cp).unwrap();
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let reader_barrier = barrier.clone();
    let reader = std::thread::spawn(move || {
        reader_barrier.wait();
        let mut vertices = VertexBuffer::default();
        meshgen::generate_chunk_mesh(&mut vertices, &snapshot);
        assert_eq!(snapshot.at_voxel(ivec3(15, 8, 8)).tex_id, 1);
        assert_eq!(snapshot.get_voxel_rel(ivec3(16, 8, 8)).unwrap().tex_id, 1);
        vertices.vertex_count()
    });
    sys.get_voxel_mut(p).unwrap().tex_id = 2;
    sys.despawn_chunk(IVec3::ZERO, &mut ecs.commands());
    ecs.flush();
    barrier.wait();
    assert_eq!(reader.join().unwrap(), 30, "snapshot still occludes the shared cube face");
    let current = sys.snapshot_chunk(cp).unwrap();
    assert_eq!(current.at_voxel(ivec3(15, 8, 8)).tex_id, 2);
    assert!(current.get_voxel_rel(ivec3(16, 8, 8)).is_none());
    let mut vertices = VertexBuffer::default();
    meshgen::generate_chunk_mesh(&mut vertices, &current);
    assert_eq!(vertices.vertex_count(), 36, "new mesh exposes the unloaded neighbor face");
}

#[test]
fn face_edge_corner_and_negative_edits_invalidate_all_mesh_inputs() {
    for (p, count) in [
        (ivec3(0, 8, 8), 2),
        (ivec3(0, 0, 8), 4),
        (IVec3::ZERO, 8),
        (ivec3(-1, -1, -1), 8),
        (ivec3(-8, -8, -8), 1),
    ] {
        let (mut ecs, mut sys) = fixture();
        let dependents = Chunk::mesh_dependents(p);
        assert_eq!(dependents.len(), count);
        assert!(dependents.iter().all(|cp| Chunk::is_chunkpos(*cp)));
        for cp in &dependents {
            insert(&mut ecs, &mut sys, *cp);
        }
        let tickets: Vec<_> = dependents.iter().map(|cp| (*cp, dispatch(&mut sys, *cp))).collect();
        sys.chunks_remesh.clear();
        sys.get_voxel_mut(p).unwrap().tex_id = 7;
        assert_eq!(sys.chunks_remesh.len(), count);
        for (cp, ticket) in tickets {
            assert!(!sys.accept_mesh(cp, ticket), "old mesh survived edit at {p}");
            assert!(sys.chunks_remesh.contains(&cp), "latest revision lost its rebuild request");
        }
    }
}

#[test]
fn unloading_and_replacing_chunks_rejects_old_results_without_clearing_new_tasks() {
    let (mut ecs, mut sys) = fixture();
    let cp = IVec3::ZERO;
    insert(&mut ecs, &mut sys, cp);
    let old_entity = sys.get_chunk(cp).unwrap().entity;
    let old = dispatch(&mut sys, cp);
    sys.despawn_chunk(cp, &mut ecs.commands());
    ecs.flush();
    insert(&mut ecs, &mut sys, cp);
    assert_ne!(sys.get_chunk(cp).unwrap().entity, old_entity);
    let new = dispatch(&mut sys, cp);
    assert!(!sys.accept_mesh(cp, old));
    assert_eq!(sys.chunks_meshing.get(&cp), Some(&new));
    assert!(sys.accept_mesh(cp, new));
    assert!(!sys.accept_mesh(cp, new), "duplicate delivery must not install twice");

    let old = dispatch(&mut sys, cp);
    insert(&mut ecs, &mut sys, cp);
    assert!(!sys.accept_mesh(cp, old), "replacement must invalidate an in-flight mesh");
}

#[test]
fn world_exit_cancels_workers_and_old_epoch_cannot_complete_new_world_tasks() {
    let (mut ecs, mut old_world) = fixture();
    insert(&mut ecs, &mut old_world, IVec3::ZERO);
    let old_ticket = dispatch(&mut old_world, IVec3::ZERO);
    let cancelled = old_world.cancelled.clone();
    let old_epoch = old_world.epoch;
    drop(old_world);
    assert!(cancelled.load(Ordering::Relaxed));
    let (mut ecs, mut new_world) = fixture();
    insert(&mut ecs, &mut new_world, IVec3::ZERO);
    assert_ne!(new_world.epoch, old_epoch);
    let new_ticket = dispatch(&mut new_world, IVec3::ZERO);
    assert!(!new_world.accept_mesh(IVec3::ZERO, old_ticket));
    assert_eq!(new_world.chunks_meshing.get(&IVec3::ZERO), Some(&new_ticket));
    new_world.chunks_loading.insert(IVec3::X * 16, 1);
    assert!(!new_world.accept_load(IVec3::X * 16, old_epoch, 1));
    assert_eq!(new_world.chunks_loading.get(&(IVec3::X * 16)), Some(&1));
}

#[test]
fn late_generation_cannot_replace_received_network_chunk() {
    let (mut ecs, mut sys) = fixture();
    let cp = IVec3::ZERO;
    let epoch = sys.epoch;
    let request = sys.next_revision();
    sys.chunks_loading.insert(cp, request);
    insert(&mut ecs, &mut sys, cp);
    let entity = sys.get_chunk(cp).unwrap().entity;
    assert!(!sys.accept_load(cp, epoch, request));
    assert_eq!(sys.get_chunk(cp).unwrap().entity, entity);
    assert!(!sys.chunks_loading.contains_key(&cp));
}

#[test]
fn population_and_tree_edits_update_owned_neighbors_without_changing_snapshots() {
    let (mut ecs, mut sys) = fixture();
    let cp = IVec3::ZERO;
    insert(&mut ecs, &mut sys, cp);
    insert(&mut ecs, &mut sys, IVec3::X * 16);
    let before = sys.snapshot_chunk(cp).unwrap();
    sys.edit_neighborhood(cp, |chunk| {
        crate::voxel::worldgen::gen_tree(chunk, ivec3(15, 4, 8), 0.0);
    });
    assert_eq!(sys.get_voxel(ivec3(15, 4, 8)).unwrap().tex_id, VoxTex::Log);
    assert_eq!(sys.get_voxel(ivec3(16, 7, 8)).unwrap().tex_id, VoxTex::Leaves);
    assert!(before.at_voxel(ivec3(15, 4, 8)).is_nil());
    assert!(before.get_voxel_rel(ivec3(16, 7, 8)).unwrap().is_nil());
    assert!(sys.get_chunk(cp).unwrap().neighbor_chunks.iter().all(Option::is_none));
    assert!(sys.get_chunk(IVec3::X * 16).unwrap().neighbor_chunks.iter().all(Option::is_none));
}

#[test]
fn neighbor_arrival_invalidates_old_mesh_and_new_snapshot_has_neighbor() {
    let (mut ecs, mut sys) = fixture();
    insert(&mut ecs, &mut sys, IVec3::ZERO);
    let old_snapshot = sys.snapshot_chunk(IVec3::ZERO).unwrap();
    let ticket = dispatch(&mut sys, IVec3::ZERO);
    insert(&mut ecs, &mut sys, IVec3::X * 16);
    assert!(!sys.accept_mesh(IVec3::ZERO, ticket));
    assert!(old_snapshot.get_voxel_rel(ivec3(16, 8, 8)).is_none());
    assert!(sys.snapshot_chunk(IVec3::ZERO).unwrap().get_voxel_rel(ivec3(16, 8, 8)).is_some());
}

#[test]
fn force_blocky_is_snapshot_local_and_invalidates_inflight_mesh() {
    let (mut ecs, mut sys) = fixture();
    insert(&mut ecs, &mut sys, IVec3::ZERO);
    let snapshot = sys.snapshot_chunk(IVec3::ZERO).unwrap();
    let ticket = dispatch(&mut sys, IVec3::ZERO);
    sys.set_force_blocky(true);
    assert!(!snapshot.force_blocky);
    assert!(sys.snapshot_chunk(IVec3::ZERO).unwrap().force_blocky);
    assert!(!sys.accept_mesh(IVec3::ZERO, ticket));
}

#[test]
fn light_propagation_crosses_owned_chunks_and_skips_missing_neighbors() {
    let (mut ecs, mut sys) = fixture();
    insert(&mut ecs, &mut sys, ivec3(-16, 0, 0));
    insert(&mut ecs, &mut sys, IVec3::ZERO);
    let snapshot = sys.snapshot_chunk(IVec3::ZERO).unwrap();
    let mut queue = vec![(ivec3(-1, 8, 8), crate::voxel::VoxLight::new(0, 4, 0, 0))];
    crate::voxel::lighting::compute_voxel_light(&mut sys, &mut queue);
    assert_eq!(sys.get_voxel(ivec3(0, 8, 8)).unwrap().light.red(), 3);
    assert_eq!(snapshot.at_voxel(ivec3(0, 8, 8)).light.red(), 0);
    assert!(queue.is_empty());
}

#[test]
fn center_is_populated_once_when_all_twenty_six_neighbors_arrive() {
    let (mut ecs, mut sys) = fixture();
    insert(&mut ecs, &mut sys, IVec3::ZERO);
    for dir in &Chunk::NEIGHBOR_DIR[..25] {
        insert(&mut ecs, &mut sys, *dir * Chunk::LEN);
    }
    assert!(!sys.get_chunk(IVec3::ZERO).unwrap().is_populated);
    insert(&mut ecs, &mut sys, Chunk::NEIGHBOR_DIR[25] * Chunk::LEN);
    assert!(sys.get_chunk(IVec3::ZERO).unwrap().is_populated);
    let snapshot = sys.snapshot_chunk(IVec3::ZERO).unwrap();
    assert!(snapshot.is_neighbors_all_loaded());
    let storage = sys.get_chunk(IVec3::ZERO).unwrap().voxel.clone();
    insert(&mut ecs, &mut sys, Chunk::NEIGHBOR_DIR[25] * Chunk::LEN);
    assert!(Arc::ptr_eq(&storage, &sys.get_chunk(IVec3::ZERO).unwrap().voxel));
}

#[test]
fn generated_chunk_and_network_updates_cannot_mutate_shared_snapshot_storage() {
    let cp = ivec3(-16, -16, -16);
    let mut generated = Chunk::new(cp);
    let original = generated.clone();
    crate::voxel::worldgen::generate_chunk(&mut generated);
    assert!(original.at_voxel(IVec3::ZERO).is_nil());
    assert!(!generated.at_voxel(IVec3::ZERO).is_nil());
    let snapshot = generated.clone();
    let data = vec![crate::net::CellData::from_cell(0, &Vox::new(VoxTex::Leaves, VoxShape::Leaves, 0.0))];
    crate::net::CellData::to_chunk(&data, &mut generated);
    assert_ne!(snapshot.at_voxel(IVec3::ZERO).tex_id, VoxTex::Leaves);
    assert_eq!(generated.at_voxel(IVec3::ZERO).tex_id, VoxTex::Leaves);
}
