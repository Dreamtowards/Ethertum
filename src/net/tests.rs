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
