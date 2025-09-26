use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    time::Duration,
};

use bevy::prelude::*;
use bevy_renet::renet::{
    ChannelConfig, ClientId, DefaultChannel, RenetClient, RenetServer, SendType,
};
use bevy_renet::netcode::{
    ClientAuthentication, NetcodeClientTransport, NetcodeServerTransport, ServerAuthentication, ServerConfig, NETCODE_USER_DATA_BYTES,
};

pub mod netproc_client;
mod netproc_server;
mod packet;

use crate::util::current_timestamp;
pub use netproc_client::ClientNetworkPlugin;
pub use netproc_server::ServerNetworkPlugin;
pub use packet::{CPacket, CellData, SPacket};

const PROTOCOL_ID: u64 = 1;

pub fn new_netcode_server_transport(public_addr_port: u16, max_clients: usize) -> NetcodeServerTransport {
    let public_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), public_addr_port);
    let socket = UdpSocket::bind(public_addr).unwrap();
    let server_config = ServerConfig {
        current_time: current_timestamp(),
        max_clients,
        protocol_id: PROTOCOL_ID,
        public_addresses: vec![public_addr],
        authentication: ServerAuthentication::Unsecure,
    };
    NetcodeServerTransport::new(server_config, socket).unwrap()
}

pub fn new_netcode_client_transport(server_addr: SocketAddr, user_data: Option<Vec<u8>>) -> NetcodeClientTransport {
    // let server_addr = "127.0.0.1:5000".parse().unwrap();
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    let current_time = current_timestamp();
    let client_id = current_time.as_millis() as u64;

    let user_data = user_data.map(|vec| {
        let mut data = [0u8; NETCODE_USER_DATA_BYTES];
        assert!(vec.len() <= NETCODE_USER_DATA_BYTES);
        data[..vec.len()].copy_from_slice(&vec[..]);
        data
    });

    let authentication = ClientAuthentication::Unsecure {
        protocol_id: PROTOCOL_ID,
        client_id,
        server_addr,
        user_data,
    };
    NetcodeClientTransport::new(current_time, authentication, socket).unwrap()
}

fn net_channel_config(max_memory_usage_bytes: usize) -> Vec<ChannelConfig> {
    vec![
        ChannelConfig {
            channel_id: 0,
            max_memory_usage_bytes,
            send_type: SendType::Unreliable,
        },
        ChannelConfig {
            channel_id: 1,
            max_memory_usage_bytes,
            send_type: SendType::ReliableUnordered {
                resend_time: Duration::from_millis(300),
            },
        },
        ChannelConfig {
            channel_id: 2,
            max_memory_usage_bytes,
            send_type: SendType::ReliableOrdered {
                resend_time: Duration::from_millis(300),
            },
        },
    ]
}

// An unique id shared in Server and Client. in client with a big offset to avoid id collision.

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EntityId(u32);

impl EntityId {
    pub fn from_server(entity: Entity) -> EntityId {
        EntityId(entity.index())
    }

    pub fn client_entity(&self) -> Entity {
        Entity::from_raw(10_000 + self.0)
    }

    pub fn raw(&self) -> u32 {
        self.0
    }
}

// fn ui_client_net(
//     mut ctx: EguiContexts,
//     mut client: ResMut<RenetClient>,
//     mut server_addr: Local<String>,
//     mut chat_msg: Local<String>,

//     mut commands: Commands,
// ) {
//     egui::Window::new("Client Network").show(ctx.ctx_mut(), |ui| {
//         ui.text_edit_singleline(&mut *server_addr);

//         if client.is_connected() {
//             if ui.button("Disconnect").clicked() {
//                 client.disconnect();
//             }

//             if ui.button("Handshake").clicked() {
//                 client.send_packet(&CPacket::Handshake {
//                     protocol_version: chat_msg.parse().unwrap_or(0),
//                 });
//             }

//             ui.text_edit_singleline(&mut *chat_msg);
//             if ui.button("ChatMessage").clicked() {
//                 client.send_packet(&CPacket::ChatMessage { message: chat_msg.clone() });
//             }

//             if ui.button("Server Query").clicked() {
//                 client.send_packet(&CPacket::ServerQuery {});
//             }
//             if ui.button("Ping").clicked() {
//                 client.send_packet(&CPacket::Ping {
//                     client_time: current_timestamp_millis(),
//                 });
//             }
//             if ui.button("Login").clicked() {
//                 client.send_packet(&CPacket::Login { uuid: 1, access_token: 123, username: "Some".into() });
//             }
//         } else {
//             if ui.button("Connect Server").clicked() {
//                 let addr = (server_addr).parse().unwrap();
//                 commands.insert_resource(new_netcode_client_transport(addr));
//                 commands.insert_resource(RenetClient::new(ConnectionConfig::default()));
//             }
//         }
//     });
// }

// fn ui_server_net(mut ctx: EguiContexts, mut server: ResMut<RenetServer>, mut server_addr: Local<String>, mut commands: Commands) {
//     egui::Window::new("Server Network").show(ctx.ctx_mut(), |ui| {
//         ui.text_edit_singleline(&mut *server_addr);

//         if ui.button("Bind Endpoint").clicked() {
//             commands.insert_resource(RenetServer::new(ConnectionConfig::default()));
//             commands.insert_resource(new_netcode_server_transport(server_addr.parse().unwrap(), 64));
//             info!("Server bind endpoint at {}", *server_addr);
//         }
//         if ui.button("Stop Server").clicked() {
//             server.disconnect_all();
//         }
//     });
// }

pub trait RenetServerHelper {
    fn send_packet<P: Serialize>(&mut self, client_id: ClientId, packet: &P);

    fn send_packet_disconnect(&mut self, client_id: ClientId, reason: String);

    fn send_packet_chat(&mut self, client_id: ClientId, message: String);

    fn broadcast_packet<P: Serialize>(&mut self, packet: &P);

    fn broadcast_packet_except<P: Serialize>(&mut self, except_id: ClientId, packet: &P);

    fn broadcast_packet_chat(&mut self, message: String);
}
impl RenetServerHelper for RenetServer {
    fn send_packet<P: Serialize>(&mut self, client_id: ClientId, packet: &P) {
        self.send_message(client_id, DefaultChannel::ReliableOrdered, bincode::serialize(packet).unwrap());
    }
    fn send_packet_disconnect(&mut self, client_id: ClientId, reason: String) {
        self.send_packet(client_id, &SPacket::Disconnect { reason });
    }
    fn send_packet_chat(&mut self, client_id: ClientId, message: String) {
        self.send_packet(client_id, &SPacket::Chat { message });
    }
    fn broadcast_packet<P: Serialize>(&mut self, packet: &P) {
        self.broadcast_message(DefaultChannel::ReliableOrdered, bincode::serialize(packet).unwrap());
    }
    fn broadcast_packet_except<P: Serialize>(&mut self, except_id: ClientId, packet: &P) {
        self.broadcast_message_except(except_id, DefaultChannel::ReliableOrdered, bincode::serialize(packet).unwrap());
    }
    fn broadcast_packet_chat(&mut self, message: String) {
        info!("[BroadcastChat] {}", &message);
        self.broadcast_packet(&SPacket::Chat { message });
    }
}

pub trait RenetClientHelper {
    fn send_packet<P: Serialize>(&mut self, packet: &P);
}
impl RenetClientHelper for RenetClient {
    fn send_packet<P: Serialize>(&mut self, packet: &P) {
        self.send_message(DefaultChannel::ReliableOrdered, bincode::serialize(packet).unwrap());
    }
}
