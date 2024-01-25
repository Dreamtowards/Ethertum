
use bevy::{prelude::*, log::LogPlugin};

use ethertia::net::NetworkClientPlugin;

fn main() {

    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(LogPlugin::default())
        .add_plugins(NetworkClientPlugin)
        .run();
}