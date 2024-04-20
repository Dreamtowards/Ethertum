pub mod dedicated_server;

mod integrated_server;

pub mod prelude {
    pub use super::dedicated_server::{DedicatedServerPlugin, PlayerInfo, ServerInfo, ServerSettings};
    pub use super::integrated_server::IntegratedServerPlugin;
}
