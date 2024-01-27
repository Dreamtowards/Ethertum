
use std::{net::{UdpSocket, SocketAddr}, time::SystemTime};

use bevy::{prelude::*};
use bevy_egui::{EguiContexts, egui};
use bevy_renet::{
    RenetServerPlugin, 
    renet::{
        ConnectionConfig, 
        RenetServer, 
        transport::{ServerConfig, ServerAuthentication, NetcodeServerTransport, ClientAuthentication, NetcodeClientTransport}, ServerEvent, DefaultChannel, RenetClient
    }, 
    transport::{NetcodeServerPlugin, NetcodeClientPlugin}, RenetClientPlugin
};


const PROTOCOL_ID: u64 = 1;

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

    mut commands: Commands,
) {
    egui::Window::new("Network").show(ctx.ctx_mut(), |ui| {
        ui.label("Server:");

        ui.text_edit_singleline(&mut *server_addr);

        if ui.button("Bind Endpoint").clicked() {

        }

        ui.label("Client:");
        
        ui.text_edit_singleline(&mut *server_addr);

        if ui.button("Connect Server").clicked() {
            let addr = (server_addr).parse().unwrap();
            commands.insert_resource(new_netcode_client_transport(addr));
            // commands.insert_resource(RenetClient::new(ConnectionConfig::default()));
        }

        if ui.button("Send Pack").clicked() {
            client.send_message(DefaultChannel::ReliableOrdered, "Some Data Message");
        }

        if ui.button("Disconnect").clicked() {
            client.disconnect();
        }
    });
}

fn ui_server_net(
    mut ctx: EguiContexts, 
    mut client: ResMut<RenetClient>,
    mut server_addr: Local<String>,

    mut commands: Commands,
) {
    egui::Window::new("Network").show(ctx.ctx_mut(), |ui| {
        ui.label("Server:");

        ui.text_edit_singleline(&mut *server_addr);

        if ui.button("Bind Endpoint").clicked() {

        }
    });
}



fn new_netcode_server_transport(public_addr: SocketAddr, max_clients: usize) -> NetcodeServerTransport {
    // let public_addr = "127.0.0.1:4000".parse().unwrap();  // SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)
    let socket = UdpSocket::bind(public_addr).unwrap();
    let server_config = ServerConfig {
        current_time: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap(),
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
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    let client_id = current_time.as_millis() as u64;
    let authentication = ClientAuthentication::Unsecure { 
        protocol_id: PROTOCOL_ID,
        client_id, 
        server_addr: server_addr, 
        user_data: None
    };
    NetcodeClientTransport::new(current_time, authentication, socket).unwrap()
}


fn server_sys(
    mut server_events: EventReader<ServerEvent>,
    mut server: ResMut<RenetServer>,
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                println!("Client {client_id} connected {}", server.connected_clients());

                server.send_message(*client_id, DefaultChannel::ReliableOrdered, "You connected");
                server.broadcast_message(DefaultChannel::ReliableOrdered, "A Client connected");
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                println!("Client {client_id} disconnected: {reason}");
                
                server.broadcast_message(DefaultChannel::ReliableOrdered, "A Client Disconnected");
            }
        }
    }
    
    // Receive message from all clients
    for client_id in server.clients_id() {
        while let Some(bytes) = server.receive_message(client_id, DefaultChannel::ReliableOrdered) {
            // Handle received message

            // let p = bincode::deserialize(&bytes[..]);

            info!("Server Received: {}", String::from_utf8_lossy(&bytes));
        }
    }
}

fn client_sys(
    // mut client_events: EventReader<ClientEvent>,
    mut client: ResMut<RenetClient>,
) {

    while let Some(message) = client.receive_message(DefaultChannel::ReliableOrdered) {
        // let server_message = bincode::

        
        info!("Client Received: {}", String::from_utf8_lossy(&message));
    }

}






pub mod packet {

}


pub enum CPacket {
    
    // Handshake & Server Query & Login

    Handshake {
        protocol_version: u32,
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
        protocol_version: u32,
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




// Handshake
struct CPacketHandshake {
    protocol_version: u32,
}

struct SPacketDisconnect {
    reason: String,
}

// Server Query

struct CPacketServerQuery {
}

struct SPacketServerInfo {
    motd: String,
    num_players_limit: u32,
    num_players_online: u32,
    // online_players: Vec<(u64 uuid, String name)>
    protocol_version: u32,
    favicon: String,
}

struct CPacketPing {
    client_time: u64,
}
struct SPacketPong {
    client_time: u64,
    server_time: u64,
}


// Login

struct CPacketLogin {
    uuid: u64,
    access_token: u64,
}
struct SPacketLoginSuccess {
    // uuid, username
}


// Play

struct CPacketChatMessage {
    message: String,
}

struct SPacketChat {
    message: String,
}