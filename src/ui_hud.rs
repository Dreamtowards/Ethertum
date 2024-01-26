
use bevy::prelude::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {

        app.insert_resource(UiResource::default());

        app.add_systems(Startup, ui_init_resources);


    }
}

// #[derive(Resource, Default)]
// pub struct UiResource {
//     font: Handle<Font>,
// }
// impl UiResource {
//     fn text_style(&self) -> TextStyle {
//         TextStyle {
//             font: self.font.clone(),
//             font_size: 14.,
//             color: Color::WHITE,
//         }
//     }
// }

// fn ui_init_resources(
//     asset_server: Res<AssetServer>,
//     mut ui_res: ResMut<UiResource>,
// ) {
//     ui_res.font = asset_server.load("fonts/menlo.ttf");
// }

// fn ui_button() {

// }

// fn ui_main_menu(
//     mut commands: Commands,
//     mut ui_res: ResMut<UiResource>,
// ) {

//     commands.spawn(ButtonBundle {
//         style: Style {
//             width: Val::Px(200.0), 
//             height: Val::Px(80.0),
//             border: UiRect::all(Val::Px(2.)),
//             ..default()
//         },
//         background_color: Color::DARK_GRAY.into(),
//         ..default() 
//     }).with_children(|parent| {
//         parent.spawn(TextBundle {
//             text: Text::from_section("Button1", ui_res.text_style()),
//             ..default()
//         });
//     });
// }

