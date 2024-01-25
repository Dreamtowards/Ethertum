
use std::time::Duration;

use bevy::{prelude::*, log::LogPlugin, app::ScheduleRunnerPlugin};

use ethertia::net::NetworkServerPlugin;


fn main() {
    App::new()
        .add_plugins(MinimalPlugins
            // .set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f32(1.0 / 30.0)))  // fixed fps
        )
        .add_plugins(LogPlugin::default())
        .add_plugins(NetworkServerPlugin)
        .run();
}