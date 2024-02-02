
use bevy::prelude::*;
use bevy_renet::renet::{DefaultChannel, RenetClient, RenetConnectionStatus};

use crate::{game::{ClientInfo, WorldInfo}, ui::CurrentUI, util::current_timestamp_millis};

use super::SPacket;



pub fn client_sys(
    // mut client_events: EventReader<ClientEvent>,
    mut client: ResMut<RenetClient>,
    mut last_connected: Local<u32>,
    mut clientinfo: ResMut<ClientInfo>,

    mut next_ui: ResMut<NextState<CurrentUI>>,
    mut cmds: Commands,
) {
    if *last_connected != 1 && client.is_connecting() {
        *last_connected = 1;

    } else if *last_connected != 2 && client.is_connected() {
        *last_connected = 2;

    } else if *last_connected != 0 && client.is_disconnected() {
        *last_connected = 0;

        cmds.remove_resource::<WorldInfo>();  // todo: cli.close_world();
        next_ui.set(CurrentUI::DisconnectedReason);
    }
    

    while let Some(bytes) = client.receive_message(DefaultChannel::ReliableOrdered) {
        info!("CLI Recv PACKET: {}", String::from_utf8_lossy(&bytes));
        let packet: SPacket = bincode::deserialize(&bytes[..]).unwrap();
        match &packet {
            SPacket::Disconnect { reason } => {
                info!("Disconnected: {}", reason);
                clientinfo.disconnected_reason = reason.clone();
                client.disconnect();
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
            }
        }
    }
}
