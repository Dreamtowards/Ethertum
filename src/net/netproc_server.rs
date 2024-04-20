use std::time::Duration;

use bevy::{prelude::*, utils::HashSet};
use bevy_renet::{
    renet::{transport::NetcodeServerTransport, ConnectionConfig, DefaultChannel, RenetServer, ServerEvent},
    transport::NetcodeServerPlugin,
    RenetServerPlugin,
};

use crate::{
    net::{packet::CellData, CPacket, EntityId, RenetServerHelper, SPacket, PROTOCOL_ID},
    server::prelude::*,
    util::{current_timestamp_millis, AsRefMut},
    voxel::{ChunkSystem, ServerChunkSystem},
};

pub struct ServerNetworkPlugin;

impl Plugin for ServerNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenetServerPlugin);
        app.add_plugins(NetcodeServerPlugin);

        app.insert_resource(RenetServer::new(ConnectionConfig {
            server_channels_config: super::net_channel_config(20 * 1024 * 1024),
            ..default()
        }));

        app.add_systems(Startup, bind_server_endpoint);
        app.add_systems(Update, server_sys);

        // app.add_systems(Update, ui_server_net);
    }
}

fn bind_server_endpoint(mut cmds: Commands, serv: Res<ServerInfo>) {
    let addr = serv.cfg().addr().parse().unwrap();

    cmds.insert_resource(super::new_netcode_server_transport(addr, 64));
    info!("Server bind endpoint at {}", addr);
}

