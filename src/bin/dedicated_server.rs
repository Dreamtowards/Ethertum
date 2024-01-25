
use bevy::prelude::*;

use ethertia::net::NetworkServerPlugin;


fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(NetworkServerPlugin)
        .run();
}