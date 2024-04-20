

pub mod game_client;
pub mod character_controller;
pub mod ui;


#[cfg(feature = "target_native_os")]
pub mod editor;


pub mod prelude {
    use super::*;
    pub use game_client::{ClientInfo, condition, ClientSettings, GameClientPlugin, EthertiaClient, ServerListItem};
    
    pub use character_controller::{
        CharacterController, 
        CharacterControllerCamera, 
        CharacterControllerPlugin, 
        CharacterControllerBundle
    };
}