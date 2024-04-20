pub mod character_controller;
pub mod game_client;
pub mod ui;

#[cfg(feature = "target_native_os")]
pub mod editor;

pub mod prelude {
    use super::*;
    pub use game_client::{condition, ClientInfo, ClientSettings, EthertiaClient, GameClientPlugin, ServerListItem};

    pub use character_controller::{CharacterController, CharacterControllerBundle, CharacterControllerCamera, CharacterControllerPlugin};
}