pub fn server_sys(
    mut server_events: EventReader<ServerEvent>,
    mut server: ResMut<RenetServer>,
    transport: Res<NetcodeServerTransport>,
    mut serverinfo: ResMut<ServerInfo>,
    // mut worldinfo: ResMut<WorldInfo>,
    chunk_sys: ResMut<ServerChunkSystem>,
    mut cmds: Commands,
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                let result_string: String = transport
                    .user_data(*client_id)
                    .unwrap_or([0; bevy_renet::renet::transport::NETCODE_USER_DATA_BYTES])
                    .iter()
                    .map(|&byte| byte as char)
                    .collect();

                info!("Cli Connected {} {}", client_id, result_string);
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                info!("Cli Disconnected {} {}", client_id, reason);

                if let Some(player) = serverinfo.online_players.remove(client_id) {
                    server.broadcast_packet_chat(format!("Player {} left. ({}/N)", player.username, serverinfo.online_players.len()));

                    server.broadcast_packet(&SPacket::EntityDel { entity_id: player.entity_id });
                }
            }
        }
    }

    // Receive message from all clients
    for client_id in server.clients_id() {
        while let Some(bytes) = server.receive_message(client_id, DefaultChannel::ReliableOrdered) {
            // info!("Server Received: {}", String::from_utf8_lossy(&bytes));
            let packet: CPacket = bincode::deserialize(&bytes[..]).unwrap();
            match packet {
                CPacket::Handshake { protocol_version } => {
                    if protocol_version < PROTOCOL_ID {
                        server.send_packet_disconnect(client_id, "Client outdated.".into());
                    } else if protocol_version > PROTOCOL_ID {
                        server.send_packet_disconnect(client_id, "Server outdated.".into());
                    }
                }
                CPacket::ServerQuery {} => {
                    server.send_packet(
                        client_id,
                        &SPacket::ServerInfo {
                            motd: "Motd".into(),
                            num_players_limit: 64,
                            num_players_online: 0,
                            protocol_version: PROTOCOL_ID,
                            favicon: "None".into(),
                        },
                    );
                }
                CPacket::Login {
                    uuid,
                    access_token,
                    username,
                } => {
                    info!("Login Requested: {} {} {}", uuid, access_token, username);

                    if serverinfo.online_players.values().any(|v| &v.username == &username) {
                        server.send_packet_disconnect(client_id, format!("Player {} already logged in", &username));
                        continue;
                    }
                    // 模拟登录验证
                    std::thread::sleep(Duration::from_millis(800));

                    let entity_id = EntityId::from_server(cmds.spawn(TransformBundle::default()).id());

                    // Login Success
                    server.send_packet(client_id, &SPacket::LoginSuccess { player_entity: entity_id });

                    server.broadcast_packet_chat(format!("Player {} joined. ({}/N)", &username, serverinfo.online_players.len() + 1));

                    server.broadcast_packet_except(
                        client_id,
                        &SPacket::EntityNew {
                            entity_id,
                            name: username.clone(),
                        },
                    );

                    // Send Server Players to the client. Note: Before insert of online_players
                    for player in serverinfo.online_players.values() {
                        server.send_packet(
                            client_id,
                            &SPacket::EntityNew {
                                entity_id: player.entity_id,
                                name: player.username.clone(),
                            },
                        );
                        server.send_packet(
                            client_id,
                            &SPacket::EntityPos {
                                entity_id: player.entity_id,
                                position: player.position,
                            },
                        );
                    }

                    serverinfo.online_players.insert(
                        client_id,
                        PlayerInfo {
                            username,
                            user_id: uuid,
                            client_id,
                            entity_id,
                            position: Vec3::ZERO,
                            chunks_loaded: HashSet::default(),
                            chunks_load_distance: IVec2::new(-1, -1), // 4 2
                            ping_rtt: 0,
                        },
                    );
                }
                // Play Stage:
                _ => {
                    // Requires Logged in.
                    // 这几行应该有语法糖简化..
                    let player = serverinfo.online_players.get_mut(&client_id);
                    if player.is_none() {
                        server.send_packet_disconnect(client_id, "illegal play-stage packet. you have not login yet".into());
                        continue;
                    }
                    let player = player.unwrap();

                    match packet {
                        CPacket::ChatMessage { message } => {
                            if message.starts_with('/') {
                                let args = shlex::split(&message[1..]).unwrap();

                                if args[0] == "time" {
                                    if args[1] == "set" {
                                        let daytime = args[2].parse::<f32>().unwrap();
                                        server.broadcast_packet(&SPacket::WorldTime { daytime });
                                    } else {
                                        server.send_packet_chat(client_id, "Current time is ".into());
                                    }
                                }
                                info!("[CMD]: {:?}", args);
                            } else {
                                server.broadcast_packet_chat(format!("<{}>: {}", player.username, message.clone()));
                            }
                        }
                        CPacket::LoadDistance { load_distance } => {
                            player.chunks_load_distance = load_distance;
                        }
                        CPacket::PlayerPos { position } => {
                            // todo: check diff, skip the same

                            player.position = position;

                            server.broadcast_packet_except(
                                client_id,
                                &SPacket::EntityPos {
                                    entity_id: player.entity_id,
                                    position,
                                },
                            );
                        }
                        CPacket::Ping { client_time, last_rtt } => {
                            player.ping_rtt = last_rtt;

                            server.send_packet(
                                client_id,
                                &SPacket::Pong {
                                    client_time,
                                    server_time: current_timestamp_millis(),
                                },
                            );
                        }
                        CPacket::PlayerList => {
                            let playerlist = serverinfo.online_players.iter().map(|e| (e.1.username.clone(), e.1.ping_rtt)).collect();
                            server.send_packet(client_id, &SPacket::PlayerList { playerlist });
                        }
                        CPacket::ChunkModify { chunkpos, voxel } => {
                            // todo: NonLock
                            let chunk = chunk_sys.get_chunk(chunkpos).unwrap();

                            CellData::to_chunk(&voxel, chunk.as_ref_mut());

                            server.broadcast_packet(&SPacket::ChunkModify { chunkpos, voxel });
                        }
                        _ => {
                            warn!("Unknown Packet {:?}", packet);
                        }
                    }
                }
            }
        }
    }
}
