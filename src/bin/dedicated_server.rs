use bevy::{log::LogPlugin, prelude::*};

fn main() {
    #[cfg(feature = "target_native_os")]
    {
        App::new()
            .add_plugins(
                MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(std::time::Duration::from_secs_f32(1.0 / 30.0)))  // fixed fps
            )
            .add_plugins(LogPlugin::default())
            .add_plugins(ethertia::game_server::GameServerPlugin)
            .run();
    }
}
