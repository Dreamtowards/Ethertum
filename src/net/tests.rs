use bevy::prelude::*;
use bevy_renet::{
    renet::{ConnectionConfig, DefaultChannel, RenetClient},
    RenetServer,
};

use super::{packet::CellData, validation::*, CPacket, EntityId, SPacket, PROTOCOL_ID};
use crate::{
    server::prelude::ServerInfo,
    voxel::{Chunk, VoxShape},
};

fn bytes<T: serde::Serialize>(packet: &T) -> Vec<u8> {
    bincode::serialize(packet).unwrap()
}

#[test]
fn client_transport_rejects_oversized_user_data_without_panicking() {
    let result = super::new_netcode_client_transport(
        "127.0.0.1:4060".parse().unwrap(),
        Some(vec![0; super::NETCODE_USER_DATA_BYTES + 1]),
    );
    let Err(error) = result else { panic!("oversized user data unexpectedly accepted"); };
    assert!(error.to_string().contains("Connection user data exceeds"));
}

#[test]
fn decode_rejects_corruption_and_length_attacks_with_legacy_wire_format() {
    assert!(matches!(decode_client(&bytes(&CPacket::PlayerList)), Ok(CPacket::PlayerList)));
    for bad in [vec![], vec![255; 4], vec![0; MAX_PACKET_BYTES + 1]] {
        assert!(decode_client(&bad).is_err());
        assert!(decode_server(&bad).is_err());
    }
    let valid = bytes(&CPacket::ChatMessage { message: "你好".into() });
    for len in 0..valid.len() {
        assert!(decode_client(&valid[..len]).is_err());
    }
    let mut trailing = valid.clone();
    trailing.push(0);
    assert!(decode_client(&trailing).is_err());
    let mut forged = valid;
    forged[4..12].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(decode_client(&forged).is_err());
    let mut forged = bytes(&SPacket::ChunkNew {
        chunkpos: IVec3::ZERO,
        voxel: vec![],
    });
    forged[16..24].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(decode_server(&forged).is_err());
}

#[test]
fn validate_bounds_before_world_mutation() {
    for position in [Vec3::splat(f32::NAN), Vec3::splat(f32::INFINITY), Vec3::splat(f32::MAX)] {
        assert!(decode_client(&bytes(&CPacket::PlayerPos { position })).is_err());
        assert!(decode_server(&bytes(&SPacket::EntityPos {
            entity_id: EntityId(1),
            position
        }))
        .is_err());
    }
    assert!(decode_client(&bytes(&CPacket::PlayerPos {
        position: vec3(-16.5, 0.0, 16.0)
    }))
    .is_ok());
    for load_distance in [ivec2(i32::MAX, 1), ivec2(-2, 0), ivec2(1, -1), ivec2(17, 3), ivec2(4, 9)] {
        assert!(decode_client(&bytes(&CPacket::LoadDistance { load_distance })).is_err());
    }
    for load_distance in [ivec2(4, 3), IVec2::splat(-1), ivec2(16, 8)] {
        assert!(decode_client(&bytes(&CPacket::LoadDistance { load_distance })).is_ok());
    }
    for username in ["".into(), " ".into(), "x\n".into(), "x".repeat(65)] {
        assert!(decode_client(&bytes(&CPacket::Login {
            uuid: 1,
            access_token: 2,
            username
        }))
        .is_err());
    }
    assert!(decode_client(&bytes(&CPacket::ChatMessage { message: "x".repeat(1025) })).is_err());
    for daytime in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
        assert!(decode_server(&bytes(&SPacket::WorldTime { daytime })).is_err());
    }
}

#[test]
fn chunk_indices_are_checked_and_full_negative_chunk_still_decodes() {
    let cell = CellData {
        local_idx: 0,
        tex_id: 1,
        shape_id: VoxShape::Cube,
        isoval: 128,
    };
    let cells: Vec<_> = (0..Chunk::LEN3).map(|i| CellData { local_idx: i as u16, ..cell }).collect();
    assert!(decode_server(&bytes(&SPacket::ChunkNew {
        chunkpos: ivec3(-16, -32, 0),
        voxel: cells
    }))
    .is_ok());
    for (chunkpos, voxel) in [
        (ivec3(1, 0, 0), vec![cell]),
        (IVec3::ZERO, vec![CellData { local_idx: 4096, ..cell }]),
        (IVec3::ZERO, vec![cell; Chunk::LEN3 + 1]),
        (IVec3::splat(i32::MIN), vec![cell]),
    ] {
        assert!(decode_client(&bytes(&CPacket::ChunkModify {
            chunkpos,
            voxel: voxel.clone()
        }))
        .is_err());
        assert!(decode_server(&bytes(&SPacket::ChunkModify { chunkpos, voxel })).is_err());
    }
}

const BAD_COMMANDS: &[&str] = &[
    "/",
    "/   ",
    "/time",
    "/time set",
    "/time set abc",
    "/time set NaN",
    "/time set inf",
    "/time set -1",
    "/time set 2",
    "/time set 0.5 extra",
    "/time set \"",
    "/unknown",
];

