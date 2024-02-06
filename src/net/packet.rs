
use bevy::{ecs::entity::Entity, math::{IVec3, Vec3}};
use serde::{Deserialize, Serialize};

use crate::voxel::{Cell, Chunk};



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



// Compressed Cell data.
#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CellData {
    local_idx: u16,  // 12 bits
    density: u8, // val = (v / 255.0) - 0.5
    mtl_id: u16,
}

impl CellData {
    pub fn from_chunk(chunk: &Chunk) -> Vec<CellData> {
        let mut data = Vec::new();
        for i in 0..Chunk::LOCAL_IDX_CAP {
            let c = chunk.get_cell(Chunk::local_idx_pos(i as i32));
            if !c.is_empty() {
                data.push(CellData {
                    local_idx: i as u16,
                    mtl_id: c.mtl,
                    density: ((c.value + 0.5).clamp(0.0, 1.0) * 255.0) as u8
                });
            }
        }
        data
    }
    pub fn to_chunk(data: &Vec<CellData>, chunk: &mut Chunk) {
        for c in data {
            chunk.set_cell(
                Chunk::local_idx_pos(c.local_idx as i32), 
                &Cell::new(c.density as f32 / 255.0 - 0.5, c.mtl_id)
            );
        }
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
    },

    PlayerList,  // RequestPlayerList
    
    // ChunkModify {
    //     chunkpos: IVec3,
    //     voxel: Vec<CellData>,
    // }
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
        name: String, // temporary way.
        // type: {Player}
    },
    EntityDel {
        entity_id: EntityId,
    },
    EntityPos {
        entity_id: EntityId,
        position: Vec3,
    },

    PlayerList {
        // name, ping
        playerlist: Vec<(String, u32)>
    },

    ChunkNew {
        chunkpos: IVec3,
        voxel: Vec<CellData>,  // or use full-chunk fixed array?
    },
    ChunkDel {
        chunkpos: IVec3,
    },
    // ChunkModify {
    //     chunkpos: IVec3,
    //     voxel: Vec<CellData>,
    // }

}
