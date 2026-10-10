use bevy::{asset::AssetPlugin, ecs::system::RunSystemOnce, prelude::*};

use crate::{
    client::{client_world::on_world_exit, prelude::*},
    net::client_entities::ClientEntities,
};

#[test]
fn world_exit_keeps_the_startup_camera_for_menus_and_reentry() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            watch_for_changes_override: Some(false),
            ..default()
        },
    ))
    .init_asset::<Image>()
    .init_resource::<ClientEntities>()
    .add_systems(Startup, super::setup_camera_system);
    app.update();

    let camera = app
        .world_mut()
        .query_filtered::<Entity, With<CharacterControllerCamera>>()
        .single(app.world())
        .unwrap();

    for _ in 0..2 {
        let world_entity = app.world_mut().spawn(DespawnOnWorldUnload).id();
        app.world_mut().run_system_once(on_world_exit).unwrap();
        assert!(app.world().get_entity(world_entity).is_err());
        assert!(app.world().get::<Camera3d>(camera).is_some());
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<CharacterControllerCamera>>()
                .iter(app.world())
                .count(),
            1
        );
    }
}
