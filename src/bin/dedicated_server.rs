use bevy::{log::LogPlugin, prelude::*};

use ethertia::game_server::GameServerPlugin;

fn main() {
    App::new()
        .add_plugins(
            MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(std::time::Duration::from_secs_f32(1.0 / 30.0)))  // fixed fps
        )
        .add_plugins(LogPlugin::default())
        .add_plugins(GameServerPlugin)
        .run();
}
