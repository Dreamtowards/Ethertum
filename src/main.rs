
use bevy::prelude::*;

use bevy_atmosphere::prelude::*;

use bevy_editor_pls::prelude::*;



fn main() {
    App::new()
        .insert_resource(AtmosphereModel::default())
        .insert_resource(CycleTimer(Timer::new(
            bevy::utils::Duration::from_millis(50), // Update our atmosphere every 50ms (in a real game, this would be much slower, but for the sake of an example we use a faster update)
            TimerMode::Repeating,
        )))
        .add_plugins((DefaultPlugins, AtmospherePlugin, EditorPlugin::default()))
        .add_systems(Startup, setup_environment)
        // .add_systems(Update, change_gradient)
        .add_systems(Update, change_nishita)
        .add_systems(Update, file_drop)
        // .add_systems(Update, daylight_cycle)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((Camera3dBundle::default(), AtmosphereCamera::default()));
}


// Marker for updating the position of the light, not needed unless we have multiple lights
#[derive(Component)]
struct Sun;

// Timer for updating the daylight cycle (updating the atmosphere every frame is slow, so it's better to do incremental changes)
#[derive(Resource)]
struct CycleTimer(Timer);


fn file_drop(
    mut dnd_evr: EventReader<FileDragAndDrop>,
) {
    for ev in dnd_evr.read() {
        println!("{:?}", ev);
        
        if let FileDragAndDrop::DroppedFile { window, path_buf } = ev {
            println!("Dropped file with path: {:?}, in window id: {:?}", path_buf, window);
        }
    }
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

fn change_gradient(mut commands: Commands, keys: Res<Input<KeyCode>>) {
    if keys.just_pressed(KeyCode::Key1) {
        info!("Changed to Atmosphere Preset 1 (Default Gradient)");
        commands.insert_resource(AtmosphereModel::new(Gradient::default()));
    } else if keys.just_pressed(KeyCode::Key2) {
        info!("Changed to Atmosphere Preset 2 (Cotton Candy)");
        commands.insert_resource(AtmosphereModel::new(Gradient {
            ground: Color::rgb(1.0, 0.5, 0.75),
            horizon: Color::WHITE,
            sky: Color::rgb(0.5, 0.75, 1.0),
        }));
    } else if keys.just_pressed(KeyCode::Key3) {
        info!("Changed to Atmosphere Preset 3 (80's Sunset)");
        commands.insert_resource(AtmosphereModel::new(Gradient {
            sky: Color::PURPLE,
            horizon: Color::PINK,
            ground: Color::ORANGE,
        }));
    } else if keys.just_pressed(KeyCode::Key4) {
        info!("Changed to Atmosphere Preset 4 (Winter)");
        commands.insert_resource(AtmosphereModel::new(Gradient {
            ground: Color::rgb(0.0, 0.1, 0.2),
            horizon: Color::rgb(0.3, 0.4, 0.5),
            sky: Color::rgb(0.7, 0.8, 0.9),
        }));
    } else if keys.just_pressed(KeyCode::Key5) {
        info!("Changed to Atmosphere Preset 5 (Nether)");
        commands.insert_resource(AtmosphereModel::new(Gradient {
            ground: Color::BLACK,
            horizon: Color::rgb(0.2, 0.0, 0.0),
            sky: Color::rgb(0.5, 0.1, 0.0),
        }));
    } else if keys.just_pressed(KeyCode::Key6) {
        info!("Changed to Atmosphere Preset 6 (Golden)");
        commands.insert_resource(AtmosphereModel::new(Gradient {
            ground: Color::ORANGE_RED,
            horizon: Color::ORANGE,
            sky: Color::GOLD,
        }));
    } else if keys.just_pressed(KeyCode::Key7) {
        info!("Changed to Atmosphere Preset 7 (Noir)");
        commands.insert_resource(AtmosphereModel::new(Gradient {
            ground: Color::BLACK,
            horizon: Color::BLACK,
            sky: Color::WHITE,
        }));
    } else if keys.just_pressed(KeyCode::Key8) {
        info!("Changed to Atmosphere Preset 8 (Midnight)");
        commands.insert_resource(AtmosphereModel::new(Gradient {
            ground: Color::BLACK,
            horizon: Color::BLACK,
            sky: Color::MIDNIGHT_BLUE,
        }));
    } else if keys.just_pressed(KeyCode::Key9) {
        info!("Changed to Atmosphere Preset 9 (Greenery)");
        commands.insert_resource(AtmosphereModel::new(Gradient {
            ground: Color::rgb(0.1, 0.2, 0.0),
            horizon: Color::rgb(0.3, 0.4, 0.1),
            sky: Color::rgb(0.6, 0.8, 0.2),
        }));
    } else if keys.just_pressed(KeyCode::Key0) {
        info!("Reset Atmosphere to Default");
        commands.remove_resource::<AtmosphereModel>();
    }
}


fn change_nishita(mut commands: Commands, keys: Res<Input<KeyCode>>) {
    if keys.just_pressed(KeyCode::Key1) {
        info!("Changed to Atmosphere Preset 1 (Sunset)");
        commands.insert_resource(AtmosphereModel::new(Nishita {
            sun_position: Vec3::new(0., 0., -1.),
            ..default()
        }));
    } else if keys.just_pressed(KeyCode::Key2) {
        info!("Changed to Atmosphere Preset 2 (Noir Sunset)");
        commands.insert_resource(AtmosphereModel::new(Nishita {
            sun_position: Vec3::new(0., 0., -1.),
            rayleigh_coefficient: Vec3::new(1e-5, 1e-5, 1e-5),
            ..default()
        }));
    } else if keys.just_pressed(KeyCode::Key3) {
        info!("Changed to Atmosphere Preset 3 (Magenta)");
        commands.insert_resource(AtmosphereModel::new(Nishita {
            rayleigh_coefficient: Vec3::new(2e-5, 1e-5, 2e-5),
            ..default()
        }));
    } else if keys.just_pressed(KeyCode::Key4) {
        info!("Changed to Atmosphere Preset 4 (Strong Mie)");
        commands.insert_resource(AtmosphereModel::new(Nishita {
            mie_coefficient: 5e-5,
            ..default()
        }));
    } else if keys.just_pressed(KeyCode::Key5) {
        info!("Changed to Atmosphere Preset 5 (Larger Scale)");
        commands.insert_resource(AtmosphereModel::new(Nishita {
            rayleigh_scale_height: 16e3,
            mie_scale_height: 2.4e3,
            ..default()
        }));
    } else if keys.just_pressed(KeyCode::Key6) {
        info!("Changed to Atmosphere Preset 6 (Weak Intensity)");
        commands.insert_resource(AtmosphereModel::new(Nishita {
            sun_intensity: 11.0,
            ..default()
        }));
    } else if keys.just_pressed(KeyCode::Key7) {
        info!("Changed to Atmosphere Preset 7 (Half Radius)");
        commands.insert_resource(AtmosphereModel::new(Nishita {
            ray_origin: Vec3::new(0., 6372e3 / 2., 0.),
            planet_radius: 6371e3 / 2.,
            atmosphere_radius: 6471e3 / 2.,
            ..default()
        }));
    } else if keys.just_pressed(KeyCode::Key8) {
        info!("Changed to Atmosphere Preset 8 (Sideways World)");
        commands.insert_resource(AtmosphereModel::new(Nishita {
            ray_origin: Vec3::new(6372e3, 0., 0.),
            ..default()
        }));
    } else if keys.just_pressed(KeyCode::Key9) {
        info!("Changed to Atmosphere Preset 9 (Inverted Mie Direction)");
        commands.insert_resource(AtmosphereModel::new(Nishita {
            mie_direction: -0.758,
            ..default()
        }));
    } else if keys.just_pressed(KeyCode::Key0) {
        info!("Reset Atmosphere to Default");
        commands.remove_resource::<AtmosphereModel>();
    }
}

// Simple environment
fn setup_environment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Our Sun
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

    // Spawn our camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(5., 0., 5.),
            ..default()
        },
        AtmosphereCamera::default(), // Marks camera as having a skybox, by default it doesn't specify the render layers the skybox can be seen on
    ));
}