#[test]
fn commands_require_complete_arguments_and_finite_time() {
    for command in BAD_COMMANDS {
        assert!(parse_command(command).is_err(), "{command}");
    }
    for value in ["0", "0.5", "1", "\"0.5\""] {
        assert!(matches!(parse_command(&format!("/time set {value}")), Ok(ChatCommand::SetTime(_))));
    }
}

fn server_fixture() -> (App, RenetClient) {
    let mut app = App::new();
    let mut server = RenetServer::new(ConnectionConfig::default());
    let client = server.new_local_client(1);
    app.insert_resource(server)
        .init_resource::<ServerInfo>()
        .add_systems(Update, super::netproc_server::server_sys);
    (app, client)
}

fn deliver(app: &mut App, client: &mut RenetClient, data: Vec<u8>) {
    client.send_message(DefaultChannel::ReliableOrdered, data);
    app.world_mut().resource_mut::<RenetServer>().process_local_client(1, client).unwrap();
    app.update();
}

fn login(app: &mut App, client: &mut RenetClient) {
    deliver(
        app,
        client,
        bytes(&CPacket::Login {
            uuid: 1,
            access_token: 2,
            username: "Tester".into(),
        }),
    );
}

// Exercise the same Renet event emitter and observer used in the game, without
// installing a socket transport or changing the existing parser-only fixtures.
fn lifecycle_fixture() -> (App, RenetClient) {
    let (mut app, client) = server_fixture();
    app.add_observer(super::netproc_server::server_event_observer)
        .add_systems(PreUpdate, bevy_renet::RenetServerPlugin::emit_server_events_system);
    (app, client)
}

#[test]
fn repeated_server_login_and_disconnect_releases_owned_entities() {
    let (mut app, mut client) = lifecycle_fixture();
    let baseline = app.world_mut().query::<&Transform>().iter(app.world()).count();
    let mut previous = None;
    for _ in 0..16 {
        login(&mut app, &mut client);
        let player = &app.world().resource::<ServerInfo>().online_players[&1];
        let entity = player.server_entity;
        assert_ne!(Some(entity), previous, "reconnect must use a new full ECS handle");
        assert!(app.world().get::<Transform>(entity).is_some());
        assert_eq!(EntityId::from_server(entity), player.entity_id);
        assert_eq!(app.world_mut().query::<&Transform>().iter(app.world()).count(), baseline + 1);

        app.world_mut().resource_mut::<RenetServer>().disconnect_local_client(1, &mut client);
        app.update();
        assert!(app.world().resource::<ServerInfo>().online_players.is_empty());
        assert!(app.world().get_entity(entity).is_err());
        assert_eq!(app.world_mut().query::<&Transform>().iter(app.world()).count(), baseline);
        previous = Some(entity);
        client = app.world_mut().resource_mut::<RenetServer>().new_local_client(1);
    }
}

#[test]
fn disconnect_preserves_other_player_and_broadcasts_delete_only_once() {
    use bevy_renet::{renet::{DisconnectReason, ServerEvent}, RenetServerEvent};

    let (mut app, mut first) = lifecycle_fixture();
    login(&mut app, &mut first);
    let departing = app.world().resource::<ServerInfo>().online_players[&1].server_entity;
    let departing_id = app.world().resource::<ServerInfo>().online_players[&1].entity_id;
    let mut second = app.world_mut().resource_mut::<RenetServer>().new_local_client(2);
    second.send_message(DefaultChannel::ReliableOrdered, bytes(&CPacket::Login {
        uuid: 2, access_token: 0, username: "Other".into(),
    }));
    app.world_mut().resource_mut::<RenetServer>().process_local_client(2, &mut second).unwrap();
    app.update();
    let survivor = app.world().resource::<ServerInfo>().online_players[&2].server_entity;
    app.world_mut().resource_mut::<RenetServer>().process_local_client(2, &mut second).unwrap();
    while second.receive_message(DefaultChannel::ReliableOrdered).is_some() {}

    app.world_mut().resource_mut::<RenetServer>().disconnect_local_client(1, &mut first);
    app.update();
    // Deliberately replay a disconnect notification after cleanup.
    app.world_mut().trigger(RenetServerEvent(ServerEvent::ClientDisconnected {
        client_id: 1, reason: DisconnectReason::Transport,
    }));
    app.update();
    assert!(app.world().get_entity(departing).is_err());
    assert!(app.world().get::<Transform>(survivor).is_some());
    let players = &app.world().resource::<ServerInfo>().online_players;
    assert_eq!(players.len(), 1);
    assert_eq!(players[&2].server_entity, survivor);

    app.world_mut().resource_mut::<RenetServer>().process_local_client(2, &mut second).unwrap();
    let mut deletes = 0;
    let mut departures = 0;
    while let Some(data) = second.receive_message(DefaultChannel::ReliableOrdered) {
        match decode_server(&data).unwrap() {
            SPacket::EntityDel { entity_id } if entity_id == departing_id => deletes += 1,
            SPacket::Chat { message } if message.contains("Tester left.") => departures += 1,
            _ => {}
        }
    }
    assert_eq!(deletes, 1);
    assert_eq!(departures, 1);
}

