
use std::{net::{UdpSocket, SocketAddr}, time::SystemTime};

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use bevy_renet::{
    RenetServerPlugin, 
    renet::{
        transport::{ServerConfig, ServerAuthentication, NetcodeServerTransport, ClientAuthentication, NetcodeClientTransport}, ClientId, ConnectionConfig, DefaultChannel, RenetClient, RenetServer, ServerEvent
    }, 
    transport::{NetcodeServerPlugin, NetcodeClientPlugin}, RenetClientPlugin
};
use serde::{Deserialize, Serialize};

use crate::util::{current_timestamp, current_timestamp_millis};


const PROTOCOL_ID: u64 = 1;


fn new_netcode_server_transport(public_addr: SocketAddr, max_clients: usize) -> NetcodeServerTransport {
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

fn new_netcode_client_transport(server_addr: SocketAddr) -> NetcodeClientTransport {
    // let server_addr = "127.0.0.1:5000".parse().unwrap();
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let current_time = current_timestamp();
    let client_id = current_time.as_millis() as u64;
    let authentication = ClientAuthentication::Unsecure { 
        protocol_id: PROTOCOL_ID,
        client_id, 
        server_addr: server_addr, 
        user_data: None
    };
    NetcodeClientTransport::new(current_time, authentication, socket).unwrap()
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

        app.add_systems(Update, server_sys);
        
        app.add_systems(Update, ui_server_net);
    }
}

pub struct NetworkClientPlugin;

impl Plugin for NetworkClientPlugin {
    fn build(&self, app: &mut App) {
        
        app.add_plugins(RenetClientPlugin);
        app.add_plugins(NetcodeClientPlugin);

        app.insert_resource(RenetClient::new(ConnectionConfig::default()));
        
        app.add_systems(Update, client_sys);

        app.add_systems(Update, ui_client_net);

    }
}





fn ui_client_net(
    mut ctx: EguiContexts, 
    mut client: ResMut<RenetClient>,
    mut server_addr: Local<String>,
    mut chat_msg: Local<String>,

    mut commands: Commands,
) {
    egui::Window::new("Client Network").show(ctx.ctx_mut(), |ui| {
       
        
        ui.text_edit_singleline(&mut *server_addr);

        if client.is_connected() {
            if ui.button("Disconnect").clicked() {
                client.disconnect();
            }

            if ui.button("Handshake").clicked() {
                client.send_packet(&CPacket::Handshake { protocol_version: chat_msg.parse().unwrap_or(0) });
            }

            ui.text_edit_singleline(&mut *chat_msg);
            if ui.button("ChatMessage").clicked() {
                client.send_packet(&CPacket::ChatMessage { message: chat_msg.clone() });
            }

            if ui.button("Server Query").clicked() {
                client.send_packet(&CPacket::ServerQuery { });
            }
            if ui.button("Ping").clicked() {
                client.send_packet(&CPacket::Ping { client_time: current_timestamp_millis() });
            }
            if ui.button("Login").clicked() {
                client.send_packet(&CPacket::Login { uuid: 1, access_token: 123 });
            }

        } else {
            if ui.button("Connect Server").clicked() {
                let addr = (server_addr).parse().unwrap();
                commands.insert_resource(new_netcode_client_transport(addr));
                commands.insert_resource(RenetClient::new(ConnectionConfig::default()));
            }
        }

    });
}

fn ui_server_net(
    mut ctx: EguiContexts, 
    mut server: ResMut<RenetServer>,
    mut server_addr: Local<String>,

    mut commands: Commands,
) {
    egui::Window::new("Server Network").show(ctx.ctx_mut(), |ui| {

        ui.text_edit_singleline(&mut *server_addr);

        if ui.button("Bind Endpoint").clicked() {
            commands.insert_resource(RenetServer::new(ConnectionConfig::default()));
            commands.insert_resource(new_netcode_server_transport(server_addr.parse().unwrap(), 64));
            info!("Server bind endpoint at {}", *server_addr);
        }
        if ui.button("Stop Server").clicked() {
            server.disconnect_all();
        }
    });
}




