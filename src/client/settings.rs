// ClientSettings Configs

use crate::prelude::*;

pub const CLIENT_SETTINGS_FILE: &str = "client.settings.json";

fn on_app_init(mut cfg: ResMut<ClientSettings>) {
    info!("Loading {CLIENT_SETTINGS_FILE}");
    if let Ok(str) = std::fs::read_to_string(CLIENT_SETTINGS_FILE) {
        if let Ok(val) = serde_json::from_str(&str) {
            *cfg = val;
        }
    }
}

fn on_app_exit(mut exit_events: EventReader<bevy::app::AppExit>, cfg: Res<ClientSettings>) {
    for _ in exit_events.read() {
        info!("Program Terminate");

        info!("Saving {CLIENT_SETTINGS_FILE}");
        std::fs::write(CLIENT_SETTINGS_FILE, serde_json::to_string_pretty(&*cfg).unwrap()).unwrap();
    }
}

pub fn build_plugin(app: &mut App) {
    app.insert_resource(ClientSettings::default());
    app.register_type::<ClientSettings>();

    app.add_systems(PreStartup, on_app_init); // load settings
    app.add_systems(Last, on_app_exit); // save settings
}

#[derive(Resource, Deserialize, Serialize, Reflect)]
#[reflect(Resource)]
pub struct ClientSettings {
    #[reflect(ignore)]
    pub serverlist: Vec<ServerListItem>,

    pub fov: f32,
    pub username: String,
    pub hud_padding: f32,
    pub vsync: bool,

    pub chunks_load_distance: IVec2,
}

impl Default for ClientSettings {
    fn default() -> Self {
        Self {
            serverlist: Vec::default(),
            fov: 85.,
            username: crate::util::generate_simple_user_name(),
            hud_padding: 24.,
            vsync: true,

            chunks_load_distance: IVec2::new(4, 3),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct ServerListItem {
    pub name: String,
    pub addr: String,

    #[serde(skip)]
    pub ui: crate::ui::serverlist::UiServerInfo,
}
