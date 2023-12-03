
use bevy::{prelude::*, utils::HashMap};
use bevy_atmosphere::prelude::*;


pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {

        // Atmosphere
        app.insert_resource(AtmosphereModel::default());
        app.add_plugins(AtmospherePlugin);
        

        app.insert_resource(CycleTimer(Timer::new(
                bevy::utils::Duration::from_millis(50), // Update our atmosphere every 50ms (in a real game, this would be much slower, but for the sake of an example we use a faster update)
                TimerMode::Repeating,
            )));
        app.add_systems(Startup, startup);
        app.add_systems(Update, daylight_cycle);
        

    }
}


mod chunk;
use chunk::Chunk;


struct World {

    // ChunkSystem
    chunks: HashMap<IVec3, Chunk>,

    worldinfo: WorldInfo,

    is_paused: bool,
}


struct WorldInfo {
    
    seed: u64,

    name: String,

    daytime: f32,

    // seconds a day time long
    daytime_length: f32,  

    // seconds
    time_inhabited: f32,

    time_created: u64,
    time_modified: u64,
}








#[derive(Component)]
struct Sun;

// Timer for updating the daylight cycle (updating the atmosphere every frame is slow, so it's better to do incremental changes)
#[derive(Resource)]
struct CycleTimer(Timer);


// Simple environment
fn startup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(5., 0., 5.),
            ..default()
        },
        AtmosphereCamera::default(), // Marks camera as having a skybox, by default it doesn't specify the render layers the skybox can be seen on
    ));

    // Sun
    commands.spawn((
        DirectionalLightBundle {
            ..Default::default()
        },
        Sun, // Marks the light as Sun
    ));

    // Simple transform shape just for reference
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(StandardMaterial::from(Color::rgb(0.8, 0.8, 0.8))),
        ..Default::default()
    });

    commands.spawn(SceneBundle {
        scene: assets.load("spaceship.glb#Scene0"),
        transform: Transform::from_xyz(0., 0., -10.),
        ..default()
    });

}



// We can edit the Atmosphere resource and it will be updated automatically
fn daylight_cycle(
    mut atmosphere: AtmosphereMut<Nishita>,
    mut query: Query<(&mut Transform, &mut DirectionalLight), With<Sun>>,
    mut timer: ResMut<CycleTimer>,
    time: Res<Time>,
) {
    timer.0.tick(time.delta());

    if timer.0.finished() {
        let t = time.elapsed_seconds_wrapped() / 22.0;
        atmosphere.sun_position = Vec3::new(0., t.sin(), t.cos());

        if let Some((mut light_trans, mut directional)) = query.single_mut().into() {
            light_trans.rotation = Quat::from_rotation_x(-t);
            directional.illuminance = t.sin().max(0.0).powf(2.0) * 100000.0;
        }
    }
}
