
use bevy::{ecs::entity::Entity, math::Vec3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EntityId(u32);

impl EntityId {

    pub fn from_server(entity: Entity) -> EntityId {
        EntityId(entity.index())
    }

    pub fn client_entity(&self) -> Entity {
        Entity::from_raw(1_000_000 + self.0)
    }

    pub fn raw(&self) -> u32 {
        self.0
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub enum CPacket {
    // Handshake & Server Query & Login
    Handshake { protocol_version: u64 },
    ServerQuery {},
    Ping { client_time: u64 },

    Login { uuid: u64, access_token: u64, username: String },

    // Play
    ChatMessage { message: String },

    PlayerPos {
        position: Vec3,
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

    EntityNew {
        entity_id: EntityId,
        // type: {Player}
    },
    EntityPos {
        entity_id: EntityId,
        position: Vec3,
    }
}