#[test]
fn disconnect_tolerates_an_entity_already_removed_elsewhere() {
    let (mut app, mut client) = lifecycle_fixture();
    login(&mut app, &mut client);
    let entity = app.world().resource::<ServerInfo>().online_players[&1].server_entity;
    assert!(app.world_mut().despawn(entity));
    let unrelated = app.world_mut().spawn(Transform::default()).id();
    app.world_mut().resource_mut::<RenetServer>().disconnect_local_client(1, &mut client);
    app.update();
    assert!(app.world().resource::<ServerInfo>().online_players.is_empty());
    assert!(app.world().get::<Transform>(unrelated).is_some());
}

#[test]
fn rejected_packet_then_transport_removal_releases_server_entity() {
    let (mut app, mut client) = lifecycle_fixture();
    login(&mut app, &mut client);
    let entity = app.world().resource::<ServerInfo>().online_players[&1].server_entity;
    deliver(&mut app, &mut client, vec![255; 4]);
    assert!(app.world().resource::<RenetServer>().disconnect_reason(1).is_some());
    // Netcode removes a rejected connection and queues its disconnect event.
    app.world_mut().resource_mut::<RenetServer>().remove_connection(1);
    app.update();
    assert!(app.world().resource::<ServerInfo>().online_players.is_empty());
    assert!(app.world().get_entity(entity).is_err());
}

#[test]
fn server_survives_bad_commands_and_processes_next_valid_message() {
    let (mut app, mut client) = server_fixture();
    login(&mut app, &mut client);
    for command in BAD_COMMANDS {
        deliver(&mut app, &mut client, bytes(&CPacket::ChatMessage { message: (*command).into() }));
        app.world_mut()
            .resource_mut::<RenetServer>()
            .process_local_client(1, &mut client)
            .unwrap();
        let mut saw_error = false;
        while let Some(data) = client.receive_message(DefaultChannel::ReliableOrdered) {
            if matches!(decode_server(&data), Ok(SPacket::Chat { message }) if !message.starts_with("Player ")) {
                saw_error = true;
            }
        }
        assert!(saw_error, "{command}");
    }
    deliver(
        &mut app,
        &mut client,
        bytes(&CPacket::ChatMessage {
            message: "/time set 0.5".into(),
        }),
    );
    app.world_mut()
        .resource_mut::<RenetServer>()
        .process_local_client(1, &mut client)
        .unwrap();
    assert!(
        matches!(decode_server(&client.receive_message(DefaultChannel::ReliableOrdered).unwrap()), Ok(SPacket::WorldTime { daytime }) if daytime == 0.5)
    );
    deliver(&mut app, &mut client, bytes(&CPacket::PlayerPos { position: Vec3::ONE }));
    assert_eq!(app.world().resource::<ServerInfo>().online_players[&1].position, Vec3::ONE);
}

#[test]
fn server_rejects_malformed_client_but_keeps_serving_other_clients() {
    let (mut app, mut attacker) = server_fixture();
    let mut good = app.world_mut().resource_mut::<RenetServer>().new_local_client(2);
    deliver(&mut app, &mut attacker, vec![255; 4]);
    assert!(app.world().resource::<RenetServer>().disconnect_reason(1).is_some());
    good.send_message(
        DefaultChannel::ReliableOrdered,
        bytes(&CPacket::Login {
            uuid: 2,
            access_token: 3,
            username: "Good".into(),
        }),
    );
    app.world_mut().resource_mut::<RenetServer>().process_local_client(2, &mut good).unwrap();
    app.update();
    assert_eq!(app.world().resource::<ServerInfo>().online_players[&2].username, "Good");
}

#[test]
fn server_rejects_prelogin_play_duplicate_login_and_wrong_protocol() {
    let (mut app, mut client) = server_fixture();
    // 同一消息批次里的 Login 也不能绕过此前的拒绝。
    client.send_message(DefaultChannel::ReliableOrdered, bytes(&CPacket::PlayerPos { position: Vec3::ONE }));
    login(&mut app, &mut client);
    app.update();
    assert!(app.world().resource::<ServerInfo>().online_players.is_empty());
    let (mut app, mut client) = server_fixture();
    login(&mut app, &mut client);
    let entity = app.world().resource::<ServerInfo>().online_players[&1].entity_id;
    login(&mut app, &mut client);
    assert_eq!(app.world().resource::<ServerInfo>().online_players[&1].entity_id, entity);
    assert!(app.world().resource::<RenetServer>().disconnect_reason(1).is_some());
    let (mut app, mut client) = server_fixture();
    client.send_message(
        DefaultChannel::ReliableOrdered,
        bytes(&CPacket::Handshake {
            protocol_version: PROTOCOL_ID + 1,
        }),
    );
    login(&mut app, &mut client);
    app.update();
    assert!(app.world().resource::<ServerInfo>().online_players.is_empty());
}
