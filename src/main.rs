// #![rustfmt::skip]

use bevy::prelude::*;
use ethertia::util::vtx::VertexBuffer;

fn main() {
    // std::env::set_var("RUST_BACKTRACE", "full");
    // std::env::set_var("RUST_LOG", "info");

    // let mut vbuf = VertexBuffer::default();
    // ethertia::voxel::meshgen::put_grass(&mut vbuf, Vec3::X * 0.0, 1);
    // ethertia::voxel::meshgen::put_leaves(&mut vbuf, Vec3::X * 5.0, 1);
    // ethertia::voxel::meshgen::put_face(&mut vbuf, 1, Vec3::X * 10.0, Quat::IDENTITY, Vec2::ONE);
    // std::fs::write("tmp.obj", vbuf.export_obj().unwrap()).unwrap();
    // return;

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: bevy::window::WindowResolution::new(1280., 720.),
                title: format!("Ethertia {}", ethertia::VERSION_NAME),
                prevent_default_event_handling: true, // web: avoid twice esc to pause problem.
                ..default()
            }),
            ..default()
        }))
        .add_plugins(ethertia::client::prelude::ClientGamePlugin)
        .run();
}
