use bevy::{platform::collections::HashSet, prelude::*};
use bevy_renet::{
    netcode::{NetcodeServerPlugin, NetcodeServerTransport},
    renet::{ConnectionConfig, DefaultChannel, ServerEvent},
    RenetServer, RenetServerEvent, RenetServerPlugin,
};

use crate::{
    net::{packet::CellData, CPacket, EntityId, RenetServerHelper, SPacket, PROTOCOL_ID},
    server::prelude::*,
    util::current_timestamp_millis,
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
        app.add_observer(server_event_observer);

        // app.add_systems(Update, ui_server_net);
    }
}

fn bind_server_endpoint(mut cmds: Commands, cfg: Res<ServerSettings>, mut exit: MessageWriter<AppExit>) {
    let result = cfg.validate().and_then(|_| super::new_netcode_server_transport(cfg.port, cfg.num_player_limit as usize));
    match result {
        Ok(transport) => {
            cmds.insert_resource(transport);
            info!("Server bind endpoint at port {} (limit {})", cfg.port, cfg.num_player_limit);
        }
        Err(error) => {
            error!("Server startup failed: {error:#}");
            exit.write(AppExit::error());
        }
    }
}

pub fn server_event_observer(trigger: On<RenetServerEvent>, mut server: ResMut<RenetServer>, mut serverinfo: ResMut<ServerInfo>, mut cmds: Commands) {
    match trigger.0 {
        ServerEvent::ClientConnected { client_id } => {
            info!("Cli Connected {}", client_id);
        }
        ServerEvent::ClientDisconnected { client_id, reason } => {
            info!("Cli Disconnected {} {}", client_id, reason);

            if let Some(player) = serverinfo.online_players.remove(&client_id) {
                // Remove the owned entity as well as its network record. A repeated
                // disconnect or an entity already removed elsewhere is harmless.
                cmds.entity(player.server_entity).try_despawn();
                server.broadcast_packet_chat(format!("Player {} left. ({}/N)", player.username, serverinfo.online_players.len()));

                server.broadcast_packet(&SPacket::EntityDel { entity_id: player.entity_id });
            }
        }
    }
}

pub fn server_sys(
    mut server: ResMut<RenetServer>,

    mut serverinfo: ResMut<ServerInfo>,
    // mut worldinfo: ResMut<WorldInfo>,
    // chunk_sys: ResMut<ServerChunkSystem>,
    mut cmds: Commands,
) {
    // Receive message from all clients
    for client_id in server.clients_id() {
        while let Some(bytes) = server.receive_message(client_id, DefaultChannel::ReliableOrdered) {
            // info!("Server Received: {}", String::from_utf8_lossy(&bytes));
            let packet = match super::validation::decode_client(&bytes) {
                Ok(packet) => packet,
                Err(reason) => {
                    warn!("Rejected packet from client {}: {}", client_id, reason);
                    server.disconnect(client_id);
                    break;
                }
            };
            match packet {
                CPacket::Handshake { protocol_version } => {
                    if protocol_version < PROTOCOL_ID {
                        server.send_packet_disconnect(client_id, "Client outdated.".into());
                        server.disconnect(client_id);
                        break;
                    } else if protocol_version > PROTOCOL_ID {
                        server.send_packet_disconnect(client_id, "Server outdated.".into());
                        server.disconnect(client_id);
                        break;
                    }
                }
                // CPacket::ServerQuery {} => {
                //     server.send_packet(
                //         client_id,
                //         &SPacket::ServerInfo {
                //             motd: "Motd".into(),
                //             num_players_limit: 64,
                //             num_players_online: 0,
                //             protocol_version: PROTOCOL_ID,
                //             favicon: "None".into(),
                //         },
                //     );
                // }
                CPacket::Login {
                    uuid,
                    access_token: _,
                    username,
                } => {
                    info!("Login Requested: {} {}", uuid, username);

                    if serverinfo.online_players.contains_key(&client_id) {
                        server.send_packet_disconnect(client_id, "Already logged in.".into());
                        server.disconnect(client_id);
                        break;
                    }

                    if serverinfo.online_players.values().any(|v| &v.username == &username) {
                        server.send_packet_disconnect(client_id, format!("Player {} already logged in", &username));
                        server.disconnect(client_id);
                        break;
                    }
                    // 原型登录尚无真实认证；不要在 Bevy 主循环模拟阻塞验证。

                    let server_entity = cmds.spawn(Transform::default()).id();
                    let entity_id = EntityId::from_server(server_entity);

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
                            server_entity,
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
                    let Some(player) = serverinfo.online_players.get_mut(&client_id) else {
                        server.send_packet_disconnect(client_id, "illegal play-stage packet. you have not login yet".into());
                        server.disconnect(client_id);
                        break;
                    };

                    match packet {
                        CPacket::ChatMessage { message } => {
                            if message.starts_with('/') {
                                match super::validation::parse_command(&message) {
                                    Ok(super::validation::ChatCommand::SetTime(daytime)) => {
                                        server.broadcast_packet(&SPacket::WorldTime { daytime });
                                    }
                                    Err(reason) => server.send_packet_chat(client_id, reason.into()),
                                }
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
                        // CPacket::ChunkModify { chunkpos, voxel } => {
                        // todo: NonLock
                        // let chunk = chunk_sys.get_chunk(chunkpos).unwrap();

                        // CellData::to_chunk(&voxel, chunk.as_ref_mut());

                        // server.broadcast_packet(&SPacket::ChunkModify { chunkpos, voxel });
                        // }
                        _ => {
                            warn!("Unknown Packet {:?}", packet);
                        }
                    }
                }
            }
        }
    }
}
