
// Client
pub mod game_client;
pub mod ui;
pub mod character_controller;

#[cfg(feature = "target_native_os")]
pub mod editor;

pub mod util;
pub mod voxel;
pub mod net;
pub mod item;

pub mod game_server;


use crossbeam_channel as channel_impl;