

use std::{str::FromStr, time::Duration};

use bevy::{prelude::*, utils::HashMap};
use bevy_renet::{client_just_connected, renet::{transport::NetcodeServerTransport, ClientId, DefaultChannel, RenetServer, ServerEvent}};

use crate::{net::{CPacket, RenetServerHelper, SPacket, PROTOCOL_ID}, util::current_timestamp_millis};

use super::packet::EntityId;

struct PlayerInfo {
    username: String,
    user_id: u64,

    entity_id: EntityId,

}

#[derive(Default)]
pub struct ServerInfo {
    online_players: HashMap<ClientId, PlayerInfo>,
}

impl ServerInfo {


}

pub fn server_sys(
    mut server_events: EventReader<ServerEvent>,
    mut server: ResMut<RenetServer>,
    transport: Res<NetcodeServerTransport>,
    mut serverinfo: Local<ServerInfo>,

    mut cmds: Commands,
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                let result_string: String = transport.user_data(*client_id)
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
                CPacket::Ping { client_time } => {
                    server.send_packet(
                        client_id,
                        &SPacket::Pong {
                            client_time: client_time,
                            server_time: current_timestamp_millis(),
                        },
                    );
                }
                CPacket::Login { uuid, access_token, username } => {
                    info!("Login Requested: {} {} {}", uuid, access_token, username);

                    if serverinfo.online_players.values().any(|v| &v.username == &username) {
                        server.send_packet_disconnect(client_id, format!("Player {} already logged in", &username));
                        continue;
                    }
                    // 模拟登录验证
                    std::thread::sleep(Duration::from_millis(800));

                    // Login Success
                    server.send_packet(client_id, &SPacket::LoginSuccess {});

                    server.broadcast_packet_chat(format!("Player {} joined. ({}/N)", &username, serverinfo.online_players.len()+1));
                    

                    let entity_id = cmds.spawn(TransformBundle::default()).id();
                    let entity_id = EntityId::from_server(entity_id);

                    // Send Server Players to the client. Note: Before insert of online_players
                    for player in serverinfo.online_players.values() {
                        server.send_packet(client_id, &SPacket::EntityNew { entity_id: player.entity_id, name: player.username.clone() });
                    }

                    server.broadcast_packet_except(client_id, &SPacket::EntityNew { entity_id, name: username.clone() });

                    serverinfo.online_players.insert(client_id, PlayerInfo { 
                        username, 
                        user_id: uuid,
                        entity_id,
                    });
                }
                // Play Stage:
                _ => {
                    // Requires Logged in.
                    // 这几行应该有语法糖简化..
                    let player = serverinfo.online_players.get(&client_id);
                    if player.is_none() {
                        continue;
                    } 
                    let player = player.unwrap();
                    
                    match packet {
                        CPacket::ChatMessage { message } => {
                            server.broadcast_packet_chat(format!("<{}>: {}", player.username, message.clone()));
                        }
                        CPacket::PlayerPos { position } => {
                            // todo: check diff, skip the same

                            server.broadcast_packet_except(client_id, 
                                &SPacket::EntityPos { entity_id: player.entity_id, position });
                        }
                        CPacket::PlayerList => {

                            let playerlist = serverinfo.online_players.iter()
                                .map(|e| (e.1.username.clone(), server.network_info(*e.0).unwrap().rtt as u32)).collect();
                            server.send_packet(client_id, &SPacket::PlayerList { playerlist })
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