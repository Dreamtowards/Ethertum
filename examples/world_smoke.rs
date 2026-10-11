//! `cargo run --example world_smoke --locked`
//! Exercises the real integrated server, renderer and world exit/reentry path.
//! Two clients: start with `--listen PORT`, then another with `--peer 127.0.0.1:PORT`.
//! Add `--edit` to verify local edits spanning a chunk corner and collider rebuilds.
use avian3d::prelude::{LinearVelocity, Position, SpatialQuery, SpatialQueryFilter};
use bevy::{
    app::AppExit,
    camera::RenderTarget,
    prelude::*,
    render::{
        render_resource::TextureFormat,
        view::screenshot::{Screenshot, ScreenshotCaptured},
    },
    winit::WinitSettings,
};
use ethertia::{
    client::prelude::*,
    net::{CPacket, RenetClientHelper},
    server::prelude::*,
    voxel::{ChunkSystem, ClientChunkSystem, Vox, VoxShape},
};

#[derive(Resource, Default)]
struct Smoke {
    phase: u8,
    next_at: f64,
    image: Handle<Image>,
    camera: Option<Entity>,
    screenshots: usize,
    expect_peer: bool,
    peer_address: Option<String>,
    seen_peer: Option<Entity>,
    edit: bool,
}

fn main() -> AppExit {
    let runtime_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/world-smoke")
        .join(std::process::id().to_string());
    std::fs::create_dir_all(&runtime_dir).expect("create isolated smoke runtime directory");
    std::env::set_current_dir(&runtime_dir).expect("use isolated smoke runtime directory");
    let mut smoke = Smoke::default();
    let mut listen_port = None;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--edit" => smoke.edit = true,
            "--listen" => {
                let port: u16 = arguments.next().expect("listen port").parse().expect("valid listen port");
                assert_ne!(port, 0);
                listen_port = Some(port);
                smoke.expect_peer = true;
            }
            "--peer" => smoke.peer_address = Some(arguments.next().expect("peer server address")),
            _ => panic!("Unknown smoke argument: {argument}"),
        }
    }
    assert!(!(smoke.expect_peer && smoke.peer_address.is_some()));
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: concat!(env!("CARGO_MANIFEST_DIR"), "/assets").into(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Ethertia world smoke".into(),
                    resolution: (1280, 720).into(),
                    visible: false,
                    ..default()
                }),
                ..default()
            }),
    )
    .add_plugins(ClientGamePlugin)
    .insert_resource(bevy::audio::GlobalVolume::new(bevy::audio::Volume::Linear(0.0)))
    .insert_resource(WinitSettings::game())
    .insert_resource(smoke)
    .add_systems(PostStartup, setup)
    .add_systems(Last, step);
    if let Some(port) = listen_port {
        app.world_mut().resource_mut::<ServerSettings>().port = port;
    }
    app.run()
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    cameras: Query<Entity, With<CharacterControllerCamera>>,
    mut cfg: ResMut<ClientSettings>,
    mut cli: ResMut<ClientInfo>,
    mut smoke: ResMut<Smoke>,
) {
    let camera = cameras.single().expect("exactly one startup camera");
    let image = images.add(Image::new_target_texture(1280, 720, TextureFormat::Rgba8UnormSrgb, None));
    commands.entity(camera).insert(RenderTarget::Image(image.clone().into()));
    smoke.image = image;
    smoke.camera = Some(camera);
    if smoke.peer_address.is_some() {
        smoke.phase = 20;
    }
    cfg.chunks_load_distance = IVec2::new(2, 3);
    cfg.fov = 85.0;
    cfg.username = format!("smoke_{}", std::process::id());
    cli.enable_cursor_look = false;
}

