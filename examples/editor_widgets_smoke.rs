//! Real renderer smoke: restored egui title, opt-in gallery, widgets and cleanup.
//! cargo run --example editor_widgets_smoke --locked --offline
use bevy::{
    app::AppExit,
    input::{
        mouse::{MouseButtonInput, MouseScrollUnit, MouseWheel},
        ButtonState,
    },
    prelude::*,
    render::view::screenshot::{Screenshot, ScreenshotCaptured},
    window::{CursorMoved, PrimaryWindow, WindowEvent},
    winit::WinitSettings,
};
use bevy_egui::{egui, EguiFullOutput, EguiInput, EguiPostUpdateSet, PrimaryEguiContext};
use ethertia::{client::prelude::*, server::prelude::ServerSettings, ui::CurrentUI};

#[derive(Resource, Default)]
struct Smoke {
    phase: u8,
    next_at: f64,
    screenshots: usize,
    click_position: Option<egui::Pos2>,
}
#[derive(Resource, Default)]
struct Labels(std::collections::BTreeMap<String, egui::Pos2>);

fn main() -> AppExit {
    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/editor-widgets-smoke")
        .join(std::process::id().to_string());
    std::fs::create_dir_all(&runtime).unwrap();
    std::env::set_current_dir(&runtime).unwrap();
    let port = std::net::UdpSocket::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: concat!(env!("CARGO_MANIFEST_DIR"), "/assets").into(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Ethertia UI smoke".into(),
                    resolution: (1280, 720).into(),
                    visible: false,
                    ..default()
                }),
                ..default()
            }),
    )
    .add_plugins(ClientGamePlugin)
    .insert_resource(bevy::audio::GlobalVolume::new(bevy::audio::Volume::Linear(0.0)))
    .insert_resource(WinitSettings::game())
    .init_resource::<Smoke>()
    .init_resource::<Labels>()
    .add_systems(
        PostUpdate,
        capture_labels.after(EguiPostUpdateSet::EndPass).before(EguiPostUpdateSet::ProcessOutput),
    )
    .add_systems(Last, step);
    app.world_mut().resource_mut::<ServerSettings>().port = port;
    app.run()
}

fn collect_labels(shape: &egui::epaint::Shape, labels: &mut Labels) {
    match shape {
        egui::epaint::Shape::Text(text) => {
            labels
                .0
                .insert(text.galley.job.text.clone(), text.pos + text.galley.rect.center().to_vec2());
        }
        egui::epaint::Shape::Vec(shapes) => {
            for shape in shapes {
                collect_labels(shape, labels);
            }
        }
        _ => {}
    }
}
fn capture_labels(outputs: Query<&EguiFullOutput, With<PrimaryEguiContext>>, mut labels: ResMut<Labels>) {
    labels.0.clear();
    for output in &outputs {
        if let Some(output) = &output.0 {
            for shape in &output.shapes {
                collect_labels(&shape.shape, &mut labels);
            }
        }
    }
}
fn screenshot(commands: &mut Commands, name: &'static str) {
    commands
        .spawn(Screenshot::primary_window())
        .observe(move |capture: On<ScreenshotCaptured>, mut smoke: ResMut<Smoke>| {
            capture.image.clone().try_into_dynamic().unwrap().save(name).unwrap();
            smoke.screenshots += 1;
        });
}
fn egui_pointer(input: &mut EguiInput, position: egui::Pos2, pressed: bool) {
    input.events.push(egui::Event::PointerMoved(position));
    input.events.push(egui::Event::PointerButton {
        pos: position,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    });
}

