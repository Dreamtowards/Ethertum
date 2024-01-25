
use std::{net::{UdpSocket, SocketAddr}, time::SystemTime};

use bevy::prelude::*;
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
        app.insert_resource(new_renet_server(addr, 64));
        info!("Server bind endpoint at {}", addr);

        app.add_systems(Update, server_sys);
    }
}

pub struct NetworkClientPlugin;

impl Plugin for NetworkClientPlugin {
    fn build(&self, app: &mut App) {
        
        let addr = "127.0.0.1:4000".parse().unwrap();
        
        app.add_plugins(RenetClientPlugin);
        app.add_plugins(NetcodeClientPlugin);

        let (client, transport) = new_renet_client(addr);
        app.insert_resource(client);
        app.insert_resource(transport);
        
        app.add_systems(Update, client_sys);

        app.add_systems(Update, ui_net);

    }
}

fn ui_net(
    mut ctx: EguiContexts, 
    mut server_addr: Local<String>,
) {
    egui::Window::new("Network").show(ctx.ctx_mut(), |ui| {
        ui.label("Server:");

        ui.text_edit_singleline(&mut *server_addr);

        if ui.button("Bind Endpoint").clicked() {

        }

        ui.label("Client:");
        
        ui.text_edit_singleline(&mut *server_addr);

        if ui.button("Connect Server").clicked() {
            
        }
    });
}




fn new_renet_server(public_addr: SocketAddr, max_clients: usize) -> NetcodeServerTransport {
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

fn new_renet_client(server_addr: SocketAddr) -> (RenetClient, NetcodeClientTransport) {
    // let server_addr = "127.0.0.1:5000".parse().unwrap();
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    let client_id = current_time.as_millis() as u64;
    let authentication = ClientAuthentication::Unsecure { 
        protocol_id: PROTOCOL_ID,
        client_id: client_id, 
        server_addr: server_addr, 
        user_data: None
    };

    let transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();
    let client = RenetClient::new(ConnectionConfig::default());

    (client, transport)
}


fn server_sys(
    mut server_events: EventReader<ServerEvent>,
    mut server: ResMut<RenetServer>,
) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                println!("Client {client_id} connected");

                server.send_message(*client_id, DefaultChannel::ReliableOrdered, "You connected");
                server.broadcast_message(DefaultChannel::ReliableOrdered, "A Client connected");
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                println!("Client {client_id} disconnected: {reason}");
                
                server.send_message(*client_id, DefaultChannel::ReliableOrdered, "You connected");
                server.broadcast_message(DefaultChannel::ReliableOrdered, "A Client Disconnected");
            }
        }
    }
    
    // Receive message from all clients
    for client_id in server.clients_id() {
        while let Some(message) = server.receive_message(client_id, DefaultChannel::ReliableOrdered) {
            // Handle received message

            info!("Server Received: {}", String::from_utf8_lossy(&message));
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






