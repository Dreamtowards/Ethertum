pub mod character_controller;
pub mod game_client;
pub mod ui;

mod input;
mod settings;

pub mod prelude {
    use super::*;
    pub use settings::{ClientSettings, ServerListItem};
    pub use game_client::{condition, ClientGamePlugin, ClientInfo, DespawnOnWorldUnload, EthertiaClient, WorldInfo};
    pub use input::InputAction;
    pub use crate::item::{Inventory, ItemStack};

    pub use character_controller::{CharacterController, CharacterControllerBundle, CharacterControllerCamera, CharacterControllerPlugin};
}

#[cfg(feature = "target_native_os")]
pub mod editor;
