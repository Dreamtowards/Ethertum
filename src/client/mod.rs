pub mod character_controller;
pub mod game_client;
pub mod ui;

mod input;
mod settings;
mod client_world;

pub mod prelude {
    use super::*;
    pub use settings::{ClientSettings, ServerListItem};
    pub use client_world::{WorldInfo, DespawnOnWorldUnload, ClientPlayerInfo};
    pub use game_client::{condition, ClientGamePlugin, ClientInfo, EthertiaClient};
    pub use input::InputAction;
    pub use ui::{CurrentUI, UiExtra};
    pub use character_controller::{CharacterController, CharacterControllerBundle, CharacterControllerCamera, CharacterControllerPlugin};

    pub use crate::item::{Inventory, ItemStack};
}

#[cfg(feature = "target_native_os")]
pub mod editor;
