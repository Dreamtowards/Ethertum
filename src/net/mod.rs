use std::{net::{SocketAddr, UdpSocket}, time::Duration};

use crate::util::{current_timestamp, current_timestamp_millis};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use bevy_renet::{
    renet::{
        transport::{ClientAuthentication, NetcodeClientTransport, NetcodeServerTransport, ServerAuthentication, ServerConfig},
        ClientId, ConnectionConfig, DefaultChannel, RenetClient, RenetServer, ServerEvent,
    },
    transport::{NetcodeClientPlugin, NetcodeServerPlugin},
    RenetClientPlugin, RenetServerPlugin,
};
use serde::{Deserialize, Serialize};

mod packet;
pub use packet::{CPacket, SPacket};

mod client_handler;
mod server_handler;


const PROTOCOL_ID: u64 = 1;

pub fn new_netcode_server_transport(public_addr: SocketAddr, max_clients: usize) -> NetcodeServerTransport {
    // let public_addr = "127.0.0.1:4000".parse().unwrap();  // SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)
    let socket = UdpSocket::bind(public_addr).unwrap();
    let server_config = ServerConfig {
        current_time: current_timestamp(),
        max_clients: max_clients,
        protocol_id: PROTOCOL_ID,
        public_addresses: vec![public_addr],
        authentication: ServerAuthentication::Unsecure,
    };
    NetcodeServerTransport::new(server_config, socket).unwrap()
}

pub fn new_netcode_client_transport(server_addr: SocketAddr, client_id: u64) -> NetcodeClientTransport {
    // let server_addr = "127.0.0.1:5000".parse().unwrap();
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let authentication = ClientAuthentication::Unsecure {
        protocol_id: PROTOCOL_ID,
        client_id,
        server_addr: server_addr,
        user_data: None,
    };
    NetcodeClientTransport::new(current_timestamp(), authentication, socket).unwrap()
}

pub struct NetworkServerPlugin;

impl Plugin for NetworkServerPlugin {
    fn build(&self, app: &mut App) {
        let addr = "127.0.0.1:4000".parse().unwrap();

        app.add_plugins(RenetServerPlugin);
        app.add_plugins(NetcodeServerPlugin);

        app.insert_resource(RenetServer::new(ConnectionConfig::default()));
        app.insert_resource(new_netcode_server_transport(addr, 64));
        info!("Server bind endpoint at {}", addr);

        app.add_systems(Update, server_handler::server_sys);

        // app.add_systems(Update, ui_server_net);
    }
}

pub struct NetworkClientPlugin;

impl Plugin for NetworkClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenetClientPlugin);
        app.add_plugins(NetcodeClientPlugin);

        app.insert_resource(RenetClient::new(ConnectionConfig::default()));

        app.add_systems(Update, client_handler::client_sys);

        // app.add_systems(Update, ui_client_net);
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

    fn broadcast_packet<P: Serialize>(&mut self, packet: &P);
    
    fn send_packet_disconnect(&mut self, client_id: ClientId, reason: String);
    
    fn broadcast_packet_chat(&mut self, message: String);
}
impl RenetServerHelper for RenetServer {
    fn send_packet<P: Serialize>(&mut self, client_id: ClientId, packet: &P) {
        self.send_message(client_id, DefaultChannel::ReliableOrdered, bincode::serialize(packet).unwrap());
    }
    fn broadcast_packet<P: Serialize>(&mut self, packet: &P) {
        self.broadcast_message(DefaultChannel::ReliableOrdered, bincode::serialize(packet).unwrap());
    }
    fn send_packet_disconnect(&mut self, client_id: ClientId, reason: String) {
        self.send_packet(client_id, &SPacket::Disconnect { reason });
    }
    fn broadcast_packet_chat(&mut self, message: String) {
        info!("[BroadcastMessage]: {}", &message);
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