fn step(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut smoke: ResMut<Smoke>,
    mut client: EthertiaClient,
    mut net: Option<ResMut<bevy_renet::RenetClient>>,
    server: Res<ServerInfo>,
    server_cfg: Res<ServerSettings>,
    world: Option<Res<WorldInfo>>,
    mut chunks: Option<ResMut<ClientChunkSystem>>,
    cameras: Query<Entity, With<CharacterControllerCamera>>,
    mut players: ParamSet<(
        Query<(Entity, &mut Transform, &mut Position, &mut LinearVelocity, &mut CharacterController)>,
        SpatialQuery,
    )>,
    remote_players: Query<(Entity, &Name, &Transform), (With<Mesh3d>, Without<CharacterController>)>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs_f64();
    if now > 120.0 {
        error!("World smoke timed out in phase {}", smoke.phase);
        client.exit_world();
        exit.write(AppExit::error());
        return;
    }
    assert_eq!(cameras.single().ok(), smoke.camera, "startup camera lost");
    if now < smoke.next_at {
        return;
    }
    match smoke.phase {
        0 | 6 => {
            if smoke.phase == 6 && smoke.screenshots < 2 {
                return;
            }
            assert!(world.is_none());
            assert_eq!(server.online_players.len(), usize::from(smoke.phase == 6 && smoke.expect_peer));
            client.connect_server(format!("127.0.0.1:{}", server_cfg.port));
            smoke.phase += 1;
        }
        1 | 7 => {
            if smoke.expect_peer && !remote_players.iter().any(|(_, name, _)| name.as_str().starts_with("smoke_")) {
                return;
            }
            let mut query = players.p0();
            if let Ok((_, mut transform, mut position, mut velocity, mut controller)) = query.single_mut() {
                net.as_mut().expect("connected client").send_packet(&CPacket::LoadDistance {
                    load_distance: client.cfg.chunks_load_distance,
                });
                transform.translation = Vec3::new(8.0, 32.0, 8.0);
                position.0 = transform.translation;
                controller.is_flying = true;
                controller.pitch = -0.4;
                velocity.0 = Vec3::ZERO;
                smoke.phase += 1;
                smoke.next_at = now + 15.0;
            }
        }
        2 => {
            let mut query = players.p0();
            let (_, mut transform, mut position, mut velocity, _) = query.single_mut().unwrap();
            // Move across the x=16 chunk boundary, then allow real streaming to catch up.
            position.0.x = 24.0;
            transform.translation = position.0;
            velocity.0 = Vec3::ZERO;
            smoke.phase = if smoke.edit { 30 } else { 3 };
            smoke.next_at = now + 8.0;
        }
        30 => {
            let chunks = chunks.as_mut().expect("loaded chunk system");
            for x in 15..=16 {
                for y in 47..=48 {
                    for z in 15..=16 {
                        let voxel = chunks.get_voxel_mut(IVec3::new(x, y, z)).expect("corner chunks streamed");
                        assert!(voxel.is_nil(), "edit fixture must start in air");
                        *voxel = Vox::new(1, VoxShape::Cube, 0.0);
                    }
                }
            }
            smoke.phase = 31;
            smoke.next_at = now + 4.0;
        }
        31 | 32 => {
            let spatial_query = players.p1();
            for x in 15..=16 {
                for z in 15..=16 {
                    let hit = spatial_query.cast_ray(
                        Vec3::new(x as f32 + 0.5, 52.0, z as f32 + 0.5),
                        Dir3::NEG_Y,
                        100.0,
                        false,
                        &SpatialQueryFilter::default(),
                    );
                    if smoke.phase == 31 {
                        assert!(
                            hit.is_some_and(|hit| (hit.distance - 3.0).abs() < 0.05),
                            "placed corner cubes need current colliders"
                        );
                    } else {
                        assert!(
                            hit.is_none_or(|hit| hit.distance > 5.0),
                            "deleted corner cubes must not retain old colliders"
                        );
                    }
                }
            }
            if smoke.phase == 31 {
                let chunks = chunks.as_mut().unwrap();
                for x in 15..=16 {
                    for y in 47..=48 {
                        for z in 15..=16 {
                            *chunks.get_voxel_mut(IVec3::new(x, y, z)).unwrap() = Vox::default();
                        }
                    }
                }
                smoke.phase = 32;
                smoke.next_at = now + 4.0;
            } else {
                info!("Corner edit smoke passed: eight chunks rebuilt after placement and removal");
                smoke.phase = 3;
            }
        }
        3 => {
            let (entity, origin) = {
                let mut query = players.p0();
                let (entity, _, position, _, _) = query.single_mut().unwrap();
                (entity, position.0)
            };
            let filter = SpatialQueryFilter::default().with_excluded_entities([entity]);
            let hit = players
                .p1()
                .cast_ray(origin, Dir3::NEG_Y, 256.0, false, &filter)
                .expect("streamed terrain must have a collider below the player");
            let mut query = players.p0();
            let (_, mut transform, mut position, mut velocity, mut controller) = query.single_mut().unwrap();
            position.0.y = position.0.y - hit.distance + 4.0;
            transform.translation = position.0;
            velocity.0 = Vec3::ZERO;
            controller.is_flying = false;
            smoke.phase = 4;
            smoke.next_at = now + 3.0;
        }
        4 | 8 => {
            assert!(world.is_some());
            assert_eq!(server.online_players.len(), if smoke.expect_peer { 2 } else { 1 });
            if smoke.expect_peer {
                assert!(
                    remote_players
                        .iter()
                        .any(|(_, name, transform)| name.as_str().starts_with("smoke_") && transform.translation.z > 16.0),
                    "peer position must synchronize across the z=16 boundary"
                );
            }
            if smoke.phase == 4 {
                let mut query = players.p0();
                let (_, _, position, velocity, controller) = query.single_mut().unwrap();
                assert!(position.0.is_finite() && velocity.0.is_finite());
                assert!(controller.is_grounded, "player must land on streamed terrain");
            }
            let path = match (smoke.expect_peer, smoke.phase) {
                (false, 4) => "target/world-smoke-first.png",
                (false, _) => "target/world-smoke-reentry.png",
                (true, 4) => "target/world-smoke-dual-first.png",
                (true, _) => "target/world-smoke-dual-reentry.png",
            };
            capture(&mut commands, &smoke, path);
            client.exit_world();
            smoke.phase += 1;
            smoke.next_at = now + 3.0;
        }
        5 => {
            assert!(world.is_none());
            assert!(players.p0().is_empty());
            // Wait for the actual UDP disconnect event before reconnecting.
            if server.online_players.len() != usize::from(smoke.expect_peer) {
                return;
            }
            capture(
                &mut commands,
                &smoke,
                if smoke.expect_peer {
                    "target/world-smoke-dual-menu.png"
                } else {
                    "target/world-smoke-menu.png"
                },
            );
            smoke.phase = 6;
            smoke.next_at = now + 3.0;
        }
        9 => {
            assert!(world.is_none());
            assert!(players.p0().is_empty());
            if server.online_players.is_empty() && smoke.screenshots == 3 {
                info!("World smoke passed: login, chunk boundary movement, collision, exit, same-user reentry, cleanup, screenshots");
                exit.write(AppExit::Success);
                smoke.phase = 10;
            }
        }
        20 => {
            client.connect_server(smoke.peer_address.clone().unwrap());
            smoke.phase = 21;
        }
        21 => {
            let mut query = players.p0();
            if let Ok((_, mut transform, mut position, mut velocity, mut controller)) = query.single_mut() {
                net.as_mut().expect("connected client").send_packet(&CPacket::LoadDistance {
                    load_distance: client.cfg.chunks_load_distance,
                });
                position.0 = Vec3::new(0.0, 32.0, 24.0);
                transform.translation = position.0;
                velocity.0 = Vec3::ZERO;
                controller.is_flying = true;
                controller.pitch = -0.4;
                smoke.phase = 22;
            }
        }
        22 => {
            if let Some((entity, _, _)) = remote_players
                .iter()
                .find(|(_, name, transform)| name.as_str().starts_with("smoke_") && transform.translation.x > 16.0)
            {
                smoke.seen_peer = Some(entity);
                smoke.phase = 23;
                info!("Peer observed host position across the x=16 boundary");
            }
        }
        23 => {
            if remote_players.get(smoke.seen_peer.unwrap()).is_err() {
                assert!(!remote_players.iter().any(|(_, name, _)| name.as_str().starts_with("smoke_")));
                smoke.phase = 24;
                info!("Peer observed host disconnect cleanup");
            }
        }
        24 => {
            if let Some((entity, _, _)) = remote_players.iter().find(|(_, name, _)| name.as_str().starts_with("smoke_")) {
                assert_ne!(Some(entity), smoke.seen_peer, "reentry must allocate a new mapped entity");
                smoke.seen_peer = Some(entity);
                smoke.phase = 25;
                smoke.next_at = now + 10.0;
                info!("Peer observed host reentry as a new entity");
            }
        }
        25 => {
            capture(&mut commands, &smoke, "target/world-smoke-peer.png");
            smoke.phase = 26;
        }
        26 => {
            if remote_players.get(smoke.seen_peer.unwrap()).is_err() {
                client.exit_world();
                smoke.phase = 27;
                smoke.next_at = now + 3.0;
            }
        }
        27 => {
            assert!(world.is_none());
            assert!(players.p0().is_empty());
            if smoke.screenshots == 1 {
                info!("Peer smoke passed: position sync, disconnect, reentry, final cleanup");
                exit.write(AppExit::Success);
                smoke.phase = 28;
            }
        }
        _ => {}
    }
}

fn capture(commands: &mut Commands, smoke: &Smoke, path: &'static str) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    commands
        .spawn(Screenshot::image(smoke.image.clone()))
        .observe(move |event: On<ScreenshotCaptured>, mut smoke: ResMut<Smoke>| {
            event
                .image
                .clone()
                .try_into_dynamic()
                .expect("readable screenshot")
                .to_rgb8()
                .save(&path)
                .expect("save smoke screenshot");
            smoke.screenshots += 1;
            info!("Smoke screenshot saved: {}", path.display());
        });
}
