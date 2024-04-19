// Client
pub mod character_controller;
pub mod game_client;
pub mod ui;

#[cfg(feature = "target_native_os")]
pub mod editor;

// Common
pub mod item;
pub mod net;
pub mod util;
pub mod voxel;

// Server
pub mod game_server;

// Util
use crossbeam_channel as channel_impl;

pub static VERSION: &str = std::env!("CARGO_PKG_VERSION");
pub static VERSION_NAME: &str = concat!(std::env!("CARGO_PKG_VERSION"), " 2024.03c5");

pub mod wfc;