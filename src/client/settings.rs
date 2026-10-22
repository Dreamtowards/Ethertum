// ClientSettings Configs

use crate::prelude::*;
use anyhow::Context;
use std::path::Path;

pub const CLIENT_SETTINGS_FILE: &str = "client.settings.json";

#[derive(Resource, Default)]
pub(crate) struct ClientSettingsStatus {
    pub load_error: Option<String>,
}

fn load_settings(path: &Path) -> (ClientSettings, ClientSettingsStatus) {
    let result = (|| -> anyhow::Result<ClientSettings> {
        let settings: ClientSettings = crate::util::settings::load(path)?.unwrap_or_default();
        settings.validate().with_context(|| format!("Invalid settings in {}", path.display()))?;
        Ok(settings)
    })();
    match result {
        Ok(settings) => (settings, ClientSettingsStatus::default()),
        Err(error) => (ClientSettings::default(), ClientSettingsStatus { load_error: Some(format!("{error:#}")) }),
    }
}

fn save_settings(path: &Path, cfg: &ClientSettings, status: &ClientSettingsStatus) -> anyhow::Result<()> {
    anyhow::ensure!(status.load_error.is_none(), "Original configuration is protected after a load failure; repair or move {} and restart", path.display());
    cfg.validate()?;
    crate::util::settings::save(path, cfg)
}

fn on_app_init(mut cfg: ResMut<ClientSettings>, mut status: ResMut<ClientSettingsStatus>) {
    info!("Loading {CLIENT_SETTINGS_FILE}");
    let (loaded, loaded_status) = load_settings(Path::new(CLIENT_SETTINGS_FILE));
    if let Some(error) = &loaded_status.load_error {
        warn!("Using default client settings; original file will be preserved: {error}");
    }
    *cfg = loaded;
    *status = loaded_status;
}

fn on_app_exit(mut exit_events: MessageReader<bevy::app::AppExit>, cfg: Res<ClientSettings>, status: Res<ClientSettingsStatus>) {
    for _ in exit_events.read() {
        info!("Program Terminate");

        info!("Saving {CLIENT_SETTINGS_FILE}");
        if let Err(error) = save_settings(Path::new(CLIENT_SETTINGS_FILE), &cfg, &status) {
            error!("Cannot save client settings: {error:#}");
        }
    }
}

pub fn build_plugin(app: &mut App) {
    app.insert_resource(ClientSettings::default());
    app.init_resource::<ClientSettingsStatus>();
    app.register_type::<ClientSettings>();

    app.add_systems(PreStartup, on_app_init); // load settings
    app.add_systems(Last, on_app_exit); // save settings
}

#[derive(Resource, Deserialize, Serialize, Reflect)]
#[reflect(Resource)]
#[serde(default)]
pub struct ClientSettings {
    #[reflect(ignore)]
    pub serverlist: Vec<ServerListItem>,

    pub fov: f32,
    pub username: String,
    pub hud_padding: f32,
    pub vsync: bool,

    pub chunks_load_distance: IVec2,
}

impl ClientSettings {
    pub(crate) fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(self.fov.is_finite() && (10.0..=170.0).contains(&self.fov), "FOV must be in 10..=170");
        anyhow::ensure!(self.hud_padding.is_finite() && (0.0..=48.0).contains(&self.hud_padding), "HUD padding must be in 0..=48");
        anyhow::ensure!(crate::net::valid_username(&self.username), "Username must contain 1..=64 UTF-8 bytes, non-whitespace text, and no control characters");
        anyhow::ensure!(crate::net::valid_load_distance(self.chunks_load_distance), "Chunk distance must be XZ 0..=16, Y 0..=8, or (-1, -1) to disable loading");
        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_ranges_and_matches_network_distance_rules() {
        let mut cfg = ClientSettings::default();
        cfg.validate().unwrap();
        for distance in [IVec2::ZERO, IVec2::new(16, 8), IVec2::splat(-1)] {
            cfg.chunks_load_distance = distance;
            cfg.validate().unwrap();
        }
        for distance in [IVec2::new(-1, 0), IVec2::new(0, -1), IVec2::new(17, 8), IVec2::new(16, 9)] {
            cfg.chunks_load_distance = distance;
            assert!(cfg.validate().is_err());
        }
        cfg.chunks_load_distance = IVec2::ZERO;
        for fov in [f32::NAN, f32::INFINITY, 9.0, 171.0] {
            cfg.fov = fov;
            assert!(cfg.validate().is_err());
        }
        cfg.fov = 85.0;
        for padding in [f32::NAN, f32::INFINITY, -1.0, 49.0] {
            cfg.hud_padding = padding;
            assert!(cfg.validate().is_err());
        }
        cfg.hud_padding = 24.0;
        for name in [" ".to_string(), "a\nb".to_string(), "名".repeat(22)] {
            cfg.username = name;
            assert!(cfg.validate().is_err());
        }
    }

    #[test]
    fn load_failure_preserves_original_file_and_missing_file_can_be_saved() {
        let dir = std::env::temp_dir().join(format!("ethertia-client-settings-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("settings.json");
        let (defaults, status) = load_settings(&path);
        assert!(status.load_error.is_none());
        save_settings(&path, &defaults, &status).unwrap();
        for original in ["{", r#"{"chunks_load_distance":[-1,0]}"#] {
            std::fs::write(&path, original).unwrap();
            let (fallback, status) = load_settings(&path);
            assert!(status.load_error.is_some());
            fallback.validate().unwrap();
            assert!(save_settings(&path, &fallback, &status).is_err());
            assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        }
        assert!(save_settings(&dir, &defaults, &ClientSettingsStatus::default()).is_err());
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