fn step(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut smoke: ResMut<Smoke>,
    labels: Res<Labels>,
    mut cli: ResMut<ClientInfo>,
    mut window: Single<(Entity, &mut Window), With<PrimaryWindow>>,
    mut egui_input: Single<&mut EguiInput, With<PrimaryEguiContext>>,
    nodes: Query<(&Name, &ComputedNode, &UiGlobalTransform)>,
    texts: Query<&Text>,
    controls: Query<(&Name, Has<bevy::ui::InteractionDisabled>, Has<bevy::ui::Checked>)>,
    sliders: Query<(&Name, &bevy::ui_widgets::SliderValue)>,
    scrolls: Query<(&Name, &ScrollPosition)>,
    scale: Res<UiScale>,
    focus: Res<bevy::input_focus::InputFocus>,
    mut window_events: MessageWriter<WindowEvent>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs_f64();
    if now > 90.0 {
        error!("UI smoke timed out at phase {}", smoke.phase);
        exit.write(AppExit::error());
        return;
    }
    if now < smoke.next_at {
        return;
    }
    let count = |name: &str| nodes.iter().filter(|(n, _, _)| n.as_str() == name).count();
    match smoke.phase {
        0 => {
            if now < 3.0 || !labels.0.contains_key("Singleplayer") {
                return;
            }
            assert_eq!(cli.curr_ui, CurrentUI::MainMenu);
            assert_eq!(count("Widget gallery"), 0);
            screenshot(&mut commands, "egui-title.png");
            smoke.next_at = now + 0.5;
        }
        1 | 4 | 6 | 27 | 29 | 32 | 34 => {
            let label = match smoke.phase {
                1 => "Singleplayer",
                4 | 27 | 32 => "View",
                _ => "Bevy UI / Feathers widgets",
            };
            let Some(&position) = labels.0.get(label) else {
                return;
            };
            smoke.click_position = Some(position);
            egui_pointer(&mut egui_input, position, true);
            smoke.next_at = now + 0.2;
        }
        2 | 5 | 7 | 28 | 30 | 33 | 35 => {
            egui_pointer(&mut egui_input, smoke.click_position.unwrap(), false);
            smoke.next_at = now + 0.3;
        }
        3 => {
            assert_eq!(cli.curr_ui, CurrentUI::LocalWorldList);
            cli.curr_ui = CurrentUI::MainMenu;
            smoke.next_at = now + 0.5;
        }
        8 => {
            assert_eq!(cli.curr_ui, CurrentUI::WidgetGallery);
            if count("Gallery normal") != 1 {
                return;
            }
            assert_eq!(count("Widget gallery"), 1);
            screenshot(&mut commands, "gallery.png");
            smoke.next_at = now + 0.5;
        }
        9 | 13 | 17 | 23 | 36 => {
            let name = match smoke.phase {
                9 => "Gallery normal",
                13 => "Gallery checkbox",
                17 => "Gallery slider",
                23 => "Fields & containers",
                _ => "Close widget gallery",
            };
            let (_, node, transform) = nodes.iter().find(|(n, _, _)| n.as_str() == name).unwrap();
            let mut position = transform.translation;
            if smoke.phase == 17 {
                position.x -= node.size().x * 0.3;
            }
            if smoke.phase == 23 {
                position.y -= node.size().y * 0.5 - 10.0;
            }
            window_events.write(WindowEvent::CursorMoved(CursorMoved {
                window: window.0,
                position: position / window.1.scale_factor(),
                delta: None,
            }));
            smoke.next_at = now + 0.2;
        }
        10 | 14 | 18 | 37 => {
            window_events.write(WindowEvent::MouseButtonInput(MouseButtonInput {
                button: MouseButton::Left,
                state: ButtonState::Pressed,
                window: window.0,
            }));
        }
        11 | 15 | 20 | 38 => {
            window_events.write(WindowEvent::MouseButtonInput(MouseButtonInput {
                button: MouseButton::Left,
                state: ButtonState::Released,
                window: window.0,
            }));
            smoke.next_at = now + 0.2;
        }
        12 => {
            assert!(texts.iter().any(|text| text.0 == "Clicks: 1"));
        }
        16 => {
            assert!(!controls.iter().find(|(n, _, _)| n.as_str() == "Gallery disabled").unwrap().1);
            assert!(!controls.iter().find(|(n, _, _)| n.as_str() == "Gallery checkbox").unwrap().2);
        }
        19 => {
            let (_, node, transform) = nodes.iter().find(|(n, _, _)| n.as_str() == "Gallery slider").unwrap();
            window_events.write(WindowEvent::CursorMoved(CursorMoved {
                window: window.0,
                position: (transform.translation + Vec2::new(node.size().x * 0.3, 0.0)) / window.1.scale_factor(),
                delta: None,
            }));
            smoke.next_at = now + 0.2;
        }
        21 => {
            assert!(sliders.iter().find(|(n, _)| n.as_str() == "Gallery slider").unwrap().1 .0 > 20.0);
            window.1.resolution.set(640.0, 360.0);
            smoke.next_at = now + 0.5;
        }
        22 => {
            screenshot(&mut commands, "gallery-small.png");
            smoke.next_at = now + 0.5;
        }
        24 => {
            window_events.write(WindowEvent::MouseWheel(MouseWheel {
                window: window.0,
                phase: bevy::input::touch::TouchPhase::Moved,
                unit: MouseScrollUnit::Line,
                x: 0.0,
                y: -10.0,
            }));
            smoke.next_at = now + 0.2;
        }
        25 => {
            assert!(scrolls.iter().find(|(n, _)| n.as_str() == "Fields & containers").unwrap().1.y > 0.0);
            window.1.resolution.set(1280.0, 720.0);
            smoke.next_at = now + 0.5;
        }
        26 => {
            screenshot(&mut commands, "gallery-interacted.png");
            smoke.next_at = now + 0.5;
        }
        31 => {
            assert_eq!(cli.curr_ui, CurrentUI::MainMenu);
            assert_eq!(count("Widget gallery"), 0);
            assert_eq!(scale.0, 1.0);
            assert!(focus.get().is_none());
            assert!(labels.0.contains_key("Singleplayer"));
            screenshot(&mut commands, "egui-restored.png");
            smoke.next_at = now + 0.5;
        }
        39 => {
            assert_eq!(cli.curr_ui, CurrentUI::MainMenu);
            assert_eq!(count("Widget gallery"), 0);
            assert_eq!(scale.0, 1.0);
            assert!(focus.get().is_none());
            assert_eq!(smoke.screenshots, 5);
            info!("UI smoke passed: egui title and routing, View checkbox open/close/reopen, native close, widgets, resize, scroll and cleanup");
            exit.write(AppExit::Success);
        }
        _ => unreachable!(),
    }
    smoke.phase += 1;
}
