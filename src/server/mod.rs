pub mod dedicated_server;

mod integrated_server;

pub mod prelude {
    pub use super::dedicated_server::{PlayerInfo, DedicatedServerPlugin, ServerInfo, ServerSettings};
    pub use super::integrated_server::IntegratedServerPlugin;
}
