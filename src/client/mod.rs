pub mod character_controller;
pub mod game_client;
pub mod ui;

mod client_world;
mod input;
mod settings;

pub mod prelude {
    use super::*;
    pub use character_controller::{CharacterController, CharacterControllerBundle, CharacterControllerCamera, CharacterControllerPlugin};
    pub use client_world::{ClientPlayerInfo, DespawnOnWorldUnload, WorldInfo};
    pub use game_client::{condition, ClientGamePlugin, ClientInfo, EthertiaClient};
    pub use input::InputAction;
    pub use settings::{ClientSettings, ServerListItem};
    pub use ui::{CurrentUI, UiExtra};

    pub use crate::item::{Inventory, ItemStack};
}

#[cfg(feature = "target_native_os")]
pub mod editor;
