#[cfg(feature = "mesh")]
pub mod mesh;
#[cfg(feature = "scene")]
pub mod scene;

mod util;

use bevy::app::{App, Plugin};
use bevy::asset::AssetApp;
use serde::{Deserialize, Serialize};

const EXTENSIONS: &[&str; 2] = &["obj", "OBJ"];

/// Adds support for OBJ asset loading
#[derive(Default)]
pub struct ObjPlugin;

impl Plugin for ObjPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "mesh")]
        app.init_asset_loader::<mesh::ObjLoader>();
        #[cfg(feature = "scene")]
        app.init_asset_loader::<scene::ObjLoader>();
    }
}

/// OBJ asset loader settings
#[derive(Default, Serialize, Deserialize)]
pub struct ObjSettings {
    /// Force compute the normals even if the mesh contains normals
    pub force_compute_normals: bool,
    /// Prefer flat normals over smooth normals when computing them
    pub prefer_flat_normals: bool,
}