fn server_sys(
    mut server_events: EventReader<ServerEvent>,
    mut server: ResMut<RenetServer>,
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                println!("Client {client_id} connected {}", server.connected_clients());

            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                println!("Client {client_id} disconnected: {reason}");
            }
        }
    }
    
    // Receive message from all clients
    for client_id in server.clients_id() {
        while let Some(bytes) = server.receive_message(client_id, DefaultChannel::ReliableOrdered) {
            let packet: CPacket = bincode::deserialize(&bytes[..]).unwrap();

            match packet {
                CPacket::Handshake { protocol_version } => {
                    if protocol_version < PROTOCOL_ID {
                        server.send_packet(client_id, &SPacket::Disconnect { reason: "Client outdated.".into() })
                    } else if protocol_version > PROTOCOL_ID {
                        server.send_packet(client_id, &SPacket::Disconnect { reason: "Server outdated.".into() })
                    }
                },
                CPacket::ServerQuery { } => {
                    server.send_packet(client_id, &SPacket::ServerInfo { 
                        motd: "Motd".into(), 
                        num_players_limit: 64, 
                        num_players_online: 0, 
                        protocol_version: PROTOCOL_ID, 
                        favicon: "None".into(),
                    });
                },
                CPacket::Ping { client_time } => {
                    server.send_packet(client_id, &SPacket::Pong { client_time: client_time, server_time: current_timestamp_millis() });
                },
                CPacket::Login { uuid, access_token } => {
                    info!("Login Requested: {} {}", uuid, access_token);
                    server.send_packet(client_id, &SPacket::LoginSuccess { });
                },
                CPacket::ChatMessage { message } => {
                    server.broadcast_packet(&SPacket::Chat { message: format!("<ClientX>: {}", message.clone()) });
                },
            }

            info!("Server Received: {}", String::from_utf8_lossy(&bytes));
        }
    }
}

fn client_sys(
    // mut client_events: EventReader<ClientEvent>,
    mut client: ResMut<RenetClient>,
) {

    while let Some(bytes) = client.receive_message(DefaultChannel::ReliableOrdered) {
        info!("Client Received: {}", String::from_utf8_lossy(&bytes));
        let packet: SPacket = bincode::deserialize(&bytes[..]).unwrap();
        match &packet {
            SPacket::Disconnect { reason } => {
                info!("Disconnected: {}", reason);
                client.disconnect();
            },
            SPacket::ServerInfo { motd, num_players_limit, num_players_online, protocol_version, favicon } => {
                info!("ServerInfo: {:?}", &packet);
            }
            SPacket::Pong { client_time, server_time } => {
                let curr = current_timestamp_millis();
                info!("Ping: {}ms = cs {} + sc {}", curr-client_time, server_time-client_time, curr-server_time);
            },
            SPacket::LoginSuccess { } => {
                info!("Login Success!");
            },
            SPacket::Chat { message } => {
                info!("[Chat]: {}", message);
            }
        }

        
    }

}




trait RenetServerHelper  {

    fn send_packet<P: Serialize>(&mut self, client_id: ClientId, packet: &P);

    fn broadcast_packet<P: Serialize>(&mut self, packet: &P);
}
impl RenetServerHelper for RenetServer {
    fn send_packet<P: Serialize>(&mut self, client_id: ClientId, packet: &P) {
        self.send_message(client_id, DefaultChannel::ReliableOrdered, bincode::serialize(packet).unwrap());
    }
    fn broadcast_packet<P: Serialize>(&mut self, packet: &P) {
        self.broadcast_message(DefaultChannel::ReliableOrdered, bincode::serialize(packet).unwrap());
    }
}

trait RenetClientHelper {
    fn send_packet<P: Serialize>(&mut self, packet: &P);
}
impl RenetClientHelper for RenetClient {
    fn send_packet<P: Serialize>(&mut self, packet: &P) {
        self.send_message(DefaultChannel::ReliableOrdered, bincode::serialize(packet).unwrap());
    }
}





#[derive(Debug, Serialize, Deserialize)]
pub enum CPacket {
    
    // Handshake & Server Query & Login

    Handshake {
        protocol_version: u64,
    },
    ServerQuery {

    },
    Ping {
        client_time: u64,
    },

    Login {
        uuid: u64,
        access_token: u64,
    },

    // Play

    ChatMessage {
        message: String,
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SPacket {

    // Handshake & Server Query & Login

    Disconnect {
        reason: String,
    },
    ServerInfo {
        motd: String,
        num_players_limit: u32,
        num_players_online: u32,
        // online_players: Vec<(u64 uuid, String name)>
        protocol_version: u64,
        favicon: String,
    },
    Pong {
        client_time: u64,
        server_time: u64,
    },
    LoginSuccess {
        // uuid, username
    },

    // Play

    Chat {
        message: String,
    },
}




// // Handshake
// struct CPacketHandshake {
//     protocol_version: u32,
// }

// struct SPacketDisconnect {
//     reason: String,
// }

// // Server Query

// struct CPacketServerQuery {
// }

// struct SPacketServerInfo {
//     motd: String,
//     num_players_limit: u32,
//     num_players_online: u32,
//     // online_players: Vec<(u64 uuid, String name)>
//     protocol_version: u32,
//     favicon: String,
// }

// struct CPacketPing {
//     client_time: u64,
// }
// struct SPacketPong {
//     client_time: u64,
//     server_time: u64,
// }


// // Login

// struct CPacketLogin {
//     uuid: u64,
//     access_token: u64,
// }
// struct SPacketLoginSuccess {
//     // uuid, username
// }


// // Play

// struct CPacketChatMessage {
//     message: String,
// }

// struct SPacketChat {
//     message: String,
// }