//! 网络边界：先限制解码大小，再校验字段，最后由处理器检查连接状态。
use bevy::math::{IVec3, Vec3};
use bincode::Options;
use serde::de::DeserializeOwned;

use super::{packet::CellData, CPacket, SPacket};
use crate::voxel::Chunk;

// 完整区块（4096 个 CellData）仍可传输；不是 Renet 的分片/通道缓存大小。
pub(super) const MAX_PACKET_BYTES: usize = 64 * 1024;
const MAX_CHAT_BYTES: usize = 1024;
const MAX_USERNAME_BYTES: usize = 64;
const MAX_WORLD_COORD: f32 = 1_000_000.0;

fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, &'static str> {
    if bytes.len() > MAX_PACKET_BYTES {
        return Err("Packet exceeds size limit.");
    }
    // bincode::serialize 使用定长整数。DefaultOptions 默认是变长，必须显式保持
    // 旧编码；限制读取预算并拒绝尾随字节，不改变协议枚举和 PROTOCOL_ID。
    bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .with_limit(MAX_PACKET_BYTES as u64)
        .reject_trailing_bytes()
        .deserialize(bytes)
        .map_err(|_| "Malformed packet.")
}

fn valid_position(p: Vec3) -> bool {
    p.is_finite() && p.abs().max_element() <= MAX_WORLD_COORD
}

pub(crate) fn valid_username(name: &str) -> bool {
    !name.trim().is_empty() && name.len() <= MAX_USERNAME_BYTES && !name.chars().any(char::is_control)
}

pub(crate) fn valid_load_distance(distance: bevy::math::IVec2) -> bool {
    // Only the paired -1/-1 sentinel disables loading.
    distance == bevy::math::IVec2::splat(-1)
        || ((0..=super::MAX_LOAD_DISTANCE_XZ).contains(&distance.x) && (0..=super::MAX_LOAD_DISTANCE_Y).contains(&distance.y))
}

fn valid_chunk(cp: IVec3, cells: &[CellData]) -> bool {
    Chunk::is_chunkpos(cp)
        && cp.as_vec3().abs().max_element() <= MAX_WORLD_COORD
        && cells.len() <= Chunk::LEN3
        && cells.iter().all(|cell| (cell.local_idx as usize) < Chunk::LEN3)
}

pub(super) fn decode_client(bytes: &[u8]) -> Result<CPacket, &'static str> {
    let packet = decode(bytes)?;
    let valid = match &packet {
        CPacket::Login { username, .. } => valid_username(username),
        CPacket::ChatMessage { message } => message.len() <= MAX_CHAT_BYTES,
        CPacket::PlayerPos { position } => valid_position(*position),
        CPacket::LoadDistance { load_distance } => valid_load_distance(*load_distance),
        CPacket::ChunkModify { chunkpos, voxel } => valid_chunk(*chunkpos, voxel),
        _ => true,
    };
    if valid {
        Ok(packet)
    } else {
        Err("Invalid packet fields.")
    }
}

pub(super) fn decode_server(bytes: &[u8]) -> Result<SPacket, &'static str> {
    let packet = decode(bytes)?;
    let valid = match &packet {
        SPacket::EntityPos { position, .. } => valid_position(*position),
        SPacket::EntityNew { name, .. } => valid_username(name),
        SPacket::ChunkNew { chunkpos, voxel } | SPacket::ChunkModify { chunkpos, voxel } => valid_chunk(*chunkpos, voxel),
        SPacket::ChunkDel { chunkpos } => valid_chunk(*chunkpos, &[]),
        SPacket::WorldTime { daytime } => daytime.is_finite() && (0.0..=1.0).contains(daytime),
        SPacket::Chat { message } | SPacket::Disconnect { reason: message } => message.len() <= 4096,
        SPacket::PlayerList { playerlist } => playerlist.len() <= super::MAX_CLIENTS as usize && playerlist.iter().all(|(name, _)| valid_username(name)),
        _ => true,
    };
    if valid {
        Ok(packet)
    } else {
        Err("Invalid packet fields.")
    }
}

pub(super) enum ChatCommand {
    SetTime(f32),
}

pub(super) fn parse_command(message: &str) -> Result<ChatCommand, &'static str> {
    let command = message.strip_prefix('/').ok_or("Commands must start with /.")?;
    let args = shlex::split(command).ok_or("Invalid command quoting.")?;
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["time", "set", value] => {
            let daytime = value.parse::<f32>().map_err(|_| "Time must be a number in 0..=1.")?;
            if !daytime.is_finite() || !(0.0..=1.0).contains(&daytime) {
                return Err("Time must be a finite number in 0..=1.");
            }
            Ok(ChatCommand::SetTime(daytime))
        }
        ["time", ..] => Err("Usage: /time set <0..=1>"),
        [] => Err("Empty command."),
        _ => Err("Unknown command."),
    }
}
