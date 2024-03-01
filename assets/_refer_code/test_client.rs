
use bevy::{prelude::*, log::LogPlugin};

use ethertia::net::NetworkClientPlugin;

fn main() {

    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(LogPlugin::default())
        .add_plugins(NetworkClientPlugin)
        .run();
}





    // Logical Player
    // crate::net::spawn_player(Entity::from_raw(1000), true, &cli.username, &mut cmds, &mut meshes, &mut materials);
    // commands.spawn((
    //     PbrBundle {
    //         mesh: meshes.add(Mesh::from(shape::Capsule {
    //             radius: 0.3,
    //             depth: 1.3,
    //             ..default()
    //         })),
    //         material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
    //         transform: Transform::from_xyz(0.0, 0.0, 0.0),
    //         ..default()
    //     },
    //     CharacterControllerBundle::new(
    //         Collider::capsule(1.3, 0.3),
    //         CharacterController {
    //             is_flying: true,
    //             enable_input: false,
    //             ..default()
    //         },
    //     ),
    //     Name::new("Player"),
    //     DespawnOnWorldUnload,
    // ));

    
    // // sky
    // cmds.spawn(PbrBundle {
    //     mesh: meshes.add(Mesh::from(shape::Box::default())),
    //     material: materials.add(StandardMaterial {
    //         base_color: Color::hex("888888").unwrap(),
    //         unlit: true,
    //         cull_mode: None,
    //         ..default()
    //     }),
    //     transform: Transform::from_scale(Vec3::splat(1_000_000.0)),
    //     ..default()
    // });

    // commands.spawn((
    //     SceneBundle {
    //         scene: assets.load("spaceship.glb#Scene0"),
    //         transform: Transform::from_xyz(0., 0., -10.),
    //         ..default()
    //     },
    //     AsyncSceneCollider::new(Some(ComputedCollider::TriMesh)),
    //     RigidBody::Static,
    // ));

    // // Floor
    // commands.spawn((
    //     SceneBundle {
    //         scene: asset_server.load("playground.glb#Scene0"),
    //         transform: Transform::from_xyz(0.5, -5.5, 0.5),
    //         ..default()
    //     },
    //     AsyncSceneCollider::new(Some(ComputedCollider::TriMesh)),
    //     RigidBody::Static,
    //     DespawnOnWorldUnload,
    // ));

    // // Cube
    // commands.spawn((
    //     RigidBody::Dynamic,
    //     AngularVelocity(Vec3::new(2.5, 3.4, 1.6)),
    //     Collider::cuboid(1.0, 1.0, 1.0),
    //     PbrBundle {
    //         mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
    //         material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
    //         transform: Transform::from_xyz(0.0, 4.0, 0.0),
    //         ..default()
    //     },
    // ));