// Client
pub mod client;
pub use client::ui;

// Server
pub mod server;

// Common
pub mod item;
pub mod net;
pub mod util;
pub mod voxel;

// Util
use crossbeam_channel as channel_impl;

pub static VERSION: &str = std::env!("CARGO_PKG_VERSION");
pub static VERSION_NAME: &str = concat!(std::env!("CARGO_PKG_VERSION"), " 2024.03c5");


pub mod prelude {
    pub use bevy::prelude::*;
    pub use serde::{Serialize, Deserialize};
}
