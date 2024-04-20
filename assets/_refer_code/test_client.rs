
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










    
// pub fn ui_panel_info(
// ) {
// }

// fn ui_input_server_line(ui: &mut Ui, widget: impl Widget) {
//     ui.horizontal(|ui| {
//         let end_width = 100.;
//         let end_margin = 1.;

//         ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
//             ui.add_space(end_margin);
//             ui.add_sized([end_width, 10.], widget);
//         });
//     });
// }

// pub fn ui_serverlist_add(
//     mut ctx: EguiContexts, 
//     mut next_ui: ResMut<NextState<CurrentUI>>,
//     mut cli: ResMut<ClientInfo>,

//     mut _name: Local<String>,
//     mut _addr: Local<String>,
// ) {
//     new_egui_window("ServerList ItemEdit").show(ctx.ctx_mut(), |ui| {

//         ui.vertical_centered(|ui| {

//             ui.text_edit_singleline(&mut *_name);
            
//             ui.text_edit_singleline(&mut *_addr);

//             ui.set_enabled(!_name.is_empty() && !_addr.is_empty());
//             let save = ui.button("Save").clicked();
//             if save {
//                 cli.cfg.serverlist.push(ServerListItem { name: _name.clone(), addr: _addr.clone() });
//             }
//             ui.set_enabled(true);

//             if save || ui.button("Cancel").clicked() {
                
//                 _name.clear();
//                 _addr.clear();
//                 next_ui.set(CurrentUI::WtfServerList);
//             }
//         });
//     });
// }



    // fn load_obj(cmds: &mut Commands, asset_server: &Res<AssetServer>, materials: &mut ResMut<Assets<StandardMaterial>>, name: &str, has_norm: bool, pos: Vec3) {

    //     cmds.spawn((
    //         PbrBundle {
    //             mesh: asset_server.load(format!("models/{name}/mesh.obj")),
    //             material: materials.add(StandardMaterial {
    //                 base_color_texture: Some(asset_server.load(format!("models/{name}/diff.png"))),
    //                 normal_map_texture: if has_norm {Some(asset_server.load(format!("models/{name}/norm.png")))} else {None},
    //                 // double_sided: true,
    //                 alpha_mode: AlphaMode::Mask(0.5),
    //                 cull_mode: None,
    //                 ..default()
    //             }),
    //             transform: Transform::from_translation(pos),
    //             ..default()
    //         },
    //         // AsyncCollider(ComputedCollider::ConvexHull),
    //         // RigidBody::Static,
    //         DespawnOnWorldUnload,
    //     ));
    // }

    // ui.horizontal(|ui| {

    //     static mut PATH: String = String::new();
    //     ui.text_edit_singleline(unsafe { crate::util::raw::as_mut(std::ptr::addr_of_mut!(PATH)) });

    //     if ui.button("Load").clicked() {
    //         load_obj(&mut cmds, &asset_server, &mut materials, unsafe{PATH.as_str()}, false, query_campos.single().translation);
    //     }
        
    // });

    // load_obj(&mut cmds, &asset_server, &mut materials, "bucket", true, vec3(0., 0., -5.*1.));
    // load_obj(&mut cmds, &asset_server, &mut materials, "bench", false, vec3(0., 0., -5.*2.));
    // load_obj(&mut cmds, &asset_server, &mut materials, "bookcase", false, vec3(0., 0., -5.*3.));






// fn error_handler(In(result): In<anyhow::Result<()>>) {
//     let hm = crate::hashmap![
//         "foo" => 100,
//         "bar" => 200,
//     ];

//     if let Err(err) = result {
//         panic!("{}", err)
//     }
// }