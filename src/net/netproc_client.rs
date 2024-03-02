//! Client Networking Handler

use std::sync::{Arc, RwLock};

use bevy::{prelude::*, render::{primitives::Aabb, render_resource::PrimitiveTopology}, utils::HashMap};
use bevy_mod_billboard::BillboardTextBundle;
use bevy_renet::renet::{DefaultChannel, DisconnectReason, RenetClient};
use bevy_xpbd_3d::components::{Collider, RigidBody};
use leafwing_input_manager::{action_state::ActionState, axislike::DualAxis};

use crate::{
    character_controller::{CharacterController, CharacterControllerBundle}, 
    game_client::{ClientInfo, DespawnOnWorldUnload, InputAction, WorldInfo}, 
    ui::CurrentUI, util::current_timestamp_millis, voxel::{Chunk, ChunkComponent, ChunkSystem, ClientChunkSystem}
};

use super::{packet::CellData, SPacket};



pub fn client_sys(
    // mut client_events: EventReader<ClientEvent>,
    mut net_client: ResMut<RenetClient>,
    mut last_connected: Local<u32>,  // 0=NonConnection, 1=Connecting, 2=Connected
    mut cli: ResMut<ClientInfo>,

    mut chats: ResMut<crate::ui::hud::ChatHistory>,
    mut next_ui: ResMut<NextState<CurrentUI>>,
    mut cmds: Commands,
    
    // 临时测试 待移除:
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,

    mut chunk_sys: ResMut<ClientChunkSystem>,

    mut entity_s2c: Local<HashMap<Entity, Entity>>,
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

        cmds.remove_resource::<WorldInfo>();  // todo: cli.close_world();
        if net_client.disconnect_reason().unwrap() != DisconnectReason::DisconnectedByClient {
            next_ui.set(CurrentUI::DisconnectedReason);
        }
        
        info!("Disconnected. {}", cli.disconnected_reason);
    }
    

    while let Some(bytes) = net_client.receive_message(DefaultChannel::ReliableOrdered) {
        // info!("CLI Recv PACKET: {}", String::from_utf8_lossy(&bytes));
        let packet: SPacket = bincode::deserialize(&bytes[..]).unwrap();
        match &packet {
            SPacket::Disconnect { reason } => {
                info!("DisconnectedPacket: {}", reason);
                cli.disconnected_reason = reason.clone();
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

                cli.ping = (curr - *client_time, *client_time, *server_time, curr);
                info!(
                    "Ping: {}ms = cs {} + sc {}",
                    curr - *client_time,
                    *server_time as i64 - *client_time as i64,
                    curr as i64 - *server_time as i64
                );
            }
            SPacket::LoginSuccess { player_entity } => {
                info!("Login Success!");
                
                next_ui.set(CurrentUI::None);
                
                spawn_player(player_entity.client_entity(), true, &cli.cfg.username, &mut cmds, &asset_server, &mut meshes, &mut materials);


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

                spawn_player(entity_id.client_entity(), false, &name, &mut cmds, &asset_server, &mut meshes, &mut materials);
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

                cli.playerlist = playerlist.clone();  // should move?
            }

            SPacket::ChunkNew { chunkpos, voxel } => {

                let mut chunk = Chunk::new(*chunkpos);

                CellData::to_chunk(voxel, &mut chunk);
                
                // todo 封装函数
                {
                    let aabb = Aabb::from_min_max(Vec3::ZERO, Vec3::ONE * (Chunk::SIZE as f32));

                    chunk.mesh_handle = meshes.add(Mesh::new(PrimitiveTopology::TriangleList));
                    chunk.mesh_handle_foliage = meshes.add(Mesh::new(PrimitiveTopology::TriangleList));
    
                    chunk.entity = cmds
                        .spawn((
                            ChunkComponent::new(*chunkpos),
                            MaterialMeshBundle {
                                mesh: chunk.mesh_handle.clone(),
                                material: chunk_sys.shader_terrain.clone(),  //materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
                                transform: Transform::from_translation(chunkpos.as_vec3()),
                                visibility: Visibility::Hidden, // Hidden is required since Mesh is empty. or WGPU will crash. even if use default Inherite
                                ..default()
                            },
                            aabb,
                            RigidBody::Static,
                        ))
                        .with_children(|parent| {
                             parent.spawn((
                                MaterialMeshBundle {
                                    mesh: chunk.mesh_handle_foliage.clone(),
                                    material: materials.add(StandardMaterial {
                                        base_color_texture: Some(asset_server.load("baked/atlas_diff_foli.png")),
                                        // normal_map_texture: if has_norm {Some(asset_server.load(format!("models/{name}/norm.png")))} else {None},
                                        double_sided: true,
                                        alpha_mode: AlphaMode::Mask(0.5),
                                        cull_mode: None,
                                        ..default()
                                    }),
                                    // visibility: Visibility::Visible, // Hidden is required since Mesh is empty. or WGPU will crash
                                    ..default()
                                },
                                aabb,
                            ));
                        })
                        .set_parent(chunk_sys.entity)
                        .id();
                }

                let chunkptr = Arc::new(RwLock::new(chunk));

                chunk_sys.spawn_chunk(chunkptr);

                // info!("ChunkNew: {} ({})", chunkpos, chunk_sys.num_chunks());
            }
            SPacket::ChunkDel { chunkpos } => {
                // info!("ChunkDel: {} ({})", chunkpos, chunk_sys.num_chunks());

                if let Some(chunkptr) = chunk_sys.despawn_chunk(*chunkpos) {
                    let entity = chunkptr.read().unwrap().entity;
                    
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
                let mut chunk = chunk_sys.get_chunk(*chunkpos).unwrap().write().unwrap();

                CellData::to_chunk(voxel, &mut chunk);

            }
        }
    }
}




fn spawn_player(
    entity: Entity,
    is_theplayer: bool,
    name: &String,
    cmds: &mut Commands,
    asset_server: &Res<AssetServer>,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // cmds.add(|world: &mut World| {
    //     let meshes = world.get_resource_mut::<Assets<Mesh>>().unwrap();
    //     let materials = world.get_resource_mut::<Assets<StandardMaterial>>().unwrap();
    // });

    let mut ec = cmds.get_or_spawn(entity);
    
    ec.insert((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Capsule {
                depth: 1.3,
                radius: 0.3,
                ..default()
            })),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        DespawnOnWorldUnload,
    )).with_children(|parent| {
        if !is_theplayer {
            parent.spawn(BillboardTextBundle {
                transform: Transform::from_translation(Vec3::new(0., 1., 0.)).with_scale(Vec3::splat(0.005)),
                text: Text::from_sections([
                    TextSection {
                        value: name.clone(),
                        style: TextStyle {
                            font_size: 32.0,
                            color: Color::WHITE,
                            ..default()
                        }
                    },
                ]).with_alignment(TextAlignment::Center),
                ..default() 
            });
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
        ec.insert((
            CharacterControllerBundle::new(
                Collider::capsule(1.3, 0.3),
                CharacterController {
                    is_flying: true,
                    enable_input: false,
                    ..default()
                },
            ),
            leafwing_input_manager::InputManagerBundle::<InputAction> {
                // Stores "which actions are currently activated"
                action_state: ActionState::default(),
                // Describes how to convert from player inputs into those actions
                input_map: leafwing_input_manager::input_map::InputMap::default()
                    .insert(InputAction::Move, DualAxis::left_stick())
                    .insert(InputAction::Look, DualAxis::right_stick())
                    .build(),
            },
        )).with_children(|parent| {
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