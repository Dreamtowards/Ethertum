//! Client Networking Handler

use bevy::{ecs::system::EntityCommands, prelude::*, utils::HashMap};
use bevy_renet::{
    renet::{DefaultChannel, DisconnectReason, RenetClient},
    transport::NetcodeClientPlugin,
    RenetClientPlugin,
};
use bevy_xpbd_3d::plugins::collision::Collider;

use crate::{
    client::prelude::*,
    client::ui::CurrentUI,
    util::{current_timestamp_millis, AsRefMut},
    voxel::{Chunk, ChunkSystem, ClientChunkSystem},
};

use super::{packet::CellData, SPacket};

pub struct ClientNetworkPlugin;

impl Plugin for ClientNetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenetClientPlugin);
        app.add_plugins(NetcodeClientPlugin);

        // 待考证: resource_exists::<RenetClient> 之前会造成 获取未加载的ChunkSystemClient
        app.add_systems(Update, client_sys.run_if(condition::in_world));

        // app.add_systems(Update, ui_client_net);
    }
}

pub fn client_sys(
    // mut client_events: EventReader<ClientEvent>,
    mut net_client: ResMut<RenetClient>,
    mut last_connected: Local<u32>, // 0=NonConnection, 1=Connecting, 2=Connected
    mut cli: ResMut<ClientInfo>,
    cfg: Res<ClientSettings>,

    mut chats: ResMut<crate::client::ui::hud::ChatHistory>,
    mut cmds: Commands,
    mut chunk_sys: ResMut<ClientChunkSystem>,
    mut worldinfo: ResMut<WorldInfo>,

    // 临时测试 待移除:
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    entity_s2c: Local<HashMap<Entity, Entity>>,
) {
    if *last_connected != 1 && net_client.is_connecting() {
        *last_connected = 1;
    } else if *last_connected != 2 && net_client.is_connected() {
        *last_connected = 2;
    } else if *last_connected != 0 && net_client.is_disconnected() {
        *last_connected = 0;

        if cli.disconnected_reason.is_empty() {
            cli.disconnected_reason = net_client.disconnect_reason().unwrap().to_string();
        }

        cmds.remove_resource::<WorldInfo>(); // todo: cli.close_world();
        if net_client.disconnect_reason().unwrap() != DisconnectReason::DisconnectedByClient {
            cli.curr_ui = CurrentUI::DisconnectedReason;
        }

        info!("Disconnected. {}", cli.disconnected_reason);
    }

    while let Some(bytes) = net_client.receive_message(DefaultChannel::ReliableOrdered) {
        // info!("CLI Recv PACKET: {}", String::from_utf8_lossy(&bytes));
        let packet: SPacket = bincode::deserialize(&bytes[..]).unwrap();
        match &packet {
            SPacket::Disconnect { reason } => {
                info!("DisconnectedPacket: {}", reason);
                cli.disconnected_reason.clone_from(reason);
                net_client.disconnect_due_to_transport();
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

                cli.ping = (
                    curr - *client_time,
                    *server_time as i64 - *client_time as i64,
                    curr as i64 - *server_time as i64,
                    *client_time,
                );
                // info!("Ping: rtt {}ms = c2s {} + s2c {}", cli.ping.0, cli.ping.1, cli.ping.2);
            }
            SPacket::LoginSuccess { player_entity } => {
                info!("Login Success!");

                cli.curr_ui = CurrentUI::None;

                spawn_player(
                    &mut cmds.get_or_spawn(player_entity.client_entity()), // 为什么在这生成 因为要指定id，以及其他player也是在这生成
                    true,
                    &cfg.username,
                    &asset_server,
                    &mut meshes,
                    &mut materials,
                );

                // cmds.insert_resource(WorldInfo::default());  // moved to Click Connect. 要在用之前初始化，如果现在标记 那么就来不及初始化 随后就有ChunkNew数据包 要用到资源
            }
            SPacket::Chat { message } => {
                info!("[Chat]: {}", message);
                chats.scrollback.push(message.clone());
            }
            SPacket::EntityNew { entity_id, name } => {
                info!("Spawn EntityNew {}", entity_id.raw());

                // 客户端此时可能的确存在这个entity 因为内置的broadcast EntityPos 会发给所有客户端，包括没登录的
                // assert!(cmds.get_entity(entity_id.client_entity()).is_none(), "The EntityId already occupied in client.");

                spawn_player(
                    &mut cmds.get_or_spawn(entity_id.client_entity()),
                    false,
                    name,
                    &asset_server,
                    &mut meshes,
                    &mut materials,
                );
            }
            SPacket::EntityPos { entity_id, position } => {
                info!("EntityPos {} -> {}", entity_id.raw(), position);

                cmds.get_or_spawn(entity_id.client_entity())
                    .insert(Transform::from_translation(*position));
            }
            SPacket::EntityDel { entity_id } => {
                info!("DeSpawn EntityDel {}", entity_id.raw());

                cmds.get_entity(entity_id.client_entity()).unwrap().despawn_recursive();
            }
            SPacket::PlayerList { playerlist } => {
                cli.playerlist.clone_from(playerlist); // should move?
            }
            SPacket::WorldTime { daytime } => {
                worldinfo.daytime = *daytime;
            }
            SPacket::ChunkNew { chunkpos, voxel } => {
                let mut chunk = Chunk::new(*chunkpos);

                CellData::to_chunk(voxel, &mut chunk);

                chunk_sys.spawn_chunk(chunk, &mut cmds, &mut *meshes);

                // info!("ChunkNew: {} ({})", chunkpos, chunk_sys.num_chunks());
            }
            SPacket::ChunkDel { chunkpos } => {
                // info!("ChunkDel: {} ({})", chunkpos, chunk_sys.num_chunks());

                if let Some(chunkptr) = chunk_sys.despawn_chunk(*chunkpos) {
                    let entity = chunkptr.entity;

                    // bug crash: "Attempting to create an EntityCommands for entity 9649v15, which doesn't exist."
                    // why the entity may not exists even if it in the chunk_sys?
                    if let Some(cmds) = cmds.get_entity(entity) {
                        cmds.despawn_recursive();
                    }
                }
            }
            SPacket::ChunkModify { chunkpos, voxel } => {
                info!("ChunkModify: {}", chunkpos);

                chunk_sys.mark_chunk_remesh(*chunkpos);

                // 这不全面。如果修改了edge 那么应该更新3个区块。然而这里只会更新一个区块
                for data in voxel {
                    let lp = Chunk::local_idx_pos(data.local_idx as i32);
                    let neib = Chunk::at_boundary_naive(lp);
                    if neib != -1 {
                        chunk_sys.mark_chunk_remesh(*chunkpos + Chunk::NEIGHBOR_DIR[neib as usize] * Chunk::SIZE);
                    }
                }

                // todo: NonLock
                let chunk = chunk_sys.get_chunk(*chunkpos).unwrap();

                CellData::to_chunk(voxel, chunk.as_ref_mut());
            }
        }
    }
}

