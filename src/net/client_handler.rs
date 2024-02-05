
use bevy::prelude::*;
use bevy_renet::renet::{DefaultChannel, DisconnectReason, RenetClient};

use crate::{game::{ClientInfo, WorldInfo}, ui::CurrentUI, util::current_timestamp_millis};

use super::SPacket;



pub fn client_sys(
    // mut client_events: EventReader<ClientEvent>,
    mut client: ResMut<RenetClient>,
    mut last_connected: Local<u32>,
    mut clientinfo: ResMut<ClientInfo>,

    mut chats: ResMut<crate::ui::hud::ChatHistory>,
    mut next_ui: ResMut<NextState<CurrentUI>>,
    mut cmds: Commands,
    
    // 临时测试 待移除:
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if *last_connected != 1 && client.is_connecting() {
        *last_connected = 1;

    } else if *last_connected != 2 && client.is_connected() {
        *last_connected = 2;

    } else if *last_connected != 0 && client.is_disconnected() {
        *last_connected = 0;

        cmds.remove_resource::<WorldInfo>();  // todo: cli.close_world();
        if client.disconnect_reason().unwrap() != DisconnectReason::DisconnectedByClient {
            next_ui.set(CurrentUI::DisconnectedReason);
        }
    }
    

    while let Some(bytes) = client.receive_message(DefaultChannel::ReliableOrdered) {
        info!("CLI Recv PACKET: {}", String::from_utf8_lossy(&bytes));
        let packet: SPacket = bincode::deserialize(&bytes[..]).unwrap();
        match &packet {
            SPacket::Disconnect { reason } => {
                info!("Disconnected: {}", reason);
                clientinfo.disconnected_reason = reason.clone();
                client.disconnect_due_to_transport();
            }
            SPacket::ServerInfo {
                motd,
                num_players_limit,
                num_players_online,
                protocol_version,
                favicon,
            } => {
                info!("ServerInfo: {:?}", &packet);
            }
            SPacket::Pong { client_time, server_time } => {
                let curr = current_timestamp_millis();
                info!(
                    "Ping: {}ms = cs {} + sc {}",
                    curr - client_time,
                    server_time - client_time,
                    curr - server_time
                );
            }
            SPacket::LoginSuccess {} => {
                info!("Login Success!");
                
                next_ui.set(CurrentUI::None);
                cmds.insert_resource(WorldInfo::default());
            }
            SPacket::Chat { message } => {
                info!("[Chat]: {}", message);
                chats.scrollback.push(message.clone());
            }
            SPacket::EntityNew { entity_id } => {
                info!("Spawn EntityNew {}", entity_id.raw());

                // 客户端此时可能的确存在这个entity 因为内置的broadcast EntityPos 会发给所有客户端，包括没登录的
                // assert!(cmds.get_entity(entity_id.client_entity()).is_none(), "The EntityId already occupied in client.");

                cmds.get_or_spawn(entity_id.client_entity())
                    .insert(PbrBundle {
                        mesh: meshes.add(Mesh::from(shape::Capsule {
                            radius: 0.4,
                            depth: 1.0,
                            ..default()
                        })),
                        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
                        transform: Transform::from_xyz(0.0, 0.0, 0.0),
                        ..default()
                    });
            }
            SPacket::EntityPos { entity_id, position } => {
                info!("EntityPos {} -> {}", entity_id.raw(), position);
                
                cmds.get_or_spawn(entity_id.client_entity())
                    .insert(Transform::from_translation(*position));
            }
        }
    }
}
