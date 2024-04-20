use bevy::prelude::*;

fn main() {
    #[cfg(feature = "target_native_os")]
    {
        let frame_time = std::time::Duration::from_secs_f32(1.0 / 30.0);

        App::new()
            .add_plugins(
                MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(frame_time)), // fixed fps
            )
            .add_plugins(bevy::log::LogPlugin::default())
            .add_plugins(ethertia::server::prelude::ServerGamePlugin)
            .run();
    }
}