pub fn spawn_player(
    ec: &mut EntityCommands,
    is_theplayer: bool,
    _name: &String,
    // cmds: &mut Commands,
    _asset_server: &Res<AssetServer>,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // cmds.add(|world: &mut World| {
    //     let meshes = world.get_resource_mut::<Assets<Mesh>>().unwrap();
    //     let materials = world.get_resource_mut::<Assets<StandardMaterial>>().unwrap();
    // });
    // let mut ec = cmds.get_or_spawn(entity);

    ec.insert((
        PbrBundle {
            mesh: meshes.add(Capsule3d {
                radius: 0.3,
                half_length: 0.9,
            }),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6)),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        DespawnOnWorldUnload,
    ))
    .with_children(|parent| {
        if !is_theplayer {
            // parent.spawn(BillboardTextBundle {
            //     transform: Transform::from_translation(Vec3::new(0., 1., 0.)).with_scale(Vec3::splat(0.005)),
            //     text: Text::from_sections([
            //         TextSection {
            //             value: name.clone(),
            //             style: TextStyle {
            //                 font_size: 32.0,
            //                 color: Color::WHITE,
            //                 ..default()
            //             }
            //         },
            //     ]).with_alignment(JustifyText::Center),
            //     ..default()
            // });
        }
        parent.spawn(SpotLightBundle {
            spot_light: SpotLight {
                color: Color::YELLOW,
                intensity: 3200.,
                ..default()
            },
            transform: Transform::from_xyz(0., 0.3, 0.3),
            ..default()
        });
    });

    if is_theplayer {
        ec.insert((CharacterControllerBundle::new(
            Collider::capsule(0.9, 0.3),
            CharacterController {
                is_flying: true,
                enable_input: false,
                ..default()
            },
        ),))
            .with_children(|parent| {
                // pointy "nose" for player
                parent.spawn(SpriteBundle {
                    transform: Transform {
                        translation: Vec3::new(15., 0., 0.),
                        rotation: Quat::from_rotation_z(std::f32::consts::PI / 4.),
                        ..default()
                    },
                    sprite: Sprite {
                        color: Color::ORANGE,
                        custom_size: Some(Vec2::splat(50. / f32::sqrt(2.))),
                        ..default()
                    },
                    ..default()
                });
            });
    }
}
