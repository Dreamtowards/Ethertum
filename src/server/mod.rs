pub mod game_server;

pub mod integrated_server;

pub mod prelude {
    pub use super::game_server::{PlayerInfo, ServerGamePlugin, ServerInfo, ServerSettings};
}
