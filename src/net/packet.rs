use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum CPacket {
    // Handshake & Server Query & Login
    Handshake { protocol_version: u64 },
    ServerQuery {},
    Ping { client_time: u64 },

    Login { uuid: u64, access_token: u64 },

    // Play
    ChatMessage { message: String },
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
